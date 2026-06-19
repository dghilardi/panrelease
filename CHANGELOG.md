# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.19.0] 2026-06-19

### Added
- `panrelease show --format docker-tag` to print the current version with Docker-compatible build metadata formatting (`+` -> `_`)

## [0.18.0] 2026-02-23
### Added
- Generic package manager: support for custom version files via `packageManager = "Generic"` with configurable `file`, `format` (json/xml/toml), and `versionField` (dot notation)
- TOML format-preserving codec (`TomlString`) for Generic package manager

### Fixed
- JSON parser now handles empty strings (`""`) correctly instead of failing with a parse error
- JSON parser now supports all standard escape sequences (`\/`, `\t`, `\r`, `\b`, `\f`, `\u`) instead of only `\"`, `\n`, `\\`

## [0.17.0] 2026-02-22
### Added
- Add `panrelease init` subcommand

## [0.16.0] 2026-02-22

### Fixed
- CLI entry point now prints a clean `error: ...` message on stderr instead of a Rust panic trace
- WASM/Node.js filesystem backend no longer panics on non-UTF-8 paths or JS bridge errors
- `apply_with_slug` returns an error instead of panicking when a branch slug contains characters invalid in semver build metadata (e.g. slashes in `feature/my-branch`)
- `extract_master_mod` defensive fix: replace two unreachable `expect()` calls with proper error returns
- Process execution errors now include the command name and stderr output, making failures from `git`, hooks, and package managers actionable
- "Could not find repo dir" error now reports the directory that was searched for `.git`
- Dirty staging area error now lists the uncommitted files instead of a generic message
- `.panproject.toml` TOML parse errors now include the file path
- Module validation errors now mention the module name and the expected manifest filename
- Error chain is preserved when a release fails (no longer flattened to a single string)
- Silent failures during changelog auto-population now emit `log::warn!` messages (visible with `RUST_LOG=warn`)
- JSON path extraction errors now report the traversal path, actual value type, and available keys
- Missing CLI tools (e.g. `git` not installed) now produce a clear "command not found — is it in your PATH?" error instead of a raw OS error 2

## [0.15.1] 2026-02-22

### Fixed
- Changelog update no longer panics when `## [Unreleased]` is absent: replaced unsupported lookahead regex (`(?=\n## )`) with plain string insertion

## [0.15.0] 2026-02-09

### Added
- New `show` command to display the current project version

## [0.14.0] 2026-02-08

### Added
- Auto-populate changelog from commit messages with configurable commit format parsing
- Support for Conventional Commits and Gitmoji commit message formats
- Auto-detection mode that picks the best-matching commit format across all commits
- Commit-to-changelog mapping: `feat` -> Added, `fix` -> Fixed, `refactor`/`perf` -> Changed, etc.
- Smart merge with existing `[Unreleased]` entries using `git blame` for deduplication
- Configurable scope inclusion and unmatched commit handling (optional "Other" section)
- Breaking change detection with `[BREAKING]` prefix in changelog entries
- New `[changelog]` configuration section in `.panproject.toml`

## [0.13.9] 2026-02-07

## [0.13.8] 2026-02-07

## [0.13.7] 2026-02-07

## [0.13.6] 2026-02-07

## [0.13.5] 2026-02-07

## [0.13.4] 2026-02-07

## [0.13.3] 2026-02-07

### Fixed
- Update `wasm-bindgen-cli` version to 0.2.92 to match `wasm-bindgen` dependency

## [0.13.2] 2026-02-07

### Fixed
- Make `update-informer` an optional dependency to prevent `rustls`/`ring` from being compiled for wasm32-unknown-unknown

## [0.13.1] 2026-02-07

### Fixed
- Fix WASM build: add `getrandom` js feature for wasm32-unknown-unknown support
- Fix WASM build: pass `--lib` to cargo build to avoid output filename collision between bin and lib targets

## [0.13.0] 2026-02-07

### Added
- Strict mode: opt-in branch-aware release validation via `[vcs.strict]` config
- Automatic slug inference from branch name for post-releases (replaces `dev` default)
- Branch classification: mainline, feature (`feat/`, `feature/`), hotfix (`hotfix/`, `fix/`)
- Base version validation against merge-base with mainline
- `current_branch()`, `merge_base()`, and `latest_version_tag()` methods on both git backends

### Fixed
- `current_branch()` in CLI git backend (was broken/incomplete)

## [0.12.4] 2024-07-09
### Added
- Support for gradle.properties

## [0.12.3] 2023-09-17
### Fixed
- Fix args passed in git commit signed version

## [0.12.2] 2023-09-17
### Modified
- Use cli git as default

### Fixed
- Do not allow unsigned commit if force_sign flag is true also in library mode

## [0.12.1] 2023-09-16

## [0.12.0] 2023-09-16
### Add
- Add `force_sign` config field for git configuration

## [0.11.3] 2023-05-26

## [0.11.2] 2023-05-18
### Fix
- default tag template if no .panproject is defined

## [0.11.1] 2023-05-15

## [0.11.0] 2023-05-15
### Changed
- Use `cargo check` instead of `cargo generate-lockfile` after version change

## [0.10.0] 2023-04-09
### Add
- `tag_template` config field for git configuration

### Fix
- changelog and modules path detection

## [0.8.0] 2023-03-31

## [0.7.1] 2023-02-14

## [0.7.0] 2023-02-13

## [0.6.0] 2022-12-20

## [0.5.0] 2022-12-19
### Added
- Implementation for maven packages
- Implementation for npm packages

## [0.4.0] 2022-12-14

### Added
- update changelog during release
- autodetect single-module projects
