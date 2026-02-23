# Architecture Overview

This document describes the technical architecture of Panrelease, its design principles, and how the codebase is organized.

## Table of Contents

- [Design Principles](#design-principles)
- [High-Level Architecture](#high-level-architecture)
- [Module Structure](#module-structure)
- [Core Abstractions](#core-abstractions)
- [Release Workflow](#release-workflow)
- [Dual Target Support](#dual-target-support)
- [Data Flow](#data-flow)

## Design Principles

1. **Trait-based abstractions** - Core functionality is defined through traits, enabling different implementations (native vs WASM, CLI git vs libgit2)
2. **Package manager agnosticism** - All package managers implement the same `PanPackage` trait
3. **Platform independence** - A `FileSystem` trait abstracts I/O, enabling native and WASM targets
4. **Configuration-driven** - Behavior is controlled by `.panproject.toml`, not hardcoded assumptions
5. **Minimal dependencies** - Only essential crates are used, with optional features for alternatives

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Entry Points                         │
│  ┌──────────────────┐    ┌────────────────────────────┐  │
│  │ bin/panrelease.rs │    │ lib.rs (WASM/wasm-bindgen) │  │
│  │   (Native CLI)    │    │    (Node.js module)        │  │
│  └────────┬─────────┘    └──────────┬─────────────────┘  │
│           │                         │                     │
│           └──────────┬──────────────┘                     │
│                      ▼                                    │
│              ┌───────────────┐                            │
│              │  engine.rs    │  CLI arg parsing +         │
│              │               │  config loading            │
│              └───────┬───────┘                            │
│                      ▼                                    │
│           ┌─────────────────────┐                         │
│           │  project/core.rs    │  Release orchestration  │
│           │    PanProject       │                         │
│           └──┬──────────┬───┬──┘                         │
│              │          │   │                             │
│     ┌────────┘    ┌─────┘   └──────┐                     │
│     ▼             ▼                ▼                      │
│ ┌────────┐  ┌──────────┐   ┌────────────┐               │
│ │  git/  │  │ package/ │   │ CHANGELOG  │               │
│ │GitRepo │  │PanPackage│   │  updater   │               │
│ └────────┘  └──────────┘   └────────────┘               │
│                  │                                        │
│    ┌─────┬──────┼──────┬────────┬───────┐                 │
│    ▼     ▼      ▼      ▼        ▼       ▼                 │
│ Cargo   Npm   Maven  Gradle  Generic  (future)            │
│                                                           │
│ ┌─────────────────────────────────────────────────────┐  │
│ │              system/ (FileSystem trait)              │  │
│ │  ┌─────────────────┐    ┌────────────────────────┐  │  │
│ │  │ NativeFileSystem │    │   NodeJsFileSystem     │  │  │
│ │  │  (std::fs)       │    │   (wasm-bindgen/js)    │  │  │
│ │  └─────────────────┘    └────────────────────────┘  │  │
│ └─────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Module Structure

### `src/bin/panrelease.rs` - CLI Binary
The native entry point. Initializes logging, checks for version updates, and delegates to `engine::run()`.

### `src/lib.rs` - WASM Library
Exports a `run()` function via `#[wasm_bindgen]` for use from JavaScript/Node.js. Uses `NodeJsSystem` as the filesystem implementation.

### `src/engine.rs` - Orchestration
Parses CLI arguments using `clap`, loads configuration, initializes `PanProject`, and triggers the release workflow.

### `src/args.rs` - Argument Parsing
Defines the `RelArgs` struct and the `BumpLevel` enum. Handles parsing of bump types (`major`, `minor`, `patch`, `post`, or explicit version).

### `src/conf/` - Configuration
- **`loader.rs`** - Loads and deserializes `.panproject.toml` files

### `src/project/` - Project Management
- **`core.rs`** - `PanProject` struct: the main release orchestrator
- **`config.rs`** - Configuration data structures (`PanProjectConfig`, `VcsConfig`, `GitConfig`, `PackageManager`)
- **`module.rs`** - `PanModule` wraps a package manager implementation with module metadata

### `src/package/` - Package Managers
Each file implements the `PanPackage` trait:
- **`cargo.rs`** - Reads/writes `Cargo.toml`, runs `cargo check` for lockfile updates
- **`npm.rs`** - Reads/writes `package.json`, detects and updates lockfiles (npm/yarn/pnpm)
- **`maven.rs`** - Parses `pom.xml` using custom XML parser, supports property-based versions
- **`gradle.rs`** - Reads/writes `gradle.properties`
- **`generic.rs`** - User-configurable package manager supporting JSON, XML, and TOML files with dot-notation path to version field

### `src/git/` - Git Integration
- **`mod.rs`** - `GitRepo` struct and common interface
- **`cligit.rs`** - Default implementation using system `git` CLI commands
- **`libgit.rs`** - Alternative implementation using the `git2` library (behind `libgit` feature flag)

### `src/parser/` - File Parsers
- **`json.rs`** - Format-preserving JSON manipulation (custom `nom`-based parser)
- **`xml/`** - Format-preserving XML parser built with `nom` for `pom.xml`
- **`tomlcodec.rs`** - Format-preserving TOML manipulation (wraps `toml_edit`)

### `src/system/` - Platform Abstraction
- **`contract.rs`** - `FileSystem` and `EnvVars` traits
- **`native_system.rs`** - Implementation using `std::fs`
- **`nodejs_system.rs`** - Implementation using `wasm-bindgen` and `js-sys`

### `src/runner.rs` - Command Execution
Abstracts shell command execution for hooks across native and WASM environments.

### `src/utils.rs` - Utilities
Helper functions used across the codebase.

## Core Abstractions

### `PanPackage` Trait

The central abstraction for package manager support:

```rust
pub trait PanPackage {
    fn extract_version(&self) -> anyhow::Result<semver::Version>;
    fn set_version(&mut self, version: &semver::Version) -> anyhow::Result<()>;
    fn persist(&self) -> anyhow::Result<()>;
    fn hook_after_rel(&self) -> anyhow::Result<()>;
}
```

To add a new package manager, implement this trait and register it in the `PackageManager` enum.

### `FileSystem` Trait

Abstracts file I/O to support both native and WASM targets:

```rust
pub trait FileSystem {
    fn read_string(path: &Path) -> Result<String>;
    fn write_string(path: &Path, content: &str) -> Result<()>;
    fn current_dir() -> Result<PathBuf>;
    fn is_a_dir(path: &Path) -> bool;
    fn is_a_file(path: &Path) -> bool;
}
```

### `PackageManager` Enum

Detected automatically or configured explicitly:

```rust
pub enum PackageManager {
    Cargo,      // Detects Cargo.toml
    Npm,        // Detects package.json
    Maven,      // Detects pom.xml
    Gradle,     // Detects gradle.properties
    Generic {   // User-configured: any JSON, XML, or TOML file
        file: String,
        format: GenericFormat,
        version_field: String,
    },
}
```

## Release Workflow

The release process follows these steps:

```
1. PanProject::load()
   ├── Find Git root directory
   ├── Load .panproject.toml configuration
   └── Open Git repository

2. PanProject::release()
   ├── Verify staging area is clean
   ├── Extract current version from main module
   ├── Calculate new version (based on bump level)
   ├── For each module:
   │   ├── Set new version in package manifest
   │   ├── Persist changes to disk
   │   └── Execute after_rel hooks
   ├── Update CHANGELOG.md
   │   ├── Ensure [Unreleased] section exists
   │   └── Add new version section with date
   └── Git operations
       ├── Stage all changed files
       ├── Commit with version as message
       └── Create tag (using tag_template)
```

## Dual Target Support

Panrelease compiles to two targets:

### Native (default)
- Standard Rust binary
- Uses `std::fs` for file operations
- Uses system `git` CLI (or `git2` with `libgit` feature)
- Direct process execution for hooks

### WebAssembly (WASM)
- Compiled with `wasm32-unknown-unknown` target
- JavaScript bindings via `wasm-bindgen`
- File operations delegated to Node.js via `js-sys`
- Distributed as npm package
- Build process: `cargo build --target wasm32-unknown-unknown` + `wasm-bindgen`

The `FileSystem` trait is the key enabler - all I/O goes through this abstraction, allowing seamless switching between native and WASM implementations.

## Data Flow

```
User CLI Input
     │
     ▼
  clap parsing ──► RelArgs { level_or_version: BumpLevel }
     │
     ▼
  Config loading ──► PanProjectConfig { vcs, modules }
     │
     ▼
  PanProject::load() ──► PanProject { path, conf, repo }
     │
     ▼
  extract_master().extract_version() ──► current: semver::Version
     │
     ▼
  BumpLevel::apply(current) ──► new: semver::Version
     │
     ▼
  For each module:
     PanModule::set_version(new) ──► Updates in-memory manifest
     PanModule::persist()        ──► Writes manifest to disk
     PanModule::hook_after_rel() ──► Runs post-release commands
     │
     ▼
  update_changelog(new) ──► Modifies CHANGELOG.md
     │
     ▼
  GitRepo::update_and_commit(new) ──► git add + commit + tag
```

## Adding New Features

### Adding a Package Manager

1. Create `src/package/newpm.rs`
2. Implement `PanPackage` trait
3. Add variant to `PackageManager` enum in `src/project/config.rs`
4. Add detection logic in `PackageManager::detect()`
5. Add validation in `PanProjectConfig::validate_module()`
6. Wire up in `PanModule::new()` (in `src/project/module.rs`)

### Adding a VCS Backend

1. Add variant to `VcsConfig` enum
2. Implement the Git-equivalent operations
3. Update `PanProject::load()` to handle the new VCS

### Adding CLI Options

1. Add fields to the clap struct in `src/args.rs`
2. Thread the options through `engine::run()` to where they're needed
