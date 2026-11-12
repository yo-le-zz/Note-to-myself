//! Opérations sur les notes pour l'interface : lecture déchiffrée, enregistrement, suppression.
//! Tout ce qui touche à la base passe ici, ce qui garde le code de l'interface simple et testable.

use crate::crypto::{Cipher, Key};
use crate::store::{self, Record};
use crate::vault;
use rusqlite::Connection;
use std::path::Path;

/// Clé et algorithme d'un coffre déverrouillé.
#[derive(Clone, Debug)]
pub struct Unlocked {
    pub key: Key,
    pub cipher: Cipher,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub uid: String,
    /// Numéro d'affichage : 1 pour la plus ancienne note, stable tant qu'on ne supprime pas.
    pub number: usize,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    /// La note n'a pas pu être déchiffrée (autre mot de passe, données altérées) : elle est en lecture seule.
    pub unreadable: bool,
}

/// Lit les notes visibles, la plus récente en premier, déchiffrées si le coffre est ouvert.
/// Une note illisible n'empêche pas d'afficher les autres.
pub fn load(connection: &Connection, vault: Option<&Unlocked>) -> rusqlite::Result<Vec<Note>> {
    let mut notes: Vec<Note> = store::live_records(connection)?
        .into_iter()
        .enumerate()
        .map(|(index, mut record)| {
            let unreadable = match vault {
                Some(v) => vault::decrypt_record_fields(&v.key, v.cipher, &mut record).is_err(),
                None => {
                    crate::crypto::is_encrypted(&record.title)
                        || crate::crypto::is_encrypted(&record.body)
                }
            };
            if unreadable {
                record.title = String::new();
                record.body = String::new();
            }
            Note {
                uid: record.uid,
                number: index + 1,
                title: record.title,
                body: record.body,
                created_at: record.created_at,
                updated_at: record.updated_at,
                unreadable,
            }
        })
        .collect();
    notes.reverse();
    Ok(notes)
}

fn open(path: &Path) -> Result<Connection, String> {
    let connection = store::open(path).map_err(|e| format!("database open failed: {e}"))?;
    store::ensure_schema(&connection)
        .map_err(|e| format!("database schema migration failed: {e}"))?;
    Ok(connection)
}

fn seal(
    vault: Option<&Unlocked>,
    uid: &str,
    title: &str,
    body: &str,
) -> Result<(String, String), String> {
    let Some(v) = vault else {
        return Ok((title.to_string(), body.to_string()));
    };
    let mut record = Record {
        id: 0,
        uid: uid.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        created_at: String::new(),
        updated_at: String::new(),
        deleted: false,
        rev: 1,
    };
    vault::encrypt_record_fields(&v.key, v.cipher, &mut record).map_err(|e| e.to_string())?;
    Ok((record.title, record.body))
}

/// Crée (`editing = None`) ou met à jour une note, puis relit toutes les notes.
pub fn save(
    path: &Path,
    editing: Option<&str>,
    title: &str,
    body: &str,
    now: &str,
    vault: Option<&Unlocked>,
) -> Result<Vec<Note>, String> {
    let connection = open(path)?;
    match editing {
        Some(uid) => {
            let (title, body) = seal(vault, uid, title, body)?;
            let changed = store::update(&connection, uid, &title, &body, now)
                .map_err(|e| format!("note update failed: {e}"))?;
            if !changed {
                return Err("the note to update no longer exists".into());
            }
        }
        None => {
            let uid = store::new_uid();
            let (title, body) = seal(vault, &uid, title, body)?;
            store::insert_with_uid(&connection, &uid, &title, &body, now)
                .map_err(|e| format!("note insert failed: {e}"))?;
        }
    }
    load(&connection, vault).map_err(|e| format!("reading saved notes failed: {e}"))
}

pub fn delete(
    path: &Path,
    uid: &str,
    now: &str,
    vault: Option<&Unlocked>,
) -> Result<Vec<Note>, String> {
    let connection = open(path)?;
    store::soft_delete(&connection, uid, now).map_err(|e| format!("note delete failed: {e}"))?;
    load(&connection, vault).map_err(|e| format!("reading notes failed: {e}"))
}

pub fn reload(path: &Path, vault: Option<&Unlocked>) -> Result<Vec<Note>, String> {
    let connection = open(path)?;
    load(&connection, vault).map_err(|e| format!("reading notes failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{self, Kdf};

    fn temp_db() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ntm-notes-{}.sqlite3", store::new_uid()))
    }

    fn unlocked(password: &str) -> Unlocked {
        let (_, key) = vault::create(
            password,
            None,
            Cipher::Aes256Gcm,
            Kdf::Argon2id {
                m_kib: 8_192,
                t: 1,
                p: 1,
            },
            &Default::default(),
        )
        .unwrap();
        Unlocked {
            key,
            cipher: Cipher::Aes256Gcm,
        }
    }

    #[test]
    fn plain_notes_roundtrip_and_numbering() {
        let path = temp_db();
        save(&path, None, "un", "corps 1", "2026-01-01", None).unwrap();
        let notes = save(&path, None, "deux", "corps 2", "2026-01-02", None).unwrap();
        assert_eq!(
            notes.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(),
            vec!["deux", "un"]
        );
        assert_eq!(notes[0].number, 2);
        let edited = save(
            &path,
            Some(&notes[1].uid),
            "un bis",
            "x",
            "2026-01-03",
            None,
        )
        .unwrap();
        assert_eq!(edited[1].title, "un bis");
        let after_delete = delete(&path, &notes[0].uid, "2026-01-04", None).unwrap();
        assert_eq!(after_delete.len(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn encrypted_notes_are_unreadable_on_disk_and_readable_with_the_key() {
        let path = temp_db();
        let vault = unlocked("mot de passe solide");
        let notes = save(
            &path,
            None,
            "secret",
            "contenu confidentiel",
            "2026-01-01",
            Some(&vault),
        )
        .unwrap();
        assert_eq!(notes[0].body, "contenu confidentiel");

        let raw = std::fs::read(&path).unwrap_or_default();
        let wal = std::fs::read(path.with_extension("sqlite3-wal")).unwrap_or_default();
        for bytes in [raw, wal] {
            assert!(!String::from_utf8_lossy(&bytes).contains("contenu confidentiel"));
        }
        let connection = store::open(&path).unwrap();
        let stored = store::live_records(&connection).unwrap();
        assert!(crypto::is_encrypted(&stored[0].body));

        // Mauvaise clé : la note est signalée illisible, jamais affichée.
        let wrong = unlocked("un autre mot de passe");
        let unreadable = load(&connection, Some(&wrong)).unwrap();
        assert!(unreadable[0].unreadable && unreadable[0].body.is_empty());
        // Coffre verrouillé : même résultat.
        assert!(load(&connection, None).unwrap()[0].unreadable);

        let edited = save(
            &path,
            Some(&notes[0].uid),
            "secret",
            "modifié",
            "2026-01-02",
            Some(&vault),
        )
        .unwrap();
        assert_eq!(edited[0].body, "modifié");
        drop(connection);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn editing_a_missing_note_is_an_error() {
        let path = temp_db();
        assert!(save(&path, Some("absent"), "t", "b", "now", None).is_err());
        let _ = std::fs::remove_file(path);
    }
}
