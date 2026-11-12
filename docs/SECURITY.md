# Security

## Encryption

When encryption is enabled, the title and body of every note, and the values stored by plugins, are
encrypted before they reach the database.

| Part | Choice |
| --- | --- |
| Ciphers | AES-256-GCM (default) or ChaCha20-Poly1305, random 96-bit nonce per value |
| Key derivation | Argon2id (default, 64 MiB × 3), Argon2id strong (256 MiB × 4), scrypt (N=2^16), PBKDF2-HMAC-SHA256 (600 000) |
| Key file | Optional. Its SHA-256 is appended to the password before derivation, so both are needed |
| Authenticated data | The note id and the field name, so values cannot be swapped between notes or fields |
| Memory | Keys are zeroed when dropped; locking forgets the key and clears loaded notes |

The password is never stored. The settings file holds only the salt, the derivation parameters and a
*verifier* (a known text encrypted with the key) used to tell whether the password is right. These are
not secret and travel with the configuration when you transfer it to another PC. Parameters read from
`settings.json` are range-checked, so a tampered file cannot downgrade the derivation or exhaust memory.

Changing the password or algorithm re-encrypts everything inside one database transaction: it either
fully succeeds or leaves the database untouched.

**There is no recovery.** If you lose the password (or the key file), the notes cannot be decrypted.
Keep an encrypted backup and remember its own password.

What is *not* encrypted: note identifiers, dates and revision counters (needed for sync), the
settings file (it contains no note content), and the notes you are editing while the app is unlocked
(they are in memory). Search works on the decrypted notes in memory.

## Plugins

Plugins are Rhai scripts run in a sandbox. The language has no file, network or process access, and
the plugin API has **no function that touches the network, the disk or other programs**. A plugin
therefore cannot send your notes anywhere.

On top of that:

- Permissions are **detected from the code** (`src/plugin/analyze.rs`), never read from the manifest.
  A `permissions` key in `plugin.toml` is ignored and flagged.
- `eval`, `Fn`, `call`, `curry`, `is_def_fn`, `is_def_var`, `global`, `import` and `export` are
  rejected: names of functions cannot be computed at run time, so the analysis cannot be bypassed.
- Permissions are checked again at run time; an approved plugin can only use what was detected.
- Approval is bound to a hash of the plugin's manifest and code: any change disables it until you
  approve it again. Plugins received through configuration transfer are never pre-approved.
- Limits: 500 000 operations, 2 seconds, 64 functions, bounded strings/arrays/maps, at most 256
  requested actions per call, 128 KiB of source.
- Risky combinations are flagged (for example reading notes and writing to the clipboard).
- Panels are *descriptions*: the app validates and draws them, limits their size, and only lets the user
  change widgets that are currently displayed. A live theme preview is never saved and stops when you
  leave the panel; saving a theme needs the `theme.edit` permission and goes through the same validation
  and contrast correction as any theme.
- Plugins act through *effects* that the application applies; they never touch its state directly.

## Synchronization

- Mutual authentication with a shared secret (PBKDF2-SHA256, 210 000 iterations) → a fresh session key
  per connection (HKDF) from both sides' random nonces. Every frame is authenticated (HMAC-SHA256) or
  encrypted (AES-256-GCM) with a direction byte and a strict counter: replays, reflection and
  reordering are rejected. The encryption mode is part of the key derivation, so it cannot be
  downgraded on the way.
- Notes travel exactly as stored: with encryption enabled, the network and the peer only see
  ciphertext, and syncing does not need your vault password.
- By default only loopback, private networks, link-local, Tailscale (100.64.0.0/10 and
  fd7a:115c:a1e0::/48) are accepted.
- The shared secret is never sent and never copied with the configuration. `settings.json` is
  written with owner-only permissions on Unix.
- Received notes are checked against the hash announced in the manifest and applied in one
  transaction. Received themes and plugins are re-validated, and their file names come from validated
  content, never from the network.
- Frames are capped at 64 MiB, connections at 8, and failed authentication is slowed down.
- A peer that is not the primary cannot overwrite the primary (Push mode and configuration push are
  refused).

Choose a long random secret (the *Generate* button makes one). A weak secret can be guessed offline by
someone who recorded a handshake.

## Reporting a vulnerability

Please open a private security advisory on the GitHub repository rather than a public issue.
