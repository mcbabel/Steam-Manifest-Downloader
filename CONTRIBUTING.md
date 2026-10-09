# Contributing to Steam Manifest Downloader

Thanks for your interest in contributing! This document explains how to set
up the project, how commits and pull requests should look, and how patches
are reviewed.

> By contributing, you agree that your contributions will be licensed under
> the project's **GNU General Public License v2.0**, the same license as the
> rest of the codebase. See [LICENSE](./LICENSE).

---

## Before you start

- **Security issues**: do **not** open a public issue. Follow
  [SECURITY.md](./SECURITY.md) instead.
- **Large features or breaking changes**: open a
  [feature request](../../issues/new?template=feature_request.md) first so
  we can agree on scope before code is written.
- This project does not support or encourage piracy. Contributions must be
  compatible with the use cases described in the README disclaimer.

---

## Development setup

This is a [Tauri v2](https://v2.tauri.app/) app — Rust backend, vanilla JS
frontend — plus a terminal version (`src-tui`). Both share the Rust core in
`src-core`. DepotDownloaderMod is embedded as an optional download engine.

### Prerequisites

- **Rust** (latest stable) + Cargo — <https://rustup.rs/>
- **Tauri CLI**:
  ```bash
  cargo install tauri-cli --version "^2.0"
  ```
- **Platform build deps**:
  - Windows: Visual Studio Build Tools with C++ workload, WebView2.
  - Linux: `webkit2gtk-4.1`, `libayatana-appindicator3`, `librsvg2`,
    `build-essential`, `curl`, `wget`, `file`.
- **.NET 9 runtime** *(optional)*, only to test the DepotDownloaderMod engine.

See [the build guide in the docs](./docs/documentation/index.html#building-from-source)
for more detailed, up-to-date instructions.

### Run the app in dev mode

```bash
cargo tauri dev
```

This compiles the Rust backend, serves the frontend from `public/`, and opens
the app window with hot-reload for frontend changes. Rust changes trigger a
rebuild automatically.

### Build a release binary

```bash
cargo tauri build
```

---

## Branching model

- `main` — latest stable release. Do **not** open PRs against `main`.
- `dev` — active development. **All pull requests target `dev`.**
- Release promotion from `dev` to `main` is handled by the maintainer.

Create your feature branch off `dev`:

```bash
git checkout dev
git pull
git checkout -b feat/my-change
```

---

## Commit messages

This project uses **[Conventional Commits](https://www.conventionalcommits.org/)**.
Every commit message (and PR title) must follow this form:

```
<type>: <short imperative subject>

[optional body]

[optional footer]
```

### Allowed types

| Type       | Use for                                                              |
| ---------- | -------------------------------------------------------------------- |
| `feat`     | A new user-visible feature                                           |
| `fix`      | A bug fix                                                            |
| `refactor` | Code change that neither fixes a bug nor adds a feature              |
| `perf`     | Performance improvement                                              |
| `docs`     | Documentation only                                                   |
| `style`    | Formatting, whitespace, lints with no code behavior change           |
| `test`     | Adding or fixing tests                                               |
| `build`    | Build system, packaging, bundler configuration                       |
| `ci`       | GitHub Actions or other CI configuration                             |
| `chore`    | Misc maintenance that does not fit the above                         |

### Examples

```
feat: add per-depot re-download in Step 4
fix: reject non-numeric App IDs at command boundary
refactor: dedupe stdout/stderr streaming in depot_runner
ci: only trigger dev build on app code changes
```

### Breaking changes

Add `!` after the type and describe the break in the body:

```
refactor!: remove KernelOS and PrintedWaste sources

BREAKING CHANGE: the `search_alternative` Tauri command is gone; search now
uses Internet Archive only.
```

---

## Code style

### Rust

- Run `cargo fmt` and `cargo clippy` before committing.
- Prefer `Result<T, String>` for errors surfaced to the frontend; reserve
  panics for true invariants validated upstream.
- Validate user-controlled values at the trust boundary (the `#[command]`
  entry point), not deep inside the pipeline.
- When streaming external-process output to the frontend, use the existing
  `spawn_stream_forwarder` helper rather than hand-rolling a new loop.

### Frontend (vanilla JS, no framework)

- The app logic lives in `public/js/app/`, one plain script per area
  (`select.js`, `progress.js`, `history.js`, `emulator.js`, `settings.js`
  and so on). They share one global scope and load in the order listed at
  the end of `public/index.html`: `core.js` first with `state`, `els` and the
  shared helpers, `main.js` last with the start-up code. Only declare
  functions and constants at the top level of a file; code that runs on
  start-up belongs in `main.js` or the `init*` functions it calls.
- Always HTML-escape interpolated values before assigning to `innerHTML`.
  Use the `escapeHtml` helper in `public/js/app/core.js`.
- Prefer event delegation on containers over re-attaching listeners after
  each re-render.
- No `console.log` debug noise in merged code.

### General

- Don't add features, abstractions, or error handling the task doesn't
  require. Three similar lines beats a premature helper.
- Comments explain **why** something is done when it is non-obvious. Avoid
  comments that restate what the code already says.

---

## Pull request process

1. **One PR, one concern.** Don't mix unrelated changes. Refactors and
   behavior changes go in separate PRs when reasonable.
2. **Target `dev`.** PRs against `main` will be closed.
3. **Fill out the PR template.** The checklist helps the reviewer.
4. **Keep commits clean.** Squash fixup commits before review if you can.
5. **CI must be green.** The dev-build workflow runs on every PR.
6. **Be responsive.** Review feedback usually lands within a few days; if a
   PR is idle for two weeks it may be closed and can always be reopened.

### What a good PR looks like

- A title like `fix: crash when .lua file contains no addappid calls`.
- 1–3 sentences summarizing the change and its motivation.
- A test plan that the reviewer can reproduce.
- Screenshots or a short clip for UI changes.

---

## Reporting bugs / requesting features

Use the provided issue templates:

- [Bug report](../../issues/new?template=bug_report.md)
- [Feature request](../../issues/new?template=feature_request.md)
- [Question / Support](../../issues/new?template=question.md)

Blank issues are allowed for cases that don't fit a template, but please
include enough context for someone else to reproduce or understand the
problem.

---

## Code of conduct

Participation in this project is governed by the
[Code of Conduct](./CODE_OF_CONDUCT.md). Report unacceptable behavior to
<mcbabel.sup@protonmail.com>.