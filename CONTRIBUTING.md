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

The desktop app (`src-gui`) is written in Rust from the window down to the
last button, with egui for drawing and taffy for the layout. There is also a
terminal version (`src-tui`). Both share the Rust core in `src-core`.
DepotDownloaderMod is embedded as an optional download engine.

### Prerequisites

- **Rust** (latest stable) + Cargo — <https://rustup.rs/>
- **Platform build deps**:
  - Windows: Visual Studio Build Tools with C++ workload.
  - Linux: `build-essential`.
- **cargo-packager**, only to build an installer, AppImage or DMG:
  ```bash
  cargo install cargo-packager --locked
  ```
- **Python 3**, only if you change `src-gui/tools/index.html` (see below).
- **.NET 9 runtime** *(optional)*, only to test the DepotDownloaderMod engine.

See [the build guide in the docs](./docs/documentation/index.html#building-from-source)
for more detailed, up-to-date instructions.

### Run the app in dev mode

```bash
cargo run -p smd-gui
```

### Build a release binary

```bash
cargo build --release -p smd-gui
```

The result is `target/release/steam-manifest-downloader`, one file that runs
on its own.

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

BREAKING CHANGE: the `search_alternative` command is gone; search now
uses Internet Archive only.
```

---

## Code style

### Rust

- Run `cargo fmt` and `cargo clippy` before committing.
- Prefer `Result<T, String>` for errors surfaced to the interface; reserve
  panics for true invariants validated upstream.
- Validate user-controlled values at the trust boundary (the command
  entry point), not deep inside the pipeline.
- When streaming external-process output to the interface, use the existing
  `spawn_stream_forwarder` helper rather than hand-rolling a new loop.

### Desktop interface (`src-gui`)

- **Look**: the app reads `src-gui/assets/style.css` itself. Change colours,
  spacing and sizes there, not in Rust.
- **Structure**: screens and dialogs are described in
  `src-gui/tools/index.html`.
  After a change run `python3 src-gui/tools/gen_views.py`; it rewrites
  `src-gui/src/app/view/`. Do not edit those files by hand.
- **Behaviour**: what a button, field or dialog does lives in
  `src-gui/src/app/logic/`, one module per area (`select.rs`, `progress.rs`,
  `history.rs`, `emulator.rs`, `settings.rs` and so on). The modules work on
  `app.ui`, a store of element state by id: `app.ui.show("id")`,
  `app.ui.set_text("id", ..)`, `app.ui.set_class("id", "name", on)`.
- **Lists and other dynamic content** are written as markup strings and
  handed to `dom.markup(..)`. Always escape values with `escape_html` before
  you put them into such a string.
- **Texts** come from `locales/*.json` through `t!(app, "key")`. Every
  text needs an English and a German entry.
- **Backend calls** go through `app.tasks.call("command", args, |app, result| ..)`
  so the window never blocks.

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
<conduct@mcbabel.de>.