use std::{
    io::Cursor,
    sync::{Arc, RwLock},
    thread::JoinHandle,
};

use log::{debug, error, info};
use pipewire::{
    context::Context,
    keys::{
        APP_ICON_NAME, APP_ID, APP_NAME, AUDIO_CHANNELS, MEDIA_CATEGORY, MEDIA_NAME, MEDIA_ROLE,
        MEDIA_TYPE, NODE_NAME,
    },
    main_loop::MainLoop,
    properties::properties,
    spa::{
        param::audio::{AudioFormat, AudioInfoRaw, MAX_CHANNELS},
        pod::{
            serialize::{GenError, PodSerializer},
            Object, Pod, Value,
        },
        sys::{
            SPA_PARAM_EnumFormat, SPA_PROP_channelVolumes, SPA_TYPE_OBJECT_Format,
            SPA_AUDIO_CHANNEL_FL, SPA_AUDIO_CHANNEL_FR,
        },
        utils::Direction,
    },
    stream::{Stream, StreamFlags, StreamState},
    sys::pw_stream_control,
};
use symphonia::core::audio;

use crate::{
    audio::{
        output::{AudioOutput, AudioSink},
        source::{AudioSource, Empty},
    },
    error::Error,
};

const DEFAULT_CHANNEL_COUNT: usize = 2;
const DEFAULT_SAMPLE_RATE: u32 = 44_100;

enum PipewireMsg {
    Open,
    Close,
    Play(Box<dyn AudioSource>),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    SetMediaTitle(String),
}

pub struct PipewireOutput {
    mainloop_handle: JoinHandle<Result<(), Error>>,
    sink: PipewireSink,
}

impl PipewireOutput {
    pub fn open() -> Result<Self, Error> {
        info!("opening audio output: pipewire");
        pipewire::init();
        let (mainloop_send, mainloop_recv) = pipewire::channel::channel::<PipewireMsg>();
        let mainloop_handle = std::thread::spawn(move || Self::run(mainloop_recv));
        let sink = PipewireSink {
            channel_count: DEFAULT_CHANNEL_COUNT,
            sample_rate: DEFAULT_SAMPLE_RATE,
            mainloop_send,
        };
        Ok(Self {
            mainloop_handle,
            sink,
        })
    }

    fn run(mainloop_recv: pipewire::channel::Receiver<PipewireMsg>) -> Result<(), Error> {
        let audio_source = Arc::new(RwLock::new(Box::new(Empty) as Box<dyn AudioSource>));
        let audio_is_playing = Arc::new(RwLock::new(false));
        let audio_volume = Arc::new(RwLock::new(0f32));
        let mainloop = MainLoop::new(None)?;
        let context = Context::new(&mainloop)?;
        let core = context.connect(Some(properties! {
            *APP_NAME => "Psst",
            *APP_ID => "music.player.psst",
            *APP_ICON_NAME => "Psst"
        }))?;
        let registry = core.get_registry()?;

        let stream = Stream::new(
            &core,
            "psst",
            properties! {
                *MEDIA_TYPE => "Audio",
                *MEDIA_CATEGORY => "Playback",
                *MEDIA_ROLE => "Music",
                // *MEDIA_NAME => "artist - title",
                *AUDIO_CHANNELS => "2",
                *NODE_NAME => "Psst",
                *APP_NAME => "Psst",
                *APP_ID => "music.player.psst",
                *APP_ICON_NAME => "Psst",
            },
        )?;

        // let _core_listener = core
        //     .add_listener_local()
        //     .info(|_| {})
        //     .done({
        //         let mainloop = mainloop.clone();
        //         move |id, seq| {
        //             info!("Core sync done for ID: {} seq: {}", id, seq.seq());
        //             if id == PW_ID_CORE {
        //                 mainloop.quit();
        //             }
        //         }
        //     })
        //     .register();

        let listener = stream
            .add_local_listener::<()>()
            .state_changed({
                move |_stream, _userdata, _old, new| {
                    debug!("State changed: {_old:?} -> {new:?}");
                    match new {
                        StreamState::Error(x) => {
                            error!("stream error: {x}");
                        }
                        StreamState::Unconnected => {
                            debug!("stream unconnected");
                        }
                        StreamState::Connecting => {
                            debug!("stream connecting");
                        }
                        _ => {}
                    }
                }
            })
            .control_info({
                let audio_volume = audio_volume.clone();
                move |_stream, _userdata, id, control_ptr: *const pw_stream_control| {
                    debug!("control info: id: {id}");
                    if id == SPA_PROP_channelVolumes {
                        debug!("volume set from pipewire control: {:?}", control_ptr);
                        // TODO: Call player controller to update volume on AppState
                        unsafe {
                            let control = *control_ptr;
                            if control.n_values > 0 {
                                let values = std::slice::from_raw_parts(
                                    control.values,
                                    control.n_values as usize,
                                );
                                // Ideally we could set volume per channel, but here we are overwriting with the first channel
                                let mut volume =
                                    audio_volume.write().expect("failed to lock audio_volume");
                                for v in values.iter() {
                                    if *v != volume.clone() {
                                        *volume = *v;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .process({
                let audio_source = audio_source.clone();
                let audio_is_playing = audio_is_playing.clone();
                let audio_volume = audio_volume.clone();
                move |stream_ref, _| {
                    let is_playing = audio_is_playing
                        .read()
                        .expect("failed to lock audio_is_playing")
                        .clone();
                    let volume = audio_volume
                        .read()
                        .expect("failed to lock audio_volume")
                        .clone();
                    // Why not two channels?
                    // let stride = size_of::<f32>() * DEFAULT_CHANNEL_COUNT;
                    let stride = size_of::<f32>() * 1;
                    while let Some(mut buffer) = stream_ref.dequeue_buffer() {
                        for data in buffer.datas_mut() {
                            let mut written = 0;

                            if let Some(d) = data.data() {
                                let n_samples = d.len() / stride;
                                let ptr = d.as_mut_ptr() as *mut f32;
                                let slice =
                                    unsafe { std::slice::from_raw_parts_mut(ptr, n_samples) };
                                if is_playing {
                                    written = audio_source
                                        .write()
                                        .expect("failed to lock audio source")
                                        .write(slice);

                                    // Let pipewire handle the volume scaling.
                                    // let scaled_volume = volume.powf(4.0);
                                    // slice[..written]
                                    //     .iter_mut()
                                    //     .for_each(|s| *s *= scaled_volume);

                                    // Mute any remaining samples.
                                    slice[written..].iter_mut().for_each(|s| *s = 0.0);
                                }
                            } else {
                                error!("Buffer data is null or not writable");
                                continue;
                            }

                            let chunk = data.chunk_mut();
                            *chunk.offset_mut() = 0;
                            *chunk.stride_mut() = stride as _;
                            *chunk.size_mut() = (stride * written) as _;
                        }
                    }
                }
            })
            .register()?;

        core.sync(0)?;

        let mut positions = [0; MAX_CHANNELS];
        positions[0] = SPA_AUDIO_CHANNEL_FL;
        positions[1] = SPA_AUDIO_CHANNEL_FR;

        let mut audio_info = AudioInfoRaw::new();
        audio_info.set_rate(DEFAULT_SAMPLE_RATE);
        audio_info.set_format(AudioFormat::F32LE);
        audio_info.set_channels(DEFAULT_CHANNEL_COUNT as u32);
        audio_info.set_position(positions);

        let pod_raw = PodSerializer::serialize(
            Cursor::new(Vec::new()),
            &Value::Object(Object {
                type_: SPA_TYPE_OBJECT_Format,
                id: SPA_PARAM_EnumFormat,
                properties: audio_info.into(),
            }),
        )
        .map(|data| data.0.into_inner())?;

        let mut params = [Pod::from_bytes(&pod_raw).expect("failed to create pod")];

        stream.connect(
            Direction::Output,
            None,
            StreamFlags::AUTOCONNECT | StreamFlags::RT_PROCESS | StreamFlags::MAP_BUFFERS,
            &mut params,
        )?;

        let _receiver = mainloop_recv.attach(mainloop.as_ref(), {
            let mainloop = mainloop.clone();
            let audio_source = audio_source.clone();
            let is_playing = audio_is_playing.clone();
            let audio_volume = audio_volume.clone();
            move |msg| {
                match msg {
                    PipewireMsg::Open => {
                        debug!("PipewireMsg::Open");
                        stream.set_active(true).expect("failed to activate stream");
                    }
                    PipewireMsg::Close => {
                        debug!("PipewireMsg::Close");
                        let mut new_is_playing =
                            is_playing.write().expect("failed to lock is_playing");
                        *new_is_playing = false;
                        stream
                            .set_active(false)
                            .expect("failed to deactivate stream");
                        mainloop.quit();
                    }
                    PipewireMsg::Play(source) => {
                        debug!("PipewireMsg::Play");
                        debug!(
                            "PipewireMsg::Play: channel_count: {:?}",
                            source.channel_count()
                        );
                        debug!("PipewireMsg::Play: sample_rate: {:?}", source.sample_rate());

                        let mut new_source =
                            audio_source.write().expect("failed to lock audio source");
                        *new_source = source;
                        let mut new_is_playing =
                            is_playing.write().expect("failed to lock is_playing");
                        *new_is_playing = true;
                        stream.set_active(true).expect("failed to activate stream");
                    }
                    PipewireMsg::Pause => {
                        debug!("PipewireMsg::Pause");
                        let mut new_is_playing =
                            is_playing.write().expect("failed to lock is_playing");
                        *new_is_playing = false;
                        stream
                            .set_active(false)
                            .expect("failed to deactivate stream");
                    }
                    PipewireMsg::Resume => {
                        debug!("PipewireMsg::Resume");
                        let mut new_is_playing =
                            is_playing.write().expect("failed to lock is_playing");
                        *new_is_playing = true;
                        stream.set_active(true).expect("failed to activate stream");
                    }
                    PipewireMsg::Stop => {
                        debug!("PipewireMsg::Stop");
                        let mut new_is_playing =
                            is_playing.write().expect("failed to lock is_playing");
                        *new_is_playing = false;
                        stream
                            .set_active(false)
                            .expect("failed to deactivate stream");
                    }
                    PipewireMsg::SetVolume(volume) => {
                        debug!("PipewireMsg::SetVolume: {}", volume);
                        let values = [volume];
                        stream
                            .set_control(SPA_PROP_channelVolumes, &values)
                            .expect("failed to set volume");
                        let mut new_audio_volume =
                            audio_volume.write().expect("failed to lock volume");
                        *new_audio_volume = volume;
                    }
                    PipewireMsg::SetMediaTitle(title) => {
                        debug!("PipewireMsg::SetMediaTitle: {}", title);
                        let props = properties! {
                            *MEDIA_NAME => title.clone(),
                        };
                        unsafe {
                            pipewire::sys::pw_stream_update_properties(
                                stream.as_raw_ptr(),
                                props.dict().as_raw_ptr(),
                            );
                        }
                    }
                };
            }
        });

        info!("mainloop starting");
        mainloop.run();
        info!("mainloop stopped");

        Ok(())
    }
}

impl AudioOutput for PipewireOutput {
    type Sink = PipewireSink;

    fn sink(&self) -> Self::Sink {
        self.sink.clone()
    }
}

#[derive(Clone)]
pub struct PipewireSink {
    channel_count: usize,
    sample_rate: u32,
    mainloop_send: pipewire::channel::Sender<PipewireMsg>,
}

impl PipewireSink {
    fn send(&self, msg: PipewireMsg) {
        if self.mainloop_send.send(msg).is_err() {
            error!("output stream actor is dead");
        }
    }
}

impl AudioSink for PipewireSink {
    fn channel_count(&self) -> usize {
        self.channel_count
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn set_volume(&self, volume: f32) {
        self.send(PipewireMsg::SetVolume(volume));
    }

    fn play(&self, source: impl AudioSource) {
        self.send(PipewireMsg::Play(Box::new(source)));
    }

    fn pause(&self) {
        self.send(PipewireMsg::Pause);
    }

    fn resume(&self) {
        self.send(PipewireMsg::Resume);
    }

    fn stop(&self) {
        self.send(PipewireMsg::Stop);
    }

    fn close(&self) {
        self.send(PipewireMsg::Close);
    }
}

impl From<GenError> for Error {
    fn from(err: GenError) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}

impl From<pipewire::Error> for Error {
    fn from(err: pipewire::Error) -> Error {
        Error::AudioOutputError(Box::new(err))
    }
}
