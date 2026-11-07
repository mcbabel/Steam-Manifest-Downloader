# Privacy

Steam Manifest Downloader can optionally send **anonymous usage statistics**
so the maintainer can see which features are worth investing in and which
are broken. This document describes exactly what is sent, what is **not**
sent, and how you can opt in or out.

## Telemetry is opt-in

On first launch the app shows a dialog asking whether you want to help, with
three answers: **Allow everything** (statistics and [usage
behaviour](#usage-behaviour)), **Statistics only**, or **No thanks**.
**Nothing is transmitted unless you pick one of the first two.** Declining is
the default if you close the dialog. You can change your choice anytime in
**Settings → Privacy & diagnostics**, where both have their own switch.
If you agreed to the statistics in an earlier version, the app asks once
whether usage behaviour may be added; nothing of it is sent before you say
yes.

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
  - `lua_parsed` — a `.lua` / `.st` file or a depot key file was successfully
    parsed (with a depot count and the file type, like `lua` or `json` —
    **not** the IDs)
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
  - `update_checked` / `update_installed` — the auto-updater ran, whether the
    check failed, and how the update was installed (`installer`, `appimage`
    or `release_page`)
  - `update_dismissed` — how the update dialog was answered (`later`, `skip`,
    `github` or `release_page`)
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
  - `denuvo_notice` — the warning that Steam lists Denuvo for a game was shown,
    and where (`select`, `emulator` or `cli`). Never which game
  - `drm_restored` — an original program file with Steam DRM was put back,
    one by one (`single`) or while reverting the patch (`revert`)
  - `emu_saves` — saves were backed up or restored (`backup`, `restore`) and
    whether it worked, never a path
  - `emu_achievements` — achievements were locked again one by one or all, or
    the stats cleared (`lock_one`, `lock_all`, `clear_stats`), never which ones
  - `emu_profile` — a profile picture, overlay font or sound was set or removed
    (`avatar`, `font`, `achievement`, `friend`), for a font whether it came
    from the installed fonts or a file, and whether it worked. Never the file
    or font name
  - `emu_network` — the other players' addresses were saved, with how many as
    a range (never the addresses), or lobby_connect was started and whether it
    worked
  - `emu_mod` — a workshop mod was added or removed, whether it worked, whether
    the workshop page had details and whether Steam lists it for another game.
    Never the workshop ID or the mod's name
  - `coldclient_used` — gbe_fork was set up with the ColdClientLoader, the
    game's Steam entry was changed afterwards, or it was removed again
    (`setup`, `shortcut`, `removed`), whether it worked, what happened to the
    Steam entry (`updated`, `steam_running`, `none`) and whether desktop or
    start menu shortcuts were changed. Never the game or a path
  - `emu_gbe_updated` — a patched game was updated to the newest gbe_fork
    release, and whether the old version was known (`known` or `unknown`)
  - `game_data_written` — the emulator step wrote game data for gbe_fork,
    where the achievements came from (`web_api`, `keyless` or `none`), how
    many languages, depots, branches, achievements, achievement languages,
    stats, leaderboards, inventory items and cloud save folders it found, as
    ranges, whether Achievement Watcher schemas were written, the media choice
    (`off`, `images` or `all`) and which hints were shown, by name (for
    example `needsKey` or `inventoryFailed`)
  - `controller_applied` — the emulator step wrote a controller layout, whether
    it came from Steam or from your own file (`steam`, `file` or `none`, never
    the file name), how many action sets as a range, whether button icons were
    copied, the gbe_fork build (`regular` or `experimental`), whether Steam
    Input through Steam is on and which hints were shown, by name (for example
    `noLayout` or `glyphsMissing`)
  - `manifest_tool` — the newest manifest ID was fetched for one depot or for
    all at once (from the button, the setting or an update, with how many
    depots changed as a range), or a manifest file was picked for a depot.
    Never the ID or the file
  - `emu_section_viewed` — a section of the emulator step was opened
    (`files`, `gamedata`, `controller`, `emulator` or `extras`), once per
    section per app start
  - `remote_notice` — a note from the remote file was shown, its link opened
    or it was dismissed, with the note's ID and level (`info` or `warning`)
  - `offline_notice` — the app showed that there is no internet connection,
    once per app start
  - `emu_settings_view` — the emulator settings were searched or filtered to
    the changed ones (`search` or `changed_only`), and for a search how many
    settings matched, as a range. The search text itself is never sent
  - `welcome_shown` / `welcome_choice` — the welcome after the first start was
    shown, and which way to start was picked (`upload`, `search`, `patch`,
    `location` or `closed`)
  - `advice_shown` / `advice_action` — a "what you can do" hint after a failed
    search or download was shown, which one by name (for example
    `search_no_results` or `no_api_key`), and which of its buttons was used
  - `whats_new_shown` — the "What's new" notes were shown after an update
  - `feedback_prompt` — how the one-time "star on GitHub" card was answered
    (`star`, `issue` or `dismiss`; the terminal version reports `shown`)
  - `shortcut_key` — a keyboard shortcut was used (`open`, `search`,
    `history`, `settings`)
  - `settings_saved` — which settings were changed, by name only (for example
    `proxy` or `max_retries`), never the new value, and `emulator` when the
    Steam Web API key or the media choice was saved in the emulator step
  - `cli_command` — which command line command ran (`download`, `search`,
    `history`) and which of its options were used, as true or false
  - `heartbeat` and `session_end` — see *Time in the app* below
  - `account_login` — a Steam sign-in was started, finished, cancelled, expired
    or failed (`step`), and where the finished sign-in is kept (`keychain`,
    `file` or `session`). Never the account name, the SteamID or the token
  - `account_logout` — you signed out of your Steam account
  - `account_depots` — depots were loaded from your own account, with their
    count
  - `playtest_opened` — you opened a game's playtest from the hint in the
    search
  - `steam_keys_used` — you let SMD look for missing depot keys in your
    installed Steam, and how many it found (`found`, `missing`). Never the
    keys or depot IDs
  - `key_file_used` — you added depot keys from a key file in step 2, and how
    many matched (`found`, `missing`). Never the keys, IDs or the file name

### Settings snapshot

`app_start` carries which options are switched on, so the maintainer knows
which settings matter. Only on/off values and fixed labels are sent, never a
value you typed: the engine (`native` / `ddm`), whether Like Steam, auto start,
always fetching the newest manifests, notes and links online,
DLC, the speed limit, auto update and keep-files-on-cancel are on, how the
game folder is named (`default`, `underscore` or `dash`), the proxy
type, how many manifest sources are configured (as a range) and whether they
differ from the defaults, whether a Hubcap or Ryuu key and a Steam folder are
set (true or false, never the key or the path), whether a Steam Web API key
is set (true or false), the media choice for game data (`off`, `images` or
`all`), the retry and chunk counts, the download server choice (`auto`,
`region` or `custom`, with the number of the Steam region but never the
address of your own server), and the game language and platforms picked for Like Steam (`auto`, a fixed
label like `german`, or a platform list like `windows,linux`).

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
| `account` | whether the download used your own Steam account |
| `branch` | `public`, `other` or `password`: the kind of branch, never its name |
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
| `server` / `server_fallback` | the download server setting (`auto`, `region` or `custom`) and whether a chosen region or own server had to fall back to Steam's servers. Never the server's address |
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

## Usage behaviour

Only with its own consent (see above), the app and the terminal interface
also record how you move through them, so the maintainer can see where
people get stuck, where questions come up and what needs a better
explanation. For every screen you visit (a step, a settings tab, a dialog)
one `ui_screen` event is sent when you leave it:

| Field | What it is |
|---|---|
| `screen`, `next` | the screen's internal name, like `select` or `settings.privacy`, and the one you went to (`exit` when the app closed) |
| `visit` | the how-many-th visit to that screen in this session |
| `dwell_s` | seconds spent there |
| `first_action_s` | seconds until the first click or key, empty if there was none |
| `longest_pause_s` | the longest stretch without any input |
| `back` | 1 when the next screen was an earlier step |
| `clicks` | which controls were used, with counts: their internal id or class in the app (`btn-search`), the kind of action in the terminal interface (`StartDownload`) |
| `dead` | clicks on things that are not clickable, by the area they landed in |
| `rage` | three or more fast clicks on the same thing |
| `hovers` | elements with a tooltip you stopped on for a moment |
| `keys` | terminal interface: keys that did nothing on that screen, only `char` for any letter, never which one |

Names come from the app's own markup and code. Numbers inside a name (like
the ID in a depot card) are replaced with `n` before anything is sent. Never
part of it: what you type into a field, the text on the screen, game names,
IDs, paths, mouse positions or screenshots. `behavior_consent` records when
you switch it on or off.

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

## Notes and links online

At start the app loads a small signed file with current links and notes from
`smd.mcbabel.de` (`/remote/smd-remote.json` and its `.sig`). Nothing is sent
but the request itself, no ID and no settings. The file is only used when its
signature matches the key built into the app. Switch it off under Settings,
Privacy, Notes and links online; the built-in links stay then. Dismissed notes
are remembered on your PC only.

To tell you when there is no internet connection, the desktop app asks
`api.steampowered.com` for the Steam server time at start, and every 20
seconds while it is offline. Nothing else is sent.

## Steam account

If you sign in with your Steam account, the sign-in token is kept in the
system keychain (or, only if you ask for it with `smd login --store-file`, in a
file in the data directory that only you can read). It is only sent to Steam:
the Steam network and `api.steampowered.com` for the sign-in itself. Your
account name, SteamID and token are never sent to the maintainer, never
written into the settings, the history or a log, and signing out revokes the
sign-in at Steam. Branch passwords are only sent to Steam and never stored.

## What is NEVER collected

- Steam App IDs, depot IDs, manifest IDs
- Contents of your `.lua` / `.st` files
- Game names, cover art, descriptions
- File paths, directory contents, download targets
- Your GitHub / Steam credentials, API keys, or any tokens
- Your Steam account name, SteamID, sign-in token, branch names or branch
  passwords
- The text of error messages, or anything you typed into a field
- Screen contents, mouse positions or screenshots, also with usage behaviour
  switched on
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
events are sent and no installation UUID is used. The **Usage behaviour**
switch below it stops only the screen and click records and keeps the
statistics. Already-transmitted
events can't be un-sent (they've left your machine), but they're
anonymous and will age out of the server logs per the retention policy.

## Questions

Open an issue with the `question` template or email
`privacy@mcbabel.de`.