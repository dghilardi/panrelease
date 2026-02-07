# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
