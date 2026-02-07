# Panrelease

[![Test Status](https://github.com/dghilardi/panrelease/workflows/Tests/badge.svg?event=push)](https://github.com/dghilardi/panrelease/actions)
[![Crate](https://img.shields.io/crates/v/panrelease.svg)](https://crates.io/crates/panrelease)
[![API](https://docs.rs/panrelease/badge.svg)](https://docs.rs/panrelease)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![npm version](https://badge.fury.io/js/panrelease.svg)](https://www.npmjs.com/package/panrelease)

**Panrelease** is a versatile release automation tool that manages version bumping, changelog updates, and Git operations across multiple package managers. It supports **Cargo** (Rust), **npm** (Node.js), **Maven**, and **Gradle** (Java) projects in a unified workflow.

## Features

- **Multi-Language Support** - Unified version management for Cargo, npm, Maven, and Gradle projects
- **Semantic Versioning** - Automatic major, minor, patch, and post-release version bumping
- **Changelog Automation** - Automatically updates CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/) format
- **Git Integration** - Commits version changes, creates tags, and supports GPG signing
- **Multi-Module Projects** - Manages monorepos with multiple packages
- **Custom Hooks** - Execute arbitrary commands after releases (build, test, deploy)
- **Cross-Platform** - Available as native CLI binary and npm package (via WebAssembly)
- **Configurable Tag Templates** - Customize Git tag naming (e.g., `v{{version}}`)

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Usage](#usage)
- [Package Manager Support](#package-manager-support)
- [Multi-Module Projects](#multi-module-projects)
- [Hooks](#hooks)
- [Strict Mode](#strict-mode)
- [Git Integration](#git-integration)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Installation

### Using Cargo (Rust)

```bash
cargo install panrelease
```

### Using npm/yarn/pnpm (Node.js)

```bash
# npm
npm install -g panrelease

# yarn
yarn global add panrelease

# pnpm
pnpm add -g panrelease
```

### From Source

```bash
git clone https://github.com/dghilardi/panrelease.git
cd panrelease
cargo build --release
```

The binary will be available at `target/release/panrelease`.

## Quick Start

1. **Initialize your project** by creating a `.panproject.toml` file in your project root:

```toml
[vcs]
software = "Git"

[modules.main]
path = "."
packageManager = "Cargo"  # or "Npm", "Maven", "Gradle"
main = true
```

2. **Create a release** with a version bump:

```bash
# Patch release (1.0.0 -> 1.0.1)
panrelease patch

# Minor release (1.0.0 -> 1.1.0)
panrelease minor

# Major release (1.0.0 -> 2.0.0)
panrelease major

# Set explicit version
panrelease 2.0.0
```

That's it! Panrelease will:
- Update version in your package manifest (Cargo.toml, package.json, pom.xml, etc.)
- Update CHANGELOG.md with the new version and date
- Commit the changes
- Create a Git tag

## Configuration

Panrelease is configured via a `.panproject.toml` file in your project root.

### Basic Configuration

```toml
[vcs]
software = "Git"
force_sign = false              # Require GPG-signed commits
tag_template = "{{version}}"    # Git tag format (e.g., "v{{version}}" -> "v1.0.0")

[modules.myapp]
path = "."
packageManager = "Cargo"        # Cargo | Npm | Maven | Gradle
main = true                     # Designates primary module for version detection
```

### Configuration Options

| Section | Field | Type | Description |
|---------|-------|------|-------------|
| `vcs` | `software` | string | Version control system (`Git`) |
| `vcs` | `force_sign` | bool | Require GPG-signed commits (default: `false`) |
| `vcs` | `tag_template` | string | Git tag template (default: `{{version}}`) |
| `vcs.strict` | `mainline` | string | Mainline branch name for [strict mode](#strict-mode) validation |
| `modules.<name>` | `path` | string | Path to module relative to project root |
| `modules.<name>` | `packageManager` | string | Package manager: `Cargo`, `Npm`, `Maven`, `Gradle` |
| `modules.<name>` | `main` | bool | Mark as primary module for version extraction |

For detailed configuration options, see [docs/CONFIGURATION.md](docs/CONFIGURATION.md).

## Usage

### Command Line Interface

```bash
panrelease [OPTIONS] <BUMP_TYPE>
```

### Bump Types

| Type | Description | Example |
|------|-------------|---------|
| `major` | Increment major version | `1.2.3` -> `2.0.0` |
| `minor` | Increment minor version | `1.2.3` -> `1.3.0` |
| `patch` | Increment patch version | `1.2.3` -> `1.2.4` |
| `post` | Create post-release with build metadata | `1.2.3` -> `1.2.3+feat.r1` |
| `X.Y.Z` | Set explicit version | `1.2.3` -> `2.0.0` |

### Options

```
-h, --help       Print help information
-V, --version    Print version information
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Set log level (`error`, `warn`, `info`, `debug`, `trace`) |

### Examples

```bash
# Standard patch release
panrelease patch

# Minor release with debug logging
RUST_LOG=debug panrelease minor

# Set specific version
panrelease 1.5.0

# Post-release for feature branch
panrelease post
```

## Package Manager Support

### Cargo (Rust)

Updates `Cargo.toml` and runs `cargo check` to update `Cargo.lock`.

```toml
[modules.rust-lib]
path = "."
packageManager = "Cargo"
main = true
```

### npm (Node.js)

Updates `package.json` and automatically detects/updates the appropriate lockfile (`package-lock.json`, `yarn.lock`, or `pnpm-lock.yaml`).

```toml
[modules.node-app]
path = "."
packageManager = "Npm"
main = true
```

### Maven (Java)

Updates `pom.xml`, supporting both direct version fields and property-based versions.

```toml
[modules.java-lib]
path = "."
packageManager = "Maven"
main = true
```

### Gradle (Java/Kotlin)

Updates `gradle.properties` file.

```toml
[modules.gradle-app]
path = "."
packageManager = "Gradle"
main = true
```

## Multi-Module Projects

Panrelease excels at managing monorepos with multiple packages:

```toml
[vcs]
software = "Git"
tag_template = "v{{version}}"

[modules.core]
path = "packages/core"
packageManager = "Cargo"
main = true  # Primary module determines project version

[modules.cli]
path = "packages/cli"
packageManager = "Cargo"

[modules.web]
path = "packages/web"
packageManager = "Npm"

[modules.server]
path = "packages/server"
packageManager = "Maven"
```

All modules are updated simultaneously during a release.

## Hooks

Execute custom commands after releases using hooks:

```toml
[modules.myapp]
path = "."
packageManager = "Cargo"
main = true

[modules.myapp.hooks.after_rel]
build = ["cargo", "build", "--release"]
test = ["cargo", "test"]
publish = ["cargo", "publish"]
```

Hooks are executed in the order they are defined.

## Strict Mode

Strict mode is an opt-in feature that enforces branch-aware release validation. When enabled, Panrelease checks which branch you are on, restricts what kind of release is allowed, and automatically infers the build metadata slug from the branch name.

### Enabling Strict Mode

Add a `[vcs.strict]` section to your `.panproject.toml`:

```toml
[vcs]
software = "Git"

[vcs.strict]
mainline = "main"   # your mainline branch name (e.g., "main", "master")
```

### Branch Classification

Strict mode classifies the current branch into one of three kinds:

| Branch Pattern | Kind | Example |
|----------------|------|---------|
| Matches `mainline` exactly | Mainline | `main` |
| `feat/*` or `feature/*` | Feature | `feat/user-registration` |
| `hotfix/*` or `fix/*` | Hotfix | `fix/timeout-error` |

Any other branch name is rejected with an error.

### Release Rules

Strict mode enforces different rules depending on the branch kind:

**Mainline branches** can only produce clean releases (major, minor, patch). Post-releases are not allowed from mainline.

```bash
# On main branch:
panrelease patch   # OK  -> 1.2.4
panrelease minor   # OK  -> 1.3.0
panrelease post    # ERROR: cannot release a post-version from mainline
```

**Feature and hotfix branches** can only produce post-releases. The build metadata slug is automatically derived from the branch name.

```bash
# On feat/user-registration:
panrelease post    # OK  -> 1.2.3+user-registration.r1
panrelease patch   # ERROR: feature branches can only produce post-releases
```

### Slug Inference

The branch name after the prefix is sanitized into a slug used as build metadata:

- Non-alphanumeric characters are replaced with hyphens
- Consecutive hyphens are collapsed
- Leading/trailing hyphens are removed
- Hotfix branches get a `fix-` prefix in the slug

| Branch | Inferred Slug |
|--------|--------------|
| `feat/user-registration` | `user-registration` |
| `feature/hello@.-world` | `hello-world` |
| `hotfix/npe-fix` | `fix-npe-fix` |
| `fix/timeout` | `fix-timeout` |

### Additional Validations

When releasing from a feature or hotfix branch, strict mode also verifies:

- The version base (major.minor.patch) matches the latest tagged version reachable from the merge-base with mainline
- The mainline version at the branch point is a clean version (not a post-release)
- The build metadata slug matches the expected slug for the current branch
- Releases from detached HEAD are not allowed

If any check fails, Panrelease prints a descriptive error message with a suggested fix (e.g., rebase to latest mainline).

## Git Integration

### Automatic Operations

During a release, Panrelease automatically:
1. Verifies the staging area is clean
2. Updates version in package manifests
3. Updates CHANGELOG.md
4. Executes post-release hooks
5. Commits all changes with the version as the commit message
6. Creates a Git tag based on `tag_template`

### GPG Signing

Enable GPG-signed commits and tags:

```toml
[vcs]
software = "Git"
force_sign = true
```

### Tag Templates

Customize Git tag naming:

```toml
[vcs]
tag_template = "v{{version}}"     # v1.0.0
# or
tag_template = "release-{{version}}"  # release-1.0.0
# or
tag_template = "{{version}}"      # 1.0.0 (default)
```

## Documentation

- [Architecture Overview](docs/ARCHITECTURE.md) - Technical design and code structure
- [Configuration Guide](docs/CONFIGURATION.md) - Detailed configuration reference
- [Contributing Guidelines](CONTRIBUTING.md) - How to contribute
- [Security Policy](SECURITY.md) - Reporting vulnerabilities
- [Changelog](CHANGELOG.md) - Release history

## Requirements

- **Git** - Required for version control operations
- **Rust 1.70+** (for building from source)
- Package manager CLI (optional):
  - `cargo` for Rust projects
  - `npm`/`yarn`/`pnpm` for Node.js projects
  - `mvn` for Maven projects (optional, only for hooks)
  - `gradle` for Gradle projects (optional, only for hooks)

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting a pull request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- CLI powered by [clap](https://github.com/clap-rs/clap)
- Semantic versioning via [semver](https://github.com/dtolnay/semver)

---

Made with care by [dghilardi](https://github.com/dghilardi)
