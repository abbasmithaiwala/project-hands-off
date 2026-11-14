# Security policy

## Reporting a vulnerability

Please do not open a public issue for a security problem. Report it privately through
[GitHub's private vulnerability reporting](../../security/advisories/new) for this repository, with:

- what you found and where,
- steps to reproduce, or a proof of concept,
- the version or commit you tested.

You will get an acknowledgement within a few days and a fix or a plan as soon as it is understood.
Credit is given in the release notes unless you prefer otherwise.

## Supported versions

Security fixes go into the latest release. Please update before reporting.

## What the server protects

- **Your OpenRouter key** stays on the server. It is write-only through the API (never returned), never logged,
  and never sent to the browser.
- **Uploads** are limited by size and dimensions and are decoded as images only. File names are never used as paths.
- **Browsers** may call the API only from the origins in `HANDS_OFF_CORS_ORIGINS`, and responses carry
  `nosniff`, `X-Frame-Options: DENY` and `Referrer-Policy: no-referrer`.
- **Model output** (suggestions, control placement, summaries) is validated and clamped before use.

## What is out of scope

Hands-Off has no accounts or authentication: it is meant to run for you, on your machine or your private network.
Do not expose it to the public internet without putting authentication in front of it (see
[docs/deployment.md](docs/deployment.md)). Reports that depend on an unauthenticated public deployment are not
vulnerabilities in the application.
