# Releasing

This document describes semantic versioning and release operations for this Rust `xtask` template.

## Scope

- Rust-only repository.
- Single source of truth for version: `[workspace.package].version` in root `Cargo.toml`.
- Releases are triggered by pushing git tags.
- Supported tags:
- Stable: `rust-vX.Y.Z`
- Pre-release: `rust-vX.Y.Z-alpha.N`
- Pre-release: `rust-vX.Y.Z-beta.N`

## SemVer Policy

- `MAJOR` (`X`): breaking changes.
- `MINOR` (`Y`): backward-compatible feature additions.
- `PATCH` (`Z`): backward-compatible fixes.
- `alpha`/`beta`: validation stages before stable release.

Examples:
- `0.1.0 -> 0.1.1` (patch)
- `0.1.1 -> 0.2.0` (minor)
- `0.2.0 -> 1.0.0` (major)
- `1.2.0-alpha.1 -> 1.2.0-beta.1 -> 1.2.0` (pre-release to stable)

## Standard Release Flow

1. Decide the next version according to the SemVer policy.
2. Update `[workspace.package].version` in root `Cargo.toml`.
3. Run local validation:
```bash
cargo xtask ci
```
4. Commit with a message suitable for release notes.
5. Create an annotated tag:
```bash
git tag -a rust-vX.Y.Z -m "Release X.Y.Z"
```
6. Push branch and tag:
```bash
git push origin main
git push origin rust-vX.Y.Z
```
7. Verify GitHub Actions and attached GitHub Release artifacts.

## Pre-release Flow

1. Set the pre-release version in `Cargo.toml`.
2. Tag with `rust-vX.Y.Z-alpha.N` or `rust-vX.Y.Z-beta.N`.
3. Push the tag and confirm the release is marked as prerelease.
4. For stable release, remove the suffix and tag `rust-vX.Y.Z`.

## Enforced CI Validation

- The release workflow validates tag format using regex.
- The version extracted from the tag must match `[workspace.package].version`.
- Any mismatch fails the workflow and blocks release publishing.

## Recovery from Tag Mistakes

If an incorrect tag was pushed:

```bash
git tag -d rust-vX.Y.Z
git push --delete origin rust-vX.Y.Z
```

Then fix `Cargo.toml` or the tag name and push the correct tag.

## Documentation Notes

- `AGENTS.md` is the current operational guide.
