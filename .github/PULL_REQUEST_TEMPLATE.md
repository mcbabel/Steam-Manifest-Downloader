<!--
Thanks for your pull request!

Please make sure:
- The PR targets the `dev` branch (not `main`).
- The title follows Conventional Commits, e.g. `feat: ...`, `fix: ...`,
  `refactor: ...`, `ci: ...`, `docs: ...`.
- You have read CONTRIBUTING.md.
-->

## Summary

<!-- What does this PR do and why? 1–3 sentences. -->

## Related issues

<!-- e.g. Closes #123, Refs #456. Leave blank if none. -->

## Type of change

- [ ] Bug fix (non-breaking)
- [ ] New feature (non-breaking)
- [ ] Breaking change (fix or feature that would break existing behavior)
- [ ] Refactor / cleanup (no behavior change)
- [ ] Docs / CI / build only

## How was this tested?

<!--
Describe the steps you ran to verify this works. If UI changes, mention
the OS you tested on (Windows / Linux) and attach screenshots or a short
clip where useful.
-->

- [ ] `cargo check` / `cargo build` passes locally
- [ ] Tested with `cargo run -p smd-gui`
- [ ] Tested on Windows
- [ ] Tested on Linux

## Checklist

- [ ] PR targets `dev`
- [ ] Commits follow Conventional Commits (`type: subject`)
- [ ] No unrelated changes mixed in
- [ ] Values put into markup strings are escaped with `escape_html`
- [ ] No new `println!` debug noise left behind
- [ ] I agree to contribute my changes under the project's **GPL-2.0** license