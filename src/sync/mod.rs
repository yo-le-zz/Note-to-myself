//! Synchronisation entre PC par TCP (réseau local ou Tailscale).
//!
//! - Authentification mutuelle par secret partagé : chaque trame est protégée par un HMAC (ou chiffrée
//!   en AES-256-GCM si le chiffrement du transport est activé), avec un compteur anti-rejeu et une clé
//!   de session propre à chaque connexion.
//! - Les notes circulent telles qu'elles sont stockées : si le coffre est activé, ce sont des chiffrés
//!   que le réseau ne voit jamais en clair, et la synchronisation n'a pas besoin du mot de passe.
//! - La décision (qui envoie quoi) est prise par `plan.rs`.

pub mod plan;

use crate::crypto::{self, Key};
use crate::plugin::Plugin;
use crate::settings::{Settings, SyncMode};
use crate::store::{self, Record};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use plan::{Entry, PlanInput};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub const PROTOCOL_VERSION: u32 = 1;
const MAGIC: &[u8; 4] = b"NTM1";
const MAX_FRAME: usize = 64 * 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_CONNECTIONS: usize = 8;
const MAX_BUNDLE_ITEMS: usize = 500;

#[derive(Debug)]
pub enum SyncError {
    Io(String),
    /// Secret incorrect ou trame altérée.
    Auth,
    Protocol(String),
    Refused(String),
    Database(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(m) => write!(f, "network error: {m}"),
            Self::Auth => write!(f, "authentication failed (wrong sync secret?)"),
            Self::Protocol(m) => write!(f, "protocol error: {m}"),
            Self::Refused(m) => write!(f, "refused by peer: {m}"),
            Self::Database(m) => write!(f, "database error: {m}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<rusqlite::Error> for SyncError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e.to_string())
    }
}

type Result<T> = std::result::Result<T, SyncError>;

/// Empreinte non secrète de la configuration du coffre : deux PC ne synchronisent leurs notes que
/// s'ils ont le même coffre (ou aucun).
pub fn vault_fingerprint(settings: &Settings) -> String {
    if !settings.security.enabled {
        return "none".into();
    }
    let data = format!("{}|{}", settings.security.salt, settings.security.verifier);
    crypto::to_hex(&crypto::sha256(data.as_bytes()))[..16].to_string()
}

/// Réseaux acceptés quand « réseau de confiance uniquement » est actif : boucle locale, réseaux privés,
/// lien local et plage CGNAT 100.64.0.0/10 (Tailscale), ainsi que les adresses IPv6 locales uniques.
pub fn is_trusted_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || (o[0] == 100 && (64..=127).contains(&o[1]))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_trusted_ip(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            v6.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    }
}

// --- Clés ------------------------------------------------------------------------------------

static KEY_CACHE: Mutex<Option<([u8; 32], Key)>> = Mutex::new(None);

/// Clé de synchronisation dérivée du secret partagé (PBKDF2, mise en cache : le calcul est volontairement lent).
pub fn sync_key(secret: &str) -> Result<Key> {
    if secret.chars().count() < 8 {
        return Err(SyncError::Refused(
            "the sync secret must be at least 8 characters".into(),
        ));
    }
    let id = crypto::sha256(secret.as_bytes());
    if let Ok(cache) = KEY_CACHE.lock() {
        if let Some((cached_id, key)) = cache.as_ref() {
            if *cached_id == id {
                return Ok(key.clone());
            }
        }
    }
    let key = crypto::derive_key(
        secret,
        None,
        b"note-to-myself-sync-v1",
        &crypto::Kdf::Pbkdf2 {
            iterations: 210_000,
        },
    )
    .map_err(|e| SyncError::Protocol(e.to_string()))?;
    if let Ok(mut cache) = KEY_CACHE.lock() {
        *cache = Some((id, key.clone()));
    }
    Ok(key)
}

fn session_key(sync_key: &Key, nonce_c: &[u8; 16], nonce_s: &[u8; 16], encrypt: bool) -> Key {
    let mut salt = [0u8; 32];
    salt[..16].copy_from_slice(nonce_c);
    salt[16..].copy_from_slice(nonce_s);
    let hk = hkdf::Hkdf::<Sha256>::new(Some(&salt), sync_key.as_bytes());
    let mut out = [0u8; 32];
    // Le mode de chiffrement entre dans la dérivation : le modifier en route fait échouer l'authentification.
    let info = if encrypt {
        b"ntm-session-enc".as_slice()
    } else {
        b"ntm-session-mac".as_slice()
    };
    hk.expand(info, &mut out).expect("hkdf length");
    Key::from_bytes(out)
}

// --- Canal authentifié ---------------------------------------------------------------------------

struct Channel<S: Read + Write> {
    stream: S,
    key: Key,
    encrypt: bool,
    send_dir: u8,
    recv_dir: u8,
    send_counter: u64,
    recv_counter: u64,
}

impl<S: Read + Write> Channel<S> {
    fn new(stream: S, key: Key, encrypt: bool, is_client: bool) -> Self {
        let (send_dir, recv_dir) = if is_client { (0, 1) } else { (1, 0) };
        Self {
            stream,
            key,
            encrypt,
            send_dir,
            recv_dir,
            send_counter: 0,
            recv_counter: 0,
        }
    }

    fn nonce(dir: u8, counter: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[0] = dir;
        nonce[4..].copy_from_slice(&counter.to_be_bytes());
        nonce
    }

    fn mac(&self, dir: u8, counter: u64) -> Hmac<Sha256> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(self.key.as_bytes()).expect("hmac key");
        mac.update(&[dir]);
        mac.update(&counter.to_be_bytes());
        mac
    }

    fn send<T: Serialize>(&mut self, message: &T) -> Result<()> {
        let payload =
            serde_json::to_vec(message).map_err(|e| SyncError::Protocol(e.to_string()))?;
        let body = if self.encrypt {
            let cipher = Aes256Gcm::new_from_slice(self.key.as_bytes()).expect("aes key");
            let nonce = Self::nonce(self.send_dir, self.send_counter);
            cipher
                .encrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: &payload,
                        aad: &[self.send_dir],
                    },
                )
                .map_err(|_| SyncError::Auth)?
        } else {
            let mut mac = self.mac(self.send_dir, self.send_counter);
            mac.update(&payload);
            let mut body = payload;
            body.extend_from_slice(&mac.finalize().into_bytes());
            body
        };
        if body.len() > MAX_FRAME {
            return Err(SyncError::Protocol("message too large".into()));
        }
        self.stream.write_all(&(body.len() as u32).to_be_bytes())?;
        self.stream.write_all(&body)?;
        self.stream.flush()?;
        self.send_counter += 1;
        Ok(())
    }

    fn recv<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes)?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        if !(16..=MAX_FRAME).contains(&len) {
            return Err(SyncError::Protocol("invalid frame size".into()));
        }
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body)?;
        let payload = if self.encrypt {
            let cipher = Aes256Gcm::new_from_slice(self.key.as_bytes()).expect("aes key");
            let nonce = Self::nonce(self.recv_dir, self.recv_counter);
            cipher
                .decrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: &body,
                        aad: &[self.recv_dir],
                    },
                )
                .map_err(|_| SyncError::Auth)?
        } else {
            if body.len() < 32 {
                return Err(SyncError::Auth);
            }
            let (payload, tag) = body.split_at(body.len() - 32);
            let mut mac = self.mac(self.recv_dir, self.recv_counter);
            mac.update(payload);
            mac.verify_slice(tag).map_err(|_| SyncError::Auth)?;
            payload.to_vec()
        };
        self.recv_counter += 1;
        serde_json::from_slice(&payload).map_err(|e| SyncError::Protocol(e.to_string()))
    }
}

// --- Messages -------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    Sync,
    ConfigPull,
    ConfigPush,
}

#[derive(Debug, Serialize, Deserialize)]
struct Hello {
    version: u32,
    device_id: String,
    device_name: String,
    is_primary: bool,
    vault_fp: String,
    op: Op,
    mode: SyncMode,
}

#[derive(Debug, Serialize, Deserialize)]
struct HelloAck {
    device_id: String,
    device_name: String,
    is_primary: bool,
    ok: bool,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDto {
    uid: String,
    title: String,
    body: String,
    created_at: String,
    updated_at: String,
    deleted: bool,
    rev: i64,
}

impl From<&Record> for RecordDto {
    fn from(r: &Record) -> Self {
        Self {
            uid: r.uid.clone(),
            title: r.title.clone(),
            body: r.body.clone(),
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
            deleted: r.deleted,
            rev: r.rev,
        }
    }
}

impl From<RecordDto> for Record {
    fn from(d: RecordDto) -> Self {
        Self {
            id: 0,
            uid: d.uid,
            title: d.title,
            body: d.body,
            created_at: d.created_at,
            updated_at: d.updated_at,
            deleted: d.deleted,
            rev: d.rev.max(1),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Apply {
    pull: Vec<String>,
    push: Vec<RecordDto>,
    drop_remote: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFiles {
    pub manifest: String,
    pub source: String,
}

/// Configuration complète transférée à un autre PC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub settings: Settings,
    /// (nom de fichier, contenu JSON) des thèmes personnalisés.
    pub themes: Vec<(String, String)>,
    pub plugins: Vec<PluginFiles>,
}

#[derive(Debug, Serialize, Deserialize)]
enum Msg {
    Hello(Hello),
    HelloAck(HelloAck),
    Manifest(Vec<Entry>),
    Apply(Apply),
    Records(Vec<RecordDto>),
    Commit(Vec<(String, String)>),
    Config(Box<Bundle>),
    Done,
}

// --- Contexte ---------------------------------------------------------------------------------------

/// Tout ce dont une synchronisation a besoin sur ce PC.
#[derive(Debug, Clone)]
pub struct Local {
    pub settings: Settings,
    pub db_path: PathBuf,
    pub themes_dir: PathBuf,
    pub plugins_dir: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub peer: String,
    pub pushed: usize,
    pub pulled: usize,
    pub conflicts: usize,
    pub dropped: usize,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} sent, {} received, {} conflict(s) resolved, {} removed",
            self.peer, self.pushed, self.pulled, self.conflicts, self.dropped
        )
    }
}

#[derive(Debug)]
pub enum SyncEvent {
    NotesChanged,
    ConfigReceived(Box<Bundle>),
    Log(String),
}

pub type Events = Arc<Mutex<Vec<SyncEvent>>>;

fn push_event(events: &Events, event: SyncEvent) {
    if let Ok(mut queue) = events.lock() {
        // La file est vidée régulièrement par l'application ; on borne quand même sa taille.
        if queue.len() < 200 {
            queue.push(event);
        }
    }
}

fn open_db(path: &Path) -> Result<rusqlite::Connection> {
    let connection = store::open(path)?;
    store::ensure_schema(&connection)?;
    Ok(connection)
}

fn entries(connection: &rusqlite::Connection) -> Result<Vec<Entry>> {
    Ok(store::all_records(connection)?
        .iter()
        .map(|r| Entry {
            uid: r.uid.clone(),
            hash: r.content_hash(),
            updated_at: r.updated_at.clone(),
            deleted: r.deleted,
        })
        .collect())
}

// --- Configuration à transférer ------------------------------------------------------------------

pub fn build_bundle(local: &Local) -> Bundle {
    let mut themes = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&local.themes_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.extension().and_then(|e| e.to_str()) != Some("json")
                || themes.len() >= MAX_BUNDLE_ITEMS
            {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                if text.len() < 4096 && crate::theme::user_theme_from_json(&text).is_some() {
                    themes.push((name, text));
                }
            }
        }
    }
    let mut plugins = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&local.plugins_dir) {
        for entry in dir.flatten().take(MAX_BUNDLE_ITEMS) {
            if let Ok(plugin) = Plugin::load_dir(&entry.path()) {
                let manifest =
                    std::fs::read_to_string(entry.path().join("plugin.toml")).unwrap_or_default();
                plugins.push(PluginFiles {
                    manifest,
                    source: plugin.source,
                });
            }
        }
    }
    Bundle {
        settings: local.settings.to_transfer(),
        themes,
        plugins,
    }
}

/// Écrit les thèmes et plugins d'un paquet reçu, après validation. Les plugins reçus ne sont jamais
/// approuvés : l'utilisateur doit les approuver sur ce PC. Retourne (thèmes, plugins) installés.
pub fn install_bundle_files(
    bundle: &Bundle,
    themes_dir: &Path,
    plugins_dir: &Path,
) -> (usize, usize) {
    let mut themes = 0;
    for (_, json) in bundle.themes.iter().take(MAX_BUNDLE_ITEMS) {
        // Le nom de fichier est recalculé depuis le contenu validé : pas de chemin venu du réseau.
        if let Some(theme) = crate::theme::user_theme_from_json(json) {
            if crate::theme::save_user_theme(themes_dir, &theme).is_ok() {
                themes += 1;
            }
        }
    }
    let mut plugins = 0;
    for files in bundle.plugins.iter().take(MAX_BUNDLE_ITEMS) {
        let Ok(plugin) = Plugin::load(&files.manifest, &files.source, false) else {
            continue;
        };
        let dir = plugins_dir.join(&plugin.manifest.id);
        let written = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(dir.join("plugin.toml"), &files.manifest))
            .and_then(|_| std::fs::write(dir.join(&plugin.manifest.entry), &files.source));
        if written.is_ok() {
            plugins += 1;
        }
    }
    (themes, plugins)
}

// --- Client -------------------------------------------------------------------------------------------

fn connect(address: &str) -> Result<TcpStream> {
    let mut last = SyncError::Io(format!("cannot resolve {address}"));
    for socket in address.to_socket_addrs()? {
        match TcpStream::connect_timeout(&socket, Duration::from_secs(5)) {
            Ok(stream) => {
                stream.set_read_timeout(Some(IO_TIMEOUT))?;
                stream.set_write_timeout(Some(IO_TIMEOUT))?;
                stream.set_nodelay(true)?;
                return Ok(stream);
            }
            Err(e) => last = e.into(),
        }
    }
    Err(last)
}

/// Résultat d'une opération côté client.
#[derive(Debug)]
pub enum Outcome {
    Synced(Report),
    ConfigPulled(Box<Bundle>, String),
    ConfigPushed(String),
}

pub fn run_client(local: &Local, address: &str, op: Op) -> Result<Outcome> {
    let key = sync_key(&local.settings.sync.secret)?;
    let mut stream = connect(address)?;

    let nonce_c = crypto::random_bytes::<16>();
    let wants_encrypt = local.settings.sync.encrypt_transport;
    let mut init = Vec::with_capacity(21);
    init.extend_from_slice(MAGIC);
    init.extend_from_slice(&nonce_c);
    init.push(wants_encrypt as u8);
    stream.write_all(&init)?;
    let mut reply = [0u8; 17];
    stream.read_exact(&mut reply)?;
    let mut nonce_s = [0u8; 16];
    nonce_s.copy_from_slice(&reply[..16]);
    let encrypt = reply[16] == 1;
    if wants_encrypt && !encrypt {
        return Err(SyncError::Protocol(
            "peer refused transport encryption".into(),
        ));
    }

    let mut channel = Channel::new(
        stream,
        session_key(&key, &nonce_c, &nonce_s, encrypt),
        encrypt,
        true,
    );
    let sync = &local.settings.sync;
    channel.send(&Msg::Hello(Hello {
        version: PROTOCOL_VERSION,
        device_id: sync.device_id.clone(),
        device_name: sync.device_name.clone(),
        is_primary: sync.is_primary,
        vault_fp: vault_fingerprint(&local.settings),
        op,
        mode: sync.mode,
    }))?;
    let ack = match channel.recv::<Msg>()? {
        Msg::HelloAck(ack) => ack,
        _ => return Err(SyncError::Protocol("unexpected reply".into())),
    };
    if !ack.ok {
        return Err(SyncError::Refused(ack.reason));
    }

    match op {
        Op::ConfigPull => match channel.recv::<Msg>()? {
            Msg::Config(bundle) => {
                channel.send(&Msg::Done)?;
                Ok(Outcome::ConfigPulled(bundle, ack.device_name))
            }
            _ => Err(SyncError::Protocol("expected configuration".into())),
        },
        Op::ConfigPush => {
            channel.send(&Msg::Config(Box::new(build_bundle(local))))?;
            match channel.recv::<Msg>()? {
                Msg::Done => Ok(Outcome::ConfigPushed(ack.device_name)),
                _ => Err(SyncError::Protocol("peer did not confirm".into())),
            }
        }
        Op::Sync => client_sync(local, &mut channel, &ack).map(Outcome::Synced),
    }
}

fn client_sync(local: &Local, channel: &mut Channel<TcpStream>, ack: &HelloAck) -> Result<Report> {
    let remote_entries = match channel.recv::<Msg>()? {
        Msg::Manifest(entries) => entries,
        _ => return Err(SyncError::Protocol("expected manifest".into())),
    };
    let connection = open_db(&local.db_path)?;
    let local_entries = entries(&connection)?;
    let base = store::load_base(&connection, &ack.device_id)?;
    let plan = plan::plan(&PlanInput {
        local: &local_entries,
        remote: &remote_entries,
        base: &base,
        mode: local.settings.sync.mode,
        local_is_primary: local.settings.sync.is_primary,
        remote_is_primary: ack.is_primary,
    });

    let by_uid: HashMap<String, Record> = {
        let wanted: std::collections::HashSet<&String> = plan.push.iter().collect();
        store::all_records(&connection)?
            .into_iter()
            .filter(|r| wanted.contains(&r.uid))
            .map(|r| (r.uid.clone(), r))
            .collect()
    };
    let push: Vec<RecordDto> = plan
        .push
        .iter()
        .filter_map(|uid| by_uid.get(uid.as_str()))
        .map(RecordDto::from)
        .collect();

    channel.send(&Msg::Apply(Apply {
        pull: plan.pull.clone(),
        push,
        drop_remote: plan.drop_remote.clone(),
    }))?;
    let records = match channel.recv::<Msg>()? {
        Msg::Records(records) => records,
        _ => return Err(SyncError::Protocol("expected records".into())),
    };

    // Tout ce qui est reçu est vérifié (empreinte annoncée dans le manifeste) puis appliqué en une transaction.
    let expected: HashMap<&str, &str> = remote_entries
        .iter()
        .map(|e| (e.uid.as_str(), e.hash.as_str()))
        .collect();
    let transaction = connection.unchecked_transaction()?;
    for dto in records {
        let record: Record = dto.into();
        if !plan.pull.contains(&record.uid)
            || expected.get(record.uid.as_str()) != Some(&record.content_hash().as_str())
        {
            return Err(SyncError::Protocol(
                "received a note that does not match the manifest".into(),
            ));
        }
        store::upsert_remote(&transaction, &record)?;
    }
    for uid in &plan.drop_local {
        transaction.execute("DELETE FROM notes WHERE uid = ?1", [uid])?;
    }
    transaction.commit()?;
    store::save_base(&connection, &ack.device_id, &plan.new_base)?;

    channel.send(&Msg::Commit(
        plan.new_base
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    ))?;
    match channel.recv::<Msg>()? {
        Msg::Done => {}
        _ => return Err(SyncError::Protocol("peer did not confirm".into())),
    }
    Ok(Report {
        peer: ack.device_name.clone(),
        pushed: plan.push.len(),
        pulled: plan.pull.len(),
        conflicts: plan.conflicts.len(),
        dropped: plan.drop_local.len() + plan.drop_remote.len(),
    })
}

// --- Serveur -------------------------------------------------------------------------------------------

/// État partagé entre l'application et le thread d'écoute (mis à jour à chaque changement de réglages).
#[derive(Debug, Clone)]
pub struct ServerShared {
    pub local: Local,
}

pub struct Listener {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    pub port: u16,
}

impl Listener {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn start_listener(
    port: u16,
    shared: Arc<Mutex<ServerShared>>,
    events: Events,
) -> std::io::Result<Listener> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let handle = std::thread::spawn(move || serve(listener, shared, events, thread_stop));
    Ok(Listener {
        stop,
        handle: Some(handle),
        port,
    })
}

fn serve(
    listener: TcpListener,
    shared: Arc<Mutex<ServerShared>>,
    events: Events,
    stop: Arc<AtomicBool>,
) {
    let active = Arc::new(AtomicUsize::new(0));
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let Ok(snapshot) = shared.lock().map(|s| s.clone()) else {
                    continue;
                };
                if snapshot.local.settings.sync.trusted_networks_only && !is_trusted_ip(peer.ip()) {
                    push_event(
                        &events,
                        SyncEvent::Log(format!(
                            "connection from {} refused (untrusted network)",
                            peer.ip()
                        )),
                    );
                    continue;
                }
                if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                    active.fetch_sub(1, Ordering::SeqCst);
                    continue;
                }
                let (events, active) = (events.clone(), active.clone());
                std::thread::spawn(move || {
                    if let Err(error) = handle_connection(stream, &snapshot.local, &events) {
                        if matches!(error, SyncError::Auth) {
                            // Ralentit les essais de secret.
                            std::thread::sleep(Duration::from_millis(500));
                        }
                        push_event(
                            &events,
                            SyncEvent::Log(format!("sync from {}: {error}", peer.ip())),
                        );
                    }
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(150))
            }
            Err(_) => std::thread::sleep(Duration::from_millis(500)),
        }
    }
}

fn handle_connection(mut stream: TcpStream, local: &Local, events: &Events) -> Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.set_nodelay(true)?;
    let sync = &local.settings.sync;
    if !sync.enabled || sync.secret.is_empty() {
        return Err(SyncError::Refused("sync is not enabled".into()));
    }
    let key = sync_key(&sync.secret)?;

    let mut init = [0u8; 21];
    stream.read_exact(&mut init)?;
    if &init[..4] != MAGIC {
        return Err(SyncError::Protocol("not a Note to Myself client".into()));
    }
    let mut nonce_c = [0u8; 16];
    nonce_c.copy_from_slice(&init[4..20]);
    let encrypt = init[20] == 1 || sync.encrypt_transport;
    let nonce_s = crypto::random_bytes::<16>();
    let mut reply = Vec::with_capacity(17);
    reply.extend_from_slice(&nonce_s);
    reply.push(encrypt as u8);
    stream.write_all(&reply)?;

    let mut channel = Channel::new(
        stream,
        session_key(&key, &nonce_c, &nonce_s, encrypt),
        encrypt,
        false,
    );
    let hello = match channel.recv::<Msg>()? {
        Msg::Hello(hello) => hello,
        _ => return Err(SyncError::Protocol("expected hello".into())),
    };

    let refusal = if hello.version != PROTOCOL_VERSION {
        Some("incompatible protocol version".to_string())
    } else if hello.op == Op::Sync && hello.vault_fp != vault_fingerprint(&local.settings) {
        Some("the encryption setup differs between the two PCs: pull the configuration from the primary first".to_string())
    } else if (hello.op == Op::ConfigPush || (hello.op == Op::Sync && hello.mode == SyncMode::Push))
        && (!hello.is_primary || sync.is_primary)
    {
        Some("only the primary PC may overwrite this PC".to_string())
    } else {
        None
    };
    channel.send(&Msg::HelloAck(HelloAck {
        device_id: sync.device_id.clone(),
        device_name: sync.device_name.clone(),
        is_primary: sync.is_primary,
        ok: refusal.is_none(),
        reason: refusal.clone().unwrap_or_default(),
    }))?;
    if let Some(reason) = refusal {
        return Err(SyncError::Refused(reason));
    }

    match hello.op {
        Op::ConfigPull => {
            channel.send(&Msg::Config(Box::new(build_bundle(local))))?;
            let _ = channel.recv::<Msg>();
            push_event(
                events,
                SyncEvent::Log(format!("{} pulled the configuration", hello.device_name)),
            );
        }
        Op::ConfigPush => match channel.recv::<Msg>()? {
            Msg::Config(bundle) => {
                push_event(events, SyncEvent::ConfigReceived(bundle));
                channel.send(&Msg::Done)?;
            }
            _ => return Err(SyncError::Protocol("expected configuration".into())),
        },
        Op::Sync => server_sync(local, &mut channel, &hello, events)?,
    }
    Ok(())
}

fn server_sync(
    local: &Local,
    channel: &mut Channel<TcpStream>,
    hello: &Hello,
    events: &Events,
) -> Result<()> {
    let connection = open_db(&local.db_path)?;
    channel.send(&Msg::Manifest(entries(&connection)?))?;
    let apply = match channel.recv::<Msg>()? {
        Msg::Apply(apply) => apply,
        _ => return Err(SyncError::Protocol("expected apply".into())),
    };

    let transaction = connection.unchecked_transaction()?;
    for dto in apply.push {
        store::upsert_remote(&transaction, &dto.into())?;
    }
    if !apply.drop_remote.is_empty() {
        // Seul le mode « push » du PC principal peut supprimer définitivement (déjà vérifié à l'accueil).
        if hello.mode != SyncMode::Push {
            return Err(SyncError::Protocol("unexpected removal request".into()));
        }
        for uid in &apply.drop_remote {
            transaction.execute("DELETE FROM notes WHERE uid = ?1", [uid])?;
        }
    }
    transaction.commit()?;

    let wanted: std::collections::HashSet<&String> = apply.pull.iter().collect();
    let records: Vec<RecordDto> = store::all_records(&connection)?
        .iter()
        .filter(|r| wanted.contains(&r.uid))
        .map(RecordDto::from)
        .collect();
    channel.send(&Msg::Records(records))?;

    match channel.recv::<Msg>()? {
        Msg::Commit(base) => {
            store::save_base(&connection, &hello.device_id, &base.into_iter().collect())?;
            channel.send(&Msg::Done)?;
        }
        _ => return Err(SyncError::Protocol("expected commit".into())),
    }
    push_event(events, SyncEvent::NotesChanged);
    Ok(())
}

#[cfg(test)]
mod tests;
