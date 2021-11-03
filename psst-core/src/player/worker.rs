use std::time::Duration;

use crossbeam_channel::Sender;
use symphonia::core::audio::{SampleBuffer, SignalSpec};

use crate::{
    actor::{Actor, ActorHandle, ActorOp},
    audio::{
        decode::AudioDecoder,
        output::AudioSink,
        resample::{AudioResampler, ResamplingQuality, ResamplingSpec},
    },
    error::Error,
};

use super::{file::AudioFile, LoadedPlaybackItem, PlayerEvent, VolumeLevel};

pub struct DecodingWorker {
    pub actor: ActorHandle<Decode>,
}

impl Drop for DecodingWorker {
    fn drop(&mut self) {
        let _ = self.actor.send(Decode::Quit);
    }
}

const REPORT_POSITION_EACH: Duration = Duration::from_millis(1000);

pub enum Decode {
    Start,
    Stop,
    Seek(Duration),
    ReadPacket,
    FlushPacket,
    Quit,
}

enum DecState {
    Started,
    Stopped,
}

pub struct Decoding {
    file: AudioFile,
    source: AudioDecoder,
    norm_factor: f32,
    resampler: AudioResampler,
    samples: SampleBuffer<f32>,
    events: Sender<PlayerEvent>,
    this: Sender<Decode>,
    sink: AudioSink<f32>,
    volume: VolumeLevel,
    state: DecState,
    last_reported_position: Duration,
}

impl Decoding {
    pub fn new(
        loaded: LoadedPlaybackItem,
        events: Sender<PlayerEvent>,
        this: Sender<Decode>,
        sink: AudioSink<f32>,
        volume: VolumeLevel,
    ) -> Self {
        let LoadedPlaybackItem {
            file,
            source,
            norm_factor,
        } = loaded;
        let resampler = AudioResampler::new(
            // TODO: Make the quality configurable.
            ResamplingQuality::SincMediumQuality,
            ResamplingSpec {
                channels: source.channels().unwrap().count(),
                from_rate: source.sample_rate().unwrap() as usize,
                to_rate: sink.sample_rate() as usize,
            },
            1024 * 8,
        )
        .unwrap();
        let samples = {
            let max_frames = source.max_frames_per_packet().unwrap_or(1024 * 8);
            let channels = source.channels().unwrap();
            let rate = source.sample_rate().unwrap();
            SampleBuffer::new(max_frames, SignalSpec { rate, channels })
        };
        Self {
            file,
            source,
            norm_factor,
            resampler,
            samples,
            events,
            this,
            sink,
            volume,
            state: DecState::Stopped,
            last_reported_position: Duration::ZERO,
        }
    }

    fn frames_to_duration(&self, frames: u64) -> Duration {
        Duration::from_secs_f64(frames as f64 / self.source.sample_rate().unwrap() as f64)
    }

    fn report_position(&mut self, position: Duration) {
        if self
            .events
            .try_send(PlayerEvent::Position {
                path: self.file.path(),
                position,
            })
            .is_ok()
        {
            self.last_reported_position = position;
        }
    }

    fn report_current_position(&mut self) {
        let position = self.frames_to_duration(self.source.current_frame());
        self.report_position(position);
    }

    fn report_current_position_if_neeeded(&mut self) {
        let position = self.frames_to_duration(self.source.current_frame());
        if position.saturating_sub(self.last_reported_position) > REPORT_POSITION_EACH {
            self.report_position(position);
        }
    }

    fn is_started(&self) -> bool {
        matches!(self.state, DecState::Started)
    }
}

impl Actor for Decoding {
    type Message = Decode;
    type Error = Error;

    fn handle(&mut self, msg: Self::Message) -> Result<ActorOp, Self::Error> {
        match msg {
            Decode::Start if !self.is_started() => {
                self.this.send(Decode::ReadPacket)?;
                self.state = DecState::Started;
                Ok(ActorOp::Continue)
            }
            Decode::Stop if self.is_started() => {
                self.state = DecState::Stopped;
                Ok(ActorOp::Continue)
            }
            Decode::Seek(pos) => self.handle_seek(pos),
            Decode::ReadPacket => self.handle_read_packet(),
            Decode::FlushPacket => self.handle_flush_packet(),
            Decode::Quit => Ok(ActorOp::Shutdown),
            _ => Ok(ActorOp::Continue),
        }
    }
}

impl Decoding {
    fn handle_seek(&mut self, position: Duration) -> Result<ActorOp, Error> {
        if let Err(err) = self.source.seek(position) {
            log::error!("failed to seek: {}", err);
        } else {
            self.report_current_position();
        }
        Ok(ActorOp::Continue)
    }

    fn handle_read_packet(&mut self) -> Result<ActorOp, Error> {
        if self.is_started() {
            if let Some(packet) = self.source.next_packet() {
                self.samples.copy_interleaved_ref(packet);
                self.report_current_position_if_neeeded();
                self.this.send(Decode::FlushPacket)?;
            } else {
                self.events.send(PlayerEvent::EndOfTrack)?;
                return Ok(ActorOp::Shutdown);
            }
        }
        Ok(ActorOp::Continue)
    }

    fn handle_flush_packet(&mut self) -> Result<ActorOp, Error> {
        let samples = self.samples.samples();

        // Resample the sample buffer into a rate that the audio output supports.
        let resampled = self.resampler.resample(samples)?;

        // Apply the global volume level and the normalization factor.
        let factor = self.norm_factor * self.volume.get();
        for sample in resampled.iter_mut() {
            *sample *= factor;
        }

        // Write into the sink, block until all samples are committed to the ring
        // buffer.
        self.sink.write_blocking(resampled)?;

        if self.is_started() {
            self.this.send(Decode::ReadPacket)?;
        }
        Ok(ActorOp::Continue)
    }
}
