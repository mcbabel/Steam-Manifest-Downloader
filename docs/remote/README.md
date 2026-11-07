# Remote file

`smd-remote.json` holds notes and links the app loads at start, signed with
Ed25519. The website serves it at
`https://smd.mcbabel.de/remote/smd-remote.json` with the signature next to it
as `smd-remote.json.sig`. The app only uses a file whose signature matches the
public key built into `src-core/src/services/remote.rs`. Otherwise it keeps the
last good copy, or its built-in values.

Users can switch the download off under Settings, Privacy. Nothing but the
request itself is sent.

## Format

```json
{
  "version": 1,
  "links": { "issues": "https://…" },
  "notices": [
    {
      "id": "unique-id",
      "level": "info",
      "text": { "en": "…", "de": "…" },
      "url": "https://…",
      "min_version": "1.5.0",
      "max_version": "",
      "frontends": ["gui", "tui", "cli"]
    }
  ],
  "disabled_providers": ["steam.run", "wudrm"]
}
```

- `links` replaces the built-in links `website`, `docs_issues`, `issues`,
  `releases` and `achievement_watcher`. Only `https://` links are used.
- `notices` are shown as a banner in the app, as a message in the terminal
  version and on stderr in the CLI. `level` is `info` or `warning`. An empty
  `frontends` list means all. A dismissed notice stays hidden.
- `disabled_providers` switches off the external manifest code services, for
  example when one of them returns wrong data.

## Signing

The secret key never goes into the repository.

```sh
smd remote-sign keygen --key smd-remote.key
smd remote-sign sign --key smd-remote.key docs/remote/smd-remote.json
smd remote-sign verify docs/remote/smd-remote.json
```

`keygen` prints the public key. A new key only works in builds that carry it
in `PUBLIC_KEY`.
