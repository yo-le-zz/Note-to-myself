//! Primitives de chiffrement de l'application.
//!
//! - chiffrement authentifié : AES-256-GCM (défaut) ou ChaCha20-Poly1305 ;
//! - dérivation de clé depuis un mot de passe : Argon2id (défaut), scrypt ou
//!   PBKDF2-HMAC-SHA256, avec un fichier-clé optionnel mélangé au mot de passe ;
//! - tout le matériel secret est effacé de la mémoire à la destruction (`Zeroize`).

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::ChaCha20Poly1305;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
pub const SALT_LEN: usize = 16;
/// Préfixe des valeurs chiffrées stockées en base.
pub const ENC_PREFIX: &str = "enc:v1:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Mot de passe (ou fichier-clé) incorrect, ou données altérées.
    Decrypt,
    InvalidFormat,
    InvalidParams(String),
    Kdf(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decrypt => write!(f, "decryption failed (wrong password or corrupted data)"),
            Self::InvalidFormat => write!(f, "invalid encrypted data format"),
            Self::InvalidParams(m) => write!(f, "invalid parameters: {m}"),
            Self::Kdf(m) => write!(f, "key derivation failed: {m}"),
        }
    }
}

impl std::error::Error for CryptoError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Cipher {
    #[default]
    Aes256Gcm,
    ChaCha20Poly1305,
}

impl Cipher {
    pub const ALL: [Cipher; 2] = [Cipher::Aes256Gcm, Cipher::ChaCha20Poly1305];
}

impl fmt::Display for Cipher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Aes256Gcm => "AES-256-GCM",
            Self::ChaCha20Poly1305 => "ChaCha20-Poly1305",
        })
    }
}

/// Algorithme et coût de la dérivation de clé. Les paramètres sont stockés avec
/// la configuration : ils ne sont pas secrets et doivent suivre la base lors d'un transfert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kdf {
    Argon2id { m_kib: u32, t: u32, p: u32 },
    Scrypt { log_n: u8, r: u32, p: u32 },
    Pbkdf2 { iterations: u32 },
}

impl Default for Kdf {
    fn default() -> Self {
        Self::Argon2id {
            m_kib: 65_536,
            t: 3,
            p: 1,
        }
    }
}

/// Choix simplifiés présentés dans l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdfPreset {
    Argon2Standard,
    Argon2Strong,
    ScryptStandard,
    Pbkdf2Standard,
}

impl KdfPreset {
    pub const ALL: [KdfPreset; 4] = [
        Self::Argon2Standard,
        Self::Argon2Strong,
        Self::ScryptStandard,
        Self::Pbkdf2Standard,
    ];

    pub fn kdf(self) -> Kdf {
        match self {
            Self::Argon2Standard => Kdf::Argon2id {
                m_kib: 65_536,
                t: 3,
                p: 1,
            },
            Self::Argon2Strong => Kdf::Argon2id {
                m_kib: 262_144,
                t: 4,
                p: 2,
            },
            Self::ScryptStandard => Kdf::Scrypt {
                log_n: 16,
                r: 8,
                p: 1,
            },
            Self::Pbkdf2Standard => Kdf::Pbkdf2 {
                iterations: 600_000,
            },
        }
    }

    pub fn of(kdf: Kdf) -> Self {
        match kdf {
            Kdf::Argon2id { m_kib, .. } if m_kib >= 200_000 => Self::Argon2Strong,
            Kdf::Argon2id { .. } => Self::Argon2Standard,
            Kdf::Scrypt { .. } => Self::ScryptStandard,
            Kdf::Pbkdf2 { .. } => Self::Pbkdf2Standard,
        }
    }
}

impl fmt::Display for KdfPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Argon2Standard => "Argon2id (64 MiB, 3 passes)",
            Self::Argon2Strong => "Argon2id strong (256 MiB, 4 passes)",
            Self::ScryptStandard => "scrypt (N=2^16, r=8)",
            Self::Pbkdf2Standard => "PBKDF2-HMAC-SHA256 (600 000)",
        })
    }
}

impl Kdf {
    /// Refuse les paramètres trop faibles (fichier de config modifié) ou absurdes (déni de service).
    pub fn validate(&self) -> Result<(), CryptoError> {
        let bad = |m: &str| Err(CryptoError::InvalidParams(m.to_string()));
        match *self {
            Self::Argon2id { m_kib, t, p } => {
                if !(8_192..=1_048_576).contains(&m_kib) {
                    return bad("argon2 memory must be between 8 MiB and 1 GiB");
                }
                if !(1..=16).contains(&t) || !(1..=16).contains(&p) {
                    return bad("argon2 passes/lanes out of range");
                }
            }
            Self::Scrypt { log_n, r, p } => {
                if !(14..=22).contains(&log_n) || !(1..=32).contains(&r) || !(1..=16).contains(&p) {
                    return bad("scrypt parameters out of range");
                }
                // Mémoire nécessaire : 128 × r × N octets. Au-delà de 1 Gio, un paramètre reçu d'un
                // autre PC ou d'un fichier modifié pourrait faire planter l'application.
                if 128u64 * r as u64 * (1u64 << log_n) > 1 << 30 {
                    return bad("scrypt memory above 1 GiB");
                }
            }
            Self::Pbkdf2 { iterations } => {
                if !(100_000..=50_000_000).contains(&iterations) {
                    return bad("pbkdf2 iterations out of range");
                }
            }
        }
        Ok(())
    }
}

/// Clé symétrique de 256 bits, effacée de la mémoire à la destruction.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Key([u8; KEY_LEN]);

impl Key {
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    /// Sous-clé indépendante pour un usage précis (HKDF-SHA256).
    pub fn subkey(&self, context: &str) -> Key {
        let hk = Hkdf::<Sha256>::new(None, &self.0);
        let mut out = [0u8; KEY_LEN];
        // 32 octets est toujours une longueur valide pour HKDF-SHA256.
        hk.expand(context.as_bytes(), &mut out)
            .expect("hkdf length");
        Key(out)
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key(***)")
    }
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    OsRng.fill_bytes(&mut buf);
    buf
}

pub fn random_salt() -> [u8; SALT_LEN] {
    random_bytes::<SALT_LEN>()
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn b64_encode(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

pub fn b64_decode(text: &str) -> Result<Vec<u8>, CryptoError> {
    B64.decode(text.trim())
        .map_err(|_| CryptoError::InvalidFormat)
}

/// Dérive la clé principale. Le fichier-clé (optionnel) est condensé en SHA-256
/// puis ajouté au mot de passe : sans lui, le mot de passe seul ne suffit plus.
pub fn derive_key(
    password: &str,
    keyfile: Option<&[u8]>,
    salt: &[u8],
    kdf: &Kdf,
) -> Result<Key, CryptoError> {
    kdf.validate()?;
    if salt.len() < 8 {
        return Err(CryptoError::InvalidParams("salt too short".into()));
    }
    let mut material = password.as_bytes().to_vec();
    if let Some(file) = keyfile {
        material.extend_from_slice(&sha256(file));
    }
    let mut out = [0u8; KEY_LEN];
    let result = match *kdf {
        Kdf::Argon2id { m_kib, t, p } => argon2::Params::new(m_kib, t, p, Some(KEY_LEN))
            .map_err(|e| CryptoError::Kdf(e.to_string()))
            .and_then(|params| {
                argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
                    .hash_password_into(&material, salt, &mut out)
                    .map_err(|e| CryptoError::Kdf(e.to_string()))
            }),
        Kdf::Scrypt { log_n, r, p } => scrypt::Params::new(log_n, r, p, KEY_LEN)
            .map_err(|e| CryptoError::Kdf(e.to_string()))
            .and_then(|params| {
                scrypt::scrypt(&material, salt, &params, &mut out)
                    .map_err(|e| CryptoError::Kdf(e.to_string()))
            }),
        Kdf::Pbkdf2 { iterations } => {
            pbkdf2::pbkdf2_hmac::<Sha256>(&material, salt, iterations, &mut out);
            Ok(())
        }
    };
    material.zeroize();
    result?;
    Ok(Key(out))
}

/// Chiffre `plaintext` ; `aad` lie le chiffré à son contexte (ex. l'identifiant de la note).
/// Format : `nonce (12 octets) || chiffré+tag`.
pub fn seal(
    key: &Key,
    cipher: Cipher,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let nonce_bytes = random_bytes::<NONCE_LEN>();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let payload = Payload {
        msg: plaintext,
        aad,
    };
    let sealed = match cipher {
        Cipher::Aes256Gcm => Aes256Gcm::new_from_slice(key.as_bytes())
            .map_err(|_| CryptoError::InvalidParams("key".into()))?
            .encrypt(nonce, payload),
        Cipher::ChaCha20Poly1305 => ChaCha20Poly1305::new_from_slice(key.as_bytes())
            .map_err(|_| CryptoError::InvalidParams("key".into()))?
            .encrypt(nonce, payload),
    }
    .map_err(|_| CryptoError::Decrypt)?;
    let mut out = Vec::with_capacity(NONCE_LEN + sealed.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&sealed);
    Ok(out)
}

pub fn open(key: &Key, cipher: Cipher, sealed: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if sealed.len() < NONCE_LEN + 16 {
        return Err(CryptoError::InvalidFormat);
    }
    let (nonce_bytes, body) = sealed.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let payload = Payload { msg: body, aad };
    match cipher {
        Cipher::Aes256Gcm => Aes256Gcm::new_from_slice(key.as_bytes())
            .map_err(|_| CryptoError::InvalidParams("key".into()))?
            .decrypt(nonce, payload),
        Cipher::ChaCha20Poly1305 => ChaCha20Poly1305::new_from_slice(key.as_bytes())
            .map_err(|_| CryptoError::InvalidParams("key".into()))?
            .decrypt(nonce, payload),
    }
    .map_err(|_| CryptoError::Decrypt)
}

/// Chiffre un texte vers la forme stockable `enc:v1:<base64>`.
pub fn seal_text(key: &Key, cipher: Cipher, text: &str, aad: &str) -> Result<String, CryptoError> {
    let sealed = seal(key, cipher, text.as_bytes(), aad.as_bytes())?;
    Ok(format!("{ENC_PREFIX}{}", b64_encode(&sealed)))
}

pub fn open_text(
    key: &Key,
    cipher: Cipher,
    stored: &str,
    aad: &str,
) -> Result<String, CryptoError> {
    let encoded = stored
        .strip_prefix(ENC_PREFIX)
        .ok_or(CryptoError::InvalidFormat)?;
    let sealed = b64_decode(encoded)?;
    let plain = open(key, cipher, &sealed, aad.as_bytes())?;
    String::from_utf8(plain).map_err(|_| CryptoError::InvalidFormat)
}

pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

/// Comparaison en temps constant.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Évalue la robustesse d'un mot de passe : 0 (très faible) à 4 (très fort).
/// Heuristique simple (longueur et variété) — pas une garantie.
pub fn password_strength(password: &str) -> u8 {
    let len = password.chars().count();
    let classes = [
        password.chars().any(|c| c.is_lowercase()),
        password.chars().any(|c| c.is_uppercase()),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_alphanumeric()),
    ]
    .iter()
    .filter(|&&b| b)
    .count();
    let unique = password
        .chars()
        .collect::<std::collections::HashSet<_>>()
        .len();
    if len < 8 || unique < 5 {
        0
    } else if len < 10 || classes < 2 {
        1
    } else if len < 14 || classes < 3 {
        2
    } else if len < 20 || classes < 4 {
        3
    } else {
        4
    }
}

/// Génère un mot de passe aléatoire (alphabet sans caractères ambigus).
pub fn generate_password(len: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789-_.!?@#%";
    let len = len.clamp(12, 128);
    let mut out = String::with_capacity(len);
    while out.len() < len {
        let byte = random_bytes::<1>()[0] as usize;
        // Rejet des valeurs hors de la plus grande plage multiple de la taille : pas de biais.
        if byte < 256 - (256 % ALPHABET.len()) {
            out.push(ALPHABET[byte % ALPHABET.len()] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_kdfs() -> Vec<Kdf> {
        vec![
            Kdf::Argon2id {
                m_kib: 8_192,
                t: 1,
                p: 1,
            },
            Kdf::Scrypt {
                log_n: 14,
                r: 8,
                p: 1,
            },
            Kdf::Pbkdf2 {
                iterations: 100_000,
            },
        ]
    }

    #[test]
    fn roundtrip_every_cipher() {
        let key = Key::from_bytes(random_bytes::<32>());
        for cipher in Cipher::ALL {
            let sealed = seal_text(&key, cipher, "bonjour é 日本", "note-1").unwrap();
            assert!(is_encrypted(&sealed));
            assert_eq!(
                open_text(&key, cipher, &sealed, "note-1").unwrap(),
                "bonjour é 日本"
            );
        }
    }

    #[test]
    fn wrong_key_aad_or_tamper_is_rejected() {
        let key = Key::from_bytes(random_bytes::<32>());
        let other = Key::from_bytes(random_bytes::<32>());
        let sealed = seal_text(&key, Cipher::Aes256Gcm, "secret", "a").unwrap();
        assert_eq!(
            open_text(&other, Cipher::Aes256Gcm, &sealed, "a"),
            Err(CryptoError::Decrypt)
        );
        assert_eq!(
            open_text(&key, Cipher::Aes256Gcm, &sealed, "b"),
            Err(CryptoError::Decrypt)
        );
        // Mauvais algorithme.
        assert!(open_text(&key, Cipher::ChaCha20Poly1305, &sealed, "a").is_err());
        // Un octet modifié.
        let mut raw = b64_decode(sealed.strip_prefix(ENC_PREFIX).unwrap()).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 1;
        let tampered = format!("{ENC_PREFIX}{}", b64_encode(&raw));
        assert_eq!(
            open_text(&key, Cipher::Aes256Gcm, &tampered, "a"),
            Err(CryptoError::Decrypt)
        );
    }

    #[test]
    fn nonces_are_unique() {
        let key = Key::from_bytes(random_bytes::<32>());
        let a = seal_text(&key, Cipher::Aes256Gcm, "x", "a").unwrap();
        let b = seal_text(&key, Cipher::Aes256Gcm, "x", "a").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn truncated_or_garbage_input_is_an_error_not_a_panic() {
        let key = Key::from_bytes(random_bytes::<32>());
        assert!(open_text(&key, Cipher::Aes256Gcm, "enc:v1:", "a").is_err());
        assert!(open_text(&key, Cipher::Aes256Gcm, "enc:v1:@@@", "a").is_err());
        assert!(open_text(&key, Cipher::Aes256Gcm, "plain", "a").is_err());
    }

    #[test]
    fn kdf_is_deterministic_and_sensitive_to_every_input() {
        let salt = random_salt();
        for kdf in fast_kdfs() {
            let a = derive_key("motdepasse-long", None, &salt, &kdf).unwrap();
            let b = derive_key("motdepasse-long", None, &salt, &kdf).unwrap();
            assert_eq!(a.as_bytes(), b.as_bytes());
            let other_pw = derive_key("autre-motdepasse", None, &salt, &kdf).unwrap();
            assert_ne!(a.as_bytes(), other_pw.as_bytes());
            let other_salt = derive_key("motdepasse-long", None, &random_salt(), &kdf).unwrap();
            assert_ne!(a.as_bytes(), other_salt.as_bytes());
            let with_file = derive_key("motdepasse-long", Some(b"keyfile"), &salt, &kdf).unwrap();
            assert_ne!(a.as_bytes(), with_file.as_bytes());
        }
    }

    #[test]
    fn weak_or_absurd_kdf_parameters_are_refused() {
        let salt = random_salt();
        for kdf in [
            Kdf::Argon2id {
                m_kib: 8,
                t: 1,
                p: 1,
            },
            Kdf::Argon2id {
                m_kib: 2_000_000,
                t: 1,
                p: 1,
            },
            Kdf::Scrypt {
                log_n: 22,
                r: 32,
                p: 1,
            },
            Kdf::Scrypt {
                log_n: 4,
                r: 8,
                p: 1,
            },
            Kdf::Pbkdf2 { iterations: 1 },
        ] {
            assert!(derive_key("x", None, &salt, &kdf).is_err(), "{kdf:?}");
        }
        assert!(derive_key("x", None, b"1234", &Kdf::default()).is_err());
    }

    #[test]
    fn subkeys_are_independent() {
        let key = Key::from_bytes(random_bytes::<32>());
        assert_ne!(key.subkey("a").as_bytes(), key.subkey("b").as_bytes());
        assert_eq!(key.subkey("a").as_bytes(), key.subkey("a").as_bytes());
    }

    #[test]
    fn password_helpers() {
        assert_eq!(password_strength("abc"), 0);
        assert_eq!(password_strength("aaaaaaaaaaaaaaaa"), 0);
        assert!(password_strength("Tr0ub4dor&3-xyz-correct") >= 3);
        let generated = generate_password(24);
        assert_eq!(generated.chars().count(), 24);
        assert_ne!(generated, generate_password(24));
        assert_eq!(generate_password(1).chars().count(), 12);
    }

    #[test]
    fn constant_time_equality() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }
}
