# Contributing to Panrelease

First off, thank you for considering contributing to Panrelease! It's people like you that make Panrelease such a great tool.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Coding Guidelines](#coding-guidelines)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

### Prerequisites

- **Rust** (stable, 1.70+): Install via [rustup](https://rustup.rs/)
- **Git**: For version control
- **wasm-pack** (optional): For building the WebAssembly module

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/panrelease.git
   cd panrelease
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/dghilardi/panrelease.git
   ```

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the existing issues to avoid duplicates. When creating a bug report, include as many details as possible:

- **Use a clear and descriptive title**
- **Describe the exact steps to reproduce the problem**
- **Provide specific examples** (configuration files, command output)
- **Describe the behavior you observed and what you expected**
- **Include your environment details** (OS, Rust version, panrelease version)

### Suggesting Enhancements

Enhancement suggestions are welcome! Please provide:

- **A clear and descriptive title**
- **A detailed description of the proposed feature**
- **Explain why this enhancement would be useful**
- **List any alternatives you've considered**

### Your First Code Contribution

Unsure where to begin? Look for issues labeled:

- `good first issue` - Simple issues suitable for newcomers
- `help wanted` - Issues that need community assistance
- `documentation` - Documentation improvements

### Adding Package Manager Support

If you'd like to add support for a new package manager:

1. Create a new file in `src/package/` (e.g., `newpm.rs`)
2. Implement the `PanPackage` trait
3. Add the new package manager to the enum in `src/package/mod.rs`
4. Update the configuration parsing in `src/conf/loader.rs`
5. Add tests for the new package manager
6. Update documentation (README.md, docs/CONFIGURATION.md)

## Development Setup

### Building the Project

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- patch
```

### Building WebAssembly Module

```bash
# Install wasm-pack if not already installed
cargo install wasm-pack

# Build WASM module
./build.sh
```

### Project Structure

```
src/
├── bin/panrelease.rs   # CLI entry point
├── lib.rs              # Library entry point (WASM)
├── engine.rs           # Main orchestration logic
├── args.rs             # CLI argument parsing
├── conf/               # Configuration loading
├── git/                # Git integration
├── package/            # Package manager implementations
├── parser/             # File format parsers (JSON, XML)
├── project/            # Project and module abstractions
├── system/             # Filesystem abstraction layer
└── utils.rs            # Utility functions
```

### Running Specific Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Run tests for a specific module
cargo test package::cargo
```

## Coding Guidelines

### Rust Style

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- Use `rustfmt` for formatting: `cargo fmt`
- Use `clippy` for linting: `cargo clippy`
- All public items should have documentation comments

### Code Organization

- **Keep functions small and focused** - Each function should do one thing well
- **Use meaningful names** - Variables, functions, and types should be self-documenting
- **Handle errors explicitly** - Use `Result` and `Option` appropriately, avoid `unwrap()` in library code
- **Write tests** - New features should include tests

### Documentation

- Document public APIs with `///` doc comments
- Include examples in documentation where helpful
- Update relevant documentation files when adding features

### Error Handling

```rust
// Good: Explicit error handling
fn read_config(path: &Path) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ConfigError::ReadError(e))?;
    // ...
}

// Avoid: Panicking in library code
fn read_config(path: &Path) -> Config {
    let content = fs::read_to_string(path).unwrap(); // Don't do this
    // ...
}
```

### Testing

- Write unit tests for new functionality
- Place tests in the same file using `#[cfg(test)]` module
- Use descriptive test names that explain what's being tested

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_patch_increments_patch_version() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = bump_patch(&version);
        assert_eq!(bumped.to_string(), "1.2.4");
    }
}
```

## Commit Guidelines

### Commit Message Format

We follow a simplified conventional commits format:

```
<type>: <description>

[optional body]

[optional footer]
```

### Types

- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, missing semicolons, etc.)
- `refactor`: Code changes that neither fix bugs nor add features
- `test`: Adding or updating tests
- `chore`: Maintenance tasks, dependency updates

### Examples

```
feat: Add support for Gradle package manager

Add GradlePackage implementation that reads and updates
gradle.properties files.

Closes #42
```

```
fix: Handle missing CHANGELOG.md gracefully

Previously, panrelease would panic if no CHANGELOG.md existed.
Now it creates one automatically.
```

```
docs: Update README with multi-module examples
```

### Guidelines

- Use the imperative mood ("Add feature" not "Added feature")
- Keep the subject line under 72 characters
- Reference issues when applicable (`Closes #123`, `Fixes #456`)

## Pull Request Process

### Before Submitting

1. **Ensure your code compiles**: `cargo build`
2. **Run the test suite**: `cargo test`
3. **Format your code**: `cargo fmt`
4. **Run clippy**: `cargo clippy`
5. **Update documentation** if you've added or changed functionality

### Submitting a Pull Request

1. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes** with clear, focused commits

3. **Push your branch** to your fork:
   ```bash
   git push origin feature/my-feature
   ```

4. **Open a Pull Request** against `main` with:
   - A clear title describing the change
   - A description of what the PR does and why
   - Reference to any related issues
   - Screenshots/examples if applicable

### PR Review Process

- A maintainer will review your PR
- Address any requested changes
- Once approved, a maintainer will merge the PR

### After Your PR is Merged

- Delete your feature branch
- Pull the latest `main`:
  ```bash
  git checkout main
  git pull upstream main
  ```

## Issue Guidelines

### Creating Issues

- **Search first**: Check if the issue already exists
- **One issue per report**: Don't combine multiple bugs or feature requests
- **Use templates**: Fill out the provided issue templates completely

### Issue Labels

| Label | Description |
|-------|-------------|
| `bug` | Something isn't working |
| `enhancement` | New feature or request |
| `documentation` | Documentation improvements |
| `good first issue` | Good for newcomers |
| `help wanted` | Extra attention is needed |
| `wontfix` | This will not be worked on |

## Questions?

Feel free to open an issue for questions or reach out to the maintainers.

Thank you for contributing to Panrelease!
