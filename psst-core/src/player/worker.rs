use std::{
    ops::Range,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use crossbeam_channel::Sender;
use rb::{Consumer, Producer, RbConsumer, RbProducer, SpscRb, RB};
use symphonia::core::{
    audio::{SampleBuffer, SignalSpec},
    units::TimeBase,
};

use crate::{
    actor::{Act, Actor, ActorHandle},
    audio::{
        decode::AudioDecoder,
        output::AudioSink,
        resample::ResamplingQuality,
        source::{AudioSource, ResampledSource, StereoMappedSource},
    },
    error::Error,
};

use super::{
    file::{MediaFile, MediaPath},
    LoadedPlaybackItem, PlayerEvent,
};

pub struct PlaybackManager {
    sink: AudioSink,
    event_send: Sender<PlayerEvent>,
    current: Option<(MediaPath, Sender<Msg>)>,
}

impl PlaybackManager {
    pub fn new(sink: AudioSink, event_send: Sender<PlayerEvent>) -> Self {
        Self {
            sink,
            event_send,
            current: None,
        }
    }

    pub fn play(&mut self, loaded: LoadedPlaybackItem) {
        let path = loaded.file.path();
        let source = DecoderSource::new(
            loaded.file,
            loaded.source,
            loaded.norm_factor,
            self.event_send.clone(),
        );
        self.current = Some((path, source.actor.sender()));
        // Some output streams have different sample rate than the source, so we need to
        // resample before pushing to the sink.
        let source = ResampledSource::new(
            source,
            self.sink.sample_rate(),
            ResamplingQuality::SincMediumQuality,
        );
        // Source output streams also have a different channel count. Map the stereo
        // channels and silence the others.
        let source = StereoMappedSource::new(source, self.sink.channel_count());
        self.sink.play(source);
        self.sink.resume();
    }

    pub fn seek(&self, position: Duration) {
        if let Some((path, worker)) = &self.current {
            let _ = worker.send(Msg::Seek(position));

            // Because the position events are sent in the `DecoderSource`, doing this here
            // is slightly hacky. The alternative would be propagating `event_send` into the
            // worker.
            let _ = self.event_send.send(PlayerEvent::Position {
                path: path.to_owned(),
                position,
            });
        }
    }
}

pub struct DecoderSource {
    file: MediaFile,
    actor: ActorHandle<Msg>,
    consumer: Consumer<f32>,
    event_send: Sender<PlayerEvent>,
    total_samples: Arc<AtomicU64>,
    position: Arc<AtomicU64>,
    precision: u64,
    reported: u64,
    end_of_track: bool,
    norm_factor: f32,
    signal_spec: SignalSpec,
    time_base: TimeBase,
}

impl DecoderSource {
    pub fn new(
        file: MediaFile,
        decoder: AudioDecoder,
        norm_factor: f32,
        event_send: Sender<PlayerEvent>,
    ) -> Self {
        const REPORT_PRECISION: Duration = Duration::from_millis(900);

        // Gather the source signal parameters and compute how often we should report
        // the play-head position.
        let signal_spec = decoder.signal_spec();
        let time_base = decoder.codec_params().time_base.unwrap();
        let precision = (signal_spec.rate as f64
            * signal_spec.channels.count() as f64
            * REPORT_PRECISION.as_secs_f64()) as u64;

        // Create a ring-buffer for the decoded samples.  Worker thread is producing,
        // we are consuming in the `AudioSource` impl.
        let buffer = Worker::default_buffer();
        let consumer = buffer.consumer();

        // We keep track of the current play-head position by sharing an atomic sample
        // counter with the decoding worker.  Worker is setting this on seek, we are
        // incrementing on reading from the ring-buffer.
        let position = Arc::new(AtomicU64::new(0));

        // Because the `n_frames` count that Symphonia gives us can be a bit unreliable,
        // we track the total number of samples in this stream in this atomic, set when
        // the underlying decoder returns EOF.
        let total_samples = Arc::new(AtomicU64::new(u64::MAX));

        // Spawn the worker and kick-start the decoding.  The buffer will start filling
        // now.
        let actor = Worker::spawn_with_default_cap("audio_decoding", {
            let position = Arc::clone(&position);
            let total_samples = Arc::clone(&total_samples);
            move |this| Worker::new(this, decoder, buffer, position, total_samples)
        });
        let _ = actor.send(Msg::Read);

        Self {
            file,
            actor,
            consumer,
            event_send,
            norm_factor,
            signal_spec,
            time_base,
            total_samples,
            end_of_track: false,
            position,
            precision,
            reported: u64::MAX, // Something sufficiently distinct from any position.
        }
    }

    fn written_samples(&self, position: u64) -> u64 {
        self.position.fetch_add(position, Ordering::Relaxed) + position
    }

    fn should_report(&self, pos: u64) -> bool {
        self.reported > pos || pos - self.reported >= self.precision
    }

    fn samples_to_duration(&self, samples: u64) -> Duration {
        let frames = samples / self.signal_spec.channels.count() as u64;
        let time = self.time_base.calc_time(frames);
        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
    }
}

impl AudioSource for DecoderSource {
    fn write(&mut self, output: &mut [f32]) -> usize {
        if self.end_of_track {
            return 0;
        }
        let written = self.consumer.read(output).unwrap_or(0);

        // Apply the normalization factor.
        output[..written]
            .iter_mut()
            .for_each(|s| *s *= self.norm_factor);

        let position = self.written_samples(written as u64);
        if self.should_report(position) {
            // Send a position report, so the upper layers can visualize the playback
            // progress and preload the next track.  We cannot block here, so if the channel
            // is full, we just try the next time instead of waiting.
            if self
                .event_send
                .try_send(PlayerEvent::Position {
                    path: self.file.path(),
                    position: self.samples_to_duration(position),
                })
                .is_ok()
            {
                self.reported = position;
            }
        }

        let total_samples = self.total_samples.load(Ordering::Relaxed);
        if position >= total_samples {
            // After reading the total number of samples, we stop. Signal to the upper layer
            // this track is over and short-circuit all further reads from this source.
            if self.event_send.try_send(PlayerEvent::EndOfTrack).is_ok() {
                self.end_of_track = true;
            }
            log::debug!(
                "end of track, position: {}, total: {}",
                position,
                total_samples
            );
        }

        written
    }

    fn channel_count(&self) -> usize {
        self.signal_spec.channels.count()
    }

    fn sample_rate(&self) -> u32 {
        self.signal_spec.rate
    }
}

impl Drop for DecoderSource {
    fn drop(&mut self) {
        let _ = self.actor.send(Msg::Stop);
    }
}

enum Msg {
    Seek(Duration),
    Read,
    Stop,
}

struct Worker {
    /// Sending part of our own actor channel.
    this: Sender<Msg>,
    /// Decoder we are reading packets/samples from.
    input: AudioDecoder,
    /// Audio properties of the decoded signal.
    input_spec: SignalSpec,
    /// Sample buffer containing samples read in the last packet.
    input_packet: SampleBuffer<f32>,
    /// Ring-buffer for the output signal.
    output: SpscRb<f32>,
    /// Producing part of the output ring-buffer.
    output_producer: Producer<f32>,
    /// Shared atomic position.  We update this on seek only.
    position: Arc<AtomicU64>,
    /// Shared atomic for total number of samples.  We set this on EOF.
    total_samples: Arc<AtomicU64>,
    /// Range of samples in `resampled` that are awaiting flush into `output`.
    samples_to_write: Range<usize>,
    /// Number of samples written into the output channel.
    samples_written: u64,
    /// Are we in the middle of automatic read loop?
    is_reading: bool,
}

impl Worker {
    fn default_buffer() -> SpscRb<f32> {
        const DEFAULT_BUFFER_SIZE: usize = 128 * 1024;

        SpscRb::new(DEFAULT_BUFFER_SIZE)
    }

    fn new(
        this: Sender<Msg>,
        input: AudioDecoder,
        output: SpscRb<f32>,
        position: Arc<AtomicU64>,
        total_samples: Arc<AtomicU64>,
    ) -> Self {
        const DEFAULT_MAX_FRAMES: u64 = 8 * 1024;

        let max_input_frames = input
            .codec_params()
            .max_frames_per_packet
            .unwrap_or(DEFAULT_MAX_FRAMES);

        Self {
            output_producer: output.producer(),
            input_packet: SampleBuffer::new(max_input_frames, input.signal_spec()),
            input_spec: input.signal_spec(),
            input,
            this,
            output,
            position,
            total_samples,
            samples_written: 0,
            samples_to_write: 0..0, // Arbitrary empty range.
            is_reading: false,
        }
    }
}

impl Actor for Worker {
    type Message = Msg;
    type Error = Error;

    fn handle(&mut self, msg: Msg) -> Result<Act<Self>, Self::Error> {
        match msg {
            Msg::Seek(time) => self.on_seek(time),
            Msg::Read => self.on_read(),
            Msg::Stop => Ok(Act::Shutdown),
        }
    }
}

impl Worker {
    fn on_seek(&mut self, time: Duration) -> Result<Act<Self>, Error> {
        match self.input.seek(time) {
            Ok(timestamp) => {
                if self.is_reading {
                    self.samples_to_write = 0..0;
                } else {
                    self.this.send(Msg::Read)?;
                }
                let position = timestamp * self.input_spec.channels.count() as u64;
                self.samples_written = position;
                self.position.store(position, Ordering::Relaxed);
                self.output.clear();
            }
            Err(err) => {
                log::error!("failed to seek: {}", err);
            }
        }
        Ok(Act::Continue)
    }

    fn on_read(&mut self) -> Result<Act<Self>, Error> {
        if !self.samples_to_write.is_empty() {
            let writable = &self.input_packet.samples()[self.samples_to_write.clone()];
            if let Ok(written) = self.output_producer.write(writable) {
                self.samples_written += written as u64;
                self.samples_to_write.start += written;
                self.is_reading = true;
                self.this.send(Msg::Read)?;
                Ok(Act::Continue)
            } else {
                // Buffer is full.  Wait a bit a try again.  We also have to indicate that the
                // read loop is not running at the moment (if we receive a `Seek` while waiting,
                // we need it to explicitly kickstart reading again).
                self.is_reading = false;
                Ok(Act::WaitOr {
                    timeout: Duration::from_millis(500),
                    timeout_msg: Msg::Read,
                })
            }
        } else {
            match self.input.next_packet() {
                Some((_, packet)) => {
                    self.input_packet.copy_interleaved_ref(packet);
                    self.samples_to_write = 0..self.input_packet.samples().len();
                    self.is_reading = true;
                    self.this.send(Msg::Read)?;
                }
                None => {
                    self.is_reading = false;
                    self.total_samples
                        .store(self.samples_written, Ordering::Relaxed);
                }
            }
            Ok(Act::Continue)
        }
    }
}
