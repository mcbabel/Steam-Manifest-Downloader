# smd-telemetry-server

Tiny receiver for the anonymous telemetry that the Steam Manifest Downloader
client sends when the user opts in.

## What it does

- Listens on `127.0.0.1:9999` by default (configurable).
- Accepts `POST /v1/events` with a `crypto_box` sealed-box ciphertext as the raw body.
- Decrypts with the server private key, parses the plaintext JSON envelope, and
  appends one line per request to `events-YYYY-MM-DD.jsonl` in the data dir.
- Does **not** log client IPs. Rejects bodies above 64 KiB.
- Responds `204 No Content` on success, `400` on decrypt / parse failure.

## Running locally

1. Copy `.env.example` → `.env` and set `TELEMETRY_PRIVATE_KEY_HEX` to the
   server private key (the client is built with the matching public key).
2. `cargo run --release` in this directory (it reads `.env` only if you
   export it; simplest is: `env $(cat .env | xargs) cargo run --release`).

## Deploying

- Build: `cargo build --release`.
- Copy `target/release/smd-telemetry-server` to the host.
- Put it behind Caddy / nginx / Traefik that terminates TLS.
- In the reverse proxy, **do not** forward `X-Forwarded-For` / `X-Real-IP`.
  Either disable access logs or drop the IP field.
- Systemd unit example:
  ```ini
  [Service]
  Environment="TELEMETRY_PRIVATE_KEY_HEX=..."
  Environment="TELEMETRY_DATA_DIR=/var/lib/smd-telemetry"
  Environment="TELEMETRY_BIND=127.0.0.1:9999"
  ExecStart=/usr/local/bin/smd-telemetry-server
  DynamicUser=yes
  StateDirectory=smd-telemetry
  ```

## Report

`smd-telemetry-server report` turns the stored events into one HTML file
with active installs, success rate per version, the steps downloads fail
at, crashes, shown errors, interrupted downloads and feature use.

```sh
smd-telemetry-server report --data ./data --out report.html --days 90
```

Only the `stable` channel is counted unless `--all-channels` is given.
`--data` defaults to `TELEMETRY_DATA_DIR`. The report needs no key, it
reads the decrypted `.jsonl` files.

## Storage format

Each line is `{"received_at": "RFC3339", "payload": {...decrypted envelope...}}`.
Rotate / archive the JSONL files yourself; the server never deletes anything.

## Keys

The keypair used by the client is defined at build time in
`src-core/src/services/telemetry.rs` (`SERVER_PUBLIC_KEY`). To rotate:

1. Generate a new X25519 keypair.
2. Ship a new client release with the new public key baked in.
3. Keep both private keys on the server until the old client version is
   deprecated (the current server only supports one key — extend it when the
   need to rotate actually arises).