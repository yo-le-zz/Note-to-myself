use super::*;
use crate::settings::Settings;

struct Pc {
    local: Local,
    shared: Arc<Mutex<ServerShared>>,
    events: Events,
    listener: Option<Listener>,
    dir: PathBuf,
}

impl Pc {
    fn new(name: &str, primary: bool, secret: &str) -> Pc {
        let dir = std::env::temp_dir().join(format!("ntm-sync-{name}-{}", store::new_uid()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut settings = Settings::default();
        settings.sanitize();
        settings.sync.enabled = true;
        settings.sync.is_primary = primary;
        settings.sync.device_name = name.to_string();
        settings.sync.secret = secret.to_string();
        let local = Local {
            settings,
            db_path: dir.join("notes.sqlite3"),
            themes_dir: dir.join("themes"),
            plugins_dir: dir.join("plugins"),
        };
        open_db(&local.db_path).unwrap();
        let shared = Arc::new(Mutex::new(ServerShared::new(local.clone())));
        Pc {
            local,
            shared,
            events: Arc::default(),
            listener: None,
            dir,
        }
    }

    fn listen(&mut self) -> String {
        let listener = start_listener(0, self.shared.clone(), self.events.clone()).unwrap();
        let address = format!("127.0.0.1:{}", listener.port);
        self.listener = Some(listener);
        address
    }

    fn db(&self) -> rusqlite::Connection {
        open_db(&self.local.db_path).unwrap()
    }

    fn add(&self, title: &str, body: &str, when: &str) -> String {
        store::insert(&self.db(), title, body, when).unwrap()
    }

    fn edit(&self, uid: &str, body: &str, when: &str) {
        let title = store::record_by_uid(&self.db(), uid)
            .unwrap()
            .unwrap()
            .title;
        assert!(store::update(&self.db(), uid, &title, body, when).unwrap());
    }

    fn live(&self) -> Vec<(String, String)> {
        let mut notes: Vec<_> = store::live_records(&self.db())
            .unwrap()
            .into_iter()
            .map(|r| (r.title, r.body))
            .collect();
        notes.sort();
        notes
    }

    fn set_mode(&mut self, mode: SyncMode) {
        self.local.settings.sync.mode = mode;
        self.shared.lock().unwrap().local = self.local.clone();
    }

    fn set_transport(&mut self, encrypt: bool) {
        self.local.settings.sync.encrypt_transport = encrypt;
        self.shared.lock().unwrap().local = self.local.clone();
    }
}

impl Drop for Pc {
    fn drop(&mut self) {
        if let Some(mut l) = self.listener.take() {
            l.stop();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

const SECRET: &str = "secret-de-test-partage";

fn sync_ok(client: &Pc, address: &str) -> Report {
    match run_client(&client.local, address, Op::Sync).unwrap() {
        Outcome::Synced(report) => report,
        _ => panic!("unexpected outcome"),
    }
}

#[test]
fn smart_sync_merges_adds_edits_and_deletes() {
    let mut primary = Pc::new("primary", true, SECRET);
    let secondary = Pc::new("secondary", false, SECRET);
    let address = primary.listen();

    let shared_note = primary.add("partagée", "v1", "2026-01-01T00:00:00");
    primary.add("seulement-principal", "p", "2026-01-01T00:00:00");
    secondary.add("seulement-secondaire", "s", "2026-01-01T00:00:00");

    let report = sync_ok(&secondary, &address);
    assert_eq!((report.pushed, report.pulled), (1, 2));
    assert_eq!(primary.live(), secondary.live());
    assert_eq!(primary.live().len(), 3);

    // Modification d'un côté seulement, puis suppression de l'autre côté.
    let uid_on_secondary = store::live_records(&secondary.db())
        .unwrap()
        .into_iter()
        .find(|r| r.title == "partagée")
        .unwrap()
        .uid;
    assert_eq!(uid_on_secondary, shared_note);
    secondary.edit(&shared_note, "v2", "2026-01-02T00:00:00");
    let victim = store::live_records(&primary.db())
        .unwrap()
        .into_iter()
        .find(|r| r.title == "seulement-principal")
        .unwrap()
        .uid;
    store::soft_delete(&primary.db(), &victim, "2026-01-02T00:00:00").unwrap();

    let report = sync_ok(&secondary, &address);
    assert_eq!(report.conflicts, 0);
    assert_eq!(primary.live(), secondary.live());
    assert!(primary.live().contains(&("partagée".into(), "v2".into())));
    assert!(!primary
        .live()
        .iter()
        .any(|(t, _)| t == "seulement-principal"));

    // Une troisième synchronisation ne change plus rien.
    let again = sync_ok(&secondary, &address);
    assert_eq!((again.pushed, again.pulled, again.conflicts), (0, 0, 0));
}

#[test]
fn the_primary_has_the_last_word_on_conflicts() {
    let mut primary = Pc::new("primary", true, SECRET);
    let secondary = Pc::new("secondary", false, SECRET);
    let address = primary.listen();
    let uid = primary.add("note", "origine", "2026-01-01T00:00:00");
    sync_ok(&secondary, &address);

    // Les deux côtés modifient ; la modification du secondaire est la plus récente.
    primary.edit(&uid, "version principale", "2026-01-02T00:00:00");
    secondary.edit(&uid, "version secondaire", "2026-01-03T00:00:00");
    let report = sync_ok(&secondary, &address);
    assert_eq!(report.conflicts, 1);
    assert_eq!(
        primary.live(),
        vec![("note".to_string(), "version principale".to_string())]
    );
    assert_eq!(secondary.live(), primary.live());
}

#[test]
fn transport_works_encrypted_and_in_the_clear() {
    for encrypt in [true, false] {
        let mut primary = Pc::new("primary", true, SECRET);
        let mut secondary = Pc::new("secondary", false, SECRET);
        primary.set_transport(encrypt);
        secondary.set_transport(encrypt);
        let address = primary.listen();
        primary.add("note", "contenu", "2026-01-01T00:00:00");
        sync_ok(&secondary, &address);
        assert_eq!(secondary.live().len(), 1, "encrypt = {encrypt}");
    }
}

#[test]
fn the_client_can_force_transport_encryption() {
    let mut primary = Pc::new("primary", true, SECRET);
    let secondary = Pc::new("secondary", false, SECRET);
    primary.set_transport(false);
    let address = primary.listen();
    primary.add("note", "contenu", "2026-01-01T00:00:00");
    // Le client veut du chiffré (réglage par défaut) : le serveur s'aligne.
    sync_ok(&secondary, &address);
    assert_eq!(secondary.live().len(), 1);
}

#[test]
fn a_wrong_secret_is_rejected_and_nothing_is_copied() {
    let mut primary = Pc::new("primary", true, SECRET);
    let intruder = Pc::new("intruder", false, "un-autre-secret-complet");
    let address = primary.listen();
    primary.add("privée", "contenu", "2026-01-01T00:00:00");
    assert!(run_client(&intruder.local, &address, Op::Sync).is_err());
    assert!(intruder.live().is_empty());
    assert!(matches!(
        run_client(&Pc::new("short", false, "court").local, &address, Op::Sync),
        Err(SyncError::Refused(_))
    ));
}

#[test]
fn a_disabled_peer_refuses_connections() {
    let mut primary = Pc::new("primary", true, SECRET);
    primary.local.settings.sync.enabled = false;
    primary.shared.lock().unwrap().local = primary.local.clone();
    let secondary = Pc::new("secondary", false, SECRET);
    let address = primary.listen();
    assert!(run_client(&secondary.local, &address, Op::Sync).is_err());
}

#[test]
fn different_vaults_do_not_sync_notes() {
    let mut primary = Pc::new("primary", true, SECRET);
    let mut secondary = Pc::new("secondary", false, SECRET);
    primary.local.settings.security.enabled = true;
    primary.local.settings.security.salt = "c2VsLXNlbC1zZWw=".into();
    primary.local.settings.security.verifier = "enc:v1:xxxx".into();
    primary.shared.lock().unwrap().local = primary.local.clone();
    secondary.set_transport(true);
    let address = primary.listen();
    match run_client(&secondary.local, &address, Op::Sync) {
        Err(SyncError::Refused(reason)) => assert!(reason.contains("encryption")),
        other => panic!("expected refusal, got {other:?}"),
    }
    // En revanche la configuration (qui contient le coffre) peut être récupérée.
    match run_client(&secondary.local, &address, Op::ConfigPull).unwrap() {
        Outcome::ConfigPulled(bundle, _) => {
            assert_eq!(bundle.settings.security.salt, "c2VsLXNlbC1zZWw=")
        }
        _ => panic!("expected a bundle"),
    }
}

#[test]
fn push_mode_is_reserved_to_the_primary() {
    let mut primary = Pc::new("primary", true, SECRET);
    let mut secondary = Pc::new("secondary", false, SECRET);
    let address = primary.listen();
    primary.add("note du principal", "x", "2026-01-01T00:00:00");
    secondary.add("note locale", "y", "2026-01-01T00:00:00");

    secondary.set_mode(SyncMode::Push);
    assert!(matches!(
        run_client(&secondary.local, &address, Op::Sync),
        Err(SyncError::Refused(_))
    ));
    assert_eq!(primary.live().len(), 1, "the primary must be untouched");

    // Miroir : le secondaire devient une copie exacte du principal.
    secondary.set_mode(SyncMode::Mirror);
    let report = sync_ok(&secondary, &address);
    assert_eq!((report.pulled, report.dropped), (1, 1));
    assert_eq!(secondary.live(), primary.live());
}

#[test]
fn primary_push_makes_the_peer_a_copy() {
    let mut secondary = Pc::new("secondary", false, SECRET);
    let mut primary = Pc::new("primary", true, SECRET);
    let address = secondary.listen();
    primary.add("a garder", "x", "2026-01-01T00:00:00");
    secondary.add("a effacer", "y", "2026-01-01T00:00:00");
    primary.set_mode(SyncMode::Push);
    sync_ok(&primary, &address);
    assert_eq!(secondary.live(), primary.live());
    assert_eq!(secondary.live().len(), 1);
}

#[test]
fn union_mode_never_overwrites_or_deletes() {
    let mut primary = Pc::new("primary", true, SECRET);
    let mut secondary = Pc::new("secondary", false, SECRET);
    let address = primary.listen();
    let uid = primary.add("note", "principal", "2026-01-01T00:00:00");
    sync_ok(&secondary, &address);
    primary.edit(&uid, "modifiée ici", "2026-01-02T00:00:00");
    secondary.add("nouvelle", "z", "2026-01-02T00:00:00");
    secondary.set_mode(SyncMode::Union);
    sync_ok(&secondary, &address);
    assert!(
        secondary
            .live()
            .contains(&("note".into(), "principal".into())),
        "union must not modify"
    );
    assert!(
        primary.live().contains(&("nouvelle".into(), "z".into())),
        "union must add missing notes"
    );
}

#[test]
fn config_transfer_carries_themes_and_plugins_but_no_secret() {
    let mut primary = Pc::new("primary", true, SECRET);
    let secondary = Pc::new("secondary", false, SECRET);
    primary.local.settings.theme.id = "dracula".into();
    primary.local.settings.security.salt = "c2VsLXNlbC1zZWw=".into();
    let theme = crate::theme::user_theme_from_json(
        r##"{"name":"Perso","dark":true,"bg":"#101010","fg":"#eeeeee","accent":"#ff8800"}"##,
    )
    .unwrap();
    crate::theme::save_user_theme(&primary.local.themes_dir, &theme).unwrap();
    let plugin_dir = primary.local.plugins_dir.join("hello");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.toml"),
        "id = \"hello\"\nname = \"Hello\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        plugin_dir.join("main.rhai"),
        "fn on_start() { ui_status(\"hi\"); }",
    )
    .unwrap();
    primary.shared.lock().unwrap().local = primary.local.clone();
    let address = primary.listen();

    let Outcome::ConfigPulled(bundle, from) =
        run_client(&secondary.local, &address, Op::ConfigPull).unwrap()
    else {
        panic!("expected a bundle")
    };
    assert_eq!(from, "primary");
    assert_eq!(bundle.settings.theme.id, "dracula");
    assert_eq!(bundle.settings.security.salt, "c2VsLXNlbC1zZWw=");
    let serialized = serde_json::to_string(&bundle).unwrap();
    assert!(
        !serialized.contains(SECRET),
        "the sync secret must never be transferred"
    );

    let (themes, plugins) = install_bundle_files(
        &bundle,
        &secondary.local.themes_dir,
        &secondary.local.plugins_dir,
    );
    assert_eq!((themes, plugins), (1, 1));
    assert_eq!(
        crate::theme::load_user_themes(&secondary.local.themes_dir).len(),
        1
    );
    assert!(Plugin::load_dir(&secondary.local.plugins_dir.join("hello")).is_ok());
}

#[test]
fn bundle_files_from_the_network_cannot_escape_their_folders() {
    let dir = std::env::temp_dir().join(format!("ntm-bundle-{}", store::new_uid()));
    let bundle = Bundle {
        settings: Settings::default(),
        themes: vec![(
            "../../evil.json".into(),
            r##"{"name":"x","dark":true,"bg":"#000000","fg":"#ffffff","accent":"#ff0000"}"##.into(),
        )],
        plugins: vec![
            PluginFiles {
                manifest: "id = \"../evil\"\nname = \"x\"\nversion = \"1\"\n".into(),
                source: "let a = 1;".into(),
            },
            PluginFiles {
                manifest:
                    "id = \"bad-entry\"\nname = \"x\"\nversion = \"1\"\nentry = \"../x.rhai\"\n"
                        .into(),
                source: "let a = 1;".into(),
            },
            PluginFiles {
                manifest: "id = \"auto-trim\"\nname = \"x\"\nversion = \"1\"\n".into(),
                source: "let a = 1;".into(),
            },
            PluginFiles {
                manifest: "id = \"evil-code\"\nname = \"x\"\nversion = \"1\"\n".into(),
                source: "eval(\"1\");".into(),
            },
        ],
    };
    let (themes, plugins) =
        install_bundle_files(&bundle, &dir.join("themes"), &dir.join("plugins"));
    assert_eq!((themes, plugins), (1, 0));
    assert!(!dir.join("evil.json").exists() && !dir.parent().unwrap().join("evil.json").exists());
    assert!(
        !dir.join("plugins").exists()
            || std::fs::read_dir(dir.join("plugins")).unwrap().count() == 0
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn config_push_only_from_the_primary() {
    let mut secondary = Pc::new("secondary", false, SECRET);
    let primary = Pc::new("primary", true, SECRET);
    let address = secondary.listen();
    assert!(matches!(
        run_client(&primary.local, &address, Op::ConfigPush).unwrap(),
        Outcome::ConfigPushed(_)
    ));
    let events = secondary.events.lock().unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, SyncEvent::ConfigReceived(_))));
    drop(events);

    let mut other_primary = Pc::new("primary-b", true, SECRET);
    let address = other_primary.listen();
    assert!(matches!(
        run_client(&secondary.local, &address, Op::ConfigPush),
        Err(SyncError::Refused(_))
    ));
}

#[test]
fn untrusted_clients_are_dropped_before_any_exchange() {
    use std::net::{Ipv4Addr, Ipv6Addr};
    for ok in [
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
        IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)),
        IpAddr::V4(Ipv4Addr::new(172, 20, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7)),
        IpAddr::V4(Ipv4Addr::new(100, 127, 255, 1)),
        IpAddr::V6("fd7a:115c:a1e0::1".parse::<Ipv6Addr>().unwrap()),
        IpAddr::V6("::1".parse::<Ipv6Addr>().unwrap()),
    ] {
        assert!(is_trusted_ip(ok), "{ok}");
    }
    for bad in [
        IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
        IpAddr::V4(Ipv4Addr::new(100, 63, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(100, 128, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1)),
        IpAddr::V6("2001:4860:4860::8888".parse::<Ipv6Addr>().unwrap()),
    ] {
        assert!(!is_trusted_ip(bad), "{bad}");
    }
}

#[test]
fn garbage_and_oversized_input_cannot_crash_the_server() {
    let mut primary = Pc::new("primary", true, SECRET);
    let address = primary.listen();
    for payload in [
        b"GET / HTTP/1.1\r\n\r\n".to_vec(),
        vec![0u8; 21],
        [
            b"NTM1".as_slice(),
            &[7u8; 16],
            &[1u8],
            &[0xff, 0xff, 0xff, 0xff],
        ]
        .concat(),
    ] {
        let mut stream = TcpStream::connect(&address).unwrap();
        let _ = stream.write_all(&payload);
        let mut sink = [0u8; 64];
        let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
        let _ = stream.read(&mut sink);
    }
    // Le serveur répond toujours normalement ensuite.
    let secondary = Pc::new("secondary", false, SECRET);
    primary.add("ok", "ok", "2026-01-01T00:00:00");
    sync_ok(&secondary, &address);
    assert_eq!(secondary.live().len(), 1);
}

#[test]
fn received_notes_must_match_the_manifest() {
    // Garde-fou : le format d'une note reçue est vérifié par empreinte avant écriture.
    let record = Record {
        id: 0,
        uid: "u".into(),
        title: "t".into(),
        body: "b".into(),
        created_at: "c".into(),
        updated_at: "d".into(),
        deleted: false,
        rev: 1,
    };
    let dto = RecordDto::from(&record);
    let back: Record = dto.into();
    assert_eq!(back.content_hash(), record.content_hash());
}

#[test]
fn peer_addresses_are_normalized_or_rejected() {
    let ok = |input: &str| normalize_peer(input, 47474);
    assert_eq!(ok("100.64.0.2").as_deref(), Some("100.64.0.2:47474"));
    assert_eq!(ok("100.64.0.2:5000").as_deref(), Some("100.64.0.2:5000"));
    assert_eq!(
        ok("  mon-pc.tailnet.ts.net ").as_deref(),
        Some("mon-pc.tailnet.ts.net:47474")
    );
    assert_eq!(
        ok("fd7a:115c:a1e0::1").as_deref(),
        Some("[fd7a:115c:a1e0::1]:47474")
    );
    assert_eq!(
        ok("[fd7a:115c:a1e0::1]").as_deref(),
        Some("[fd7a:115c:a1e0::1]:47474")
    );
    assert_eq!(ok("[::1]:6000").as_deref(), Some("[::1]:6000"));
    for bad in [
        "",
        "   ",
        "host:",
        "host:0",
        "host:70000",
        "host:abc",
        "a b",
        "[::1",
        "[nope]:1",
        "a:b:c",
        "ho/st",
        "x\n:1",
    ] {
        assert_eq!(ok(bad), None, "{bad:?}");
    }
    assert_eq!(ok(&"a".repeat(300)), None);
}

#[test]
fn a_peer_cannot_inject_odd_identifiers_or_names() {
    assert!(valid_device_id("0123456789abcdef"));
    for bad in [
        "",
        "short",
        "zzzzzzzzzzzz",
        "../../etc/passwd",
        &"a".repeat(33),
    ] {
        assert!(!valid_device_id(bad), "{bad}");
    }
    assert_eq!(clean_name("pc\n\u{1b}[31mx"), "pc[31mx");
    assert_eq!(clean_name(&"é".repeat(100)).chars().count(), 40);
    assert_eq!(clean_name("  "), "unknown");
    assert!(store::valid_uid("0123abcd-_"));
    for bad in ["", "a b", "x'; DROP", &"a".repeat(65), "é"] {
        assert!(!store::valid_uid(bad), "{bad}");
    }
}

#[test]
fn the_server_pauses_while_the_vault_is_being_changed() {
    let mut primary = Pc::new("primary", true, SECRET);
    let secondary = Pc::new("secondary", false, SECRET);
    let address = primary.listen();
    primary.add("note", "x", "2026-01-01T00:00:00");
    primary.shared.lock().unwrap().maintenance = true;
    assert!(run_client(&secondary.local, &address, Op::Sync).is_err());
    assert!(secondary.live().is_empty());
    primary.shared.lock().unwrap().maintenance = false;
    sync_ok(&secondary, &address);
    assert_eq!(secondary.live().len(), 1);
}

#[test]
fn unauthenticated_oversized_frames_are_rejected_without_allocating() {
    let mut primary = Pc::new("primary", true, SECRET);
    let address = primary.listen();
    let mut stream = TcpStream::connect(&address).unwrap();
    let mut init = Vec::new();
    init.extend_from_slice(MAGIC);
    init.extend_from_slice(&[9u8; 16]);
    init.push(0);
    // Annonce une trame de 60 Mio avant toute authentification.
    init.extend_from_slice(&(60u32 * 1024 * 1024).to_be_bytes());
    stream.write_all(&init).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut sink = [0u8; 64];
    let mut closed = false;
    for _ in 0..20 {
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => {
                closed = true;
                break;
            }
            Ok(_) => continue,
        }
    }
    assert!(closed, "the server must close the connection");
}

#[test]
fn a_note_edited_during_the_exchange_is_detected_and_not_overwritten() {
    let pc = Pc::new("pc", false, SECRET);
    let uid = pc.add("note", "version planifiée", "2026-01-01T00:00:00");
    let connection = pc.db();
    let planned: HashMap<String, String> = store::all_records(&connection)
        .unwrap()
        .iter()
        .map(|r| (r.uid.clone(), r.content_hash()))
        .collect();
    assert!(!changed_since_plan(&connection, &uid, &planned).unwrap());
    // L'utilisateur modifie la note pendant que le réseau travaille.
    pc.edit(&uid, "modifiée entre-temps", "2026-01-02T00:00:00");
    assert!(changed_since_plan(&connection, &uid, &planned).unwrap());
    // Supprimée entre-temps : également détecté.
    store::soft_delete(&connection, &uid, "2026-01-03T00:00:00").unwrap();
    assert!(changed_since_plan(&connection, &uid, &planned).unwrap());
    // Une note créée entre-temps (absente du plan) l'est aussi.
    let created = pc.add("nouvelle", "x", "2026-01-04T00:00:00");
    assert!(changed_since_plan(&connection, &created, &planned).unwrap());
    // Une note absente partout n'a pas changé.
    assert!(!changed_since_plan(&connection, "inconnue", &planned).unwrap());
}
