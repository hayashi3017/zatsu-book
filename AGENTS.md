# Repository Guidelines

You are a senior Rust engineer with write access to this repository. Implement and maintain this repository as a Rust-only template for tag-driven releases with the `xtask` pattern.

## 0. Mandatory Constraints

- Use Rust tooling only. Do not introduce Node.js, npm, Homebrew, or non-Rust build systems.
- The single source of truth for versioning is `[workspace.package].version` in the root `Cargo.toml`.
- Releases are triggered by pushing Git tags through GitHub Actions.
- Tag formats:
  - Stable: `rust-vX.Y.Z`
  - Pre-release: `rust-vX.Y.Z-alpha.N` and `rust-vX.Y.Z-beta.N`
- On tag push, the release workflow must verify:
  1. The tag matches the required regex format.
  2. The version extracted from the tag equals `[workspace.package].version`.
- `CHANGELOG.md` should be one-line guidance to GitHub Releases.
- Do not include steps that require private signing keys (for example signing/notarization).

## 1. Required Repository Layout

- Root Cargo workspace (`Cargo.toml`)
- `xtask/` binary crate
- `.cargo/config.toml` with `cargo xtask` alias
- `.github/workflows/ci.yml`
- `.github/workflows/rust-release.yml`
- `RELEASING.md`
- `CHANGELOG.md`
- `README.md`
- `rust-toolchain.toml` (stable + `rustfmt`/`clippy`)
- `.gitignore` (for example `target/`)

## 2. Workspace Requirements

- Root `Cargo.toml` must include:
  - `[workspace]` with `members = ["xtask"]`
  - `resolver = "2"`
  - `[workspace.package]` with `version = "0.1.0"`, `edition = "2024"`, and license metadata
- `xtask` must:
  - Build an `xtask` binary (`cargo xtask --help`)
  - Print workspace version via `--version`
  - Provide a `hello` subcommand that prints `hello`
  - Provide task subcommands for local checks (`fmt`, `clippy`, `test`, `ci`)
- `cargo fmt`, `cargo clippy`, and `cargo test` must pass.

## 3. CI Workflow Requirements

`ci.yml` must run on pull requests and pushes to `main`, with matrix builds on Ubuntu, Windows, and macOS. It must run:
- `cargo xtask fmt`
- `cargo xtask clippy`
- `cargo xtask test`

## 4. Release Workflow Requirements

`rust-release.yml` must run on `push` tags matching `rust-v*.*.*` and contain:
- `tag-check` job (Ubuntu):
  - Validate tag format in bash (including alpha/beta patterns)
  - `tag_ver="${GITHUB_REF_NAME#rust-v}"`
  - Read `[workspace.package].version` from root `Cargo.toml`
  - Fail if `tag_ver != cargo_ver`
- `build` job (matrix):
  - Targets:
    - `x86_64-unknown-linux-gnu`
    - `x86_64-pc-windows-msvc`
    - `x86_64-apple-darwin`
    - `aarch64-apple-darwin`
  - Build command: `cargo build --release --locked -p xtask --target <target>`
  - Package artifacts:
    - Linux/macOS: `.tar.gz` with renamed binary `xtask-<target>`
    - Windows: `.zip` with renamed binary `xtask-<target>.exe`
  - Upload artifacts per target
- `release` job (Ubuntu):
  - Download all artifacts
  - Peel tag object and extract tagged commit message as release notes:
    - `commit="$(git rev-parse "${GITHUB_SHA}^{commit}")"`
    - `git log -1 --format=%B "${commit}" > release-notes.md`
  - Mark prerelease when tag contains `-alpha` or `-beta`
  - Create GitHub Release and attach all artifacts

## 5. Releasing Instructions (Human)

`RELEASING.md` must describe:
- Bump `[workspace.package].version` in `Cargo.toml`
- Run local checks via `cargo xtask ci`
- Commit message becomes release notes
- Create annotated tag:
  - `git tag -a rust-vX.Y.Z -m "Release X.Y.Z"`
- Push:
  - `git push origin main`
  - `git push origin rust-vX.Y.Z`
- GitHub Actions creates the release and uploads artifacts

## 6. Implementation Policy

- Create all required files first, then format and validate.
- Keep the template minimal.
- Do not add advanced features (signing, notarization, musl-specific packaging, or complex cross-compilation tricks).
- `README.md` should concisely cover purpose, local usage, CI, and release flow.

## 7. Done Criteria

- The repository is in a state where `cargo test --workspace` succeeds locally.
- `ci.yml` and `rust-release.yml` are valid YAML and use pinned action versions (major pinning is acceptable).
- The release workflow reliably fails when tag version and `Cargo.toml` workspace version do not match.
