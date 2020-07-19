// Automatically generated rust module for 'keyexchange.proto' file

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
pub enum Product {
    PRODUCT_CLIENT = 0,
    PRODUCT_LIBSPOTIFY = 1,
    PRODUCT_MOBILE = 2,
    PRODUCT_PARTNER = 3,
    PRODUCT_LIBSPOTIFY_EMBEDDED = 5,
}

impl Default for Product {
    fn default() -> Self {
        Product::PRODUCT_CLIENT
    }
}

impl From<i32> for Product {
    fn from(i: i32) -> Self {
        match i {
            0 => Product::PRODUCT_CLIENT,
            1 => Product::PRODUCT_LIBSPOTIFY,
            2 => Product::PRODUCT_MOBILE,
            3 => Product::PRODUCT_PARTNER,
            5 => Product::PRODUCT_LIBSPOTIFY_EMBEDDED,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Product {
    fn from(s: &'a str) -> Self {
        match s {
            "PRODUCT_CLIENT" => Product::PRODUCT_CLIENT,
            "PRODUCT_LIBSPOTIFY" => Product::PRODUCT_LIBSPOTIFY,
            "PRODUCT_MOBILE" => Product::PRODUCT_MOBILE,
            "PRODUCT_PARTNER" => Product::PRODUCT_PARTNER,
            "PRODUCT_LIBSPOTIFY_EMBEDDED" => Product::PRODUCT_LIBSPOTIFY_EMBEDDED,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ProductFlags {
    PRODUCT_FLAG_NONE = 0,
    PRODUCT_FLAG_DEV_BUILD = 1,
}

impl Default for ProductFlags {
    fn default() -> Self {
        ProductFlags::PRODUCT_FLAG_NONE
    }
}

impl From<i32> for ProductFlags {
    fn from(i: i32) -> Self {
        match i {
            0 => ProductFlags::PRODUCT_FLAG_NONE,
            1 => ProductFlags::PRODUCT_FLAG_DEV_BUILD,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for ProductFlags {
    fn from(s: &'a str) -> Self {
        match s {
            "PRODUCT_FLAG_NONE" => ProductFlags::PRODUCT_FLAG_NONE,
            "PRODUCT_FLAG_DEV_BUILD" => ProductFlags::PRODUCT_FLAG_DEV_BUILD,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Platform {
    PLATFORM_WIN32_X86 = 0,
    PLATFORM_OSX_X86 = 1,
    PLATFORM_LINUX_X86 = 2,
    PLATFORM_IPHONE_ARM = 3,
    PLATFORM_S60_ARM = 4,
    PLATFORM_OSX_PPC = 5,
    PLATFORM_ANDROID_ARM = 6,
    PLATFORM_WINDOWS_CE_ARM = 7,
    PLATFORM_LINUX_X86_64 = 8,
    PLATFORM_OSX_X86_64 = 9,
    PLATFORM_PALM_ARM = 10,
    PLATFORM_LINUX_SH = 11,
    PLATFORM_FREEBSD_X86 = 12,
    PLATFORM_FREEBSD_X86_64 = 13,
    PLATFORM_BLACKBERRY_ARM = 14,
    PLATFORM_SONOS = 15,
    PLATFORM_LINUX_MIPS = 16,
    PLATFORM_LINUX_ARM = 17,
    PLATFORM_LOGITECH_ARM = 18,
    PLATFORM_LINUX_BLACKFIN = 19,
    PLATFORM_WP7_ARM = 20,
    PLATFORM_ONKYO_ARM = 21,
    PLATFORM_QNXNTO_ARM = 22,
    PLATFORM_BCO_ARM = 23,
}

impl Default for Platform {
    fn default() -> Self {
        Platform::PLATFORM_WIN32_X86
    }
}

impl From<i32> for Platform {
    fn from(i: i32) -> Self {
        match i {
            0 => Platform::PLATFORM_WIN32_X86,
            1 => Platform::PLATFORM_OSX_X86,
            2 => Platform::PLATFORM_LINUX_X86,
            3 => Platform::PLATFORM_IPHONE_ARM,
            4 => Platform::PLATFORM_S60_ARM,
            5 => Platform::PLATFORM_OSX_PPC,
            6 => Platform::PLATFORM_ANDROID_ARM,
            7 => Platform::PLATFORM_WINDOWS_CE_ARM,
            8 => Platform::PLATFORM_LINUX_X86_64,
            9 => Platform::PLATFORM_OSX_X86_64,
            10 => Platform::PLATFORM_PALM_ARM,
            11 => Platform::PLATFORM_LINUX_SH,
            12 => Platform::PLATFORM_FREEBSD_X86,
            13 => Platform::PLATFORM_FREEBSD_X86_64,
            14 => Platform::PLATFORM_BLACKBERRY_ARM,
            15 => Platform::PLATFORM_SONOS,
            16 => Platform::PLATFORM_LINUX_MIPS,
            17 => Platform::PLATFORM_LINUX_ARM,
            18 => Platform::PLATFORM_LOGITECH_ARM,
            19 => Platform::PLATFORM_LINUX_BLACKFIN,
            20 => Platform::PLATFORM_WP7_ARM,
            21 => Platform::PLATFORM_ONKYO_ARM,
            22 => Platform::PLATFORM_QNXNTO_ARM,
            23 => Platform::PLATFORM_BCO_ARM,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Platform {
    fn from(s: &'a str) -> Self {
        match s {
            "PLATFORM_WIN32_X86" => Platform::PLATFORM_WIN32_X86,
            "PLATFORM_OSX_X86" => Platform::PLATFORM_OSX_X86,
            "PLATFORM_LINUX_X86" => Platform::PLATFORM_LINUX_X86,
            "PLATFORM_IPHONE_ARM" => Platform::PLATFORM_IPHONE_ARM,
            "PLATFORM_S60_ARM" => Platform::PLATFORM_S60_ARM,
            "PLATFORM_OSX_PPC" => Platform::PLATFORM_OSX_PPC,
            "PLATFORM_ANDROID_ARM" => Platform::PLATFORM_ANDROID_ARM,
            "PLATFORM_WINDOWS_CE_ARM" => Platform::PLATFORM_WINDOWS_CE_ARM,
            "PLATFORM_LINUX_X86_64" => Platform::PLATFORM_LINUX_X86_64,
            "PLATFORM_OSX_X86_64" => Platform::PLATFORM_OSX_X86_64,
            "PLATFORM_PALM_ARM" => Platform::PLATFORM_PALM_ARM,
            "PLATFORM_LINUX_SH" => Platform::PLATFORM_LINUX_SH,
            "PLATFORM_FREEBSD_X86" => Platform::PLATFORM_FREEBSD_X86,
            "PLATFORM_FREEBSD_X86_64" => Platform::PLATFORM_FREEBSD_X86_64,
            "PLATFORM_BLACKBERRY_ARM" => Platform::PLATFORM_BLACKBERRY_ARM,
            "PLATFORM_SONOS" => Platform::PLATFORM_SONOS,
            "PLATFORM_LINUX_MIPS" => Platform::PLATFORM_LINUX_MIPS,
            "PLATFORM_LINUX_ARM" => Platform::PLATFORM_LINUX_ARM,
            "PLATFORM_LOGITECH_ARM" => Platform::PLATFORM_LOGITECH_ARM,
            "PLATFORM_LINUX_BLACKFIN" => Platform::PLATFORM_LINUX_BLACKFIN,
            "PLATFORM_WP7_ARM" => Platform::PLATFORM_WP7_ARM,
            "PLATFORM_ONKYO_ARM" => Platform::PLATFORM_ONKYO_ARM,
            "PLATFORM_QNXNTO_ARM" => Platform::PLATFORM_QNXNTO_ARM,
            "PLATFORM_BCO_ARM" => Platform::PLATFORM_BCO_ARM,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Fingerprint {
    FINGERPRINT_GRAIN = 0,
    FINGERPRINT_HMAC_RIPEMD = 1,
}

impl Default for Fingerprint {
    fn default() -> Self {
        Fingerprint::FINGERPRINT_GRAIN
    }
}

impl From<i32> for Fingerprint {
    fn from(i: i32) -> Self {
        match i {
            0 => Fingerprint::FINGERPRINT_GRAIN,
            1 => Fingerprint::FINGERPRINT_HMAC_RIPEMD,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Fingerprint {
    fn from(s: &'a str) -> Self {
        match s {
            "FINGERPRINT_GRAIN" => Fingerprint::FINGERPRINT_GRAIN,
            "FINGERPRINT_HMAC_RIPEMD" => Fingerprint::FINGERPRINT_HMAC_RIPEMD,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Cryptosuite {
    CRYPTO_SUITE_SHANNON = 0,
    CRYPTO_SUITE_RC4_SHA1_HMAC = 1,
}

impl Default for Cryptosuite {
    fn default() -> Self {
        Cryptosuite::CRYPTO_SUITE_SHANNON
    }
}

impl From<i32> for Cryptosuite {
    fn from(i: i32) -> Self {
        match i {
            0 => Cryptosuite::CRYPTO_SUITE_SHANNON,
            1 => Cryptosuite::CRYPTO_SUITE_RC4_SHA1_HMAC,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Cryptosuite {
    fn from(s: &'a str) -> Self {
        match s {
            "CRYPTO_SUITE_SHANNON" => Cryptosuite::CRYPTO_SUITE_SHANNON,
            "CRYPTO_SUITE_RC4_SHA1_HMAC" => Cryptosuite::CRYPTO_SUITE_RC4_SHA1_HMAC,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Powscheme {
    POW_HASH_CASH = 0,
}

impl Default for Powscheme {
    fn default() -> Self {
        Powscheme::POW_HASH_CASH
    }
}

impl From<i32> for Powscheme {
    fn from(i: i32) -> Self {
        match i {
            0 => Powscheme::POW_HASH_CASH,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Powscheme {
    fn from(s: &'a str) -> Self {
        match s {
            "POW_HASH_CASH" => Powscheme::POW_HASH_CASH,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ErrorCode {
    ProtocolError = 0,
    TryAnotherAP = 2,
    BadConnectionId = 5,
    TravelRestriction = 9,
    PremiumAccountRequired = 11,
    BadCredentials = 12,
    CouldNotValidateCredentials = 13,
    AccountExists = 14,
    ExtraVerificationRequired = 15,
    InvalidAppKey = 16,
    ApplicationBanned = 17,
}

impl Default for ErrorCode {
    fn default() -> Self {
        ErrorCode::ProtocolError
    }
}

impl From<i32> for ErrorCode {
    fn from(i: i32) -> Self {
        match i {
            0 => ErrorCode::ProtocolError,
            2 => ErrorCode::TryAnotherAP,
            5 => ErrorCode::BadConnectionId,
            9 => ErrorCode::TravelRestriction,
            11 => ErrorCode::PremiumAccountRequired,
            12 => ErrorCode::BadCredentials,
            13 => ErrorCode::CouldNotValidateCredentials,
            14 => ErrorCode::AccountExists,
            15 => ErrorCode::ExtraVerificationRequired,
            16 => ErrorCode::InvalidAppKey,
            17 => ErrorCode::ApplicationBanned,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for ErrorCode {
    fn from(s: &'a str) -> Self {
        match s {
            "ProtocolError" => ErrorCode::ProtocolError,
            "TryAnotherAP" => ErrorCode::TryAnotherAP,
            "BadConnectionId" => ErrorCode::BadConnectionId,
            "TravelRestriction" => ErrorCode::TravelRestriction,
            "PremiumAccountRequired" => ErrorCode::PremiumAccountRequired,
            "BadCredentials" => ErrorCode::BadCredentials,
            "CouldNotValidateCredentials" => ErrorCode::CouldNotValidateCredentials,
            "AccountExists" => ErrorCode::AccountExists,
            "ExtraVerificationRequired" => ErrorCode::ExtraVerificationRequired,
            "InvalidAppKey" => ErrorCode::InvalidAppKey,
            "ApplicationBanned" => ErrorCode::ApplicationBanned,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ClientHello {
    pub build_info: keyexchange::BuildInfo,
    pub fingerprints_supported: Vec<keyexchange::Fingerprint>,
    pub cryptosuites_supported: Vec<keyexchange::Cryptosuite>,
    pub powschemes_supported: Vec<keyexchange::Powscheme>,
    pub login_crypto_hello: keyexchange::LoginCryptoHelloUnion,
    pub client_nonce: Vec<u8>,
    pub padding: Option<Vec<u8>>,
    pub feature_set: Option<keyexchange::FeatureSet>,
}

impl<'a> MessageRead<'a> for ClientHello {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.build_info = r.read_message::<keyexchange::BuildInfo>(bytes)?,
                Ok(160) => msg.fingerprints_supported.push(r.read_enum(bytes)?),
                Ok(240) => msg.cryptosuites_supported.push(r.read_enum(bytes)?),
                Ok(320) => msg.powschemes_supported.push(r.read_enum(bytes)?),
                Ok(402) => msg.login_crypto_hello = r.read_message::<keyexchange::LoginCryptoHelloUnion>(bytes)?,
                Ok(482) => msg.client_nonce = r.read_bytes(bytes)?.to_owned(),
                Ok(562) => msg.padding = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(642) => msg.feature_set = Some(r.read_message::<keyexchange::FeatureSet>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ClientHello {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.build_info).get_size())
        + self.fingerprints_supported.iter().map(|s| 2 + sizeof_varint(*(s) as u64)).sum::<usize>()
        + self.cryptosuites_supported.iter().map(|s| 2 + sizeof_varint(*(s) as u64)).sum::<usize>()
        + self.powschemes_supported.iter().map(|s| 2 + sizeof_varint(*(s) as u64)).sum::<usize>()
        + 2 + sizeof_len((&self.login_crypto_hello).get_size())
        + 2 + sizeof_len((&self.client_nonce).len())
        + self.padding.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
        + self.feature_set.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_message(&self.build_info))?;
        for s in &self.fingerprints_supported { w.write_with_tag(160, |w| w.write_enum(*s as i32))?; }
        for s in &self.cryptosuites_supported { w.write_with_tag(240, |w| w.write_enum(*s as i32))?; }
        for s in &self.powschemes_supported { w.write_with_tag(320, |w| w.write_enum(*s as i32))?; }
        w.write_with_tag(402, |w| w.write_message(&self.login_crypto_hello))?;
        w.write_with_tag(482, |w| w.write_bytes(&**&self.client_nonce))?;
        if let Some(ref s) = self.padding { w.write_with_tag(562, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.feature_set { w.write_with_tag(642, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct BuildInfo {
    pub product: keyexchange::Product,
    pub product_flags: Vec<keyexchange::ProductFlags>,
    pub platform: keyexchange::Platform,
    pub version: u64,
}

impl<'a> MessageRead<'a> for BuildInfo {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(80) => msg.product = r.read_enum(bytes)?,
                Ok(160) => msg.product_flags.push(r.read_enum(bytes)?),
                Ok(240) => msg.platform = r.read_enum(bytes)?,
                Ok(320) => msg.version = r.read_uint64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for BuildInfo {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_varint(*(&self.product) as u64)
        + self.product_flags.iter().map(|s| 2 + sizeof_varint(*(s) as u64)).sum::<usize>()
        + 2 + sizeof_varint(*(&self.platform) as u64)
        + 2 + sizeof_varint(*(&self.version) as u64)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(80, |w| w.write_enum(*&self.product as i32))?;
        for s in &self.product_flags { w.write_with_tag(160, |w| w.write_enum(*s as i32))?; }
        w.write_with_tag(240, |w| w.write_enum(*&self.platform as i32))?;
        w.write_with_tag(320, |w| w.write_uint64(*&self.version))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LoginCryptoHelloUnion {
    pub diffie_hellman: Option<keyexchange::LoginCryptoDiffieHellmanHello>,
}

impl<'a> MessageRead<'a> for LoginCryptoHelloUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.diffie_hellman = Some(r.read_message::<keyexchange::LoginCryptoDiffieHellmanHello>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LoginCryptoHelloUnion {
    fn get_size(&self) -> usize {
        0
        + self.diffie_hellman.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.diffie_hellman { w.write_with_tag(82, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LoginCryptoDiffieHellmanHello {
    pub gc: Vec<u8>,
    pub server_keys_known: u32,
}

impl<'a> MessageRead<'a> for LoginCryptoDiffieHellmanHello {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.gc = r.read_bytes(bytes)?.to_owned(),
                Ok(160) => msg.server_keys_known = r.read_uint32(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LoginCryptoDiffieHellmanHello {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.gc).len())
        + 2 + sizeof_varint(*(&self.server_keys_known) as u64)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.gc))?;
        w.write_with_tag(160, |w| w.write_uint32(*&self.server_keys_known))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct FeatureSet {
    pub autoupdate2: Option<bool>,
    pub current_location: Option<bool>,
}

impl<'a> MessageRead<'a> for FeatureSet {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.autoupdate2 = Some(r.read_bool(bytes)?),
                Ok(16) => msg.current_location = Some(r.read_bool(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for FeatureSet {
    fn get_size(&self) -> usize {
        0
        + self.autoupdate2.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.current_location.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.autoupdate2 { w.write_with_tag(8, |w| w.write_bool(*s))?; }
        if let Some(ref s) = self.current_location { w.write_with_tag(16, |w| w.write_bool(*s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct APResponseMessage {
    pub challenge: Option<keyexchange::APChallenge>,
    pub upgrade: Option<keyexchange::UpgradeRequiredMessage>,
    pub login_failed: Option<keyexchange::APLoginFailed>,
}

impl<'a> MessageRead<'a> for APResponseMessage {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.challenge = Some(r.read_message::<keyexchange::APChallenge>(bytes)?),
                Ok(162) => msg.upgrade = Some(r.read_message::<keyexchange::UpgradeRequiredMessage>(bytes)?),
                Ok(242) => msg.login_failed = Some(r.read_message::<keyexchange::APLoginFailed>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for APResponseMessage {
    fn get_size(&self) -> usize {
        0
        + self.challenge.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.upgrade.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
        + self.login_failed.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.challenge { w.write_with_tag(82, |w| w.write_message(s))?; }
        if let Some(ref s) = self.upgrade { w.write_with_tag(162, |w| w.write_message(s))?; }
        if let Some(ref s) = self.login_failed { w.write_with_tag(242, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct APChallenge {
    pub login_crypto_challenge: keyexchange::LoginCryptoChallengeUnion,
    pub fingerprint_challenge: keyexchange::FingerprintChallengeUnion,
    pub pow_challenge: keyexchange::PoWChallengeUnion,
    pub crypto_challenge: keyexchange::CryptoChallengeUnion,
    pub server_nonce: Vec<u8>,
    pub padding: Option<Vec<u8>>,
}

impl<'a> MessageRead<'a> for APChallenge {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.login_crypto_challenge = r.read_message::<keyexchange::LoginCryptoChallengeUnion>(bytes)?,
                Ok(162) => msg.fingerprint_challenge = r.read_message::<keyexchange::FingerprintChallengeUnion>(bytes)?,
                Ok(242) => msg.pow_challenge = r.read_message::<keyexchange::PoWChallengeUnion>(bytes)?,
                Ok(322) => msg.crypto_challenge = r.read_message::<keyexchange::CryptoChallengeUnion>(bytes)?,
                Ok(402) => msg.server_nonce = r.read_bytes(bytes)?.to_owned(),
                Ok(482) => msg.padding = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for APChallenge {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.login_crypto_challenge).get_size())
        + 2 + sizeof_len((&self.fingerprint_challenge).get_size())
        + 2 + sizeof_len((&self.pow_challenge).get_size())
        + 2 + sizeof_len((&self.crypto_challenge).get_size())
        + 2 + sizeof_len((&self.server_nonce).len())
        + self.padding.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_message(&self.login_crypto_challenge))?;
        w.write_with_tag(162, |w| w.write_message(&self.fingerprint_challenge))?;
        w.write_with_tag(242, |w| w.write_message(&self.pow_challenge))?;
        w.write_with_tag(322, |w| w.write_message(&self.crypto_challenge))?;
        w.write_with_tag(402, |w| w.write_bytes(&**&self.server_nonce))?;
        if let Some(ref s) = self.padding { w.write_with_tag(482, |w| w.write_bytes(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LoginCryptoChallengeUnion {
    pub diffie_hellman: Option<keyexchange::LoginCryptoDiffieHellmanChallenge>,
}

impl<'a> MessageRead<'a> for LoginCryptoChallengeUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.diffie_hellman = Some(r.read_message::<keyexchange::LoginCryptoDiffieHellmanChallenge>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LoginCryptoChallengeUnion {
    fn get_size(&self) -> usize {
        0
        + self.diffie_hellman.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.diffie_hellman { w.write_with_tag(82, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LoginCryptoDiffieHellmanChallenge {
    pub gs: Vec<u8>,
    pub server_signature_key: i32,
    pub gs_signature: Vec<u8>,
}

impl<'a> MessageRead<'a> for LoginCryptoDiffieHellmanChallenge {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.gs = r.read_bytes(bytes)?.to_owned(),
                Ok(160) => msg.server_signature_key = r.read_int32(bytes)?,
                Ok(242) => msg.gs_signature = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LoginCryptoDiffieHellmanChallenge {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.gs).len())
        + 2 + sizeof_varint(*(&self.server_signature_key) as u64)
        + 2 + sizeof_len((&self.gs_signature).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.gs))?;
        w.write_with_tag(160, |w| w.write_int32(*&self.server_signature_key))?;
        w.write_with_tag(242, |w| w.write_bytes(&**&self.gs_signature))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct FingerprintChallengeUnion {
    pub grain: Option<keyexchange::FingerprintGrainChallenge>,
    pub hmac_ripemd: Option<keyexchange::FingerprintHmacRipemdChallenge>,
}

impl<'a> MessageRead<'a> for FingerprintChallengeUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.grain = Some(r.read_message::<keyexchange::FingerprintGrainChallenge>(bytes)?),
                Ok(162) => msg.hmac_ripemd = Some(r.read_message::<keyexchange::FingerprintHmacRipemdChallenge>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for FingerprintChallengeUnion {
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
pub struct FingerprintGrainChallenge {
    pub kek: Vec<u8>,
}

impl<'a> MessageRead<'a> for FingerprintGrainChallenge {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.kek = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for FingerprintGrainChallenge {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.kek).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.kek))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct FingerprintHmacRipemdChallenge {
    pub challenge: Vec<u8>,
}

impl<'a> MessageRead<'a> for FingerprintHmacRipemdChallenge {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.challenge = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for FingerprintHmacRipemdChallenge {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.challenge).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.challenge))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PoWChallengeUnion {
    pub hash_cash: Option<keyexchange::PoWHashCashChallenge>,
}

impl<'a> MessageRead<'a> for PoWChallengeUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.hash_cash = Some(r.read_message::<keyexchange::PoWHashCashChallenge>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for PoWChallengeUnion {
    fn get_size(&self) -> usize {
        0
        + self.hash_cash.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.hash_cash { w.write_with_tag(82, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PoWHashCashChallenge {
    pub prefix: Option<Vec<u8>>,
    pub length: Option<i32>,
    pub target: Option<i32>,
}

impl<'a> MessageRead<'a> for PoWHashCashChallenge {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.prefix = Some(r.read_bytes(bytes)?.to_owned()),
                Ok(160) => msg.length = Some(r.read_int32(bytes)?),
                Ok(240) => msg.target = Some(r.read_int32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for PoWHashCashChallenge {
    fn get_size(&self) -> usize {
        0
        + self.prefix.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.length.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.target.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.prefix { w.write_with_tag(82, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.length { w.write_with_tag(160, |w| w.write_int32(*s))?; }
        if let Some(ref s) = self.target { w.write_with_tag(240, |w| w.write_int32(*s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CryptoChallengeUnion {
    pub shannon: Option<keyexchange::CryptoShannonChallenge>,
    pub rc4_sha1_hmac: Option<keyexchange::CryptoRc4Sha1HmacChallenge>,
}

impl<'a> MessageRead<'a> for CryptoChallengeUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.shannon = Some(r.read_message::<keyexchange::CryptoShannonChallenge>(bytes)?),
                Ok(162) => msg.rc4_sha1_hmac = Some(r.read_message::<keyexchange::CryptoRc4Sha1HmacChallenge>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for CryptoChallengeUnion {
    fn get_size(&self) -> usize {
        0
        + self.shannon.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.rc4_sha1_hmac.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.shannon { w.write_with_tag(82, |w| w.write_message(s))?; }
        if let Some(ref s) = self.rc4_sha1_hmac { w.write_with_tag(162, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CryptoShannonChallenge { }

impl<'a> MessageRead<'a> for CryptoShannonChallenge {
    fn from_reader(r: &mut BytesReader, _: &[u8]) -> Result<Self> {
        r.read_to_end();
        Ok(Self::default())
    }
}

impl MessageWrite for CryptoShannonChallenge { }

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CryptoRc4Sha1HmacChallenge { }

impl<'a> MessageRead<'a> for CryptoRc4Sha1HmacChallenge {
    fn from_reader(r: &mut BytesReader, _: &[u8]) -> Result<Self> {
        r.read_to_end();
        Ok(Self::default())
    }
}

impl MessageWrite for CryptoRc4Sha1HmacChallenge { }

#[derive(Debug, Default, PartialEq, Clone)]
pub struct UpgradeRequiredMessage {
    pub upgrade_signed_part: Vec<u8>,
    pub signature: Vec<u8>,
    pub http_suffix: Option<String>,
}

impl<'a> MessageRead<'a> for UpgradeRequiredMessage {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.upgrade_signed_part = r.read_bytes(bytes)?.to_owned(),
                Ok(162) => msg.signature = r.read_bytes(bytes)?.to_owned(),
                Ok(242) => msg.http_suffix = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for UpgradeRequiredMessage {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.upgrade_signed_part).len())
        + 2 + sizeof_len((&self.signature).len())
        + self.http_suffix.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.upgrade_signed_part))?;
        w.write_with_tag(162, |w| w.write_bytes(&**&self.signature))?;
        if let Some(ref s) = self.http_suffix { w.write_with_tag(242, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct APLoginFailed {
    pub error_code: keyexchange::ErrorCode,
    pub retry_delay: Option<i32>,
    pub expiry: Option<i32>,
    pub error_description: Option<String>,
}

impl<'a> MessageRead<'a> for APLoginFailed {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(80) => msg.error_code = r.read_enum(bytes)?,
                Ok(160) => msg.retry_delay = Some(r.read_int32(bytes)?),
                Ok(240) => msg.expiry = Some(r.read_int32(bytes)?),
                Ok(322) => msg.error_description = Some(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for APLoginFailed {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_varint(*(&self.error_code) as u64)
        + self.retry_delay.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.expiry.as_ref().map_or(0, |m| 2 + sizeof_varint(*(m) as u64))
        + self.error_description.as_ref().map_or(0, |m| 2 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(80, |w| w.write_enum(*&self.error_code as i32))?;
        if let Some(ref s) = self.retry_delay { w.write_with_tag(160, |w| w.write_int32(*s))?; }
        if let Some(ref s) = self.expiry { w.write_with_tag(240, |w| w.write_int32(*s))?; }
        if let Some(ref s) = self.error_description { w.write_with_tag(322, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ClientResponsePlaintext {
    pub login_crypto_response: keyexchange::LoginCryptoResponseUnion,
    pub pow_response: keyexchange::PoWResponseUnion,
    pub crypto_response: keyexchange::CryptoResponseUnion,
}

impl<'a> MessageRead<'a> for ClientResponsePlaintext {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.login_crypto_response = r.read_message::<keyexchange::LoginCryptoResponseUnion>(bytes)?,
                Ok(162) => msg.pow_response = r.read_message::<keyexchange::PoWResponseUnion>(bytes)?,
                Ok(242) => msg.crypto_response = r.read_message::<keyexchange::CryptoResponseUnion>(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ClientResponsePlaintext {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.login_crypto_response).get_size())
        + 2 + sizeof_len((&self.pow_response).get_size())
        + 2 + sizeof_len((&self.crypto_response).get_size())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_message(&self.login_crypto_response))?;
        w.write_with_tag(162, |w| w.write_message(&self.pow_response))?;
        w.write_with_tag(242, |w| w.write_message(&self.crypto_response))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LoginCryptoResponseUnion {
    pub diffie_hellman: Option<keyexchange::LoginCryptoDiffieHellmanResponse>,
}

impl<'a> MessageRead<'a> for LoginCryptoResponseUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.diffie_hellman = Some(r.read_message::<keyexchange::LoginCryptoDiffieHellmanResponse>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LoginCryptoResponseUnion {
    fn get_size(&self) -> usize {
        0
        + self.diffie_hellman.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.diffie_hellman { w.write_with_tag(82, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct LoginCryptoDiffieHellmanResponse {
    pub hmac: Vec<u8>,
}

impl<'a> MessageRead<'a> for LoginCryptoDiffieHellmanResponse {
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

impl MessageWrite for LoginCryptoDiffieHellmanResponse {
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
pub struct PoWResponseUnion {
    pub hash_cash: Option<keyexchange::PoWHashCashResponse>,
}

impl<'a> MessageRead<'a> for PoWResponseUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.hash_cash = Some(r.read_message::<keyexchange::PoWHashCashResponse>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for PoWResponseUnion {
    fn get_size(&self) -> usize {
        0
        + self.hash_cash.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.hash_cash { w.write_with_tag(82, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PoWHashCashResponse {
    pub hash_suffix: Vec<u8>,
}

impl<'a> MessageRead<'a> for PoWHashCashResponse {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.hash_suffix = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for PoWHashCashResponse {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.hash_suffix).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(82, |w| w.write_bytes(&**&self.hash_suffix))?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CryptoResponseUnion {
    pub shannon: Option<keyexchange::CryptoShannonResponse>,
    pub rc4_sha1_hmac: Option<keyexchange::CryptoRc4Sha1HmacResponse>,
}

impl<'a> MessageRead<'a> for CryptoResponseUnion {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(82) => msg.shannon = Some(r.read_message::<keyexchange::CryptoShannonResponse>(bytes)?),
                Ok(162) => msg.rc4_sha1_hmac = Some(r.read_message::<keyexchange::CryptoRc4Sha1HmacResponse>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for CryptoResponseUnion {
    fn get_size(&self) -> usize {
        0
        + self.shannon.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.rc4_sha1_hmac.as_ref().map_or(0, |m| 2 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.shannon { w.write_with_tag(82, |w| w.write_message(s))?; }
        if let Some(ref s) = self.rc4_sha1_hmac { w.write_with_tag(162, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CryptoShannonResponse {
    pub dummy: Option<i32>,
}

impl<'a> MessageRead<'a> for CryptoShannonResponse {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.dummy = Some(r.read_int32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for CryptoShannonResponse {
    fn get_size(&self) -> usize {
        0
        + self.dummy.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.dummy { w.write_with_tag(8, |w| w.write_int32(*s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CryptoRc4Sha1HmacResponse {
    pub dummy: Option<i32>,
}

impl<'a> MessageRead<'a> for CryptoRc4Sha1HmacResponse {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.dummy = Some(r.read_int32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for CryptoRc4Sha1HmacResponse {
    fn get_size(&self) -> usize {
        0
        + self.dummy.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.dummy { w.write_with_tag(8, |w| w.write_int32(*s))?; }
        Ok(())
    }
}

