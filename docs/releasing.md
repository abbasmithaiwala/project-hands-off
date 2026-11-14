# Releasing

Releases are cut by pushing a version tag; GitHub Actions does the rest (`.github/workflows/release.yml`).

## Cut a release

1. Make sure `main` is green.
2. Pick the version (semantic versioning). Update `version` in the root `Cargo.toml` (`[workspace.package]`),
   `web/package.json`, and `Cargo.lock` (`cargo check`), and move the "Unreleased" entries in
   [CHANGELOG.md](../CHANGELOG.md) under the new version and date.
3. Commit as `chore: release vX.Y.Z`, then tag and push:

   ```sh
   git tag -s vX.Y.Z -m "vX.Y.Z"
   git push origin main vX.Y.Z
   ```

The workflow refuses to publish when the tag does not match the version in `Cargo.toml`.

## What the workflow publishes

- **Archives** for Linux (x86_64, aarch64) and macOS (Apple silicon, Intel): the binary, the built web app,
  `LICENSE` and `THIRD_PARTY_NOTICES.md`.
- **Detection model**: `rfdetr-seg-medium.onnx` and its checksum, exported with `scripts/export_models.py`, as release assets.
  The container image downloads it from there on first start.
- **Container image** for `linux/amd64` and `linux/arm64` on `ghcr.io/abbasmithaiwala/project-hands-off`, tagged `X.Y.Z`, `X.Y` and `latest`.
- **Checksums**: `SHA256SUMS` for every asset, signed with [cosign](https://docs.sigstore.dev/) keyless signing
  (`SHA256SUMS.sig` and `SHA256SUMS.pem`); the container image is signed the same way.
- **Release notes**, taken from the matching section of `CHANGELOG.md`.

## Verify a download

```sh
sha256sum -c SHA256SUMS --ignore-missing
cosign verify-blob SHA256SUMS \
  --signature SHA256SUMS.sig --certificate SHA256SUMS.pem \
  --certificate-identity-regexp 'https://github.com/abbasmithaiwala/project-hands-off/.github/workflows/release.yml@.*' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
cosign verify ghcr.io/abbasmithaiwala/project-hands-off:X.Y.Z \
  --certificate-identity-regexp 'https://github.com/abbasmithaiwala/project-hands-off/.github/workflows/release.yml@.*' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```
