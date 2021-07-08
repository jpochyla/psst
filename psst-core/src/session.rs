use crate::{
    audio_key::{AudioKey, AudioKeyDispatcher},
    connection::{
        codec::{ShannonDecoder, ShannonEncoder, ShannonMessage},
        Credentials, Transport,
    },
    error::Error,
    item_id::{FileId, ItemId},
    mercury::{MercuryDispatcher, MercuryRequest},
    util::{deserialize_protobuf, TcpShutdown},
};
use quick_protobuf::MessageRead;
use serde::de::DeserializeOwned;
use std::{
    io,
    net::TcpStream,
    sync::{Arc, Mutex, RwLock},
};

#[derive(Clone)]
pub struct SessionConfig {
    pub login_creds: Credentials,
    pub proxy_url: Option<String>,
}

#[derive(Clone)]
pub struct SessionHandle {
    session: Arc<RwLock<Option<Arc<Session>>>>,
}

impl SessionHandle {
    pub fn new() -> Self {
        Self {
            session: Default::default(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.session.read().unwrap().is_some()
    }

    pub fn connected(&self) -> Result<Arc<Session>, Error> {
        self.session
            .read()
            .unwrap()
            .clone()
            .ok_or(Error::SessionDisconnected)
    }

    pub fn connect(&self, config: SessionConfig) -> Result<Arc<Session>, Error> {
        // First we need to drop the old session, so it counts as disconnected until we
        // successfully connect again.
        if let Some(old_session) = self.session.write().unwrap().take() {
            old_session.shutdown()
        }
        // Try to connect and block until it either succeeds or fails.
        let session = Arc::new(Session::connect(config)?);
        // Save the connected session.
        self.session.write().unwrap().replace(session.clone());
        Ok(session)
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
            mercury: Mutex::new(MercuryDispatcher::new()),
        })
    }

    pub fn service(&self) -> Result<(), Error> {
        loop {
            let msg = self.receive()?;
            self.dispatch(msg)?;
        }
    }

    pub fn shutdown(&self) {
        self.shutdown.lock().unwrap().shutdown();
    }

    pub fn has_been_shut_down(&self) -> bool {
        self.shutdown.lock().unwrap().has_been_shut_down()
    }

    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    pub fn get_mercury_protobuf<T>(&self, uri: String) -> Result<T, Error>
    where
        T: MessageRead<'static>,
    {
        let request = {
            let mut encoder = self.encoder.lock().unwrap();
            self.mercury
                .lock()
                .unwrap()
                .request(&mut encoder, MercuryRequest::get(uri))?
        };
        let response = request
            .recv()
            .expect("Failed to receive from mercury response channel");
        let payload = response.payload.first().ok_or(Error::UnexpectedResponse)?;
        let message = deserialize_protobuf(payload)?;
        Ok(message)
    }

    pub fn get_mercury_json<T>(&self, uri: String) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let request = {
            let mut encoder = self.encoder.lock().unwrap();
            self.mercury
                .lock()
                .unwrap()
                .request(&mut encoder, MercuryRequest::get(uri))?
        };
        let response = request
            .recv()
            .expect("Failed to receive from mercury response channel");
        let payload = response.payload.first().ok_or(Error::UnexpectedResponse)?;
        let message = serde_json::from_slice(payload)?;
        Ok(message)
    }

    pub fn get_audio_key(&self, track: ItemId, file: FileId) -> Result<AudioKey, Error> {
        let request = {
            let mut encoder = self.encoder.lock().unwrap();
            self.audio_key
                .lock()
                .unwrap()
                .request(&mut encoder, track, file)?
        };
        request
            .recv()
            .expect("Failed to receive from audio key response channel")
    }

    pub fn get_country_code(&self) -> Option<String> {
        self.country_code.lock().unwrap().clone()
    }

    fn dispatch(&self, msg: ShannonMessage) -> Result<(), Error> {
        match msg.cmd {
            ShannonMessage::PING => {
                self.handle_ping()?;
            }
            ShannonMessage::COUNTRY_CODE => {
                self.handle_country_code(msg.payload)?;
            }
            ShannonMessage::AES_KEY => {
                self.audio_key.lock().unwrap().handle_aes_key(msg);
            }
            ShannonMessage::AES_KEY_ERROR => {
                self.audio_key.lock().unwrap().handle_aes_key_error(msg);
            }
            ShannonMessage::MERCURY_REQ => {
                self.mercury.lock().unwrap().handle_mercury_req(msg);
            }
            _ => {
                log::debug!("ignored message: {:?}", msg.cmd);
            }
        }
        Ok(())
    }

    fn handle_ping(&self) -> Result<(), Error> {
        self.send(ShannonMessage::new(ShannonMessage::PONG, vec![0, 0, 0, 0]))?;
        Ok(())
    }

    fn handle_country_code(&self, payload: Vec<u8>) -> Result<(), Error> {
        self.country_code
            .lock()
            .unwrap()
            .replace(String::from_utf8(payload).map_err(|_| Error::UnexpectedResponse)?);
        Ok(())
    }

    fn send(&self, msg: ShannonMessage) -> io::Result<()> {
        self.encoder.lock().unwrap().encode(msg)
    }

    fn receive(&self) -> io::Result<ShannonMessage> {
        self.decoder.lock().unwrap().decode()
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::JsonError(Box::new(error))
    }
}
