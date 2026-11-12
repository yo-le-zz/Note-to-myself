//! Stockage SQLite. Les champs `title` et `body` sont stockés tels quels : en clair,
//! ou sous la forme `enc:v1:…` quand le coffre est activé (voir `vault.rs`).
//!
//! Chaque note a un identifiant global (`uid`) identique sur tous les PC, et une suppression
//! laisse une « pierre tombale » (`deleted = 1`) : c'est ce qui permet à la synchronisation de
//! propager les suppressions.

use crate::crypto;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: i64,
    pub uid: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted: bool,
    pub rev: i64,
}

impl Record {
    /// Empreinte du contenu (hors dates et numéro de révision) : sert à détecter les changements.
    pub fn content_hash(&self) -> String {
        let mut data = Vec::new();
        for part in [self.uid.as_str(), &self.title, &self.body] {
            data.extend_from_slice(&(part.len() as u64).to_le_bytes());
            data.extend_from_slice(part.as_bytes());
        }
        data.push(self.deleted as u8);
        crypto::to_hex(&crypto::sha256(&data))
    }
}

pub fn new_uid() -> String {
    crypto::to_hex(&crypto::random_bytes::<16>())
}

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    // Évite une attente SQLite longue si un autre processus (ou le thread de sync) tient le verrou.
    connection.busy_timeout(Duration::from_millis(500))?;
    // WAL : l'interface et la synchronisation peuvent travailler en parallèle.
    let _ = connection.pragma_update(None, "journal_mode", "WAL");
    // Le contenu supprimé ou remplacé est écrasé par des zéros dans le fichier : sans cela, les anciennes
    // versions d'une note (en clair, avant chiffrement) restent lisibles dans les pages libérées.
    let _ = connection.pragma_update(None, "secure_delete", "ON");
    Ok(connection)
}

fn table_columns(connection: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare("PRAGMA table_info(notes)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(columns)
}

/// Crée la table si nécessaire et la met au schéma v1 sans perdre de données.
///
/// Les anciennes versions stockaient le texte dans `content`, `text`, `note`…
/// Ces colonnes pouvaient être `NOT NULL` sans valeur par défaut : un simple
/// `ALTER TABLE ADD COLUMN body` ne les supprime pas, et chaque INSERT échouait
/// avec « NOT NULL constraint failed: notes.content ». Dans ce cas, la table est
/// reconstruite proprement ; l'ancienne est conservée sous `notes_legacy_backup`.
pub fn ensure_notes_schema(connection: &Connection) -> rusqlite::Result<()> {
    const CURRENT: [&str; 5] = ["id", "title", "body", "created_at", "updated_at"];
    const LEGACY_TEXT: [&str; 5] = ["content", "text", "note", "message", "description"];

    connection.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT ''
        )",
        [],
    )?;

    let columns = table_columns(connection)?;
    // Les colonnes ajoutées par le schéma v2 (uid, deleted, rev) ne déclenchent pas de reconstruction.
    let up_to_date = CURRENT.iter().all(|c| columns.iter().any(|name| name == c))
        && !LEGACY_TEXT
            .iter()
            .any(|c| columns.iter().any(|name| name == c));
    if up_to_date {
        return Ok(());
    }

    let has = |name: &str| columns.iter().any(|c| c == name);

    // Texte de la note : `body` s'il est rempli, sinon l'ancienne colonne.
    let mut body_sources: Vec<String> = Vec::new();
    for name in std::iter::once("body").chain(LEGACY_TEXT) {
        if has(name) {
            body_sources.push(format!("NULLIF(CAST(\"{name}\" AS TEXT), '')"));
        }
    }
    body_sources.push("''".to_string());
    let body_expr = format!("COALESCE({})", body_sources.join(", "));

    let title_expr = if has("title") {
        "COALESCE(CAST(title AS TEXT), '')"
    } else {
        "''"
    };
    let created_expr = if has("created_at") {
        "COALESCE(CAST(created_at AS TEXT), '')"
    } else {
        "''"
    };
    let updated_expr = if has("updated_at") && has("created_at") {
        "COALESCE(NULLIF(CAST(updated_at AS TEXT), ''), CAST(created_at AS TEXT), '')"
    } else if has("updated_at") {
        "COALESCE(CAST(updated_at AS TEXT), '')"
    } else if has("created_at") {
        "COALESCE(CAST(created_at AS TEXT), '')"
    } else {
        "''"
    };

    let (id_col, id_expr) = if has("id") {
        ("id, ", "id, ")
    } else {
        ("", "")
    };

    let transaction = connection.unchecked_transaction()?;
    transaction.execute("DROP TABLE IF EXISTS notes_new", [])?;
    transaction.execute(
        "CREATE TABLE notes_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT ''
        )",
        [],
    )?;
    transaction.execute(
        &format!(
            "INSERT INTO notes_new ({id_col}title, body, created_at, updated_at)
             SELECT {id_expr}{title_expr}, {body_expr}, {created_expr}, {updated_expr}
             FROM notes"
        ),
        [],
    )?;
    transaction.execute("DROP TABLE IF EXISTS notes_legacy_backup", [])?;
    transaction.execute("ALTER TABLE notes RENAME TO notes_legacy_backup", [])?;
    transaction.execute("ALTER TABLE notes_new RENAME TO notes", [])?;
    transaction.commit()
}

/// Schéma complet : v1 + identifiants globaux, suppressions logiques, révisions, état de sync, stockage des plugins.
pub fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    ensure_notes_schema(connection)?;
    let columns = table_columns(connection)?;
    let has = |name: &str| columns.iter().any(|c| c == name);
    if !has("uid") {
        connection.execute(
            "ALTER TABLE notes ADD COLUMN uid TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !has("deleted") {
        connection.execute(
            "ALTER TABLE notes ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has("rev") {
        connection.execute(
            "ALTER TABLE notes ADD COLUMN rev INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    // Les notes existantes reçoivent leur identifiant global.
    let missing: Vec<i64> = {
        let mut statement = connection.prepare("SELECT id FROM notes WHERE uid = ''")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    if !missing.is_empty() {
        let transaction = connection.unchecked_transaction()?;
        for id in missing {
            transaction.execute(
                "UPDATE notes SET uid = ?1 WHERE id = ?2",
                params![new_uid(), id],
            )?;
        }
        transaction.commit()?;
    }
    connection.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS notes_uid_idx ON notes(uid);
         CREATE TABLE IF NOT EXISTS sync_base (
            peer TEXT NOT NULL, uid TEXT NOT NULL, hash TEXT NOT NULL,
            PRIMARY KEY (peer, uid)
         );
         CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY, value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS kv (
            plugin TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL,
            PRIMARY KEY (plugin, key)
         );",
    )
}

const COLUMNS: &str = "id, uid, title, body, created_at, updated_at, deleted, rev";

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        id: row.get(0)?,
        uid: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        deleted: row.get::<_, i64>(6)? != 0,
        rev: row.get(7)?,
    })
}

/// Toutes les lignes, suppressions comprises (pour la synchronisation).
pub fn all_records(connection: &Connection) -> rusqlite::Result<Vec<Record>> {
    let mut statement =
        connection.prepare(&format!("SELECT {COLUMNS} FROM notes ORDER BY id ASC"))?;
    let rows = statement.query_map([], row_to_record)?;
    rows.collect()
}

/// Notes visibles, de la plus ancienne à la plus récente.
pub fn live_records(connection: &Connection) -> rusqlite::Result<Vec<Record>> {
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM notes WHERE deleted = 0 ORDER BY id ASC"
    ))?;
    let rows = statement.query_map([], row_to_record)?;
    rows.collect()
}

pub fn record_by_uid(connection: &Connection, uid: &str) -> rusqlite::Result<Option<Record>> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM notes WHERE uid = ?1"),
            [uid],
            row_to_record,
        )
        .optional()
}

pub fn insert(
    connection: &Connection,
    title: &str,
    body: &str,
    now: &str,
) -> rusqlite::Result<String> {
    let uid = new_uid();
    insert_with_uid(connection, &uid, title, body, now)?;
    Ok(uid)
}

/// Insertion avec un identifiant choisi à l'avance : nécessaire pour chiffrer les champs, car
/// l'identifiant fait partie des données authentifiées du chiffrement.
pub fn insert_with_uid(
    connection: &Connection,
    uid: &str,
    title: &str,
    body: &str,
    now: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO notes (uid, title, body, created_at, updated_at, deleted, rev)
         VALUES (?1, ?2, ?3, ?4, ?4, 0, 1)",
        params![uid, title, body, now],
    )?;
    Ok(())
}

/// Retourne `false` si la note n'existe plus (ou a été supprimée entre-temps).
pub fn update(
    connection: &Connection,
    uid: &str,
    title: &str,
    body: &str,
    now: &str,
) -> rusqlite::Result<bool> {
    let changed = connection.execute(
        "UPDATE notes SET title = ?1, body = ?2, updated_at = ?3, rev = rev + 1
         WHERE uid = ?4 AND deleted = 0",
        params![title, body, now, uid],
    )?;
    Ok(changed > 0)
}

/// Suppression logique : le contenu est effacé tout de suite, seule la pierre tombale reste.
pub fn soft_delete(connection: &Connection, uid: &str, now: &str) -> rusqlite::Result<bool> {
    let changed = connection.execute(
        "UPDATE notes SET title = '', body = '', deleted = 1, updated_at = ?1, rev = rev + 1
         WHERE uid = ?2 AND deleted = 0",
        params![now, uid],
    )?;
    Ok(changed > 0)
}

/// Insère ou remplace une note reçue d'un pair (identifiée par son `uid`).
pub fn upsert_remote(connection: &Connection, record: &Record) -> rusqlite::Result<()> {
    let existing = record_by_uid(connection, &record.uid)?;
    match existing {
        Some(local) => {
            connection.execute(
                "UPDATE notes SET title = ?1, body = ?2, created_at = ?3, updated_at = ?4,
                 deleted = ?5, rev = ?6 WHERE id = ?7",
                params![
                    record.title,
                    record.body,
                    record.created_at,
                    record.updated_at,
                    record.deleted as i64,
                    record.rev.max(local.rev + 1),
                    local.id
                ],
            )?;
        }
        None => {
            connection.execute(
                "INSERT INTO notes (uid, title, body, created_at, updated_at, deleted, rev)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.uid,
                    record.title,
                    record.body,
                    record.created_at,
                    record.updated_at,
                    record.deleted as i64,
                    record.rev
                ],
            )?;
        }
    }
    Ok(())
}

/// Supprime définitivement les pierres tombales plus anciennes que `cutoff` (RFC 3339).
pub fn purge_tombstones(connection: &Connection, cutoff: &str) -> rusqlite::Result<usize> {
    connection.execute(
        "DELETE FROM notes WHERE deleted = 1 AND updated_at < ?1",
        [cutoff],
    )
}

pub fn load_base(
    connection: &Connection,
    peer: &str,
) -> rusqlite::Result<std::collections::HashMap<String, String>> {
    let mut statement = connection.prepare("SELECT uid, hash FROM sync_base WHERE peer = ?1")?;
    let rows = statement.query_map([peer], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn save_base(
    connection: &Connection,
    peer: &str,
    base: &std::collections::HashMap<String, String>,
) -> rusqlite::Result<()> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute("DELETE FROM sync_base WHERE peer = ?1", [peer])?;
    for (uid, hash) in base {
        transaction.execute(
            "INSERT INTO sync_base (peer, uid, hash) VALUES (?1, ?2, ?3)",
            params![peer, uid, hash],
        )?;
    }
    transaction.commit()
}

pub fn kv_get(
    connection: &Connection,
    plugin: &str,
    key: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM kv WHERE plugin = ?1 AND key = ?2",
            params![plugin, key],
            |row| row.get(0),
        )
        .optional()
}

pub fn kv_set(
    connection: &Connection,
    plugin: &str,
    key: &str,
    value: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO kv (plugin, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT(plugin, key) DO UPDATE SET value = excluded.value",
        params![plugin, key, value],
    )?;
    Ok(())
}

pub fn kv_all(connection: &Connection) -> rusqlite::Result<Vec<(String, String, String)>> {
    let mut statement =
        connection.prepare("SELECT plugin, key, value FROM kv ORDER BY plugin, key")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    rows.collect()
}

/// Un identifiant de note venu du réseau ou d'une sauvegarde doit rester simple : il sert de clé et de
/// donnée authentifiée du chiffrement.
pub fn valid_uid(uid: &str) -> bool {
    (1..=64).contains(&uid.len())
        && uid
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn meta_get(connection: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()
}

pub fn meta_set(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Oublie l'état commun avec tous les pairs : à faire quand toutes les empreintes changent.
pub fn clear_sync_base(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM sync_base", [])?;
    Ok(())
}

/// Efface les restes de données : vide le journal WAL puis réécrit le fichier (VACUUM).
pub fn scrub(connection: &Connection) -> rusqlite::Result<()> {
    connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    connection.execute_batch("VACUUM")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        ensure_schema(&connection).unwrap();
        connection
    }

    #[test]
    fn migrates_legacy_not_null_content_column() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE notes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                INSERT INTO notes (content, created_at) VALUES ('ancienne note', '2025-01-01');",
            )
            .unwrap();

        ensure_notes_schema(&connection).unwrap();

        // Un INSERT avec le schéma v1 ne doit plus échouer.
        connection
            .execute(
                "INSERT INTO notes (title, body, created_at, updated_at) VALUES ('t', 'b', 'x', 'x')",
                [],
            )
            .unwrap();
        let old: String = connection
            .query_row("SELECT body FROM notes WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(old, "ancienne note");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        // Idempotent.
        ensure_notes_schema(&connection).unwrap();
    }

    #[test]
    fn fresh_database_is_untouched() {
        let connection = Connection::open_in_memory().unwrap();
        ensure_notes_schema(&connection).unwrap();
        ensure_notes_schema(&connection).unwrap();
        assert_eq!(table_columns(&connection).unwrap().len(), 5);
    }

    #[test]
    fn v1_database_upgrades_without_losing_notes() {
        let connection = Connection::open_in_memory().unwrap();
        ensure_notes_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO notes (title, body, created_at, updated_at) VALUES ('a','b','c','d')",
                [],
            )
            .unwrap();
        ensure_schema(&connection).unwrap();
        ensure_schema(&connection).unwrap();
        let notes = live_records(&connection).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "a");
        assert_eq!(notes[0].uid.len(), 32);
    }

    #[test]
    fn legacy_database_reaches_the_current_schema() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE notes (id INTEGER PRIMARY KEY, content TEXT NOT NULL, created_at TEXT);
                 INSERT INTO notes (content, created_at) VALUES ('vieille note', '2024');",
            )
            .unwrap();
        ensure_schema(&connection).unwrap();
        let notes = live_records(&connection).unwrap();
        assert_eq!(notes[0].body, "vieille note");
        assert!(!notes[0].uid.is_empty());
    }

    #[test]
    fn insert_update_delete_cycle() {
        let connection = memory();
        let uid = insert(&connection, "titre", "corps", "2026-01-01").unwrap();
        assert!(update(&connection, &uid, "titre 2", "corps 2", "2026-01-02").unwrap());
        let record = record_by_uid(&connection, &uid).unwrap().unwrap();
        assert_eq!((record.title.as_str(), record.rev), ("titre 2", 2));

        assert!(soft_delete(&connection, &uid, "2026-01-03").unwrap());
        assert!(live_records(&connection).unwrap().is_empty());
        let tombstone = record_by_uid(&connection, &uid).unwrap().unwrap();
        assert!(tombstone.deleted && tombstone.body.is_empty() && tombstone.title.is_empty());
        // Modifier ou supprimer une note déjà supprimée ne fait rien.
        assert!(!update(&connection, &uid, "x", "y", "z").unwrap());
        assert!(!soft_delete(&connection, &uid, "z").unwrap());
    }

    #[test]
    fn tombstones_are_purged_by_age() {
        let connection = memory();
        let old = insert(&connection, "a", "b", "2020-01-01").unwrap();
        let recent = insert(&connection, "c", "d", "2026-06-01").unwrap();
        soft_delete(&connection, &old, "2020-01-02").unwrap();
        soft_delete(&connection, &recent, "2026-06-02").unwrap();
        assert_eq!(purge_tombstones(&connection, "2026-01-01").unwrap(), 1);
        assert!(record_by_uid(&connection, &old).unwrap().is_none());
        assert!(record_by_uid(&connection, &recent).unwrap().is_some());
    }

    #[test]
    fn upsert_remote_inserts_then_replaces() {
        let connection = memory();
        let mut record = Record {
            id: 0,
            uid: "abc".into(),
            title: "t".into(),
            body: "b".into(),
            created_at: "c".into(),
            updated_at: "u1".into(),
            deleted: false,
            rev: 3,
        };
        upsert_remote(&connection, &record).unwrap();
        record.body = "b2".into();
        record.updated_at = "u2".into();
        upsert_remote(&connection, &record).unwrap();
        let all = all_records(&connection).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].body, "b2");
        assert!(all[0].rev > 3);
    }

    #[test]
    fn content_hash_ignores_dates_but_not_content() {
        let mut a = Record {
            id: 1,
            uid: "u".into(),
            title: "t".into(),
            body: "b".into(),
            created_at: "1".into(),
            updated_at: "2".into(),
            deleted: false,
            rev: 1,
        };
        let h = a.content_hash();
        a.updated_at = "other".into();
        a.rev = 9;
        assert_eq!(h, a.content_hash());
        a.body = "b2".into();
        assert_ne!(h, a.content_hash());
        a.body = "b".into();
        a.deleted = true;
        assert_ne!(h, a.content_hash());
        // Pas de collision par concaténation (« ab »+« c » ≠ « a »+« bc »).
        let x = Record {
            title: "ab".into(),
            body: "c".into(),
            ..a.clone()
        };
        let y = Record {
            title: "a".into(),
            body: "bc".into(),
            ..a
        };
        assert_ne!(x.content_hash(), y.content_hash());
    }

    #[test]
    fn sync_base_and_kv_roundtrip() {
        let connection = memory();
        let mut base = std::collections::HashMap::new();
        base.insert("u1".to_string(), "h1".to_string());
        save_base(&connection, "peer", &base).unwrap();
        assert_eq!(load_base(&connection, "peer").unwrap(), base);
        assert!(load_base(&connection, "autre").unwrap().is_empty());
        save_base(&connection, "peer", &Default::default()).unwrap();
        assert!(load_base(&connection, "peer").unwrap().is_empty());

        kv_set(&connection, "p", "k", "v1").unwrap();
        kv_set(&connection, "p", "k", "v2").unwrap();
        assert_eq!(
            kv_get(&connection, "p", "k").unwrap().as_deref(),
            Some("v2")
        );
        assert_eq!(kv_get(&connection, "p", "absent").unwrap(), None);
    }
}
