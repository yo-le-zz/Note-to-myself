<p align="center">
  <img src="packaging/icons/note-to-myself-256.png" alt="Note to Myself" width="128">
</p>

<h1 align="center">Note to Myself</h1>

<p align="center">Une pensée, rien que pour toi.<br>Une application de notes minimaliste, privée et extensible, pour Linux et Windows, écrite en Rust.</p>

## Fonctionnalités

- Écrire, modifier et supprimer des notes ; liste épurée, recherche, menu par note, raccourci Ctrl+S
- **Chiffrement** AES-256-GCM ou ChaCha20-Poly1305, clé dérivée de ton mot de passe (Argon2id, scrypt ou PBKDF2), fichier-clé optionnel, verrouillage rapide et automatique, sauvegardes chiffrées — [détails](docs/SECURITY.md)
- **170+ thèmes** avec aperçu avant d'appliquer, et personnalisation (accent, arrondi, bordures, espacement, contraste, polices à chasse fixe) ; tes propres thèmes sont enregistrables
- **Synchronisation** entre autant de PC que tu veux (réseau local, Tailscale) : mode type git avec le PC principal qui a le dernier mot, mode « le plus récent », « ajout seulement », miroir et push ; transferts chiffrés ou non, copie de toute la configuration — [détails](docs/SYNC.md)
- **Plugins** : scripts [Rhai](https://rhai.rs) dans un bac à sable, sans accès réseau ni disque ; permissions détectées dans le code ; 18 plugins livrés ; proposables par pull request — [écrire un plugin](docs/PLUGINS.md)
- Interface en 20 langues (les nouveaux écrans sont en français et en anglais), 100 % local, aucun compte
- Migration automatique et sans perte des anciennes bases

## Installation

Télécharge le paquet correspondant à ta distribution ou à ton système depuis les [releases](https://github.com/yo-le-zz/Note-to-myself/releases).

```sh
# Debian, Ubuntu, Linux Mint
sudo apt install ./note-to-myself_1.1.0_amd64.deb

# Fedora
sudo dnf install ./note-to-myself-1.1.0-1.x86_64.rpm

# openSUSE
sudo zypper install ./note-to-myself-1.1.0-1.x86_64.rpm

# Arch Linux, Manjaro
sudo pacman -U ./note-to-myself-1.1.0-1-x86_64.pkg.tar.zst
```

Windows : lance `note-to-myself-1.1.0-windows-setup.exe`, ou décompresse `note-to-myself-1.1.0-windows-portable.zip`.

## Compiler

Il faut [Rust](https://rustup.rs) (édition 2021, Rust récent). Sous Linux : `libxkbcommon-dev`, `libwayland-dev`, `libvulkan-dev`, `pkg-config`.

```sh
cargo run --release
cargo test               # tests complets
cargo test --lib         # tests du cœur seulement (rapide)
```

### Générer les paquets

```sh
./build.sh          # binaire + .deb + .rpm dans ./dist
./build.sh deb      # binaire + .deb
./build.sh rpm      # binaire + .rpm
./build.sh arch     # paquet Arch Linux (sur Arch, avec makepkg)
./build.sh bin      # binaire seulement
./build.sh --test   # lance aussi les tests avant de compiler
```

Les paquets de release (Linux, Arch, Windows) sont construits par GitHub Actions quand un tag `vX.Y.Z` est poussé (voir `.github/workflows/release.yml`). Le tag doit correspondre à la version de `Cargo.toml`.

## Données

- notes : `~/.local/share/notetomyself/notes.sqlite3` (Windows : `%APPDATA%\NoteToMyself\NoteToMyself\data`)
- réglages : `~/.config/notetomyself/settings.json`
- thèmes personnalisés : `~/.config/notetomyself/themes/`
- plugins installés : `~/.config/notetomyself/plugins/`

Pour tout sauvegarder, copie ces éléments, ou utilise la sauvegarde chiffrée (*Réglages → Sécurité*).

## Structure du projet

```
src/main.rs        fenêtre, abonnements, vue
src/app.rs         état de l'application et logique des écrans
src/ui/            écrans (accueil, écriture, notes, réglages, thèmes, sécurité, sync, plugins)
src/lib.rs         cœur sans interface graphique :
  crypto.rs          chiffrement et dérivation de clé
  vault.rs           coffre, chiffrement de la base, sauvegardes
  store.rs           SQLite, migrations, suppressions logiques
  notes.rs           lecture/écriture des notes pour l'interface
  theme.rs           thèmes et personnalisation
  sync/              protocole réseau et décision de fusion
  plugin/            moteur de plugins, analyse des permissions, plugins intégrés
  i18n.rs            traductions
plugins/           plugins livrés avec l'application
examples/          plugin d'exemple
docs/              sécurité, synchronisation, écrire un plugin
packaging/         icônes, .desktop, PKGBUILD, installateur Windows
```

Construit avec [iced](https://iced.rs), [rusqlite](https://github.com/rusqlite/rusqlite), [Rhai](https://rhai.rs) et [Lucide](https://lucide.dev) pour les icônes. Contribuer : voir [CONTRIBUTING.md](CONTRIBUTING.md).

## Licence

MIT, voir le fichier [LICENSE](LICENSE). Créé par yolezz.
