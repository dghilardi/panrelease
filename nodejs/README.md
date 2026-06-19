# Panrelease (Node.js)

[![npm version](https://img.shields.io/npm/v/panrelease.svg?style=flat-square)](https://www.npmjs.com/package/panrelease)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)

**Panrelease** is a versatile release automation tool that manages version bumping, changelog updates, and Git operations. This package provides the Node.js implementation of Panrelease, powered by WebAssembly, allowing it to run seamlessly in any Node.js environment without requiring a Rust installation.

## Features

- **Semantic Versioning** - Automatic major, minor, patch, and post-release version bumping.
- **Node.js Native Support** - Direct support for `package.json` and lockfiles (`package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`).
- **Changelog Automation** - Updates `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/) format.
- **Git Integration** - Commits version changes, creates tags, and supports GPG signing.
- **Monorepo Friendly** - Can manage multiple modules in a single project.
- **WebAssembly Powered** - Fast and consistent execution across different platforms.

## Installation

Install as a development dependency in your project:

### npm
```bash
npm install --save-dev panrelease
```

### yarn
```bash
yarn add --dev panrelease
```

### pnpm
```bash
pnpm add --save-dev panrelease
```

## Quick Start

### 1. Initialize Panrelease

Run `panrelease init` in your repository root to auto-detect the package manager and generate `.panproject.toml`:

```bash
npx panrelease init
# or interactively:
npx panrelease init --interactive
```

Alternatively, create `.panproject.toml` manually:

```toml
[vcs]
software = "Git"

[modules.root]
path = "."
packageManager = "Npm"
main = true
```

### 2. Add a Script to package.json

```json
{
  "scripts": {
    "release": "panrelease"
  }
}
```

### 3. Run a Release

```bash
# Release a new patch version (e.g., 1.0.0 -> 1.0.1)
npm run release -- release patch

# Or run directly via npx
npx panrelease release minor
```

## Commands

```
panrelease [--path <PATH>] <SUBCOMMAND>

Subcommands:
  init [--interactive]          Initialize .panproject.toml
  release <LEVEL|VERSION>       Bump and release a new version
  show [--format <FORMAT>]      Print the current project version
```

### Release Levels

| Level | Description | Example |
|-------|-------------|---------|
| `patch` | Increment patch version | `1.0.0` -> `1.0.1` |
| `minor` | Increment minor version | `1.0.0` -> `1.1.0` |
| `major` | Increment major version | `1.0.0` -> `2.0.0` |
| `post`  | Create post-release (build metadata) | `1.0.0` -> `1.0.0+dev.r1` |
| `X.Y.Z` | Set explicit version | `1.0.0` -> `2.1.0` |

> **Note:** The `post` bump type uses semver build metadata in a non-standard way. See the [main README](https://github.com/dghilardi/panrelease#readme) for a full explanation of the implications and intended use cases.

## Documentation

For full configuration options, advanced features like **hooks**, **strict mode**, and **changelog from commits**, please refer to the [Main Repository Documentation](https://github.com/dghilardi/panrelease#readme).

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/dghilardi/panrelease/blob/main/LICENSE) file for details.
