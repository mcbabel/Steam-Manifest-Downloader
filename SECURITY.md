# Security Policy

## Supported versions

Only the **latest released version** of Steam Manifest Downloader receives
security fixes. Dev builds (`dev-build.yml` artifacts) are supported on a
best-effort basis.

| Version              | Supported |
| -------------------- | --------- |
| Latest stable        | ✅        |
| Dev / pre-release    | ⚠️ Best effort |
| Older stable releases| ❌        |

## Reporting a vulnerability

**Please do not open a public GitHub issue for security problems.**

Use either of the following private channels:

1. **Preferred:** GitHub Security Advisories — open a private report at
   <https://github.com/MCbabel/Steam-Manifest-Downloader/security/advisories/new>.
2. **Email:** <security@mcbabel.de>.

Please include:

- A description of the issue and its impact.
- Steps to reproduce (a proof-of-concept is very welcome).
- The affected version / commit.
- Your environment (OS, app version, .NET version).
- Optional: a suggested fix.

### What to expect

- **Initial acknowledgement** within **72 hours**.
- **Triage and severity assessment** within **7 days**.
- **A fix or mitigation plan** for confirmed high/critical issues within
  **30 days**. Lower-severity issues may take longer.
- Coordinated disclosure: once a fix is released, you will be credited in
  the release notes unless you prefer to remain anonymous.

### Scope

In scope:

- The desktop application (Rust backend, JS frontend, Tauri shell).
- The embedded DepotDownloaderMod packaging and invocation.
- Build and release workflows in `.github/workflows/`.

Out of scope:

- Vulnerabilities in upstream dependencies that are already publicly
  tracked — please report those to the upstream project. A report is still
  welcome if we are using a dependency in a uniquely unsafe way.
- Issues that require a user to voluntarily run an attacker-provided `.lua`
  or `.st` file that was crafted specifically to exploit their own machine.
  Parser crashes and misparses are still bugs — please report those via the
  normal bug tracker.
- Social-engineering, phishing, or physical-access attacks.

## Safe-harbor

Good-faith security research that follows this policy will not result in
legal action from the maintainers. Please act in a way that is consistent
with responsible disclosure: no data exfiltration beyond what is needed to
demonstrate the issue, no disruption to other users, and no public
disclosure before a fix is available.