# Changelog

## 1.1.0

### Added
- **Encryption**: AES-256-GCM or ChaCha20-Poly1305, with key derivation from a password
  (Argon2id, scrypt or PBKDF2-HMAC-SHA256) and an optional key file. Enable, change or remove
  encryption in one click; auto-lock after inactivity; manual lock button; encrypted backups.
- **Themes**: 170+ themes (curated palettes, plus 12 hues × 8 styles), live preview with an
  Apply / Cancel banner, and customization (accent color, corner radius, border width, spacing,
  panel contrast, monospace fonts). Your own looks can be saved as themes.
- **Synchronization** between any number of PCs over a local network or Tailscale: five modes
  (Smart git-like three-way merge, Newest, Union, Mirror, Push), a designated primary PC that wins
  conflicts, automatic and on-change sync, optional transport encryption, and transfer of the whole
  configuration (including the key-derivation settings, themes and plugins).
- **Plugins**: sandboxed Rhai scripts with an API covering notes, editor, clipboard, buttons,
  note menu, themes, settings and private storage. Permissions are detected from the code. 18
  plugins ship with the app. `ntm-plugin-check` validates plugins; a workflow checks every pull request.
- Packages: Arch Linux (`.pkg.tar.zst`) and Windows (installer and portable zip), built by GitHub Actions.

### Changed
- Notes now carry a global identifier and deletions leave a tombstone (needed for sync).
  Existing databases are upgraded automatically and without loss.
- The core (storage, crypto, sync, plugins, themes) is a separate library tested without the UI.
- New interface texts are available in English and French; other languages fall back to English.

## 1.0.0
- First release: write, edit, delete and search notes, 20 languages, dark and light themes.
