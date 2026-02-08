use std::fs;
use std::path::Path;
use std::process::Command;

use panrelease::engine;
use panrelease::system::NativeSystem;

/// Run a git command in the given directory, panicking on failure.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("git {} failed: {stderr}", args.join(" "));
    }
}

/// Run a git command and return stdout.
fn git_output(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Set up a temp git repo with an initial tagged version and a Cargo.toml.
fn setup_repo(dir: &Path) {
    git(dir, &["init"]);
    git(dir, &["config", "user.email", "test@test.com"]);
    git(dir, &["config", "user.name", "Test User"]);

    // Create Cargo.toml at version 0.1.0
    fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    // Create a CHANGELOG.md with the initial release
    fs::write(
        dir.join("CHANGELOG.md"),
        "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] 2025-01-01\n\n### Added\n- Initial release\n",
    )
    .unwrap();

    // Create .panproject.toml with changelog from_commits enabled
    fs::write(
        dir.join(".panproject.toml"),
        r#"[vcs]
software = "Git"

[changelog]
from_commits = true
commit_format = "conventional"
include_scope = true
include_unmatched = false

[modules.root]
path = "."
packageManager = "Cargo"
main = true
"#,
    )
    .unwrap();

    // Create a minimal src/lib.rs so cargo check works
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "").unwrap();

    // Initial commit and tag
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "chore: initial commit"]);
    git(dir, &["tag", "0.1.0"]);
}

/// Run panrelease release in the given directory.
fn panrelease(dir: &Path, bump: &str) {
    engine::run::<_, _, NativeSystem>(vec![
        "panrelease".to_string(),
        "--path".to_string(),
        dir.to_string_lossy().to_string(),
        "release".to_string(),
        bump.to_string(),
    ])
    .expect("panrelease release failed");
}

#[test]
fn changelog_populated_from_conventional_commits() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_repo(dir);

    // --- Commit 1: feat with scope (should appear in Added) ---
    fs::write(dir.join("src/lib.rs"), "pub fn login() {}\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feat(auth): add login endpoint"]);

    // --- Commit 2: fix (should appear in Fixed) ---
    fs::write(dir.join("src/lib.rs"), "pub fn login() { /* fixed */ }\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "fix: resolve crash on empty input"]);

    // --- Commit 3: docs (should be ignored) ---
    fs::write(dir.join("README.md"), "# Test Project\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "docs: update readme"]);

    // --- Commit 4: chore (should be ignored) ---
    fs::write(dir.join(".gitignore"), "target/\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "chore: add gitignore"]);

    // --- Commit 5: non-conventional message (should be ignored with include_unmatched=false) ---
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn login() { /* fixed */ }\npub fn misc() {}\n",
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "WIP random stuff"]);

    // --- Commit 6: breaking change (should appear in Changed with [BREAKING]) ---
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn login_v2() {}\npub fn misc() {}\n",
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feat!: rewrite login API"]);

    // --- Commit 7: refactor (should appear in Changed) ---
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn login_v2() { /* clean */ }\npub fn misc() {}\n",
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "refactor: simplify auth handler"]);

    // Run panrelease
    panrelease(dir, "patch");

    // Read resulting changelog
    let changelog = fs::read_to_string(dir.join("CHANGELOG.md")).unwrap();

    // --- Assertions ---

    // New version section should exist
    assert!(
        changelog.contains("## [0.1.1]"),
        "Should contain new version header.\nChangelog:\n{changelog}"
    );

    // [Unreleased] should still be at the top
    assert!(
        changelog.contains("## [Unreleased]"),
        "Should preserve [Unreleased] section.\nChangelog:\n{changelog}"
    );

    // feat(auth) -> Added section with scope
    assert!(
        changelog.contains("### Added"),
        "Should contain Added section.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- (auth) Add login endpoint"),
        "Should contain feat commit with scope.\nChangelog:\n{changelog}"
    );

    // fix -> Fixed section
    assert!(
        changelog.contains("### Fixed"),
        "Should contain Fixed section.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Resolve crash on empty input"),
        "Should contain fix commit.\nChangelog:\n{changelog}"
    );

    // Changed section with breaking + refactor
    assert!(
        changelog.contains("### Changed"),
        "Should contain Changed section.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- [BREAKING] Rewrite login API"),
        "Should contain breaking change with prefix.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Simplify auth handler"),
        "Should contain refactor commit.\nChangelog:\n{changelog}"
    );

    // docs and chore should NOT appear
    assert!(
        !changelog.contains("Update readme"),
        "docs commits should be ignored.\nChangelog:\n{changelog}"
    );
    assert!(
        !changelog.contains("Add gitignore"),
        "chore commits should be ignored.\nChangelog:\n{changelog}"
    );

    // Non-conventional message should NOT appear (include_unmatched=false)
    assert!(
        !changelog.contains("WIP"),
        "Non-matching commits should be ignored.\nChangelog:\n{changelog}"
    );
    assert!(
        !changelog.contains("### Other"),
        "Other section should not exist.\nChangelog:\n{changelog}"
    );

    // Original release should still be there
    assert!(
        changelog.contains("## [0.1.0]"),
        "Original version should be preserved.\nChangelog:\n{changelog}"
    );

    // Verify git tag was created
    let tags = git_output(dir, &["tag", "-l"]);
    assert!(tags.contains("0.1.1"), "Tag 0.1.1 should be created");

    // Verify Cargo.toml was updated
    let cargo_toml = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo_toml.contains("version = \"0.1.1\""),
        "Cargo.toml version should be updated"
    );
}

#[test]
fn changelog_preserves_manual_entries_and_deduplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_repo(dir);

    // --- Commit 1: feat that also manually updates the changelog ---
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn register() {}\n",
    )
    .unwrap();
    fs::write(
        dir.join("CHANGELOG.md"),
        "# Changelog\n\n## [Unreleased]\n\n### Added\n- User registration feature\n\n## [0.1.0] 2025-01-01\n\n### Added\n- Initial release\n",
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feat: add user registration"]);

    // --- Commit 2: another feat that does NOT touch changelog ---
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn register() {}\npub fn logout() {}\n",
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feat: add logout endpoint"]);

    // Run panrelease
    panrelease(dir, "minor");

    let changelog = fs::read_to_string(dir.join("CHANGELOG.md")).unwrap();

    // Manual entry should be preserved
    assert!(
        changelog.contains("- User registration feature"),
        "Manual entry should be preserved.\nChangelog:\n{changelog}"
    );

    // The commit that modified the changelog should NOT create a duplicate entry
    // (git blame dedup should prevent "Add user registration" from appearing)
    let registration_count = changelog.matches("registration").count();
    assert_eq!(
        1, registration_count,
        "Should have exactly one registration entry (no duplicate from commit).\nChangelog:\n{changelog}"
    );

    // The commit that did NOT touch the changelog should appear
    assert!(
        changelog.contains("- Add logout endpoint"),
        "Non-changelog commit should generate an entry.\nChangelog:\n{changelog}"
    );

    // New version should be 0.2.0
    assert!(
        changelog.contains("## [0.2.0]"),
        "Should contain new version.\nChangelog:\n{changelog}"
    );
}

#[test]
fn changelog_include_unmatched_in_other_section() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_repo(dir);

    // Override config to enable include_unmatched
    fs::write(
        dir.join(".panproject.toml"),
        r#"[vcs]
software = "Git"

[changelog]
from_commits = true
commit_format = "conventional"
include_scope = true
include_unmatched = true

[modules.root]
path = "."
packageManager = "Cargo"
main = true
"#,
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "chore: update config"]);

    // --- Commit with non-conventional message ---
    fs::write(dir.join("src/lib.rs"), "pub fn something() {}\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "implement the thing"]);

    // --- A proper conventional commit ---
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn something() {}\npub fn fixed() {}\n",
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "fix: handle null pointer"]);

    panrelease(dir, "patch");

    let changelog = fs::read_to_string(dir.join("CHANGELOG.md")).unwrap();

    // Non-conventional commit should appear in Other
    assert!(
        changelog.contains("### Other"),
        "Should contain Other section.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Implement the thing"),
        "Unmatched commit should appear capitalized.\nChangelog:\n{changelog}"
    );

    // Conventional commit should also appear
    assert!(
        changelog.contains("### Fixed"),
        "Should contain Fixed section.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Handle null pointer"),
        "Fix commit should appear.\nChangelog:\n{changelog}"
    );

    // chore commit should still be ignored (it's a known-ignored type, not "unmatched")
    assert!(
        !changelog.contains("Update config"),
        "chore commits should still be ignored even with include_unmatched.\nChangelog:\n{changelog}"
    );
}

#[test]
fn changelog_auto_detect_picks_gitmoji() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_repo(dir);

    // Override config to use auto detection
    fs::write(
        dir.join(".panproject.toml"),
        r#"[vcs]
software = "Git"

[changelog]
from_commits = true
commit_format = "auto"

[modules.root]
path = "."
packageManager = "Cargo"
main = true
"#,
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", ":wrench: update config"]);

    // Gitmoji commits (majority)
    fs::write(dir.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", ":sparkles: add feature A"]);

    fs::write(dir.join("src/lib.rs"), "pub fn a() { /* fix */ }\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", ":bug: fix crash in feature A"]);

    fs::write(
        dir.join("src/lib.rs"),
        "pub fn a() { /* fix */ }\npub fn b() {}\n",
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", ":fire: remove deprecated code"]);

    panrelease(dir, "minor");

    let changelog = fs::read_to_string(dir.join("CHANGELOG.md")).unwrap();

    // Gitmoji should be detected and parsed
    assert!(
        changelog.contains("### Added"),
        "Should detect gitmoji and populate Added.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Add feature A"),
        "Sparkles should map to Added.\nChangelog:\n{changelog}"
    );

    assert!(
        changelog.contains("### Fixed"),
        "Should contain Fixed.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Fix crash in feature A"),
        "Bug should map to Fixed.\nChangelog:\n{changelog}"
    );

    assert!(
        changelog.contains("### Removed"),
        "Should contain Removed.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Remove deprecated code"),
        "Fire should map to Removed.\nChangelog:\n{changelog}"
    );
}

#[test]
fn changelog_disabled_does_not_populate() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_repo(dir);

    // Override config: from_commits = false
    fs::write(
        dir.join(".panproject.toml"),
        r#"[vcs]
software = "Git"

[changelog]
from_commits = false

[modules.root]
path = "."
packageManager = "Cargo"
main = true
"#,
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "chore: update config"]);

    fs::write(dir.join("src/lib.rs"), "pub fn feature() {}\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feat: add big feature"]);

    panrelease(dir, "patch");

    let changelog = fs::read_to_string(dir.join("CHANGELOG.md")).unwrap();

    // Version header should exist
    assert!(
        changelog.contains("## [0.1.1]"),
        "Version header should be present.\nChangelog:\n{changelog}"
    );

    // The new version section should be empty (no auto-populated entries)
    assert!(
        !changelog.contains("Add big feature"),
        "Should NOT contain commit entries when from_commits=false.\nChangelog:\n{changelog}"
    );

    // Extract the text between [0.1.1] and [0.1.0] — it should have no ### sections
    let v011_pos = changelog.find("## [0.1.1]").unwrap();
    let v010_pos = changelog.find("## [0.1.0]").unwrap();
    let new_version_section = &changelog[v011_pos..v010_pos];
    assert!(
        !new_version_section.contains("### "),
        "New version section should have no subsections when from_commits=false.\nSection:\n{new_version_section}"
    );
}

#[test]
fn changelog_scope_excluded_when_configured() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_repo(dir);

    // Override config: include_scope = false
    fs::write(
        dir.join(".panproject.toml"),
        r#"[vcs]
software = "Git"

[changelog]
from_commits = true
commit_format = "conventional"
include_scope = false

[modules.root]
path = "."
packageManager = "Cargo"
main = true
"#,
    )
    .unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "chore: update config"]);

    fs::write(dir.join("src/lib.rs"), "pub fn auth() {}\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feat(auth): add SSO support"]);

    panrelease(dir, "patch");

    let changelog = fs::read_to_string(dir.join("CHANGELOG.md")).unwrap();

    // Entry should NOT include the scope
    assert!(
        !changelog.contains("(auth)"),
        "Scope should not appear when include_scope=false.\nChangelog:\n{changelog}"
    );
    assert!(
        changelog.contains("- Add SSO support"),
        "Description should still appear.\nChangelog:\n{changelog}"
    );
}
