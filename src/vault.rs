//! Coffre chiffré : configuration, déverrouillage, chiffrement de toute la base et sauvegardes.
//!
//! Le mot de passe n'est jamais stocké. Seuls le sel, les paramètres de dérivation et un
//! « vérificateur » (texte connu chiffré) sont enregistrés, ce qui permet de dire si le mot de
//! passe est bon sans exposer la clé.

use crate::crypto::{self, Cipher, CryptoError, Kdf, Key};
use crate::settings::{SecuritySettings, Settings};
use crate::store::{self, Record};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const VERIFIER_TEXT: &str = "note-to-myself-vault-v1";
const VERIFIER_AAD: &str = "verifier";

#[derive(Debug)]
pub enum VaultError {
    Crypto(CryptoError),
    Database(rusqlite::Error),
    /// Le coffre n'est pas configuré ou la configuration est incomplète.
    NotConfigured,
    WrongPassword,
    Format(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Crypto(e) => write!(f, "{e}"),
            Self::Database(e) => write!(f, "database error: {e}"),
            Self::NotConfigured => write!(f, "the vault is not configured"),
            Self::WrongPassword => write!(f, "wrong password or key file"),
            Self::Format(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<CryptoError> for VaultError {
    fn from(value: CryptoError) -> Self {
        Self::Crypto(value)
    }
}

impl From<rusqlite::Error> for VaultError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value)
    }
}

fn field_aad(uid: &str, field: &str) -> String {
    format!("{uid}:{field}")
}

/// Crée une configuration de coffre neuve (sel aléatoire) et retourne la clé correspondante.
pub fn create(
    password: &str,
    keyfile: Option<&[u8]>,
    cipher: Cipher,
    kdf: Kdf,
    previous: &SecuritySettings,
) -> Result<(SecuritySettings, Key), VaultError> {
    let salt = crypto::random_salt();
    let key = crypto::derive_key(password, keyfile, &salt, &kdf)?;
    let verifier = crypto::seal_text(&key, cipher, VERIFIER_TEXT, VERIFIER_AAD)?;
    let settings = SecuritySettings {
        enabled: true,
        cipher,
        kdf,
        salt: crypto::b64_encode(&salt),
        verifier,
        uses_keyfile: keyfile.is_some(),
        auto_lock_minutes: previous.auto_lock_minutes,
    };
    Ok((settings, key))
}

/// Dérive la clé depuis le mot de passe et vérifie qu'elle ouvre bien le vérificateur.
pub fn unlock(
    settings: &SecuritySettings,
    password: &str,
    keyfile: Option<&[u8]>,
) -> Result<Key, VaultError> {
    if !settings.enabled || settings.salt.is_empty() || settings.verifier.is_empty() {
        return Err(VaultError::NotConfigured);
    }
    if settings.uses_keyfile && keyfile.is_none() {
        return Err(VaultError::WrongPassword);
    }
    let salt = crypto::b64_decode(&settings.salt)?;
    let key = crypto::derive_key(password, keyfile, &salt, &settings.kdf)?;
    match crypto::open_text(&key, settings.cipher, &settings.verifier, VERIFIER_AAD) {
        Ok(text) if text == VERIFIER_TEXT => Ok(key),
        _ => Err(VaultError::WrongPassword),
    }
}

pub fn encrypt_record_fields(
    key: &Key,
    cipher: Cipher,
    record: &mut Record,
) -> Result<(), VaultError> {
    if record.deleted {
        return Ok(());
    }
    if !crypto::is_encrypted(&record.title) {
        record.title =
            crypto::seal_text(key, cipher, &record.title, &field_aad(&record.uid, "title"))?;
    }
    if !crypto::is_encrypted(&record.body) {
        record.body =
            crypto::seal_text(key, cipher, &record.body, &field_aad(&record.uid, "body"))?;
    }
    Ok(())
}

pub fn decrypt_record_fields(
    key: &Key,
    cipher: Cipher,
    record: &mut Record,
) -> Result<(), VaultError> {
    if record.deleted {
        return Ok(());
    }
    if crypto::is_encrypted(&record.title) {
        record.title =
            crypto::open_text(key, cipher, &record.title, &field_aad(&record.uid, "title"))?;
    }
    if crypto::is_encrypted(&record.body) {
        record.body =
            crypto::open_text(key, cipher, &record.body, &field_aad(&record.uid, "body"))?;
    }
    Ok(())
}

/// Chiffre une valeur isolée (stockage des plugins).
pub fn encrypt_value(
    key: &Key,
    cipher: Cipher,
    plugin: &str,
    name: &str,
    value: &str,
) -> Result<String, VaultError> {
    Ok(crypto::seal_text(
        key,
        cipher,
        value,
        &format!("kv:{plugin}:{name}"),
    )?)
}

pub fn decrypt_value(
    key: &Key,
    cipher: Cipher,
    plugin: &str,
    name: &str,
    stored: &str,
) -> Result<String, VaultError> {
    if !crypto::is_encrypted(stored) {
        return Ok(stored.to_string());
    }
    Ok(crypto::open_text(
        key,
        cipher,
        stored,
        &format!("kv:{plugin}:{name}"),
    )?)
}

/// Applique `transform` à chaque note et valeur de plugin dans une seule transaction :
/// tout réussit ou rien ne change.
fn rewrite_all(
    connection: &Connection,
    mut note_fn: impl FnMut(&mut Record) -> Result<(), VaultError>,
    mut value_fn: impl FnMut(&str, &str, &str) -> Result<String, VaultError>,
) -> Result<(), VaultError> {
    let transaction = connection.unchecked_transaction()?;
    for mut record in store::all_records(&transaction)? {
        if record.deleted {
            continue;
        }
        note_fn(&mut record)?;
        transaction.execute(
            "UPDATE notes SET title = ?1, body = ?2 WHERE id = ?3",
            rusqlite::params![record.title, record.body, record.id],
        )?;
    }
    for (plugin, key, value) in store::kv_all(&transaction)? {
        let new_value = value_fn(&plugin, &key, &value)?;
        store::kv_set(&transaction, &plugin, &key, &new_value)?;
    }
    transaction.commit()?;
    Ok(())
}

/// Chiffre toute la base (notes et stockage des plugins) avec la clé donnée.
pub fn encrypt_database(
    connection: &Connection,
    key: &Key,
    cipher: Cipher,
) -> Result<(), VaultError> {
    rewrite_all(
        connection,
        |record| encrypt_record_fields(key, cipher, record),
        |plugin, name, value| {
            if crypto::is_encrypted(value) {
                Ok(value.to_string())
            } else {
                encrypt_value(key, cipher, plugin, name, value)
            }
        },
    )
}

/// Déchiffre toute la base (retire le chiffrement).
pub fn decrypt_database(
    connection: &Connection,
    key: &Key,
    cipher: Cipher,
) -> Result<(), VaultError> {
    rewrite_all(
        connection,
        |record| decrypt_record_fields(key, cipher, record),
        |plugin, name, value| decrypt_value(key, cipher, plugin, name, value),
    )
}

/// Rechiffre toute la base avec une autre clé (changement de mot de passe, d'algorithme ou de KDF).
pub fn rekey_database(
    connection: &Connection,
    old_key: &Key,
    old_cipher: Cipher,
    new_key: &Key,
    new_cipher: Cipher,
) -> Result<(), VaultError> {
    rewrite_all(
        connection,
        |record| {
            decrypt_record_fields(old_key, old_cipher, record)?;
            encrypt_record_fields(new_key, new_cipher, record)
        },
        |plugin, name, value| {
            let plain = decrypt_value(old_key, old_cipher, plugin, name, value)?;
            encrypt_value(new_key, new_cipher, plugin, name, &plain)
        },
    )
}

// --- Sauvegarde chiffrée ----------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct BackupNote {
    pub uid: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct BackupPayload {
    notes: Vec<BackupNote>,
    settings: Option<Settings>,
}

#[derive(Serialize, Deserialize)]
struct BackupEnvelope {
    magic: String,
    cipher: Cipher,
    kdf: Kdf,
    salt: String,
    data: String,
}

const BACKUP_MAGIC: &str = "NTM-BACKUP-1";
const BACKUP_AAD: &str = "ntm-backup";

/// Sauvegarde complète (notes en clair + configuration transférable), chiffrée avec son propre mot de passe.
pub fn export_backup(
    notes: Vec<BackupNote>,
    settings: &Settings,
    password: &str,
    cipher: Cipher,
    kdf: Kdf,
) -> Result<Vec<u8>, VaultError> {
    let salt = crypto::random_salt();
    let key = crypto::derive_key(password, None, &salt, &kdf)?;
    let payload = serde_json::to_string(&BackupPayload {
        notes,
        settings: Some(settings.to_transfer()),
    })
    .map_err(|e| VaultError::Format(e.to_string()))?;
    let data = crypto::seal_text(&key, cipher, &payload, BACKUP_AAD)?;
    let envelope = BackupEnvelope {
        magic: BACKUP_MAGIC.into(),
        cipher,
        kdf,
        salt: crypto::b64_encode(&salt),
        data,
    };
    serde_json::to_vec_pretty(&envelope).map_err(|e| VaultError::Format(e.to_string()))
}

pub fn import_backup(
    bytes: &[u8],
    password: &str,
) -> Result<(Vec<BackupNote>, Option<Settings>), VaultError> {
    let envelope: BackupEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| VaultError::Format("not a backup file".into()))?;
    if envelope.magic != BACKUP_MAGIC {
        return Err(VaultError::Format("unsupported backup version".into()));
    }
    let salt = crypto::b64_decode(&envelope.salt)?;
    let key = crypto::derive_key(password, None, &salt, &envelope.kdf)?;
    let plain = crypto::open_text(&key, envelope.cipher, &envelope.data, BACKUP_AAD)
        .map_err(|_| VaultError::WrongPassword)?;
    let payload: BackupPayload =
        serde_json::from_str(&plain).map_err(|_| VaultError::Format("corrupted backup".into()))?;
    let mut settings = payload.settings;
    if let Some(s) = settings.as_mut() {
        s.sanitize();
    }
    Ok((payload.notes, settings))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: Kdf = Kdf::Argon2id {
        m_kib: 8_192,
        t: 1,
        p: 1,
    };

    fn database_with_notes() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        store::ensure_schema(&connection).unwrap();
        store::insert(&connection, "premier", "corps un", "2026-01-01").unwrap();
        store::insert(&connection, "second", "corps deux", "2026-01-02").unwrap();
        connection
    }

    #[test]
    fn unlock_accepts_only_the_right_credentials() {
        let (settings, key) = create(
            "un bon mot de passe",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        assert!(settings.enabled);
        let again = unlock(&settings, "un bon mot de passe", None).unwrap();
        assert_eq!(key.as_bytes(), again.as_bytes());
        assert!(matches!(
            unlock(&settings, "mauvais", None),
            Err(VaultError::WrongPassword)
        ));
        assert!(matches!(
            unlock(&SecuritySettings::default(), "x", None),
            Err(VaultError::NotConfigured)
        ));
    }

    #[test]
    fn keyfile_is_required_when_configured() {
        let (settings, _) = create(
            "mot de passe",
            Some(b"fichier-cle"),
            Cipher::ChaCha20Poly1305,
            FAST,
            &Default::default(),
        )
        .unwrap();
        assert!(settings.uses_keyfile);
        assert!(unlock(&settings, "mot de passe", Some(b"fichier-cle")).is_ok());
        assert!(unlock(&settings, "mot de passe", None).is_err());
        assert!(unlock(&settings, "mot de passe", Some(b"autre")).is_err());
    }

    #[test]
    fn encrypt_then_decrypt_whole_database() {
        let connection = database_with_notes();
        store::kv_set(&connection, "plug", "k", "valeur").unwrap();
        let (_, key) = create(
            "pw-pw-pw-pw",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();

        encrypt_database(&connection, &key, Cipher::Aes256Gcm).unwrap();
        for record in store::live_records(&connection).unwrap() {
            assert!(crypto::is_encrypted(&record.title) && crypto::is_encrypted(&record.body));
            assert!(!record.body.contains("corps"));
        }
        // Idempotent : pas de double chiffrement.
        encrypt_database(&connection, &key, Cipher::Aes256Gcm).unwrap();
        let mut first = store::live_records(&connection).unwrap().remove(0);
        decrypt_record_fields(&key, Cipher::Aes256Gcm, &mut first).unwrap();
        assert_eq!(
            (first.title.as_str(), first.body.as_str()),
            ("premier", "corps un")
        );
        let stored = store::kv_get(&connection, "plug", "k").unwrap().unwrap();
        assert!(crypto::is_encrypted(&stored));

        decrypt_database(&connection, &key, Cipher::Aes256Gcm).unwrap();
        let notes = store::live_records(&connection).unwrap();
        assert_eq!(notes[1].body, "corps deux");
        assert_eq!(
            store::kv_get(&connection, "plug", "k").unwrap().as_deref(),
            Some("valeur")
        );
    }

    #[test]
    fn rekey_changes_key_and_cipher() {
        let connection = database_with_notes();
        let (_, old) = create(
            "ancien-mot-de-passe",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        let (_, new) = create(
            "nouveau-mot-de-passe",
            None,
            Cipher::ChaCha20Poly1305,
            FAST,
            &Default::default(),
        )
        .unwrap();
        encrypt_database(&connection, &old, Cipher::Aes256Gcm).unwrap();
        rekey_database(
            &connection,
            &old,
            Cipher::Aes256Gcm,
            &new,
            Cipher::ChaCha20Poly1305,
        )
        .unwrap();

        let mut record = store::live_records(&connection).unwrap().remove(0);
        assert!(decrypt_record_fields(&old, Cipher::Aes256Gcm, &mut record.clone()).is_err());
        decrypt_record_fields(&new, Cipher::ChaCha20Poly1305, &mut record).unwrap();
        assert_eq!(record.title, "premier");
    }

    #[test]
    fn failed_bulk_operation_leaves_the_database_untouched() {
        let connection = database_with_notes();
        let (_, key) = create(
            "pw-pw-pw-pw",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        let (_, wrong) = create(
            "autre-autre",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        encrypt_database(&connection, &key, Cipher::Aes256Gcm).unwrap();
        let before = store::live_records(&connection).unwrap();
        assert!(decrypt_database(&connection, &wrong, Cipher::Aes256Gcm).is_err());
        assert_eq!(before, store::live_records(&connection).unwrap());
    }

    #[test]
    fn swapping_title_and_body_is_detected() {
        let (_, key) = create(
            "pw-pw-pw-pw",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        let mut record = Record {
            id: 1,
            uid: "u1".into(),
            title: "t".into(),
            body: "b".into(),
            created_at: String::new(),
            updated_at: String::new(),
            deleted: false,
            rev: 1,
        };
        encrypt_record_fields(&key, Cipher::Aes256Gcm, &mut record).unwrap();
        std::mem::swap(&mut record.title, &mut record.body);
        assert!(decrypt_record_fields(&key, Cipher::Aes256Gcm, &mut record).is_err());
    }

    #[test]
    fn backup_roundtrip_and_wrong_password() {
        let notes = vec![BackupNote {
            uid: "u".into(),
            title: "t".into(),
            body: "b".into(),
            created_at: "c".into(),
            updated_at: "d".into(),
        }];
        let mut settings = Settings::default();
        settings.sanitize();
        settings.theme.id = "nord".into();
        let file = export_backup(
            notes,
            &settings,
            "mot-de-passe-de-secours",
            Cipher::Aes256Gcm,
            FAST,
        )
        .unwrap();
        assert!(!String::from_utf8_lossy(&file).contains("\"title\""));

        let (restored, restored_settings) =
            import_backup(&file, "mot-de-passe-de-secours").unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].body, "b");
        assert_eq!(restored_settings.unwrap().theme.id, "nord");
        assert!(matches!(
            import_backup(&file, "faux"),
            Err(VaultError::WrongPassword)
        ));
        assert!(import_backup(b"{}", "x").is_err());
        assert!(import_backup(b"pas du json", "x").is_err());
    }
}
