# Synchronization

Keep your notes identical on any number of PCs, over a local network or Tailscale.

## Set up

1. On every PC: *Settings → Sync*, enable it, choose the **same shared secret** (use *Generate* on one
   PC, copy it to the others), and a name for the PC.
2. On the PC that should have the last word, tick **This PC is the primary**. Only one PC should have it.
3. On each other PC, add the primary's address in *Other PCs* (for Tailscale: its `100.x.y.z` address or
   its MagicDNS name, with the port — 47474 by default).
4. On a new PC, press **Pull config** next to the primary: it copies every setting (including the
   encryption parameters), your custom themes and plugins. Then enter your password to unlock.
5. Press **Sync now**. From then on it syncs every N seconds and after each change.

Every PC that should be reachable must accept connections (*Accept connections from other PCs*).

## Modes

| Mode | What it does |
| --- | --- |
| **Smart** (default) | Like git: compares this PC, the other PC and their last common state per note. Additions, edits and deletions flow both ways. If a note changed on both sides, the primary wins (otherwise the most recent edit). |
| **Newest wins** | For each note that differs, the most recent edit wins. No history is used. |
| **Union** | Only copies notes that are missing, in both directions. Never edits or deletes anything. The safest mode. |
| **Mirror** | This PC becomes an exact copy of the other one; local-only notes are removed. Useful to repair a PC. |
| **Push** | The other PC becomes an exact copy of this one. Only the primary may use it. |

## Encryption and the vault

Notes are exchanged as stored. All PCs must have the same encryption setup, or none; otherwise notes
are refused until you pull the configuration. After changing the password on the primary, on each other
PC: *Pull config*, then a *Mirror* sync, then unlock with the new password.

*Encrypt network transfers* adds AES-256-GCM on top of the always-on authentication. It is on by
default; turning it off only makes sense on a network you fully trust (the content stays encrypted if
your vault is enabled).

## What is transferred with the configuration

Everything except what belongs to one machine: the sync secret, the PC identity and role, the peer list,
the port, and plugin approvals (approve plugins on each PC yourself).

## Troubleshooting

- *authentication failed*: the secrets differ.
- *refused by peer: the encryption setup differs*: pull the configuration from the primary first.
- Nothing happens: check the firewall for the chosen port, and that the other PC has sync enabled.
