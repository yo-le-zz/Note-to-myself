//! Réglages persistés (`settings.json`). Tous les champs ont une valeur par défaut :
//! un ancien fichier (v1.0.0) se charge sans perte.

use crate::crypto::{Cipher, Kdf};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Home,
    Write,
    See,
    Settings,
    Themes,
    Security,
    Sync,
    Plugins,
    /// Panneau d'interface fourni par un plugin.
    Panel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    French,
    Spanish,
    German,
    Italian,
    Portuguese,
    Dutch,
    Polish,
    Russian,
    Ukrainian,
    Turkish,
    Arabic,
    Japanese,
    Korean,
    Chinese,
    Hindi,
    Swedish,
    Greek,
    Romanian,
    Czech,
}

impl Language {
    pub const ALL: [Language; 20] = [
        Self::English,
        Self::French,
        Self::Spanish,
        Self::German,
        Self::Italian,
        Self::Portuguese,
        Self::Dutch,
        Self::Polish,
        Self::Russian,
        Self::Ukrainian,
        Self::Turkish,
        Self::Arabic,
        Self::Japanese,
        Self::Korean,
        Self::Chinese,
        Self::Hindi,
        Self::Swedish,
        Self::Greek,
        Self::Romanian,
        Self::Czech,
    ];
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::English => "English",
            Self::French => "Français",
            Self::Spanish => "Español",
            Self::German => "Deutsch",
            Self::Italian => "Italiano",
            Self::Portuguese => "Português",
            Self::Dutch => "Nederlands",
            Self::Polish => "Polski",
            Self::Russian => "Русский",
            Self::Ukrainian => "Українська",
            Self::Turkish => "Türkçe",
            Self::Arabic => "العربية",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Chinese => "中文",
            Self::Hindi => "हिन्दी",
            Self::Swedish => "Svenska",
            Self::Greek => "Ελληνικά",
            Self::Romanian => "Română",
            Self::Czech => "Čeština",
        };
        write!(f, "{name}")
    }
}

/// Apparence : thème de base + surcharges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSettings {
    /// Identifiant du thème de base (vide = ancien réglage clair/sombre).
    pub id: String,
    /// Couleur d'accent personnalisée `#rrggbb`.
    pub accent: Option<String>,
    /// Arrondi des coins, en pixels.
    pub radius: u16,
    /// Épaisseur des bordures, en pixels.
    pub border_width: u16,
    /// Espacement de l'interface, en pourcents (70 à 140).
    pub spacing: u16,
    /// Contraste des panneaux par rapport au fond, de -10 à +10.
    pub panel_contrast: i8,
    /// Police à chasse fixe dans l'éditeur.
    pub editor_monospace: bool,
    /// Police à chasse fixe dans toute l'interface.
    pub ui_monospace: bool,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            id: String::new(),
            accent: None,
            radius: 9,
            border_width: 1,
            spacing: 100,
            panel_contrast: 0,
            editor_monospace: false,
            ui_monospace: false,
        }
    }
}

impl ThemeSettings {
    pub fn clamp(&mut self) {
        self.radius = self.radius.min(28);
        self.border_width = self.border_width.min(4);
        self.spacing = self.spacing.clamp(70, 140);
        self.panel_contrast = self.panel_contrast.clamp(-10, 10);
        if let Some(accent) = &self.accent {
            if crate::theme::parse_hex(accent).is_none() {
                self.accent = None;
            }
        }
    }
}

/// Configuration du coffre chiffré. Rien ici n'est secret : sel, paramètres de dérivation
/// et vérificateur (un texte connu chiffré) peuvent donc voyager avec la configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SecuritySettings {
    pub enabled: bool,
    pub cipher: Cipher,
    pub kdf: Kdf,
    /// Sel en base64.
    pub salt: String,
    /// Texte connu chiffré avec la clé : sert à vérifier le mot de passe.
    pub verifier: String,
    pub uses_keyfile: bool,
    /// Verrouillage automatique après inactivité (0 = jamais).
    pub auto_lock_minutes: u32,
    /// Numéro de version de cette configuration, incrémenté à chaque changement du coffre. La même
    /// configuration est écrite dans la base, dans la transaction qui rechiffre les notes : après
    /// une coupure, la plus récente des deux gagne (voir `vault::reconcile`).
    pub rev: u64,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            cipher: Cipher::default(),
            kdf: Kdf::default(),
            salt: String::new(),
            verifier: String::new(),
            uses_keyfile: false,
            auto_lock_minutes: 5,
            rev: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SyncMode {
    /// Comme git : ajouts, modifications et suppressions fusionnés ; le PC principal gagne les conflits.
    #[default]
    Smart,
    /// La version la plus récente de chaque note l'emporte.
    Newest,
    /// Uniquement les notes manquantes, dans les deux sens ; rien n'est modifié ni supprimé.
    Union,
    /// Ce PC devient une copie exacte du pair (les changements locaux sont écrasés).
    Mirror,
    /// Le pair devient une copie exacte de ce PC (réservé au PC principal).
    Push,
}

impl SyncMode {
    pub const ALL: [SyncMode; 5] = [
        Self::Smart,
        Self::Newest,
        Self::Union,
        Self::Mirror,
        Self::Push,
    ];
}

impl fmt::Display for SyncMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Smart => "Smart (git-like, primary wins)",
            Self::Newest => "Newest wins",
            Self::Union => "Union (add only)",
            Self::Mirror => "Mirror (pull from peer)",
            Self::Push => "Push (peer becomes a copy)",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncSettings {
    pub enabled: bool,
    pub device_id: String,
    pub device_name: String,
    /// Ce PC a le dernier mot en cas de conflit.
    pub is_primary: bool,
    /// Accepter les connexions des autres PC.
    pub listen: bool,
    pub port: u16,
    /// Refuser les connexions hors réseau local / Tailscale.
    pub trusted_networks_only: bool,
    /// Secret d'appairage partagé. Jamais transféré avec la configuration.
    pub secret: String,
    /// Pairs `hôte:port` (adresse Tailscale, nom MagicDNS, IP locale…).
    pub peers: Vec<String>,
    pub mode: SyncMode,
    pub interval_secs: u32,
    /// Chiffrer les échanges réseau (l'intégrité est toujours protégée).
    pub encrypt_transport: bool,
    pub sync_on_save: bool,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            device_id: String::new(),
            device_name: String::new(),
            is_primary: false,
            listen: true,
            port: 47474,
            trusted_networks_only: true,
            secret: String::new(),
            peers: Vec::new(),
            mode: SyncMode::Smart,
            interval_secs: 60,
            encrypt_transport: true,
            sync_on_save: true,
        }
    }
}

/// Plugins activés : identifiant → empreinte du code que l'utilisateur a approuvé.
/// Un plugin est actif seulement si son empreinte actuelle est identique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PluginSettings {
    pub approved: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub dark_mode: bool,
    pub language: Language,
    pub show_timestamps: bool,
    pub compact_mode: bool,
    pub confirm_delete: bool,
    pub font_size: u16,
    pub theme: ThemeSettings,
    pub security: SecuritySettings,
    pub sync: SyncSettings,
    pub plugins: PluginSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark_mode: true,
            language: Language::English,
            show_timestamps: true,
            compact_mode: false,
            confirm_delete: true,
            font_size: 15,
            theme: ThemeSettings::default(),
            security: SecuritySettings::default(),
            sync: SyncSettings::default(),
            plugins: PluginSettings::default(),
        }
    }
}

impl Settings {
    /// Remet dans des bornes sûres les valeurs d'un fichier édité à la main ou reçu d'un pair.
    pub fn sanitize(&mut self) {
        self.font_size = self.font_size.clamp(12, 24);
        self.theme.clamp();
        self.sync.interval_secs = self.sync.interval_secs.clamp(10, 86_400);
        self.sync.port = self.sync.port.max(1024);
        self.security.auto_lock_minutes = self.security.auto_lock_minutes.min(1_440);
        // Un identifiant modifié à la main ou venu d'ailleurs est remplacé : il sert de clé dans la base
        // de synchronisation et ne doit contenir que de l'hexadécimal.
        let id = &self.sync.device_id;
        let id_ok = (8..=32).contains(&id.len()) && id.chars().all(|c| c.is_ascii_hexdigit());
        if !id_ok {
            self.sync.device_id = crate::crypto::to_hex(&crate::crypto::random_bytes::<8>());
        }
        let name: String = self
            .sync
            .device_name
            .chars()
            .filter(|c| !c.is_control())
            .take(40)
            .collect();
        self.sync.device_name = if name.trim().is_empty() {
            let short: String = self.sync.device_id.chars().take(6).collect();
            format!("pc-{short}")
        } else {
            name
        };
        self.sync
            .peers
            .retain(|p| !p.is_empty() && p.len() <= 200 && !p.chars().any(|c| c.is_control()));
        self.sync.peers.truncate(64);
    }

    /// Partie de la configuration copiée vers un autre PC : tout sauf ce qui est propre à la machine
    /// (identité, secret d'appairage, pairs, port, rôle) et les autorisations de plugins, qui doivent
    /// être accordées localement.
    pub fn to_transfer(&self) -> Settings {
        let mut copy = self.clone();
        let local = Settings::default();
        copy.sync = SyncSettings {
            mode: self.sync.mode,
            interval_secs: self.sync.interval_secs,
            encrypt_transport: self.sync.encrypt_transport,
            sync_on_save: self.sync.sync_on_save,
            trusted_networks_only: self.sync.trusted_networks_only,
            ..local.sync
        };
        copy.plugins.approved.clear();
        copy
    }

    /// Applique une configuration reçue en conservant l'identité locale du PC.
    ///
    /// Une configuration reçue ne peut jamais affaiblir la sécurité de ce PC : le chiffrement n'est
    /// pas désactivé, le réseau de confiance et le chiffrement du transport ne sont pas relâchés, et le
    /// verrouillage automatique reste celui d'ici. Retourne `true` si un retrait du chiffrement a été refusé.
    pub fn apply_transfer(&mut self, mut incoming: Settings) -> bool {
        let downgrade_refused = self.security.enabled && !incoming.security.enabled;
        if downgrade_refused {
            incoming.security = self.security.clone();
        } else {
            incoming.security.auto_lock_minutes = self.security.auto_lock_minutes;
            let mut same_revision = incoming.security.clone();
            same_revision.rev = self.security.rev;
            incoming.security.rev = if same_revision == self.security {
                self.security.rev
            } else {
                self.security.rev.max(incoming.security.rev) + 1
            };
        }
        incoming.sync.trusted_networks_only |= self.sync.trusted_networks_only;
        incoming.sync.encrypt_transport |= self.sync.encrypt_transport;
        incoming.sync.device_id = self.sync.device_id.clone();
        incoming.sync.device_name = self.sync.device_name.clone();
        incoming.sync.is_primary = self.sync.is_primary;
        incoming.sync.enabled = self.sync.enabled;
        incoming.sync.listen = self.sync.listen;
        incoming.sync.port = self.sync.port;
        incoming.sync.secret = self.sync.secret.clone();
        incoming.sync.peers = self.sync.peers.clone();
        incoming.plugins.approved = self.plugins.approved.clone();
        incoming.sanitize();
        *self = incoming;
        downgrade_refused
    }
}

/// Écrit un fichier lisible par son seul propriétaire (0600 sous Unix), de façon atomique : écriture
/// dans un fichier temporaire créé avec ces droits, synchronisation sur disque, puis renommage.
pub fn write_private(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    let _ = std::fs::remove_file(&tmp);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    file.write_all(data)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_settings_file_still_loads() {
        let old = r#"{"dark_mode":false,"language":"French","show_timestamps":true,
            "compact_mode":true,"confirm_delete":false,"font_size":18}"#;
        let parsed: Settings = serde_json::from_str(old).unwrap();
        assert!(!parsed.dark_mode);
        assert_eq!(parsed.language, Language::French);
        assert_eq!(parsed.font_size, 18);
        assert!(!parsed.security.enabled);
        assert_eq!(parsed.sync.port, 47474);
    }

    #[test]
    fn sanitize_clamps_and_creates_identity() {
        let mut s = Settings {
            font_size: 999,
            ..Default::default()
        };
        s.theme.radius = 500;
        s.theme.accent = Some("pas-une-couleur".into());
        s.sync.interval_secs = 1;
        s.sanitize();
        assert_eq!(s.font_size, 24);
        assert_eq!(s.theme.radius, 28);
        assert!(s.theme.accent.is_none());
        assert_eq!(s.sync.interval_secs, 10);
        assert_eq!(s.sync.device_id.len(), 16);
        assert!(s.sync.device_name.starts_with("pc-"));
    }

    #[test]
    fn transfer_never_carries_secrets_or_identity() {
        let mut primary = Settings::default();
        primary.sanitize();
        primary.sync.secret = "tres-secret".into();
        primary.sync.peers = vec!["100.64.0.2:47474".into()];
        primary.sync.is_primary = true;
        primary.plugins.approved.insert("p".into(), "hash".into());
        primary.security.enabled = true;
        primary.security.salt = "c2VsLXNlbA==".into();
        primary.theme.id = "dracula".into();

        let sent = primary.to_transfer();
        assert!(sent.sync.secret.is_empty());
        assert!(sent.sync.peers.is_empty());
        assert!(!sent.sync.is_primary);
        assert!(sent.plugins.approved.is_empty());
        // La dérivation de mot de passe et le thème, eux, sont bien copiés.
        assert_eq!(sent.security.salt, primary.security.salt);
        assert_eq!(sent.theme.id, "dracula");

        let mut secondary = Settings::default();
        secondary.sanitize();
        secondary.sync.secret = "autre".into();
        let own_id = secondary.sync.device_id.clone();
        assert!(!secondary.apply_transfer(sent));
        assert_eq!(secondary.sync.device_id, own_id);
        assert_eq!(secondary.sync.secret, "autre");
        assert_eq!(secondary.security.salt, primary.security.salt);
        assert_eq!(secondary.theme.id, "dracula");
    }

    #[test]
    fn corrupted_identity_never_panics_and_is_repaired() {
        for id in [
            "",
            "abc",
            "zz-not-hex-zz-zz",
            "é",
            "0123456789abcdef0123456789abcdef0",
        ] {
            let mut s = Settings::default();
            s.sync.device_id = id.into();
            s.sync.device_name = String::new();
            s.sanitize();
            assert!((8..=32).contains(&s.sync.device_id.len()), "{id}");
            assert!(s.sync.device_name.starts_with("pc-"));
        }
        let mut s = Settings::default();
        s.sync.device_name = "x\n\u{7}y".repeat(50);
        s.sync.peers = vec![
            "ok:1".into(),
            "bad\npeer".into(),
            String::new(),
            "a".repeat(300),
        ];
        s.sanitize();
        assert!(s.sync.device_name.chars().all(|c| !c.is_control()));
        assert!(s.sync.device_name.chars().count() <= 40);
        assert_eq!(s.sync.peers, vec!["ok:1".to_string()]);
    }

    #[test]
    fn a_received_configuration_cannot_weaken_security() {
        let mut local = Settings::default();
        local.sanitize();
        local.security.enabled = true;
        local.security.salt = "bG9jYWw=".into();
        local.security.verifier = "enc:v1:local".into();
        local.security.auto_lock_minutes = 5;
        local.sync.trusted_networks_only = true;
        local.sync.encrypt_transport = true;

        let mut incoming = Settings::default();
        incoming.security.enabled = false;
        incoming.security.auto_lock_minutes = 0;
        incoming.sync.trusted_networks_only = false;
        incoming.sync.encrypt_transport = false;
        let before = local.security.clone();
        assert!(
            local.apply_transfer(incoming),
            "the removal of encryption must be reported"
        );
        assert_eq!(local.security, before);
        assert!(local.sync.trusted_networks_only && local.sync.encrypt_transport);
    }

    #[test]
    fn a_new_vault_configuration_bumps_the_revision_and_keeps_local_auto_lock() {
        let mut local = Settings::default();
        local.sanitize();
        local.security.auto_lock_minutes = 15;
        local.security.rev = 4;
        let mut incoming = Settings::default();
        incoming.security.enabled = true;
        incoming.security.salt = "bmV3".into();
        incoming.security.rev = 2;
        incoming.security.auto_lock_minutes = 0;
        local.apply_transfer(incoming.clone());
        assert!(local.security.enabled);
        assert_eq!(local.security.rev, 5);
        assert_eq!(local.security.auto_lock_minutes, 15);
        // La même configuration reçue deux fois ne change plus rien.
        let again = local.security.clone();
        local.apply_transfer(incoming);
        assert_eq!(local.security, again);
    }

    #[test]
    fn private_files_are_written_atomically_with_owner_only_rights() {
        let dir = std::env::temp_dir().join(format!("ntm-private-{}", crate::store::new_uid()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        write_private(&path, b"one").unwrap();
        write_private(&path, b"two").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        assert!(!path.with_extension("tmp").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
