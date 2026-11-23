---
name: panrelease
description: Release and version software with panrelease. Use when the repository contains a `.panproject.toml` and the task involves bumping a version, cutting a release, creating a release tag, producing a development build version, or updating CHANGELOG.md for a release.
---

# Releasing with panrelease

Panrelease bumps the version of every module declared in `.panproject.toml` (Cargo, npm, Maven, Gradle, or custom JSON/XML/TOML files), updates `CHANGELOG.md`, runs the configured hooks, then creates one local Git commit and one tag. It never pushes.

## Rules

- Never edit version fields (`Cargo.toml`, `package.json`, `pom.xml`, `gradle.properties`, ...) by hand, and never create release tags with `git tag`. Use `panrelease release`.
- Never push commits or tags created by a release unless the user explicitly asks for it.
- Only release when the user asked for a release, or when the project instructions say to release at the end of a task.
- Read `.panproject.toml` first: strict mode (`[vcs.strict]`), changelog generation (`[changelog]`), hooks and modules change what a release does.

## Commands

```bash
panrelease show                      # current version
panrelease show --format docker-tag  # version with '+' replaced by '_'
panrelease release patch             # 1.2.3 -> 1.2.4
panrelease release minor             # 1.2.3 -> 1.3.0
panrelease release major             # 1.2.3 -> 2.0.0
panrelease release post              # 1.2.3 -> 1.2.3+<slug>.r1 (development build)
panrelease release 2.0.0             # explicit version
panrelease --path <dir> <command>    # run against another project root
```

## Release workflow

1. Make sure all work is committed: `release` fails if the working tree has uncommitted changes.
2. Update the changelog:
   - If `[changelog] from_commits = true`, entries are generated from commit messages since the last version tag (Conventional Commits or Gitmoji). Make sure commit messages follow that format; manual entries in `## [Unreleased]` are kept.
   - Otherwise, add concise entries under `## [Unreleased]` in `CHANGELOG.md` (Keep a Changelog sections: Added, Changed, Deprecated, Removed, Fixed, Security) and commit them.
3. Choose the level (see below) and run `panrelease release <level>`.
4. Report the new version (`panrelease show`) and the created tag to the user.

Panrelease moves the `## [Unreleased]` content under a new `## [<version>] <date>` heading, runs the `after_rel` hooks of each module (they can be slow, e.g. builds and tests), commits with the version as message and tags it using `tag_template`.

## Choosing the level

- `major`: breaking changes to a public API, CLI, file format, or behavior users depend on.
- `minor`: new backward-compatible features.
- `patch`: backward-compatible fixes only.
- `post`: intermediate development or staging builds that must not advance the official version.
- Below `1.0.0`, prefer `minor` for breaking changes unless the project says otherwise.
- When the impact is unclear, propose a level and ask the user before releasing.

Post-releases use semver build metadata (`1.2.3+login.r2`). Do not publish them to public registries: crates.io rejects them and npm strips the metadata.

## Strict mode

When `.panproject.toml` contains `[vcs.strict] mainline = "<branch>"`:

- On the mainline branch only `major`, `minor`, `patch` or an explicit version are allowed.
- On `feat/*`, `feature/*`, `fix/*` and `hotfix/*` branches only `post` is allowed; the slug comes from the branch name (`feat/login` -> `+login.rN`, `fix/timeout` -> `+fix-timeout.rN`).
- Other branch names and detached HEAD are rejected.
- If panrelease reports that the base version does not match the mainline tag, rebase the branch on the latest mainline instead of forcing a version.

## When a release fails

- Read the error: panrelease reports dirty files, strict-mode violations and failing hooks explicitly.
- A failure before the commit can leave modified files: inspect them with `git status` and restore them with `git restore` only after checking with the user.
- To undo a release that was not pushed, ask the user before running `git tag -d <tag>` and `git reset --hard HEAD~1`.
