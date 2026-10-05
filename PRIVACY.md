# Privacy

Steam Manifest Downloader can optionally send **anonymous usage statistics**
so the maintainer can see which features are worth investing in and which
are broken. This document describes exactly what is sent, what is **not**
sent, and how you can opt in or out.

## Telemetry is opt-in

On first launch the app shows a dialog asking whether you want to help.
**Nothing is transmitted unless you click "Yes, help out."** Declining is the
default if you close the dialog. You can change your choice anytime in
**Settings → Advanced Settings → Anonymous Usage Statistics**.

## What is collected

When telemetry is enabled, the app sends small events describing:

- App version, build channel (`stable` / `dev` / `dev-local`), OS, architecture
- A random install UUID generated once on first accept (no link to your
  identity — this is just so two events from the same session can be
  correlated)
- A random session UUID regenerated every time the app starts
- **Event counters** for a fixed list of actions:
  - `app_start` — the app was launched
  - `settings_opened` — you opened the Settings dialog
  - `theme_toggled` — you switched between dark / light theme
  - `search_performed` — you ran a search in Step 1, with whether anything was
    found, how many sources had it, and whether your configured sources were
    reachable (`found` / `missing` / `unreachable` / `no_sources`) — never the
    App ID you searched for
  - `lua_parsed` — a `.lua` / `.st` file was successfully parsed (with
    a depot count — **not** the IDs)
  - `download_started` — with a depot count, which engine is in use, how many
    manifest sources you have configured, and whether a ManifestHub key is set
  - `download_completed` — with a depot count and the failure diagnosis
    described below
  - `download_abandoned` — you closed the app while a download was still
    running, with the same diagnosis fields and the step it was on
  - `patch_applied` — the gbe_fork emulator patch was applied, with the
    diagnosis described below
  - `patch_reverted` — a patch was undone, with the same diagnosis
  - `patch_settings_saved` — emulator settings were written to an already
    patched folder
  - `shortcut_created` — a Windows shortcut was created
  - `update_checked` / `update_installed` — the auto-updater ran

### Download failure diagnosis

Roughly 40% of downloads were failing without the maintainer being able to
tell why, so `download_completed` and `download_abandoned` carry a small
diagnosis. **Every one of these fields is a label picked from a fixed list
that ships in the binary** — none of them can contain a name, an ID, a path
or a server message. The set is:

| Field | Meaning |
|---|---|
| `outcome` | `complete`, `partial`, `failed`, `cancelled` or `abandoned` |
| `depots_total` / `depots_ok` | how many depots you selected, and how many succeeded |
| `depot_bucket` | that count as a range (`1`, `2`, `3-4`, `5-8`, `9-16`, `17+`) |
| `duration_bucket` | how long it ran, as a range (`<5s` … `>60m`) |
| `engine` | `native` or `ddm` (which downloader was used) |
| `fail_stage` | which step failed — e.g. `source_probe`, `login`, `manifest_code`, `manifest_fetch`, `manifest_decode`, `depot_key`, `cdn_token`, `chunk`, `disk` |
| `fail_class` | what kind of failure — e.g. `not_found`, `rate_limited`, `http_5xx`, `timeout`, `connect`, `decode`, `io`, `no_sources_configured`, `no_api_key` |
| `source_ok` | which *kind* of source worked — `steam_direct`, `depot_source`, `hubcap`, `ryuu`, `manifesthub`, `uploaded`, `cached`. Never the URL |
| `sources_tried` | which kinds of source were attempted, with counts — so a source that is always tried and never works can be spotted |
| `fail_stages` / `sources_ok` | the same labels with counts, for jobs where depots failed differently |
| `source_count` | how many manifest sources you have configured (a number, never the URLs) |
| `had_mh_key` | whether a ManifestHub key was set — true or false, never the key |
| `resumed` | true when the download was resumed from a cancelled one rather than started fresh |
| `last_stage` | for cancelled and abandoned jobs, the pipeline step it was on |
| `job` | 8 random bytes generated per download, so a start and its outcome can be matched up. Discarded when the download ends; it links nothing across downloads and nothing to you |

The URLs you configure under Manifest Sources are treated as your data and
are never transmitted — only the category of source they fall into. The
`sources_tried` counts exist because a core fallback host once went offline for
weeks without anything noticing, and this is the field that would have caught it.

### Emulator patch diagnosis

`patch_applied`, `patch_reverted` and `patch_settings_saved` carry the same kind
of fixed-label diagnosis. The folder you patch, the files inside it and the game
it belongs to are never transmitted — only these labels:

| Field | Meaning |
|---|---|
| `entry` | where the patch was started from — `download`, `standalone` (Patch Existing Folder) or `history` |
| `outcome` | `complete`, `partial` or `failed` |
| `variant` | which emulator build was chosen — `regular` or `experimental` |
| `platforms` | what kind of library was targeted — `windows`, `linux`, `mixed` or `none`. Never a file name |
| `targets` / `failures` | how many files were touched and how many failed, as ranges (`0`, `1`, `2-3`, `4-10`, `>10`) |
| `fail_class` | why it failed, from a fixed list: `emu_binary_missing`, `interfaces_failed`, `backup_failed`, `copy_failed`, `settings_write_failed`, `no_parent_dir`, `folder_missing`, `no_backup`, `restore_failed`, `av_blocked`, `release_fetch_failed`, `emu_download_failed`, `unknown` |

`fail_class` is assigned in the Rust code at the point the error happens, so the
label can never be derived from — or contain — a file path or a system message.
This exists because a rename in an upstream emulator release silently broke every
32-bit patch, and nothing surfaced it until a user reported it by hand.

## What is NEVER collected

- Steam App IDs, depot IDs, manifest IDs
- Contents of your `.lua` / `.st` files
- Game names, cover art, descriptions
- File paths, directory contents, download targets
- Your GitHub / Steam credentials, API keys, or any tokens
- Your IP address (stripped at the reverse proxy before anything reaches
  the server log)
- Any identifier linked to your operating-system user account

## Encryption

Events are batched in memory, serialized as JSON, and encrypted with a
`crypto_box` sealed box using an X25519 public key that ships inside the
binary. Only the private key on the maintainer's telemetry server can
decrypt the payload — not the hosting provider, not intermediate proxies,
not anyone observing the connection. TLS is used on top of that.

## Retention

Decrypted events are appended to a rotating daily `.jsonl` file on the
server. They are not joined with any other dataset. There is no user
database. If you want a copy of what the server has associated with your
install UUID, or want it deleted, email the address below — but note that
because events don't carry any identifier beyond the random UUID that
only exists in your local settings file, there is typically nothing to
match against unless you send the UUID yourself.

## Opting out later

Turn the toggle off in **Settings → Advanced Settings**. After that no
events are sent and no installation UUID is used. Already-transmitted
events can't be un-sent (they've left your machine), but they're
anonymous and will age out of the server logs per the retention policy.

## Questions

Open an issue with the `question` template or email
`mcbabel.sup@protonmail.com`.