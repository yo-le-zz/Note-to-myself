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
const META_KEY: &str = "security";

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
        rev: previous.rev + 1,
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

/// Applique `note_fn` à chaque note et `value_fn` à chaque valeur de plugin dans une seule transaction :
/// tout réussit ou rien ne change. Seules les lignes réellement modifiées sont réécrites.
///
/// Si `config` est fourni, la nouvelle configuration du coffre est écrite dans la base **dans la même
/// transaction** : une coupure ne peut pas laisser des notes chiffrées avec une clé dont la
/// configuration (sel, paramètres) serait perdue. Après une modification, l'état de synchronisation est
/// oublié (toutes les empreintes ont changé), l'ancienne table de migration (en clair) est supprimée,
/// puis le fichier est nettoyé des anciennes versions en clair (journal WAL, pages libérées).
fn rewrite_all(
    connection: &Connection,
    config: Option<&SecuritySettings>,
    mut note_fn: impl FnMut(&mut Record) -> Result<(), VaultError>,
    mut value_fn: impl FnMut(&str, &str, &str) -> Result<String, VaultError>,
) -> Result<(), VaultError> {
    let transaction = connection.unchecked_transaction()?;
    let mut changed = false;
    for mut record in store::all_records(&transaction)? {
        if record.deleted {
            continue;
        }
        let (old_title, old_body) = (record.title.clone(), record.body.clone());
        note_fn(&mut record)?;
        if record.title != old_title || record.body != old_body {
            changed = true;
            transaction.execute(
                "UPDATE notes SET title = ?1, body = ?2 WHERE id = ?3",
                rusqlite::params![record.title, record.body, record.id],
            )?;
        }
    }
    for (plugin, key, value) in store::kv_all(&transaction)? {
        let new_value = value_fn(&plugin, &key, &value)?;
        if new_value != value {
            changed = true;
            store::kv_set(&transaction, &plugin, &key, &new_value)?;
        }
    }
    if let Some(config) = config {
        let json = serde_json::to_string(config).map_err(|e| VaultError::Format(e.to_string()))?;
        store::meta_set(&transaction, META_KEY, &json)?;
        changed = true;
    }
    if changed {
        store::clear_sync_base(&transaction)?;
        // Copie en clair créée par une ancienne migration : elle ne doit pas survivre au chiffrement.
        transaction.execute("DROP TABLE IF EXISTS notes_legacy_backup", [])?;
    }
    transaction.commit()?;
    if changed {
        // Nettoyage au mieux : l'opération a réussi même si le fichier ne peut pas être réécrit.
        let _ = store::scrub(connection);
    }
    Ok(())
}

/// Chiffre toute la base (notes et stockage des plugins) avec la clé donnée.
pub fn encrypt_database(
    connection: &Connection,
    key: &Key,
    cipher: Cipher,
    config: Option<&SecuritySettings>,
) -> Result<(), VaultError> {
    rewrite_all(
        connection,
        config,
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
    config: Option<&SecuritySettings>,
) -> Result<(), VaultError> {
    rewrite_all(
        connection,
        config,
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
    config: Option<&SecuritySettings>,
) -> Result<(), VaultError> {
    rewrite_all(
        connection,
        config,
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

/// Après une coupure entre la transaction de la base et l'écriture de `settings.json`, la base porte
/// la configuration la plus récente : on l'adopte (le verrouillage automatique reste celui des réglages).
/// Retourne `true` si les réglages ont été remplacés.
pub fn reconcile(connection: &Connection, settings: &mut SecuritySettings) -> bool {
    let Ok(Some(json)) = store::meta_get(connection, META_KEY) else {
        return false;
    };
    let Ok(from_database) = serde_json::from_str::<SecuritySettings>(&json) else {
        return false;
    };
    if from_database.rev <= settings.rev {
        return false;
    }
    let auto_lock = settings.auto_lock_minutes;
    *settings = from_database;
    settings.auto_lock_minutes = auto_lock;
    true
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

        encrypt_database(&connection, &key, Cipher::Aes256Gcm, None).unwrap();
        for record in store::live_records(&connection).unwrap() {
            assert!(crypto::is_encrypted(&record.title) && crypto::is_encrypted(&record.body));
            assert!(!record.body.contains("corps"));
        }
        // Idempotent : pas de double chiffrement.
        encrypt_database(&connection, &key, Cipher::Aes256Gcm, None).unwrap();
        let mut first = store::live_records(&connection).unwrap().remove(0);
        decrypt_record_fields(&key, Cipher::Aes256Gcm, &mut first).unwrap();
        assert_eq!(
            (first.title.as_str(), first.body.as_str()),
            ("premier", "corps un")
        );
        let stored = store::kv_get(&connection, "plug", "k").unwrap().unwrap();
        assert!(crypto::is_encrypted(&stored));

        decrypt_database(&connection, &key, Cipher::Aes256Gcm, None).unwrap();
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
        encrypt_database(&connection, &old, Cipher::Aes256Gcm, None).unwrap();
        rekey_database(
            &connection,
            &old,
            Cipher::Aes256Gcm,
            &new,
            Cipher::ChaCha20Poly1305,
            None,
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
        encrypt_database(&connection, &key, Cipher::Aes256Gcm, None).unwrap();
        let before = store::live_records(&connection).unwrap();
        assert!(decrypt_database(&connection, &wrong, Cipher::Aes256Gcm, None).is_err());
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

    fn temp_db_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ntm-vault-{}.sqlite3", store::new_uid()))
    }

    fn remove_db(path: &std::path::Path) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }

    #[test]
    fn no_plaintext_survives_on_disk_after_encryption() {
        let path = temp_db_path();
        let marker = "MARQUEUR-SECRET-EN-CLAIR-12345";
        {
            let connection = store::open(&path).unwrap();
            store::ensure_schema(&connection).unwrap();
            // Table de migration en clair, comme en laisse une ancienne version.
            connection
                .execute_batch("CREATE TABLE notes_legacy_backup (content TEXT)")
                .unwrap();
            connection
                .execute("INSERT INTO notes_legacy_backup VALUES (?1)", [marker])
                .unwrap();
            for i in 0..50 {
                store::insert(&connection, marker, &format!("{marker} {i}"), "2026-01-01").unwrap();
            }
            let (security, key) = create(
                "mot de passe très solide",
                None,
                Cipher::Aes256Gcm,
                FAST,
                &Default::default(),
            )
            .unwrap();
            encrypt_database(&connection, &key, Cipher::Aes256Gcm, Some(&security)).unwrap();
            let leftover: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name = 'notes_legacy_backup'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(leftover, 0, "the plaintext migration table must be dropped");
        }
        for suffix in ["", "-wal"] {
            let bytes = std::fs::read(format!("{}{suffix}", path.display())).unwrap_or_default();
            let text = String::from_utf8_lossy(&bytes);
            assert!(
                !text.contains(marker),
                "plaintext found in the database file{suffix}"
            );
        }
        remove_db(&path);
    }

    #[test]
    fn vault_configuration_is_written_in_the_same_transaction_as_the_notes() {
        let connection = database_with_notes();
        let (security, key) = create(
            "mot de passe solide",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        encrypt_database(&connection, &key, Cipher::Aes256Gcm, Some(&security)).unwrap();

        // Coupure simulée : les notes sont chiffrées mais settings.json n'a jamais été écrit.
        let mut stale = SecuritySettings::default();
        assert!(reconcile(&connection, &mut stale));
        assert_eq!(stale.salt, security.salt);
        assert_eq!(stale.rev, security.rev);
        assert!(
            unlock(&stale, "mot de passe solide", None).is_ok(),
            "the notes must stay readable"
        );

        // Réglages à jour : rien à faire. Réglages plus récents (config reçue) : ils gagnent.
        let mut current = security.clone();
        assert!(!reconcile(&connection, &mut current));
        let mut newer = security.clone();
        newer.rev += 1;
        newer.auto_lock_minutes = 42;
        assert!(!reconcile(&connection, &mut newer));
        assert_eq!(newer.auto_lock_minutes, 42);
    }

    #[test]
    fn a_failed_rekey_keeps_the_old_configuration_and_notes() {
        let connection = database_with_notes();
        let (security, key) = create(
            "ancien mot de passe",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        encrypt_database(&connection, &key, Cipher::Aes256Gcm, Some(&security)).unwrap();
        let (new_security, new_key) = create(
            "nouveau mot de passe",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &security,
        )
        .unwrap();
        let (_, wrong_old) =
            create("pas le bon", None, Cipher::Aes256Gcm, FAST, &security).unwrap();
        assert!(rekey_database(
            &connection,
            &wrong_old,
            Cipher::Aes256Gcm,
            &new_key,
            Cipher::Aes256Gcm,
            Some(&new_security)
        )
        .is_err());
        let mut from_db = SecuritySettings::default();
        assert!(reconcile(&connection, &mut from_db));
        assert_eq!(
            from_db.salt, security.salt,
            "the configuration must not advance on failure"
        );
        assert_eq!(new_security.rev, security.rev + 1);
    }

    #[test]
    fn re_running_encryption_on_an_encrypted_database_changes_nothing() {
        let connection = database_with_notes();
        let (security, key) = create(
            "mot de passe solide",
            None,
            Cipher::Aes256Gcm,
            FAST,
            &Default::default(),
        )
        .unwrap();
        encrypt_database(&connection, &key, Cipher::Aes256Gcm, Some(&security)).unwrap();
        store::save_base(
            &connection,
            "peer",
            &[("u".to_string(), "h".to_string())].into_iter().collect(),
        )
        .unwrap();
        let before = store::live_records(&connection).unwrap();
        encrypt_database(&connection, &key, Cipher::Aes256Gcm, None).unwrap();
        assert_eq!(before, store::live_records(&connection).unwrap());
        assert!(
            !store::load_base(&connection, "peer").unwrap().is_empty(),
            "no needless sync reset"
        );
    }
}
