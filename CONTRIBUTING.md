# Contributing

- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass (CI runs them
  on Linux and Windows).
- The core (`src/crypto.rs`, `store.rs`, `vault.rs`, `sync/`, `plugin/`, `theme.rs`) does not depend on
  the UI: `cargo test --lib` runs its tests quickly.
- Source comments are written in French, identifiers in English.
- Plugins: see [docs/PLUGINS.md](docs/PLUGINS.md). Security: see [docs/SECURITY.md](docs/SECURITY.md).
- New interface text goes in `src/i18n.rs` (`EXTRA`, English and French; other languages fall back to English).
