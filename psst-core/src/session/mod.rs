pub mod access_token;
pub mod audio_key;
pub mod mercury;

use std::{
    io,
    net::{Shutdown, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use quick_protobuf::MessageRead;
use serde::de::DeserializeOwned;

use crate::{
    audio::decrypt::AudioKey,
    connection::{
        shannon_codec::{ShannonDecoder, ShannonEncoder, ShannonMsg},
        Credentials, Transport,
    },
    error::Error,
    item_id::{FileId, ItemId},
    util::deserialize_protobuf,
};

use self::{
    audio_key::AudioKeyDispatcher,
    mercury::{MercuryDispatcher, MercuryRequest, MercuryResponse},
};

/// Configuration values needed to open the session connection.
#[derive(Clone)]
pub struct SessionConfig {
    pub login_creds: Credentials,
    pub proxy_url: Option<String>,
}

/// Cheap to clone, shareable service handle that holds the active session
/// worker.  Session connection is lazily opened in  `connected()`, using config
/// values set in `update_config()`.  In case the session dies or is explicitly
/// shut down, worker is disposed of, and a new session is opened on the next
/// request.
#[derive(Clone)]
pub struct SessionService {
    connected: Arc<Mutex<Option<SessionWorker>>>,
    config: Arc<Mutex<Option<SessionConfig>>>,
}

impl SessionService {
    /// Create a new session service without any configuration.  To open a
    /// session, a config needs to be set up first using `update_config`.
    pub fn empty() -> Self {
        Self {
            connected: Arc::default(),
            config: Arc::default(),
        }
    }

    /// Create a new session service with pre-set configuration.
    pub fn with_config(config: SessionConfig) -> Self {
        Self {
            connected: Arc::default(),
            config: Arc::new(Mutex::new(Some(config))),
        }
    }

    /// Replace the active session config.  If a session is already connected,
    /// shut it down and wait until it's terminated.
    pub fn update_config(&self, config: SessionConfig) {
        self.config.lock().replace(config);
        self.shutdown();
    }

    /// Returns true if a session worker is actively servicing the connected
    /// session.  We return false here after any case of I/O errors or an
    /// explicit session shutdown.
    pub fn is_connected(&self) -> bool {
        matches!(self.connected.lock().as_ref(), Some(worker) if !worker.has_terminated())
    }

    /// Return a handle for the connected session.  In case no connection is
    /// open, *synchronously* connect, start the worker and keep it as active.
    /// Although a lock is held for the whole duration  of connection setup,
    /// `SessionConnection::open` has an internal timeout, and should give up in
    /// a timely manner.
    pub fn connected(&self) -> Result<SessionHandle, Error> {
        let mut connected = self.connected.lock();
        let is_connected_and_not_terminated =
            matches!(connected.as_ref(), Some(worker) if !worker.has_terminated());
        if !is_connected_and_not_terminated {
            let connection = SessionConnection::open(
                self.config
                    .lock()
                    .as_ref()
                    .ok_or(Error::SessionDisconnected)?
                    .clone(),
            )?;
            let worker = SessionWorker::run(connection.transport);
            connected.replace(worker);
        }
        connected
            .as_ref()
            .map(SessionWorker::handle)
            .ok_or(Error::SessionDisconnected)
    }

    /// Signal a shutdown to the active worker and wait until it terminates.
    pub fn shutdown(&self) {
        if let Some(worker) = self.connected.lock().take() {
            worker.handle().request_shutdown();
            worker.join();
        }
    }
}

/// Successful connection through the Spotify Shannon-encrypted TCP channel.
pub struct SessionConnection {
    /// Credentials re-usable in the next authentication (i.e. username and
    /// password are not required anymore).
    pub credentials: Credentials,
    /// I/O codec for the Shannon messages.
    pub transport: Transport,
}

impl SessionConnection {
    /// Synchronously connect to the Spotify servers and authenticate with
    /// credentials provided in `config`.
    pub fn open(config: SessionConfig) -> Result<Self, Error> {
        // Connect to the server and exchange keys.
        let proxy_url = config.proxy_url.as_deref();
        let ap_url = Transport::resolve_ap_with_fallback(proxy_url);
        let mut transport = Transport::connect(&ap_url, proxy_url)?;
        // Authenticate with provided credentials (either username/password, or saved,
        // reusable credential blob from an earlier run).
        let credentials = transport.authenticate(config.login_creds)?;
        Ok(Self {
            credentials,
            transport,
        })
    }
}

pub struct SessionWorker {
    sender: Sender<DispatchCmd>,
    decoding_thread: JoinHandle<()>,
    encoding_thread: JoinHandle<()>,
    dispatching_thread: JoinHandle<()>,
    terminated: Arc<AtomicBool>,
}

impl SessionWorker {
    pub fn run(transport: Transport) -> Self {
        let (disp_send, disp_recv) = unbounded();
        let (msg_send, msg_recv) = unbounded();
        let terminated = Arc::new(AtomicBool::new(false));
        Self {
            decoding_thread: {
                let decoder = transport.decoder;
                let disp_send = disp_send.clone();
                thread::spawn(move || decode_shannon_messages(decoder, disp_send))
            },
            encoding_thread: {
                let encoder = transport.encoder;
                let disp_send = disp_send.clone();
                thread::spawn(move || encode_shannon_messages(encoder, msg_recv, disp_send))
            },
            dispatching_thread: {
                let stream = transport.stream;
                let terminated = terminated.clone();
                thread::spawn(move || {
                    dispatch_messages(disp_recv, msg_send, stream);
                    terminated.store(true, Ordering::SeqCst);
                })
            },
            sender: disp_send,
            terminated,
        }
    }

    pub fn handle(&self) -> SessionHandle {
        SessionHandle {
            sender: self.sender.clone(),
        }
    }

    pub fn join(self) {
        if let Err(err) = self.dispatching_thread.join() {
            log::error!("session dispatching thread panicked: {:?}", err);
        }
        if let Err(err) = self.encoding_thread.join() {
            log::error!("session encoding thread panicked: {:?}", err);
        }
        if let Err(err) = self.decoding_thread.join() {
            log::error!("session decoding thread panicked: {:?}", err);
        }
    }

    pub fn has_terminated(&self) -> bool {
        self.terminated.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
pub struct SessionHandle {
    sender: Sender<DispatchCmd>,
}

impl SessionHandle {
    pub fn get_mercury_protobuf<T>(&self, uri: String) -> Result<T, Error>
    where
        T: MessageRead<'static>,
    {
        let payload = self.get_mercury_bytes(uri)?;
        let message = deserialize_protobuf(&payload)?;
        Ok(message)
    }

    pub fn get_mercury_json<T>(&self, uri: String) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let payload = self.get_mercury_bytes(uri)?;
        let message = serde_json::from_slice(&payload)?;
        Ok(message)
    }

    pub fn get_mercury_bytes(&self, uri: String) -> Result<Vec<u8>, Error> {
        let (callback, receiver) = unbounded();
        let request = MercuryRequest::get(uri);
        self.sender
            .send(DispatchCmd::MercuryReq { callback, request })
            .ok()
            .ok_or(Error::SessionDisconnected)?;
        let response = receiver.recv().ok().ok_or(Error::SessionDisconnected)?;
        let first_part = response
            .payload
            .into_iter()
            .next()
            .ok_or(Error::UnexpectedResponse)?;
        Ok(first_part)
    }

    pub fn get_audio_key(&self, track: ItemId, file: FileId) -> Result<AudioKey, Error> {
        let (callback, receiver) = unbounded();
        self.sender
            .send(DispatchCmd::AudioKeyReq {
                callback,
                track,
                file,
            })
            .ok()
            .ok_or(Error::SessionDisconnected)?;
        receiver.recv().ok().ok_or(Error::SessionDisconnected)?
    }

    pub fn get_country_code(&self) -> Option<String> {
        let (callback, receiver) = unbounded();
        self.sender
            .send(DispatchCmd::CountryCodeReq { callback })
            .ok()?;
        receiver.recv().ok()?
    }

    pub fn request_shutdown(&self) {
        let _ = self.sender.send(DispatchCmd::Shutdown);
    }
}

/// Read Shannon messages from the TCP stream one by one and send them to
/// dispatcher for further processing.  In case the decoding fails with an error
/// (this happens also in case we explicitly shutdown the connection), report
/// the error to the dispatcher and quit.  If the dispatcher has already dropped
/// its receiving part, quit silently as well.
fn decode_shannon_messages(mut decoder: ShannonDecoder<TcpStream>, dispatch: Sender<DispatchCmd>) {
    loop {
        match decoder.decode() {
            Ok(msg) => {
                if dispatch.send(DispatchCmd::DecodedMsg(msg)).is_err() {
                    break;
                }
            }
            Err(err) => {
                let _ = dispatch.send(DispatchCmd::DecoderError(err));
                break;
            }
        };
    }
}

/// Receive Shannon messages from `messages` and encode them into the TCP stream
/// through `encoder`.  In case the encoding fails with an error (this happens
/// also in case we explicitly shutdown the connection), report the error to the
/// dispatcher and quit.  If the dispatcher has already dropped the
/// corresponding sender of `messages`, quit as well.
fn encode_shannon_messages(
    mut encoder: ShannonEncoder<TcpStream>,
    messages: Receiver<ShannonMsg>,
    dispatch: Sender<DispatchCmd>,
) {
    for msg in messages {
        match encoder.encode(msg) {
            Ok(_) => {
                // Message encoded, continue.
            }
            Err(err) => {
                let _ = dispatch.send(DispatchCmd::EncoderError(err));
                break;
            }
        }
    }
}

enum DispatchCmd {
    MercuryReq {
        request: MercuryRequest,
        callback: Sender<MercuryResponse>,
    },
    AudioKeyReq {
        track: ItemId,
        file: FileId,
        callback: Sender<Result<AudioKey, Error>>,
    },
    CountryCodeReq {
        callback: Sender<Option<String>>,
    },
    DecodedMsg(ShannonMsg),
    DecoderError(io::Error),
    EncoderError(io::Error),
    Shutdown,
}

fn dispatch_messages(
    dispatch: Receiver<DispatchCmd>,
    messages: Sender<ShannonMsg>,
    stream: TcpStream,
) {
    let mut mercury = MercuryDispatcher::new();
    let mut audio_key = AudioKeyDispatcher::new();
    let mut country_code = None;

    for disp in dispatch {
        match disp {
            DispatchCmd::MercuryReq { request, callback } => {
                let msg = mercury.enqueue_request(request, callback);
                let _ = messages.send(msg);
            }
            DispatchCmd::AudioKeyReq {
                track,
                file,
                callback,
            } => {
                let msg = audio_key.enqueue_request(track, file, callback);
                let _ = messages.send(msg);
            }
            DispatchCmd::CountryCodeReq { callback } => {
                let _ = callback.send(country_code.clone());
            }
            DispatchCmd::DecodedMsg(msg) if msg.cmd == ShannonMsg::PING => {
                let _ = messages.send(pong_message());
            }
            DispatchCmd::DecodedMsg(msg) if msg.cmd == ShannonMsg::COUNTRY_CODE => {
                country_code.replace(parse_country_code(msg).unwrap());
            }
            DispatchCmd::DecodedMsg(msg) if msg.cmd == ShannonMsg::AES_KEY => {
                audio_key.handle_aes_key(msg)
            }
            DispatchCmd::DecodedMsg(msg) if msg.cmd == ShannonMsg::AES_KEY_ERROR => {
                audio_key.handle_aes_key_error(msg)
            }
            DispatchCmd::DecodedMsg(msg) if msg.cmd == ShannonMsg::MERCURY_REQ => {
                mercury.handle_mercury_req(msg)
            }
            DispatchCmd::DecodedMsg(msg) => {
                log::debug!("ignored message: {:?}", msg.cmd);
            }
            DispatchCmd::DecoderError(err) => {
                log::error!("connection error: {:?}", err);
                let _ = stream.shutdown(Shutdown::Write);
                break;
            }
            DispatchCmd::EncoderError(err) => {
                log::error!("connection error: {:?}", err);
                let _ = stream.shutdown(Shutdown::Read);
                break;
            }
            DispatchCmd::Shutdown => {
                log::info!("connection shutdown");
                let _ = stream.shutdown(Shutdown::Both);
                break;
            }
        }
    }
}

fn pong_message() -> ShannonMsg {
    ShannonMsg::new(ShannonMsg::PONG, vec![0, 0, 0, 0])
}

fn parse_country_code(msg: ShannonMsg) -> Result<String, Error> {
    String::from_utf8(msg.payload)
        .ok()
        .ok_or(Error::UnexpectedResponse)
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::JsonError(Box::new(error))
    }
}
