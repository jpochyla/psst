use std::{
    ops::Range,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use crossbeam_channel::Sender;
use rb::{Consumer, Producer, RbProducer, SpscRb, RB};
use symphonia::core::{
    audio::{SampleBuffer, SignalSpec},
    units::TimeBase,
};

use crate::{
    actor::{Act, Actor, ActorHandle},
    audio::{
        decode::AudioDecoder,
        output::AudioSink,
        resample::{AudioResampler, ResamplingQuality, ResamplingSpec},
        source::AudioSource,
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
        let source = DecoderSource::new(loaded, self.event_send.clone(), self.sink.sample_rate());
        self.current = Some((path, source.actor.sender()));
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
    position: Arc<AtomicU64>,
    precision: u64,
    reported: u64,
    total_samples: u64,
    end_of_track: bool,
    norm_factor: f32,
    time_base: TimeBase,
    signal_spec: SignalSpec,
}

impl DecoderSource {
    pub fn new(
        loaded: LoadedPlaybackItem,
        event_send: Sender<PlayerEvent>,
        output_sample_rate: u32,
    ) -> Self {
        let LoadedPlaybackItem {
            file,
            source,
            norm_factor,
        } = loaded;

        const REPORT_PRECISION: Duration = Duration::from_millis(1000);

        // Gather the source signal parameters and compute how often we should report
        // the play-head position.
        let signal_spec = source.signal_spec();
        let chan_count = signal_spec.channels.count();
        let cp = source.codec_params();
        let total_samples = cp.n_frames.unwrap() * chan_count as u64;
        let time_base = cp.time_base.unwrap();
        let precision =
            REPORT_PRECISION.as_millis() as u64 / (signal_spec.rate as u64 * chan_count as u64);

        // Create a ring-buffer for the decoded samples.  Worker thread is producing,
        // we are consuming in the `AudioSource` impl.
        let buffer = Worker::default_buffer();
        let consumer = buffer.consumer();

        // We keep track of the current play-head position by sharing an atomic sample
        // counter with the decoding worker.  Worker is setting this on seek, we are
        // incrementing on reading from the ring-buffer.
        let position = Arc::new(AtomicU64::new(0));

        // Some output streams have different sample rate than the source, so we need to
        // resample before pushing to the sink.
        let res_spec = ResamplingSpec {
            from_rate: signal_spec.rate as usize,
            to_rate: output_sample_rate as usize,
            channels: chan_count,
        };

        // Spawn the worker and kick-start the decoding.  The buffer will start filling
        // now.
        let actor = Worker::spawn_default({
            let position = Arc::clone(&position);
            move |this| Worker::new(this, source, buffer, position, res_spec)
        });
        let _ = actor.send(Msg::Read);

        Self {
            file,
            actor,
            consumer,
            event_send,
            norm_factor,
            time_base,
            signal_spec,
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
        let written = self.consumer.write(output);

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

        if position >= self.total_samples {
            // After reading the total number of samples from the `CodecParameters`, we
            // stop. Signal to the upper layer this track is over and short-circuit all
            // further reads from this source.
            if self.event_send.try_send(PlayerEvent::EndOfTrack).is_ok() {
                self.end_of_track = true;
            }
        }

        written
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
    /// Decoder we are reading packets/samples from.
    decoder: AudioDecoder,
    /// Audio properties of the decoded signal.
    spec: SignalSpec,
    /// Sending part of our own actor channel.
    this: Sender<Msg>,
    /// Ring-buffer for the output signal.
    output: SpscRb<f32>,
    /// Producing part of the output ring-buffer.
    producer: Producer<f32>,
    /// Sample buffer containing samples read in the last packet.
    packet: SampleBuffer<f32>,
    /// Buffer containing signal resampled from `packet`.
    resampled: Vec<f32>,
    /// Resampling structure we use to resample the signal.
    resampler: AudioResampler,
    /// Shared atomic position.  Worker updates this on seek only.
    position: Arc<AtomicU64>,
    /// Range of samples in `resampled` that are awaiting flush into `output`.
    samples_to_flush: Range<usize>,
    /// Are we in the middle of automatic read loop?
    is_reading: bool,
}

impl Worker {
    fn default_buffer() -> SpscRb<f32> {
        const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

        SpscRb::new(DEFAULT_BUFFER_SIZE)
    }

    fn new(
        this: Sender<Msg>,
        decoder: AudioDecoder,
        output: SpscRb<f32>,
        position: Arc<AtomicU64>,
        res_spec: ResamplingSpec,
    ) -> Self {
        const DEFAULT_MAX_FRAMES: u64 = 1024 * 8;

        let max_frames = decoder
            .codec_params()
            .max_frames_per_packet
            .unwrap_or(DEFAULT_MAX_FRAMES);
        let max_samples = max_frames as usize * res_spec.channels;
        let max_output_size = res_spec.max_output_size(max_samples);
        let resampled = vec![0.0; max_output_size];
        let resampler =
            AudioResampler::new(ResamplingQuality::SincMediumQuality, res_spec).unwrap();

        Self {
            producer: output.producer(),
            packet: SampleBuffer::new(max_frames, decoder.signal_spec()),
            spec: decoder.signal_spec(),
            resampled,
            resampler,
            decoder,
            this,
            output,
            position,
            samples_to_flush: 0..0, // Arbitrary empty range.
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
        match self.decoder.seek(time) {
            Ok(timestamp) => {
                if self.is_reading {
                    self.samples_to_flush = 0..0;
                } else {
                    self.this.send(Msg::Read)?;
                }
                let position = timestamp * self.spec.channels.count() as u64;
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
        if !self.samples_to_flush.is_empty() {
            let writable = &self.resampled[self.samples_to_flush.clone()];
            if let Ok(written) = self.producer.write(writable) {
                self.samples_to_flush.start += written;
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
            match self.decoder.next_packet() {
                Some((_, packet)) => {
                    self.packet.copy_interleaved_ref(packet);
                    let to_flush = self
                        .resampler
                        .resample(self.packet.samples(), &mut self.resampled)?;
                    self.samples_to_flush = 0..to_flush;
                    self.is_reading = true;
                    self.this.send(Msg::Read)?;
                }
                None => {
                    self.is_reading = false;
                }
            }
            Ok(Act::Continue)
        }
    }
}
