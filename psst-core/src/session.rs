use std::{
    io,
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use crossbeam_channel::unbounded;
use quick_protobuf::MessageRead;
use serde::de::DeserializeOwned;

use crate::{
    audio_key::{AudioKey, AudioKeyDispatcher},
    connection::{
        shannon_codec::{ShannonDecoder, ShannonEncoder, ShannonMsg},
        Credentials, Transport,
    },
    error::Error,
    item_id::{FileId, ItemId},
    mercury::{self, MercuryDispatcher, MercuryRequest},
    util::{deserialize_protobuf, TcpShutdown},
};

#[derive(Clone)]
pub struct SessionConfig {
    pub login_creds: Credentials,
    pub proxy_url: Option<String>,
}

#[derive(Clone)]
pub struct SessionHandle {
    inner: Arc<Mutex<InnerHandle>>,
}

struct InnerHandle {
    config: Option<SessionConfig>,
    session: Option<Arc<Session>>,
    thread: Option<JoinHandle<()>>,
}

impl InnerHandle {
    fn set_config(&mut self, config: SessionConfig) {
        self.config.replace(config);
        self.disconnect();
    }

    fn connected(&mut self) -> Result<Arc<Session>, Error> {
        if !self.is_connected() {
            let session = Arc::new(Session::connect(
                self.config.clone().ok_or(Error::SessionDisconnected)?,
            )?);
            self.session.replace(Arc::clone(&session));
            self.thread.replace(thread::spawn({
                let session = Arc::clone(&session);
                move || match session.service() {
                    Ok(_) => {
                        log::info!("connection shutdown");
                    }
                    Err(err) => {
                        log::error!("connection error: {:?}", err);
                    }
                }
            }));
        }
        self.session.clone().ok_or(Error::SessionDisconnected)
    }

    fn disconnect(&mut self) {
        if let Some(session) = self.session.take() {
            session.shutdown();
        }
        if let Some(thread) = self.thread.take() {
            if let Err(err) = thread.join() {
                log::error!("connection thread panicked: {:?}", err);
            }
        }
    }

    fn is_connected(&self) -> bool {
        matches!(self.session.as_ref(), Some(session) if session.is_serviced())
    }
}

impl SessionHandle {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerHandle {
                config: None,
                session: None,
                thread: None,
            })),
        }
    }

    pub fn set_config(&self, config: SessionConfig) {
        self.inner.lock().unwrap().set_config(config);
    }

    pub fn connected(&self) -> Result<Arc<Session>, Error> {
        self.inner.lock().unwrap().connected()
    }

    pub fn is_connected(&self) -> bool {
        self.inner.lock().unwrap().is_connected()
    }
}

pub struct Session {
    shutdown: Mutex<TcpShutdown>,
    encoder: Mutex<ShannonEncoder<TcpStream>>,
    decoder: Mutex<ShannonDecoder<TcpStream>>,
    mercury: Mutex<MercuryDispatcher>,
    audio_key: Mutex<AudioKeyDispatcher>,
    country_code: Mutex<Option<String>>,
    credentials: Credentials,
    is_serviced: AtomicBool,
}

impl Session {
    pub fn connect(config: SessionConfig) -> Result<Self, Error> {
        // Connect to the server and exchange keys.
        let proxy_url = config.proxy_url.as_deref();
        let mut transport =
            Transport::connect(&Transport::resolve_ap_with_fallback(proxy_url), proxy_url)?;
        // Authenticate with provided credentials (either username/password, or saved,
        // reusable credential blob from an earlier run).
        let credentials = transport.authenticate(config.login_creds)?;
        // Split transport into encoding/decoding parts, so we can read/write/shutdown
        // in parallel.
        let Transport {
            stream,
            encoder,
            decoder,
        } = transport;
        Ok(Self {
            shutdown: Mutex::new(TcpShutdown::new(stream)),
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(decoder),
            credentials,
            country_code: Mutex::new(None),
            audio_key: Mutex::new(AudioKeyDispatcher::new()),
            mercury: Mutex::new(mercury::MercuryDispatcher::new()),
            is_serviced: AtomicBool::new(false),
        })
    }

    pub fn service(&self) -> Result<(), Error> {
        let service = || loop {
            let msg = self.receive()?;
            self.dispatch(msg)?;
        };
        self.is_serviced.store(true, Ordering::SeqCst);
        let result = service();
        self.is_serviced.store(false, Ordering::SeqCst);
        result
    }

    pub fn shutdown(&self) {
        self.shutdown.lock().unwrap().shutdown();
    }

    pub fn has_been_shut_down(&self) -> bool {
        self.shutdown.lock().unwrap().has_been_shut_down()
    }

    pub fn is_serviced(&self) -> bool {
        self.is_serviced.load(Ordering::SeqCst)
    }

    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }

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
        let (sender, receiver) = unbounded();
        self.encoder.lock().unwrap().encode(
            self.mercury
                .lock()
                .unwrap()
                .enqueue_request(sender, MercuryRequest::get(uri)),
        )?;
        let response = receiver.recv().ok().ok_or(Error::SessionDisconnected)?;
        response
            .payload
            .into_iter()
            .next()
            .ok_or(Error::UnexpectedResponse)
    }

    pub fn get_audio_key(&self, track: ItemId, file: FileId) -> Result<AudioKey, Error> {
        let (sender, receiver) = unbounded();
        self.encoder.lock().unwrap().encode(
            self.audio_key
                .lock()
                .unwrap()
                .enqueue_request(sender, track, file),
        )?;
        receiver.recv().ok().ok_or(Error::SessionDisconnected)?
    }

    pub fn get_country_code(&self) -> Option<String> {
        self.country_code.lock().unwrap().clone()
    }

    fn dispatch(&self, msg: ShannonMsg) -> Result<(), Error> {
        match msg.cmd {
            ShannonMsg::PING => {
                self.handle_ping()?;
            }
            ShannonMsg::COUNTRY_CODE => {
                self.handle_country_code(msg.payload)?;
            }
            ShannonMsg::AES_KEY => {
                self.audio_key.lock().unwrap().handle_aes_key(msg);
            }
            ShannonMsg::AES_KEY_ERROR => {
                self.audio_key.lock().unwrap().handle_aes_key_error(msg);
            }
            ShannonMsg::MERCURY_REQ => {
                self.mercury.lock().unwrap().handle_mercury_req(msg);
            }
            _ => {
                log::debug!("ignored message: {:?}", msg.cmd);
            }
        }
        Ok(())
    }

    fn handle_ping(&self) -> Result<(), Error> {
        self.send(ShannonMsg::new(ShannonMsg::PONG, vec![0, 0, 0, 0]))?;
        Ok(())
    }

    fn handle_country_code(&self, payload: Vec<u8>) -> Result<(), Error> {
        self.country_code
            .lock()
            .unwrap()
            .replace(String::from_utf8(payload).map_err(|_| Error::UnexpectedResponse)?);
        Ok(())
    }

    fn send(&self, msg: ShannonMsg) -> io::Result<()> {
        self.encoder.lock().unwrap().encode(msg)
    }

    fn receive(&self) -> io::Result<ShannonMsg> {
        self.decoder.lock().unwrap().decode()
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::JsonError(Box::new(error))
    }
}
