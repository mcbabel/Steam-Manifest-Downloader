# Privacy

Steam Manifest Downloader can optionally send **anonymous usage statistics**
so the maintainer can see which features are worth investing in and which
are broken. This document describes exactly what is sent, what is **not**
sent, and how you can opt in or out.

## Telemetry is opt-in

On first launch the app shows a dialog asking whether you want to help.
**Nothing is transmitted unless you click "Yes, help out."** Declining is the
default if you close the dialog. You can change your choice anytime in
**Settings → Privacy & diagnostics → Anonymous Usage Statistics**.

The command line version (`smd download`, `smd search`, `smd history`, also in
Docker) never asks and sends nothing unless you switch it on: with the
terminal interface, with `smd telemetry on`, or with the environment variable
`SMD_TELEMETRY=on`. `SMD_TELEMETRY=off` switches it off for that run, in every
version of the app. `smd telemetry status` shows the current state.

## What is collected

When telemetry is enabled, the app sends small events describing:

- App version, build channel (`stable` / `dev` / `dev-local`), OS, architecture
- Whether you use the desktop app, the terminal interface or the command line
  (`gui` / `tui` / `cli`),
  how it was installed (`installer`, `portable`, `appimage`, `system`,
  `flatpak`, `docker`, ...) and the language the app is shown in
- A random install UUID generated once on first accept (no link to your
  identity — this is just so two events from the same session can be
  correlated)
- A random session UUID regenerated every time the app starts
- **Event counters** for a fixed list of actions:
  - `app_start` — the app was launched, with the settings snapshot described
    below
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
  - `update_checked` / `update_installed` — the auto-updater ran, and whether
    the check failed
  - `update_dismissed` — how the update dialog was answered (`later`, `skip`
    or `github`)
  - `library_added` — a game was added to the Steam library, whether it worked
    and whether Steam was restarted
  - `game_launched` — a game was started from the history, and how (`steam`,
    `wine`, `native` or `direct`)
  - `proxy_tested` — the proxy test ran, with the proxy type (`none`, `http`,
    `https`, `socks`) and whether it worked. Never the address
  - `diagnostics_copied` — you copied the diagnostic info from the settings
  - `bug_report_opened` — you opened a bug report from the app, where you
    started it and whether diagnostic info and log lines were attached. The
    text of the report is never sent through telemetry; it only goes to
    GitHub if you submit it there yourself
  - `crash`, `error_shown` and `download_interrupted` — described under
    *Errors and crashes* below
  - `download_paused` — a download was paused or resumed
  - `shutdown_after` — the shut-down-after-download countdown started, was
    stopped, or turned the PC off; `followup` — whether the steps skipped by
    the shutdown were taken up on the next start
  - `queue_action` — something was added to the download queue, or the queue
    was started, stopped or cleared, with the queue length as a range
  - `history_action` — the history was opened, or an entry was updated,
    checked, downloaded again, opened in the file manager, edited, removed or
    cleared; `updates_found` — how many history entries have an update, as a
    range
  - `steamless_used`, `api_bypass`, `dlc_merged` — the DRM removal, the Steam
    API check bypass or the DLC depot merge ran, and whether it worked
  - `game_data_written` — the emulator step wrote game data for gbe_fork,
    where the achievements came from (`web_api`, `keyless` or `none`), how
    many languages, depots, branches, achievements, achievement languages,
    stats, leaderboards, inventory items and cloud save folders it found, as
    ranges, whether a controller layout and Achievement Watcher schemas were
    written, the media choice (`off`, `images` or `all`) and which hints were
    shown, by name (for example `needsKey` or `inventoryFailed`)
  - `manifest_tool` — the newest manifest ID was fetched or a manifest file
    was picked for a depot. Never the ID or the file
  - `emu_section_viewed` — a section of the emulator step was opened
    (`files`, `gamedata`, `emulator` or `extras`), once per section per app start
  - `shortcut_key` — a keyboard shortcut was used (`open`, `search`,
    `history`, `settings`)
  - `settings_saved` — which settings were changed, by name only (for example
    `proxy` or `max_retries`), never the new value, and `emulator` when the
    Steam Web API key or the media choice was saved in the emulator step
  - `cli_command` — which command line command ran (`download`, `search`,
    `history`) and which of its options were used, as true or false
  - `heartbeat` and `session_end` — see *Time in the app* below

### Settings snapshot

`app_start` carries which options are switched on, so the maintainer knows
which settings matter. Only on/off values and fixed labels are sent, never a
value you typed: the engine (`native` / `ddm`), whether Like Steam, auto start,
DLC, the speed limit, auto update and keep-files-on-cancel are on, the proxy
type, how many manifest sources are configured (as a range) and whether they
differ from the defaults, whether a Hubcap or Ryuu key and a Steam folder are
set (true or false, never the key or the path), whether a Steam Web API key
is set (true or false), the media choice for game data (`off`, `images` or
`all`), the retry and chunk counts,
and the game language and platform picked for Like Steam (`auto` or a fixed
label like `german` or `linux`).

### How a download was started

`download_started`, `download_completed` and `download_abandoned` also carry:

| Field | Meaning |
|---|---|
| `mode` | `new`, `update`, `repair` or `resume` |
| `selection` | how the depots were picked: `like_steam`, `manual`, `default`, `queued` or `resume` |
| `source` | `upload`, `search`, `hubcap` or `ryuu` |
| `queue` | whether it ran from the download queue |
| `dlc` | whether DLC was included |
| `keyless` | how many selected depots had no decryption key, as a range |
| `custom_manifest` | whether a manifest ID was entered by hand |
| `err_key` | for failures, the name of the error text that was shown, like `backend.noSources`. It names the message, it never contains it |

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

## Time in the app

To see how long the app stays open, every 15 minutes while it runs and once
when it closes, the app sends how many minutes it has been open, how many of
them the window was in front (not for the command line), and whether a
download was running. Nothing about what you looked at or typed is part of it.

## Errors and crashes

Most bugs are never reported, so the app tells the maintainer when something
goes wrong:

- `error_shown` — an error message was shown. Only the area (like `search` or
  `steam_library`) and the name of the message are sent, for example
  `backend.noSources`. The text itself, which could contain a path or a game
  name, is not sent. Each message is counted once per session.
- `crash` — the app crashed or hit a programming error. The code location
  (like `src-core/src/ops/download.rs:812` or `emulator.js:1234`) and the first line
  of the error are sent. Before anything leaves your computer, paths, anything
  in quotes and all numbers are removed from that line. Crashes are saved on
  disk and sent with the next start, at most five at a time.
- `download_interrupted` — a download was still running when the app was
  killed, crashed or the PC turned off. A small marker file is kept while a
  download runs, and the next start reports the engine, the mode, the depot
  count range and the step it was on.

The settings show a short diagnostic ID (the first 8 characters of the random
install UUID) when statistics are on. Pasting it into a bug report lets the
maintainer find the errors your app reported. It is only shown to you and only
useful if you choose to share it.

## Steam Web API key

If you enter a Steam Web API key for achievements and stats, it is stored in
your local settings file and only sent to Steam (`api.steampowered.com`) to
read the achievement list and the inventory of the game you patch. It is never
sent to the maintainer and never written into the game folder.

Writing game data talks to Steam only: the Steam network for app info,
inventory and the Steam Input layout, `api.steampowered.com`,
`steamcommunity.com` for leaderboards and the Steam CDNs for icons. With media
on, also `store.steampowered.com` and the Steam image and video CDNs.

## What is NEVER collected

- Steam App IDs, depot IDs, manifest IDs
- Contents of your `.lua` / `.st` files
- Game names, cover art, descriptions
- File paths, directory contents, download targets
- Your GitHub / Steam credentials, API keys, or any tokens
- The text of error messages, or anything you typed into a field
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

Turn the toggle off in **Settings → Privacy & diagnostics**. After that no
events are sent and no installation UUID is used. Already-transmitted
events can't be un-sent (they've left your machine), but they're
anonymous and will age out of the server logs per the retention policy.

## Questions

Open an issue with the `question` template or email
`mcbabel.sup@protonmail.com`.