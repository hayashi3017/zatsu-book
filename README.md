# rs-version-template

A minimal Rust-only template repository for tag-driven releases using the `xtask` pattern.

## Purpose

This template mirrors the release model used by `openai/codex` with a minimal setup:
- Version source of truth is root `Cargo.toml` at `[workspace.package].version`.
- Releases are driven by pushing tags like `rust-vX.Y.Z`.
- GitHub Releases are the changelog.
- Project automation is centralized in the `xtask` binary.

## Project Layout

- `Cargo.toml`: workspace settings and shared package metadata.
- `xtask/`: automation binary crate.
- `.cargo/config.toml`: `cargo xtask` alias.
- `.github/workflows/ci.yml`: CI using `xtask` commands.
- `.github/workflows/rust-release.yml`: tag validation, multi-target build, and GitHub Release.

## Local Development

```bash
cargo xtask --help
cargo xtask hello
cargo xtask ci
```

Equivalent explicit invocation:

```bash
cargo run -p xtask -- hello
```

## CI

CI runs on pushes to `main` and pull requests across Ubuntu, Windows, and macOS:
- `cargo xtask fmt`
- `cargo xtask clippy`
- `cargo xtask test`

## Release Summary

1. Bump `[workspace.package].version` in `Cargo.toml`.
2. Commit with release-note-ready commit message.
3. Create and push an annotated tag (`rust-vX.Y.Z`, optionally `-alpha.N` / `-beta.N`).
4. Release workflow validates tag/version, builds target artifacts, and publishes GitHub Release.

Detailed steps: see `RELEASING.md`.
