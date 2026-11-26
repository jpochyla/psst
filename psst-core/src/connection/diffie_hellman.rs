use num_bigint::BigUint;

pub struct DHLocalKeys {
    private_key: BigUint,
    public_key: BigUint,
}

impl DHLocalKeys {
    pub fn random() -> DHLocalKeys {
        // Spotify uses a legacy 768-bit DH group. A 32-bit exponent exposes
        // session keys to brute force; use cryptographically random 760 bits.
        let private_key = BigUint::from_bytes_be(&rand::random::<[u8; 95]>());
        let public_key = dh_generator().modpow(&private_key, &dh_prime());
        DHLocalKeys {
            private_key,
            public_key,
        }
    }

    pub fn public_key(&self) -> Vec<u8> {
        self.public_key.to_bytes_be()
    }

    pub fn shared_secret(&self, remote_key: &[u8]) -> Vec<u8> {
        let remote_key = BigUint::from_bytes_be(remote_key);
        let shared_key = remote_key.modpow(&self.private_key, &dh_prime());
        shared_key.to_bytes_be()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secrets_have_sufficient_entropy_and_agree() {
        let first = DHLocalKeys::random();
        let second = DHLocalKeys::random();
        assert!(first.private_key.bits() > 512);
        assert!(second.private_key.bits() > 512);
        assert_eq!(first.shared_secret(&second.public_key()), second.shared_secret(&first.public_key()));
    }
}

fn dh_generator() -> BigUint {
    BigUint::from(0x2_u64)
}

fn dh_prime() -> BigUint {
    BigUint::from_bytes_be(&[
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xc9, 0x0f, 0xda, 0xa2, 0x21, 0x68, 0xc2,
        0x34, 0xc4, 0xc6, 0x62, 0x8b, 0x80, 0xdc, 0x1c, 0xd1, 0x29, 0x02, 0x4e, 0x08, 0x8a, 0x67,
        0xcc, 0x74, 0x02, 0x0b, 0xbe, 0xa6, 0x3b, 0x13, 0x9b, 0x22, 0x51, 0x4a, 0x08, 0x79, 0x8e,
        0x34, 0x04, 0xdd, 0xef, 0x95, 0x19, 0xb3, 0xcd, 0x3a, 0x43, 0x1b, 0x30, 0x2b, 0x0a, 0x6d,
        0xf2, 0x5f, 0x14, 0x37, 0x4f, 0xe1, 0x35, 0x6d, 0x6d, 0x51, 0xc2, 0x45, 0xe4, 0x85, 0xb5,
        0x76, 0x62, 0x5e, 0x7e, 0xc6, 0xf4, 0x4c, 0x42, 0xe9, 0xa6, 0x3a, 0x36, 0x20, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ])
}
