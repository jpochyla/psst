use super::{Open, Sink, SinkError, SinkResult};
use crate::config::AudioFormat;
use crate::convert::Converter;
use crate::decoder::AudioPacket;
use crate::{NUM_CHANNELS, SAMPLE_RATE};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use std::thread;
use std::time::Duration;

pub enum SdlSink {
    F32(AudioQueue<f32>),
    S32(AudioQueue<i32>),
    S16(AudioQueue<i16>),
}

impl Open for SdlSink {
    fn open(device: Option<String>, format: AudioFormat) -> Self {
        info!("Using SDL sink with format: {:?}", format);

        if device.is_some() {
            warn!("SDL sink does not support specifying a device name");
        }

        let ctx = sdl2::init().expect("could not initialize SDL");
        let audio = ctx
            .audio()
            .expect("could not initialize SDL audio subsystem");

        let desired_spec = AudioSpecDesired {
            freq: Some(SAMPLE_RATE as i32),
            channels: Some(NUM_CHANNELS),
            samples: None,
        };

        macro_rules! open_sink {
            ($sink: expr, $type: ty) => {{
                let queue: AudioQueue<$type> = audio
                    .open_queue(None, &desired_spec)
                    .expect("could not open SDL audio device");
                $sink(queue)
            }};
        }
        match format {
            AudioFormat::F32 => open_sink!(Self::F32, f32),
            AudioFormat::S32 => open_sink!(Self::S32, i32),
            AudioFormat::S16 => open_sink!(Self::S16, i16),
            _ => {
                unimplemented!("SDL currently does not support {format:?} output")
            }
        }
    }
}

impl Sink for SdlSink {
    fn start(&mut self) -> SinkResult<()> {
        macro_rules! start_sink {
            ($queue: expr) => {{
                $queue.clear();
                $queue.resume();
            }};
        }
        match self {
            Self::F32(queue) => start_sink!(queue),
            Self::S32(queue) => start_sink!(queue),
            Self::S16(queue) => start_sink!(queue),
        };
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        macro_rules! stop_sink {
            ($queue: expr) => {{
                $queue.pause();
                $queue.clear();
            }};
        }
        match self {
            Self::F32(queue) => stop_sink!(queue),
            Self::S32(queue) => stop_sink!(queue),
            Self::S16(queue) => stop_sink!(queue),
        };
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        macro_rules! drain_sink {
            ($queue: expr, $size: expr) => {{
                // sleep and wait for sdl thread to drain the queue a bit
                while $queue.size() > (NUM_CHANNELS as u32 * $size as u32 * SAMPLE_RATE) {
                    thread::sleep(Duration::from_millis(10));
                }
            }};
        }

        let samples = packet
            .samples()
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let result = match self {
            Self::F32(queue) => {
                let samples_f32: &[f32] = &converter.f64_to_f32(samples);
                drain_sink!(queue, AudioFormat::F32.size());
                queue.queue_audio(samples_f32)
            }
            Self::S32(queue) => {
                let samples_s32: &[i32] = &converter.f64_to_s32(samples);
                drain_sink!(queue, AudioFormat::S32.size());
                queue.queue_audio(samples_s32)
            }
            Self::S16(queue) => {
                let samples_s16: &[i16] = &converter.f64_to_s16(samples);
                drain_sink!(queue, AudioFormat::S16.size());
                queue.queue_audio(samples_s16)
            }
        };
        result.map_err(SinkError::OnWrite)
    }
}

impl SdlSink {
    pub const NAME: &'static str = "sdl";
}
