# Security

## Reporting vulnerabilities

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately via GitHub's security advisory system:

> https://github.com/forkwright/theatron/security/advisories/new

Include: description, reproduction steps, potential impact, affected crate or
example, affected version or commit, and any suggested fix.

## Response SLA

| Severity | Acknowledgment | Fix Target |
|----------|----------------|------------|
| Critical (CVSS >= 9.0) | 24 hours | 7 days |
| High (CVSS 7.0-8.9) | 48 hours | 14 days |
| Medium (CVSS 4.0-6.9) | 5 days | 30 days |
| Low (CVSS < 4.0) | 10 days | 90 days |

## Scope

**In scope:**

- Credential, token, or path leakage through `bathron`, `keryx`, `mekhane`,
  or examples.
- Unsafe local file handling in settings, logging, dialogs, or integration
  helpers.
- Network request handling bugs in `keryx`, including malformed response
  handling and event-stream parsing.
- OS integration issues in `mekhane`, including tray, menu, hotkey, and
  window/event wiring that could trigger unexpected local actions.
- Dependency or build-script behavior that creates a practical vulnerability
  in Theatron consumers.

**Out of scope:**

- Social engineering.
- Physical access attacks.
- Vulnerabilities that require arbitrary local code execution before Theatron
  APIs are called.
- Issues only present in upstream dependencies and not made worse by Theatron;
  report those upstream. Theatron will patch promptly when a dependency fix is
  available.

## Disclosure

After a fix ships, we publish a GitHub Security Advisory when warranted,
including affected versions, fixed version, impact, remediation, and credit to
the reporter.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.x latest minor | Yes |
| 1.x previous minor | Security fixes only |
| Older 1.x minors | Best effort |
| < 1.0 | No |

## Security Standards

Theatron follows the fleet security standards maintained in
`kanon/crates/basanos/standards/SECURITY.md`. In particular:

- Do not store credentials in source code, config files, examples, or default
  environment values.
- Do not log credentials or sensitive local paths unless explicitly redacted.
- Prefer typed secret wrappers for credential-bearing APIs.
- Use TLS for credential-bearing network traffic.
- Keep generated examples safe by default: local-only, no embedded secrets, and
  no privileged filesystem writes.
