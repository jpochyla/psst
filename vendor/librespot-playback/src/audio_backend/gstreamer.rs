use std::sync::{Arc, Mutex};

use gstreamer::{
    State,
    event::{FlushStart, FlushStop},
    prelude::*,
};

use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_audio as gst_audio;

const GSTREAMER_ASYNC_ERROR_POISON_MSG: &str = "gstreamer async error mutex should not be poisoned";

use super::{Open, Sink, SinkAsBytes, SinkError, SinkResult};

use crate::{
    NUM_CHANNELS, SAMPLE_RATE, config::AudioFormat, convert::Converter, decoder::AudioPacket,
};

pub struct GstreamerSink {
    appsrc: gst_app::AppSrc,
    bufferpool: gst::BufferPool,
    pipeline: gst::Pipeline,
    format: AudioFormat,
    async_error: Arc<Mutex<Option<String>>>,
}

impl Open for GstreamerSink {
    fn open(device: Option<String>, format: AudioFormat) -> Self {
        info!("Using GStreamer sink with format: {format:?}");
        gst::init().expect("failed to init GStreamer!");

        let gst_format = match format {
            AudioFormat::F64 => gst_audio::AUDIO_FORMAT_F64,
            AudioFormat::F32 => gst_audio::AUDIO_FORMAT_F32,
            AudioFormat::S32 => gst_audio::AUDIO_FORMAT_S32,
            AudioFormat::S24 => gst_audio::AUDIO_FORMAT_S2432,
            AudioFormat::S24_3 => gst_audio::AUDIO_FORMAT_S24,
            AudioFormat::S16 => gst_audio::AUDIO_FORMAT_S16,
        };

        let gst_info = gst_audio::AudioInfo::builder(gst_format, SAMPLE_RATE, NUM_CHANNELS as u32)
            .build()
            .expect("Failed to create GStreamer audio format");
        let gst_caps = gst_info.to_caps().expect("Failed to create GStreamer caps");

        let sample_size = format.size();
        let gst_bytes = NUM_CHANNELS as usize * 2048 * sample_size;

        let pipeline = gst::Pipeline::new();
        let appsrc = gst::ElementFactory::make("appsrc")
            .build()
            .expect("Failed to create GStreamer appsrc element")
            .downcast::<gst_app::AppSrc>()
            .expect("couldn't cast AppSrc element at runtime!");
        appsrc.set_caps(Some(&gst_caps));
        appsrc.set_max_bytes(gst_bytes as u64);
        appsrc.set_block(true);

        let sink = match device {
            None => {
                // no need to dither twice; use librespot dithering instead
                gst::parse::bin_from_description(
                    "audioconvert dithering=none ! audioresample ! autoaudiosink",
                    true,
                )
                .expect("Failed to create default GStreamer sink")
            }
            Some(ref x) => gst::parse::bin_from_description(x, true)
                .expect("Failed to create custom GStreamer sink"),
        };
        pipeline
            .add(&appsrc)
            .expect("Failed to add GStreamer appsrc to pipeline");
        pipeline
            .add(&sink)
            .expect("Failed to add GStreamer sink to pipeline");
        appsrc
            .link(&sink)
            .expect("Failed to link GStreamer source to sink");

        let bus = pipeline.bus().expect("couldn't get bus from pipeline");

        let bufferpool = gst::BufferPool::new();

        let mut conf = bufferpool.config();
        conf.set_params(Some(&gst_caps), gst_bytes as u32, 0, 0);
        bufferpool
            .set_config(conf)
            .expect("couldn't configure the buffer pool");

        let async_error = Arc::new(Mutex::new(None));
        let async_error_clone = async_error.clone();

        bus.set_sync_handler(move |_bus, msg| {
            match msg.view() {
                gst::MessageView::Eos(_) => {
                    println!("gst signaled end of stream");

                    let mut async_error_storage = async_error_clone
                        .lock()
                        .expect(GSTREAMER_ASYNC_ERROR_POISON_MSG);
                    *async_error_storage = Some(String::from("gst signaled end of stream"));
                }
                gst::MessageView::Error(err) => {
                    println!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );

                    let mut async_error_storage = async_error_clone
                        .lock()
                        .expect(GSTREAMER_ASYNC_ERROR_POISON_MSG);
                    *async_error_storage = Some(format!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    ));
                }
                _ => (),
            }

            gst::BusSyncReply::Drop
        });

        pipeline
            .set_state(State::Ready)
            .expect("unable to set the pipeline to the `Ready` state");

        Self {
            appsrc,
            bufferpool,
            pipeline,
            format,
            async_error,
        }
    }
}

impl Sink for GstreamerSink {
    fn start(&mut self) -> SinkResult<()> {
        *self
            .async_error
            .lock()
            .expect(GSTREAMER_ASYNC_ERROR_POISON_MSG) = None;
        self.appsrc.send_event(FlushStop::new(true));
        self.bufferpool
            .set_active(true)
            .map_err(|e| SinkError::StateChange(e.to_string()))?;
        self.pipeline
            .set_state(State::Playing)
            .map_err(|e| SinkError::StateChange(e.to_string()))?;
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        *self
            .async_error
            .lock()
            .expect(GSTREAMER_ASYNC_ERROR_POISON_MSG) = None;
        self.appsrc.send_event(FlushStart::new());
        self.pipeline
            .set_state(State::Paused)
            .map_err(|e| SinkError::StateChange(e.to_string()))?;
        self.bufferpool
            .set_active(false)
            .map_err(|e| SinkError::StateChange(e.to_string()))?;
        Ok(())
    }

    sink_as_bytes!();
}

impl Drop for GstreamerSink {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(State::Null);
    }
}

impl SinkAsBytes for GstreamerSink {
    #[inline]
    fn write_bytes(&mut self, data: &[u8]) -> SinkResult<()> {
        if let Some(async_error) = &*self
            .async_error
            .lock()
            .expect(GSTREAMER_ASYNC_ERROR_POISON_MSG)
        {
            return Err(SinkError::OnWrite(async_error.to_string()));
        }

        let mut buffer = self
            .bufferpool
            .acquire_buffer(None)
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;

        let mutbuf = buffer.make_mut();
        mutbuf.set_size(data.len());
        mutbuf
            .copy_from_slice(0, data)
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;

        self.appsrc
            .push_buffer(buffer)
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;

        Ok(())
    }
}

impl GstreamerSink {
    pub const NAME: &'static str = "gstreamer";
}
