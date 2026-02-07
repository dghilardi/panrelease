# Configuration Guide

Panrelease is configured through a `.panproject.toml` file placed at the root of your Git repository. This document covers all available configuration options with examples.

## Table of Contents

- [File Location](#file-location)
- [Minimal Configuration](#minimal-configuration)
- [Full Configuration Reference](#full-configuration-reference)
- [VCS Configuration](#vcs-configuration)
- [Module Configuration](#module-configuration)
- [Hooks](#hooks)
- [Package Manager Details](#package-manager-details)
- [Examples](#examples)
- [Auto-Detection](#auto-detection)

## File Location

The configuration file must be named `.panproject.toml` and placed at the root of your Git repository (same level as `.git/`).

```
my-project/
├── .git/
├── .panproject.toml    <-- here
├── src/
└── ...
```

## Minimal Configuration

If no `.panproject.toml` exists, Panrelease will auto-detect the package manager based on manifest files in the current directory. However, for explicit control, create a minimal config:

```toml
[modules.main]
path = "."
packageManager = "Cargo"
main = true
```

## Full Configuration Reference

```toml
# Version Control System configuration
[vcs]
software = "Git"                    # VCS type (currently only "Git" is supported)
force_sign = false                  # Require GPG-signed commits and tags
tag_template = "{{version}}"        # Template for Git tag names

# Module definitions (one or more)
[modules.my-module]
path = "."                          # Path relative to project root
packageManager = "Cargo"            # Package manager type
main = true                         # Whether this is the primary module

# Post-release hooks for a module
[modules.my-module.hooks.after_rel]
build = ["cargo", "build", "--release"]
test = ["cargo", "test"]
```

## VCS Configuration

The `[vcs]` section configures version control behavior.

### `software` (string)

The version control system to use. Currently only `"Git"` is supported.

```toml
[vcs]
software = "Git"
```

**Default:** `"Git"`

### `force_sign` (boolean)

When `true`, Panrelease will use `git commit -S` and `git tag -s` to create GPG-signed commits and tags. Requires GPG to be configured in your Git settings.

```toml
[vcs]
force_sign = true
```

**Default:** `false`

### `tag_template` (string)

A template string for Git tag names. Use `{{version}}` as a placeholder for the version number.

```toml
[vcs]
tag_template = "v{{version}}"
```

| Template | Version | Tag |
|----------|---------|-----|
| `{{version}}` | 1.0.0 | `1.0.0` |
| `v{{version}}` | 1.0.0 | `v1.0.0` |
| `release-{{version}}` | 1.0.0 | `release-1.0.0` |
| `myapp/{{version}}` | 1.0.0 | `myapp/1.0.0` |

**Default:** `"{{version}}"`

## Module Configuration

Each `[modules.<name>]` section defines a package/module in your project.

### `path` (string, required)

The path to the module directory, relative to the project root.

```toml
[modules.core]
path = "packages/core"
```

Use `"."` for the project root:

```toml
[modules.main]
path = "."
```

### `packageManager` (string, required)

The package manager used by this module. Determines which manifest file is read and how versions are updated.

| Value | Manifest File | Description |
|-------|--------------|-------------|
| `"Cargo"` | `Cargo.toml` | Rust projects using Cargo |
| `"Npm"` | `package.json` | Node.js projects using npm/yarn/pnpm |
| `"Maven"` | `pom.xml` | Java projects using Maven |
| `"Gradle"` | `gradle.properties` | Java/Kotlin projects using Gradle |

```toml
[modules.backend]
packageManager = "Maven"
```

### `main` (boolean)

Marks this module as the primary module. The main module is used to extract the current project version before bumping. In multi-module projects, exactly one module must be marked as `main = true`.

```toml
[modules.core]
main = true
```

**Default:** `false`

**Rules:**
- If only one module is defined, it is automatically treated as the main module
- If multiple modules exist, exactly one must have `main = true`
- Having zero or multiple main modules in a multi-module project will cause an error

## Hooks

Hooks allow you to run commands after a release. They are defined per module under `[modules.<name>.hooks.after_rel]`.

### Structure

```toml
[modules.myapp.hooks.after_rel]
hook_name = ["command", "arg1", "arg2"]
```

Each hook is a named entry with a command specified as an array of strings (the command and its arguments).

### Execution

- Hooks run **after** the version has been updated in the manifest file
- Hooks run **before** the Git commit
- Hooks execute in alphabetical order by name
- Hooks run in the module's directory
- If a hook fails, the release process stops

### Examples

```toml
# Build after version bump
[modules.myapp.hooks.after_rel]
build = ["cargo", "build", "--release"]

# Run tests
[modules.myapp.hooks.after_rel]
test = ["cargo", "test"]

# Publish to registry
[modules.myapp.hooks.after_rel]
publish = ["cargo", "publish"]

# Multiple hooks (execute in alphabetical order)
[modules.myapp.hooks.after_rel]
01_build = ["cargo", "build", "--release"]
02_test = ["cargo", "test"]
03_publish = ["cargo", "publish"]
```

**Tip:** Prefix hook names with numbers to control execution order, since they run in alphabetical order.

## Package Manager Details

### Cargo

Reads and writes version in `Cargo.toml` under the `[package]` section. After updating the version, runs `cargo check` to regenerate `Cargo.lock`.

```toml
# Cargo.toml structure expected:
# [package]
# version = "1.0.0"
```

### Npm

Reads and writes version in `package.json`. Automatically detects and updates the appropriate lockfile:

| Lockfile | Tool |
|----------|------|
| `package-lock.json` | npm |
| `yarn.lock` | yarn |
| `pnpm-lock.yaml` | pnpm |

The lockfile is updated by running the corresponding package manager's install command.

### Maven

Reads and writes version in `pom.xml`. Supports two version patterns:

1. **Direct version**: `<version>1.0.0</version>` under `<project>`
2. **Property-based version**: Version stored in `<properties>` and referenced via `${property.name}` in `<version>`

### Gradle

Reads and writes the `version` property in `gradle.properties`:

```properties
# gradle.properties
version=1.0.0
```

## Examples

### Single Rust Project

```toml
[modules.main]
path = "."
packageManager = "Cargo"
main = true

[modules.main.hooks.after_rel]
test = ["cargo", "test"]
```

### Node.js Project with Prefixed Tags

```toml
[vcs]
software = "Git"
tag_template = "v{{version}}"

[modules.app]
path = "."
packageManager = "Npm"
main = true
```

### Java Maven Project with Signed Commits

```toml
[vcs]
software = "Git"
force_sign = true
tag_template = "v{{version}}"

[modules.api]
path = "."
packageManager = "Maven"
main = true
```

### Monorepo with Multiple Languages

```toml
[vcs]
software = "Git"
tag_template = "v{{version}}"

[modules.rust-core]
path = "crates/core"
packageManager = "Cargo"
main = true

[modules.rust-cli]
path = "crates/cli"
packageManager = "Cargo"

[modules.node-sdk]
path = "packages/sdk"
packageManager = "Npm"

[modules.java-client]
path = "clients/java"
packageManager = "Maven"

[modules.rust-core.hooks.after_rel]
test = ["cargo", "test"]

[modules.node-sdk.hooks.after_rel]
build = ["npm", "run", "build"]
test = ["npm", "test"]
```

### Gradle + npm Hybrid Project

```toml
[vcs]
software = "Git"

[modules.backend]
path = "backend"
packageManager = "Gradle"
main = true

[modules.frontend]
path = "frontend"
packageManager = "Npm"
```

## Auto-Detection

When no `.panproject.toml` is present, Panrelease attempts to auto-detect the package manager by checking for manifest files in this order:

1. `Cargo.toml` → Cargo
2. `pom.xml` → Maven
3. `package.json` → Npm
4. `gradle.properties` → Gradle

Auto-detection only works for single-module projects. For multi-module projects, a configuration file is required.
