pub mod codec;
pub mod diffie_hellman;

use crate::{
    connection::{
        codec::{ShannonDecoder, ShannonEncoder, ShannonMessage},
        diffie_hellman::DHLocalKeys,
    },
    error::Error,
    protocol::authentication::AuthenticationType,
    util::{deserialize_protobuf, serialize_protobuf, HTTP_CONNECT_TIMEOUT, HTTP_IO_TIMEOUT},
};
use byteorder::{ReadBytesExt, BE};
use hmac::{Hmac, Mac, NewMac};
use serde::Deserialize;
use sha1::Sha1;
use std::{
    io,
    io::{Read, Write},
    net::TcpStream,
};

// Device ID used for authentication message.
const DEVICE_ID: &str = "Psst";

// URI of access-point resolve endpoint.
const AP_RESOLVE_ENDPOINT: &str = "http://apresolve.spotify.com";

// Access-point used in case the resolving fails.
const AP_FALLBACK: &str = "ap.spotify.com:443";

#[derive(Clone)]
pub struct Credentials {
    username: String,
    auth_data: Vec<u8>,
    auth_type: AuthenticationType,
}

impl Credentials {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        Self {
            username,
            auth_type: AuthenticationType::AUTHENTICATION_USER_PASS,
            auth_data: password.into_bytes(),
        }
    }
}

pub struct Transport {
    pub encoder: ShannonEncoder<TcpStream>,
    pub decoder: ShannonDecoder<TcpStream>,
}

impl Transport {
    pub fn resolve_ap_with_fallback() -> String {
        match Self::resolve_ap() {
            Ok(ap) => ap,
            Err(err) => {
                log::error!("using AP fallback, error while resolving: {:?}", err);
                AP_FALLBACK.into()
            }
        }
    }

    pub fn resolve_ap() -> Result<String, Error> {
        #[derive(Clone, Debug, Deserialize)]
        struct APResolveData {
            ap_list: Vec<String>,
        }

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(HTTP_CONNECT_TIMEOUT)
            .timeout_read(HTTP_IO_TIMEOUT)
            .timeout_write(HTTP_IO_TIMEOUT)
            .build();
        let data: APResolveData = agent.get(AP_RESOLVE_ENDPOINT).call()?.into_json()?;
        data.ap_list
            .into_iter()
            .next()
            .ok_or(Error::UnexpectedResponse)
    }

    pub fn connect(ap: &str) -> Result<Self, Error> {
        log::trace!("connecting to {}", ap);
        let stream = TcpStream::connect(ap)?;
        log::trace!("connected");
        Self::exchange_keys(stream)
    }

    pub fn exchange_keys(mut stream: TcpStream) -> Result<Self, Error> {
        use crate::protocol::keyexchange::APResponseMessage;

        let local_keys = DHLocalKeys::random();

        // Start by sending the hello message with our public key and nonce.
        log::trace!("sending client hello");
        let client_nonce: [u8; 16] = rand::random();
        let hello = client_hello(local_keys.public_key(), client_nonce.into());
        let hello_packet = make_packet(&[0, 4], &hello);
        stream.write_all(&hello_packet)?;
        log::trace!("sent client hello");

        // Wait for the response packet with the remote public key.  Note that we are
        // keeping both the hello packet and the response packet for later (they get
        // hashed together with the shared secret to make a key pair).
        log::trace!("waiting for AP response");
        let apresp_packet = read_packet(&mut stream)?;
        let apresp: APResponseMessage = deserialize_protobuf(&apresp_packet[4..])?;
        log::trace!("received AP response");

        // Compute the challenge response and the sending/receiving keys.
        let remote_key = &apresp
            .challenge
            .expect("Missing data")
            .login_crypto_challenge
            .diffie_hellman
            .expect("Missing data")
            .gs;
        let (challenge, send_key, recv_key) = compute_keys(
            &local_keys.shared_secret(remote_key),
            &hello_packet,
            &apresp_packet,
        );

        // Respond with the computed HMAC and finish the handshake.
        log::trace!("sending client response");
        let response = client_response_plaintext(challenge);
        let response_packet = make_packet(&[], &response);
        stream.write_all(&response_packet)?;
        log::trace!("sent client response");

        // Use the derived keys to make a codec, wrapping the TCP stream.
        let encoder = ShannonEncoder::new(stream.try_clone()?, &send_key);
        let decoder = ShannonDecoder::new(stream, &recv_key);

        Ok(Self { encoder, decoder })
    }

    pub fn authenticate(&mut self, credentials: Credentials) -> Result<Credentials, Error> {
        use crate::protocol::{authentication::APWelcome, keyexchange::APLoginFailed};

        // Send a login request with the client credentials.
        let request = client_response_encrypted(credentials);
        self.encoder.encode(request)?;

        // Expect an immediate response with the authentication result.
        let response = self.decoder.decode()?;

        match response.cmd {
            ShannonMessage::AP_WELCOME => {
                let welcome_data: APWelcome =
                    deserialize_protobuf(&response.payload).expect("Missing data");
                Ok(Credentials {
                    username: welcome_data.canonical_username,
                    auth_data: welcome_data.reusable_auth_credentials,
                    auth_type: welcome_data.reusable_auth_credentials_type,
                })
            }
            ShannonMessage::AUTH_FAILURE => {
                let error_data: APLoginFailed =
                    deserialize_protobuf(&response.payload).expect("Missing data");
                Err(Error::AuthFailed {
                    code: error_data.error_code as _,
                })
            }
            _ => {
                unreachable!("unexpected message");
            }
        }
    }
}

fn read_packet(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let size = stream.read_u32::<BE>()?;
    let mut buf = vec![0_u8; size as usize];
    let (size_buf, data_buf) = buf.split_at_mut(4);
    size_buf.copy_from_slice(&size.to_be_bytes());
    stream.read_exact(data_buf)?;
    Ok(buf)
}

fn make_packet(prefix: &[u8], data: &[u8]) -> Vec<u8> {
    let size = prefix.len() + 4 + data.len();
    let mut buf = Vec::with_capacity(size);
    buf.extend_from_slice(prefix);
    buf.extend_from_slice(&(size as u32).to_be_bytes());
    buf.extend_from_slice(data);
    buf
}

fn client_hello(public_key: Vec<u8>, nonce: Vec<u8>) -> Vec<u8> {
    use crate::protocol::keyexchange::*;

    let hello = ClientHello {
        build_info: BuildInfo {
            platform: Platform::PLATFORM_LINUX_X86,
            product: Product::PRODUCT_PARTNER,
            product_flags: vec![],
            version: 109_800_078,
        },
        cryptosuites_supported: vec![Cryptosuite::CRYPTO_SUITE_SHANNON],
        fingerprints_supported: vec![],
        powschemes_supported: vec![],
        login_crypto_hello: LoginCryptoHelloUnion {
            diffie_hellman: Some(LoginCryptoDiffieHellmanHello {
                gc: public_key,
                server_keys_known: 1,
            }),
        },
        client_nonce: nonce,
        padding: Some(vec![0x1e]),
        feature_set: None,
    };

    serialize_protobuf(&hello).expect("Failed to serialize")
}

fn client_response_plaintext(challenge: Vec<u8>) -> Vec<u8> {
    use crate::protocol::keyexchange::*;

    let response = ClientResponsePlaintext {
        login_crypto_response: LoginCryptoResponseUnion {
            diffie_hellman: Some(LoginCryptoDiffieHellmanResponse { hmac: challenge }),
        },
        pow_response: PoWResponseUnion::default(),
        crypto_response: CryptoResponseUnion::default(),
    };

    serialize_protobuf(&response).expect("Failed to serialize")
}

fn compute_keys(
    shared_secret: &[u8],
    hello_packet: &[u8],
    apresp_packet: &[u8],
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut data = Vec::with_capacity(0x64);
    for i in 1..6 {
        let mut mac: Hmac<Sha1> =
            Hmac::new_varkey(&shared_secret).expect("HMAC can take key of any size");
        mac.update(hello_packet);
        mac.update(apresp_packet);
        mac.update(&[i]);
        data.extend_from_slice(&mac.finalize().into_bytes());
    }
    let mut mac: Hmac<Sha1> =
        Hmac::new_varkey(&data[..0x14]).expect("HMAC can take key of any size");
    mac.update(hello_packet);
    mac.update(apresp_packet);
    let digest = mac.finalize().into_bytes();

    (
        (&*digest).to_vec(),
        (&data[0x14..0x34]).to_vec(),
        (&data[0x34..0x54]).to_vec(),
    )
}

fn client_response_encrypted(credentials: Credentials) -> ShannonMessage {
    use crate::protocol::authentication::{ClientResponseEncrypted, LoginCredentials, SystemInfo};

    let response = ClientResponseEncrypted {
        login_credentials: LoginCredentials {
            username: Some(credentials.username),
            auth_data: Some(credentials.auth_data),
            typ: credentials.auth_type,
        },
        system_info: SystemInfo {
            device_id: Some(DEVICE_ID.to_string()),
            ..SystemInfo::default()
        },
        ..ClientResponseEncrypted::default()
    };

    let buf = serialize_protobuf(&response).expect("Failed to serialize");
    ShannonMessage::new(ShannonMessage::LOGIN, buf)
}
