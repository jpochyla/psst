// Automatically generated rust module for 'authentication.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use quick_protobuf::{MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AuthenticationType {
    AUTHENTICATION_USER_PASS = 0,
    AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS = 1,
    AUTHENTICATION_STORED_FACEBOOK_CREDENTIALS = 2,
    AUTHENTICATION_SPOTIFY_TOKEN = 3,
    AUTHENTICATION_FACEBOOK_TOKEN = 4,
}

impl Default for AuthenticationType {
    fn default() -> Self {
        AuthenticationType::AUTHENTICATION_USER_PASS
    }
}

impl From<i32> for AuthenticationType {
    fn from(i: i32) -> Self {
        match i {
            0 => AuthenticationType::AUTHENTICATION_USER_PASS,
            1 => AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
            2 => AuthenticationType::AUTHENTICATION_STORED_FACEBOOK_CREDENTIALS,
            3 => AuthenticationType::AUTHENTICATION_SPOTIFY_TOKEN,
            4 => AuthenticationType::AUTHENTICATION_FACEBOOK_TOKEN,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for AuthenticationType {
    fn from(s: &'a str) -> Self {
        match s {
            "AUTHENTICATION_USER_PASS" => AuthenticationType::AUTHENTICATION_USER_PASS,
            "AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS" => AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
            "AUTHENTICATION_STORED_FACEBOOK_CREDENTIALS" => AuthenticationType::AUTHENTICATION_STORED_FACEBOOK_CREDENTIALS,
            "AUTHENTICATION_SPOTIFY_TOKEN" => AuthenticationType::AUTHENTICATION_SPOTIFY_TOKEN,
            "AUTHENTICATION_FACEBOOK_TOKEN" => AuthenticationType::AUTHENTICATION_FACEBOOK_TOKEN,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AccountCreation {
    ACCOUNT_CREATION_ALWAYS_PROMPT = 1,
    ACCOUNT_CREATION_ALWAYS_CREATE = 3,
}

impl Default for AccountCreation {
    fn default() -> Self {
        AccountCreation::ACCOUNT_CREATION_ALWAYS_PROMPT
    }
}

impl From<i32> for AccountCreation {
    fn from(i: i32) -> Self {
        match i {
            1 => AccountCreation::ACCOUNT_CREATION_ALWAYS_PROMPT,
            3 => AccountCreation::ACCOUNT_CREATION_ALWAYS_CREATE,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for AccountCreation {
    fn from(s: &'a str) -> Self {
        match s {
            "ACCOUNT_CREATION_ALWAYS_PROMPT" => AccountCreation::ACCOUNT_CREATION_ALWAYS_PROMPT,
            "ACCOUNT_CREATION_ALWAYS_CREATE" => AccountCreation::ACCOUNT_CREATION_ALWAYS_CREATE,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CpuFamily {
    CPU_UNKNOWN = 0,
    CPU_X86 = 1,
    CPU_X86_64 = 2,
    CPU_PPC = 3,
    CPU_PPC_64 = 4,
    CPU_ARM = 5,
    CPU_IA64 = 6,
    CPU_SH = 7,
    CPU_MIPS = 8,
    CPU_BLACKFIN = 9,
}

impl Default for CpuFamily {
    fn default() -> Self {
        CpuFamily::CPU_UNKNOWN
    }
}

impl From<i32> for CpuFamily {
    fn from(i: i32) -> Self {
        match i {
            0 => CpuFamily::CPU_UNKNOWN,
            1 => CpuFamily::CPU_X86,
            2 => CpuFamily::CPU_X86_64,
            3 => CpuFamily::CPU_PPC,
            4 => CpuFamily::CPU_PPC_64,
            5 => CpuFamily::CPU_ARM,
            6 => CpuFamily::CPU_IA64,
            7 => CpuFamily::CPU_SH,
            8 => CpuFamily::CPU_MIPS,
            9 => CpuFamily::CPU_BLACKFIN,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for CpuFamily {
    fn from(s: &'a str) -> Self {
        match s {
            "CPU_UNKNOWN" => CpuFamily::CPU_UNKNOWN,
            "CPU_X86" => CpuFamily::CPU_X86,
            "CPU_X86_64" => CpuFamily::CPU_X86_64,
            "CPU_PPC" => CpuFamily::CPU_PPC,
            "CPU_PPC_64" => CpuFamily::CPU_PPC_64,
            "CPU_ARM" => CpuFamily::CPU_ARM,
            "CPU_IA64" => CpuFamily::CPU_IA64,
            "CPU_SH" => CpuFamily::CPU_SH,
            "CPU_MIPS" => CpuFamily::CPU_MIPS,
            "CPU_BLACKFIN" => CpuFamily::CPU_BLACKFIN,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Brand {
    BRAND_UNBRANDED = 0,
    BRAND_INQ = 1,
    BRAND_HTC = 2,
    BRAND_NOKIA = 3,
}

impl Default for Brand {
    fn default() -> Self {
        Brand::BRAND_UNBRANDED
    }
}

impl From<i32> for Brand {
    fn from(i: i32) -> Self {
        match i {
            0 => Brand::BRAND_UNBRANDED,
            1 => Brand::BRAND_INQ,
            2 => Brand::BRAND_HTC,
            3 => Brand::BRAND_NOKIA,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Brand {
    fn from(s: &'a str) -> Self {
        match s {
            "BRAND_UNBRANDED" => Brand::BRAND_UNBRANDED,
            "BRAND_INQ" => Brand::BRAND_INQ,
            "BRAND_HTC" => Brand::BRAND_HTC,
            "BRAND_NOKIA" => Brand::BRAND_NOKIA,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Os {
    OS_UNKNOWN = 0,
    OS_WINDOWS = 1,
    OS_OSX = 2,
    OS_IPHONE = 3,
    OS_S60 = 4,
    OS_LINUX = 5,
    OS_WINDOWS_CE = 6,
    OS_ANDROID = 7,
    OS_PALM = 8,
    OS_FREEBSD = 9,
    OS_BLACKBERRY = 10,
    OS_SONOS = 11,
    OS_LOGITECH = 12,
    OS_WP7 = 13,
    OS_ONKYO = 14,
    OS_PHILIPS = 15,
    OS_WD = 16,
    OS_VOLVO = 17,
    OS_TIVO = 18,
    OS_AWOX = 19,
    OS_MEEGO = 20,
    OS_QNXNTO = 21,
    OS_BCO = 22,
}

impl Default for Os {
    fn default() -> Self {
        Os::OS_UNKNOWN
    }
}

impl From<i32> for Os {
    fn from(i: i32) -> Self {
        match i {
            0 => Os::OS_UNKNOWN,
            1 => Os::OS_WINDOWS,
            2 => Os::OS_OSX,
            3 => Os::OS_IPHONE,
            4 => Os::OS_S60,
            5 => Os::OS_LINUX,
            6 => Os::OS_WINDOWS_CE,
            7 => Os::OS_ANDROID,
            8 => Os::OS_PALM,
            9 => Os::OS_FREEBSD,
            10 => Os::OS_BLACKBERRY,
            11 => Os::OS_SONOS,
            12 => Os::OS_LOGITECH,
            13 => Os::OS_WP7,
            14 => Os::OS_ONKYO,
            15 => Os::OS_PHILIPS,
            16 => Os::OS_WD,
            17 => Os::OS_VOLVO,
            18 => Os::OS_TIVO,
            19 => Os::OS_AWOX,
            20 => Os::OS_MEEGO,
            21 => Os::OS_QNXNTO,
            22 => Os::OS_BCO,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Os {
    fn from(s: &'a str) -> Self {
        match s {
            "OS_UNKNOWN" => Os::OS_UNKNOWN,
            "OS_WINDOWS" => Os::OS_WINDOWS,
            "OS_OSX" => Os::OS_OSX,
            "OS_IPHONE" => Os::OS_IPHONE,
            "OS_S60" => Os::OS_S60,
            "OS_LINUX" => Os::OS_LINUX,
            "OS_WINDOWS_CE" => Os::OS_WINDOWS_CE,
            "OS_ANDROID" => Os::OS_ANDROID,
            "OS_PALM" => Os::OS_PALM,
            "OS_FREEBSD" => Os::OS_FREEBSD,
            "OS_BLACKBERRY" => Os::OS_BLACKBERRY,
            "OS_SONOS" => Os::OS_SONOS,
            "OS_LOGITECH" => Os::OS_LOGITECH,
            "OS_WP7" => Os::OS_WP7,
            "OS_ONKYO" => Os::OS_ONKYO,
            "OS_PHILIPS" => Os::OS_PHILIPS,
            "OS_WD" => Os::OS_WD,
            "OS_VOLVO" => Os::OS_VOLVO,
            "OS_TIVO" => Os::OS_TIVO,
            "OS_AWOX" => Os::OS_AWOX,
            "OS_MEEGO" => Os::OS_MEEGO,
            "OS_QNXNTO" => Os::OS_QNXNTO,
            "OS_BCO" => Os::OS_BCO,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AccountType {
    Spotify = 0,
    Facebook = 1,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Spotify
    }
}

impl From<i32> for AccountType {
    fn from(i: i32) -> Self {
        match i {
            0 => AccountType::Spotify,
            1 => AccountType::Facebook,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for AccountType {
    fn from(s: &'a str) -> Self {
        match s {
            "Spotify" => AccountType::Spotify,
            "Facebook" => AccountType::Facebook,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ClientResponseEncrypted {
    pub login_credentials: authentication::LoginCredentials,
    pub account_creation: Option<authentication::AccountCreation>,
    pub fingerprint_response: Option<authentication::FingerprintResponseUnion>,
    pub peer_ticket: Option<authentication::PeerTicketUnion>,
    pub system_info: authentication::SystemInfo,
    pub platform_model: Option<String>,
    pub version_string: Option<String>,
    pub appkey: Option<authentication::LibspotifyAppKey>,
    pub client_info: Option<authentication::ClientInfo>,
}

impl<'a> MessageRead<'a> for ClientResponseEncrypted {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.login_credentials = r.read_message::<authentication::LoginCredentials>(bytes)?,
                Ok(160) => msg.account_creation = Some(r.read_enum(bytes)?),
                Ok(242) => msg.fingerprint_response = Some(r.read_message::<authentication::FingerprintResponseUnion>(bytes)?),
                Ok(322) => msg.peer_ticket = Some(r.read_message::<authentication::PeerTicketUnion>(bytes)?),
                Ok(402) => msg.system_info = r.read_message::<authentication::SystemInfo>(bytes)?,
                Ok(482) => msg.platform_model = Some(r.read_string(bytes)?.to_owned()),
                Ok(562) => msg.version_string = Some(r.read_string(bytes)?.to_owned()),
                Ok(642) => msg.appkey = Some(r.read_message::<authentication::LibspotifyAppKey>(bytes)?),
                Ok(722) => msg.client_info = Some(r.read_message::<authentication::ClientInfo>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ClientResponseEncrypted {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.login_credentials).get_size())
        + self.account_creation.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.fingerprint_response.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.peer_ticket.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + 2 + sizeof_len((&self.system_info).get_size())
        + self.platform_model.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.version_string.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.appkey.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.client_info.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_message(&self.login_credentials))?;
        if let Some(ref s) = self.account_creation { w.write_with_tag(160, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.fingerprint_response { w.write_with_tag(242, |w| w.write_message(s))?; }
        if let Some(ref s) = self.peer_ticket { w.write_with_tag(322, |w| w.write_message(s))?; }
        w.write_with_tag(402, |w| w.write_message(&self.system_info))?;
        if let Some(ref s) = self.platform_model { w.write_with_tag(482, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.version_string { w.write_with_tag(562, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.appkey { w.write_with_tag(642, |w| w.write_message(s))?; }
        if let Some(ref s) = self.client_info { w.write_with_tag(722, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LoginCredentials {
    pub username: Option<String>,
    pub typ: authentication::AuthenticationType,
    pub auth_data: Option<Vec<u8>>,
}

impl<'a> MessageRead<'a> for LoginCredentials {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.username = Some(r.read_string(bytes)?.to_owned()),
                Ok(160) => msg.typ = r.read_enum(bytes)?,
                Ok(242) => msg.auth_data = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LoginCredentials {
    fn get_size(&self) -> usize {
        0
        + self.username.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + 2 + sizeof_varint(*(&self.typ) as u64)
        + self.auth_data.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.username { w.write_with_tag(82, |w| w.write_string(&**s))?; }
        w.write_with_tag(160, |w| w.write_enum(*&self.typ as i32))?;
        if let Some(ref s) = self.auth_data { w.write_with_tag(242, |w| w.write_bytes(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct FingerprintResponseUnion {
    pub grain: Option<authentication::FingerprintGrainResponse>,
    pub hmac_ripemd: Option<authentication::FingerprintHmacRipemdResponse>,
}

impl<'a> MessageRead<'a> for FingerprintResponseUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.grain = Some(r.read_message::<authentication::FingerprintGrainResponse>(bytes)?),
                Ok(162) => msg.hmac_ripemd = Some(r.read_message::<authentication::FingerprintHmacRipemdResponse>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for FingerprintResponseUnion {
    fn get_size(&self) -> usize {
        0
        + self.grain.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.hmac_ripemd.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.grain { w.write_with_tag(82, |w| w.write_message(s))?; }
        if let Some(ref s) = self.hmac_ripemd { w.write_with_tag(162, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct FingerprintGrainResponse {
    pub encrypted_key: Vec<u8>,
}

impl<'a> MessageRead<'a> for FingerprintGrainResponse {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.encrypted_key = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for FingerprintGrainResponse {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.encrypted_key).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.encrypted_key))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct FingerprintHmacRipemdResponse {
    pub hmac: Vec<u8>,
}

impl<'a> MessageRead<'a> for FingerprintHmacRipemdResponse {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.hmac = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for FingerprintHmacRipemdResponse {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.hmac).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.hmac))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PeerTicketUnion {
    pub public_key: Option<authentication::PeerTicketPublicKey>,
    pub old_ticket: Option<authentication::PeerTicketOld>,
}

impl<'a> MessageRead<'a> for PeerTicketUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.public_key = Some(r.read_message::<authentication::PeerTicketPublicKey>(bytes)?),
                Ok(162) => msg.old_ticket = Some(r.read_message::<authentication::PeerTicketOld>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for PeerTicketUnion {
    fn get_size(&self) -> usize {
        0
        + self.public_key.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.old_ticket.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.public_key { w.write_with_tag(82, |w| w.write_message(s))?; }
        if let Some(ref s) = self.old_ticket { w.write_with_tag(162, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PeerTicketPublicKey {
    pub public_key: Vec<u8>,
}

impl<'a> MessageRead<'a> for PeerTicketPublicKey {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.public_key = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for PeerTicketPublicKey {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.public_key).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.public_key))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PeerTicketOld {
    pub peer_ticket: Vec<u8>,
    pub peer_ticket_signature: Vec<u8>,
}

impl<'a> MessageRead<'a> for PeerTicketOld {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.peer_ticket = r.read_bytes(bytes)?.to_owned(),
                Ok(162) => msg.peer_ticket_signature = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for PeerTicketOld {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.peer_ticket).len())
        + 2 + sizeof_len((&self.peer_ticket_signature).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.peer_ticket))?;
        w.write_with_tag(162, |w| w.write_bytes(&**&self.peer_ticket_signature))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct SystemInfo {
    pub cpu_family: authentication::CpuFamily,
    pub cpu_subtype: Option<u32>,
    pub cpu_ext: Option<u32>,
    pub brand: Option<authentication::Brand>,
    pub brand_flags: Option<u32>,
    pub os: authentication::Os,
    pub os_version: Option<u32>,
    pub os_ext: Option<u32>,
    pub system_information_string: Option<String>,
    pub device_id: Option<String>,
}

impl<'a> MessageRead<'a> for SystemInfo {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(80) => msg.cpu_family = r.read_enum(bytes)?,
                Ok(160) => msg.cpu_subtype = Some(r.read_uint32(bytes)?),
                Ok(240) => msg.cpu_ext = Some(r.read_uint32(bytes)?),
                Ok(320) => msg.brand = Some(r.read_enum(bytes)?),
                Ok(400) => msg.brand_flags = Some(r.read_uint32(bytes)?),
                Ok(480) => msg.os = r.read_enum(bytes)?,
                Ok(560) => msg.os_version = Some(r.read_uint32(bytes)?),
                Ok(640) => msg.os_ext = Some(r.read_uint32(bytes)?),
                Ok(722) => msg.system_information_string = Some(r.read_string(bytes)?.to_owned()),
                Ok(802) => msg.device_id = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for SystemInfo {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_varint(*(&self.cpu_family) as u64)
        + self.cpu_subtype.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.cpu_ext.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.brand.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.brand_flags.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + 2 + sizeof_varint(*(&self.os) as u64)
        + self.os_version.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.os_ext.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.system_information_string.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.device_id.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(80, |w| w.write_enum(*&self.cpu_family as i32))?;
        if let Some(ref s) = self.cpu_subtype { w.write_with_tag(160, |w| w.write_uint32(*s))?; }
        if let Some(ref s) = self.cpu_ext { w.write_with_tag(240, |w| w.write_uint32(*s))?; }
        if let Some(ref s) = self.brand { w.write_with_tag(320, |w| w.write_enum(*s as i32))?; }
        if let Some(ref s) = self.brand_flags { w.write_with_tag(400, |w| w.write_uint32(*s))?; }
        w.write_with_tag(480, |w| w.write_enum(*&self.os as i32))?;
        if let Some(ref s) = self.os_version { w.write_with_tag(560, |w| w.write_uint32(*s))?; }
        if let Some(ref s) = self.os_ext { w.write_with_tag(640, |w| w.write_uint32(*s))?; }
        if let Some(ref s) = self.system_information_string { w.write_with_tag(722, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.device_id { w.write_with_tag(802, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LibspotifyAppKey {
    pub version: u32,
    pub devkey: Vec<u8>,
    pub signature: Vec<u8>,
    pub useragent: String,
    pub callback_hash: Vec<u8>,
}

impl<'a> MessageRead<'a> for LibspotifyAppKey {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.version = r.read_uint32(bytes)?,
                Ok(18) => msg.devkey = r.read_bytes(bytes)?.to_owned(),
                Ok(26) => msg.signature = r.read_bytes(bytes)?.to_owned(),
                Ok(34) => msg.useragent = r.read_string(bytes)?.to_owned(),
                Ok(42) => msg.callback_hash = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LibspotifyAppKey {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_varint(*(&self.version) as u64)
        + 1 + sizeof_len((&self.devkey).len())
        + 1 + sizeof_len((&self.signature).len())
        + 1 + sizeof_len((&self.useragent).len())
        + 1 + sizeof_len((&self.callback_hash).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(8, |w| w.write_uint32(*&self.version))?;
        w.write_with_tag(18, |w| w.write_bytes(&**&self.devkey))?;
        w.write_with_tag(26, |w| w.write_bytes(&**&self.signature))?;
        w.write_with_tag(34, |w| w.write_string(&**&self.useragent))?;
        w.write_with_tag(42, |w| w.write_bytes(&**&self.callback_hash))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ClientInfo {
    pub limited: Option<bool>,
    pub fb: Option<authentication::ClientInfoFacebook>,
    pub language: Option<String>,
}

impl<'a> MessageRead<'a> for ClientInfo {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.limited = Some(r.read_bool(bytes)?),
                Ok(18) => msg.fb = Some(r.read_message::<authentication::ClientInfoFacebook>(bytes)?),
                Ok(26) => msg.language = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ClientInfo {
    fn get_size(&self) -> usize {
        0
        + self.limited.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.fb.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.language.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.limited { w.write_with_tag(8, |w| w.write_bool(*s))?; }
        if let Some(ref s) = self.fb { w.write_with_tag(18, |w| w.write_message(s))?; }
        if let Some(ref s) = self.language { w.write_with_tag(26, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ClientInfoFacebook {
    pub machine_id: Option<String>,
}

impl<'a> MessageRead<'a> for ClientInfoFacebook {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.machine_id = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ClientInfoFacebook {
    fn get_size(&self) -> usize {
        0
        + self.machine_id.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.machine_id { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct APWelcome {
    pub canonical_username: String,
    pub account_type_logged_in: authentication::AccountType,
    pub credentials_type_logged_in: authentication::AccountType,
    pub reusable_auth_credentials_type: authentication::AuthenticationType,
    pub reusable_auth_credentials: Vec<u8>,
    pub lfs_secret: Option<Vec<u8>>,
    pub account_info: Option<authentication::AccountInfo>,
    pub fb: Option<authentication::AccountInfoFacebook>,
}

impl<'a> MessageRead<'a> for APWelcome {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.canonical_username = r.read_string(bytes)?.to_owned(),
                Ok(160) => msg.account_type_logged_in = r.read_enum(bytes)?,
                Ok(200) => msg.credentials_type_logged_in = r.read_enum(bytes)?,
                Ok(240) => msg.reusable_auth_credentials_type = r.read_enum(bytes)?,
                Ok(322) => msg.reusable_auth_credentials = r.read_bytes(bytes)?.to_owned(),
                Ok(402) => msg.lfs_secret = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(482) => msg.account_info = Some(r.read_message::<authentication::AccountInfo>(bytes)?),
                Ok(562) => msg.fb = Some(r.read_message::<authentication::AccountInfoFacebook>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for APWelcome {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.canonical_username).len())
        + 2 + sizeof_varint(*(&self.account_type_logged_in) as u64)
        + 2 + sizeof_varint(*(&self.credentials_type_logged_in) as u64)
        + 2 + sizeof_varint(*(&self.reusable_auth_credentials_type) as u64)
        + 2 + sizeof_len((&self.reusable_auth_credentials).len())
        + self.lfs_secret.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.account_info.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.fb.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_string(&**&self.canonical_username))?;
        w.write_with_tag(160, |w| w.write_enum(*&self.account_type_logged_in as i32))?;
        w.write_with_tag(200, |w| w.write_enum(*&self.credentials_type_logged_in as i32))?;
        w.write_with_tag(240, |w| w.write_enum(*&self.reusable_auth_credentials_type as i32))?;
        w.write_with_tag(322, |w| w.write_bytes(&**&self.reusable_auth_credentials))?;
        if let Some(ref s) = self.lfs_secret { w.write_with_tag(402, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.account_info { w.write_with_tag(482, |w| w.write_message(s))?; }
        if let Some(ref s) = self.fb { w.write_with_tag(562, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct AccountInfo {
    pub spotify: Option<authentication::AccountInfoSpotify>,
    pub facebook: Option<authentication::AccountInfoFacebook>,
}

impl<'a> MessageRead<'a> for AccountInfo {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.spotify = Some(r.read_message::<authentication::AccountInfoSpotify>(bytes)?),
                Ok(18) => msg.facebook = Some(r.read_message::<authentication::AccountInfoFacebook>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for AccountInfo {
    fn get_size(&self) -> usize {
        0
        + self.spotify.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.facebook.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.spotify { w.write_with_tag(10, |w| w.write_message(s))?; }
        if let Some(ref s) = self.facebook { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct AccountInfoSpotify { }

impl<'a> MessageRead<'a> for AccountInfoSpotify {
    fn from_reader(r: &mut BytesReader, _: &[u8]) -> Result<Self> {
        r.read_to_end();
        Ok(Self::default())
    }
}

impl MessageWrite for AccountInfoSpotify { }

#[derive(Debug, Default, PartialEq, Clone)]
pub struct AccountInfoFacebook {
    pub access_token: Option<String>,
    pub machine_id: Option<String>,
}

impl<'a> MessageRead<'a> for AccountInfoFacebook {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.access_token = Some(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.machine_id = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for AccountInfoFacebook {
    fn get_size(&self) -> usize {
        0
        + self.access_token.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.machine_id.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.access_token { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.machine_id { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

