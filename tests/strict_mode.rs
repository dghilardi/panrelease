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
        panic!("git {:?} failed: {stderr}", args);
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

/// Try to run panrelease, returning Ok or Err.
fn panrelease(dir: &Path, bump: &str) -> Result<(), anyhow::Error> {
    engine::run::<_, _, NativeSystem>(vec![
        "panrelease".to_string(),
        "--path".to_string(),
        dir.to_string_lossy().to_string(),
        "release".to_string(),
        bump.to_string(),
    ])
}

/// Set up a temp git repo on main with a tagged v0.1.0 and strict mode enabled.
fn setup_strict_repo(dir: &Path) {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "test@test.com"]);
    git(dir, &["config", "user.name", "Test User"]);

    fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::write(
        dir.join("CHANGELOG.md"),
        "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] 2025-01-01\n",
    )
    .unwrap();

    fs::write(
        dir.join(".panproject.toml"),
        r#"[vcs]
software = "Git"

[vcs.strict]
mainline = "main"

[modules.root]
path = "."
packageManager = "Cargo"
main = true
"#,
    )
    .unwrap();

    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "").unwrap();

    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "initial commit"]);
    git(dir, &["tag", "0.1.0"]);
}

// =====================================================================
// Mainline tests
// =====================================================================

#[test]
fn strict_mainline_patch_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    fs::write(dir.join("src/lib.rs"), "// patch\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "fix something"]);

    panrelease(dir, "patch").expect("patch on mainline should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("version = \"0.1.1\""));

    let tags = git_output(dir, &["tag", "-l"]);
    assert!(tags.contains("0.1.1"));
}

#[test]
fn strict_mainline_minor_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    fs::write(dir.join("src/lib.rs"), "// minor\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "add feature"]);

    panrelease(dir, "minor").expect("minor on mainline should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("version = \"0.2.0\""));
}

#[test]
fn strict_mainline_major_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    fs::write(dir.join("src/lib.rs"), "// major\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "breaking change"]);

    panrelease(dir, "major").expect("major on mainline should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("version = \"1.0.0\""));
}

#[test]
fn strict_mainline_post_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    fs::write(dir.join("src/lib.rs"), "// post\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "wip"]);

    let result = panrelease(dir, "post");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("cannot release a post-version"),
        "Expected post-version error, got: {err}"
    );
}

// =====================================================================
// Feature branch tests
// =====================================================================

#[test]
fn strict_feature_post_succeeds_with_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "feat/user-registration"]);

    fs::write(dir.join("src/lib.rs"), "pub fn register() {}\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "add registration"]);

    panrelease(dir, "post").expect("post on feature branch should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.1.0+user-registration.r1\""),
        "Version should have slug from branch name. Cargo.toml:\n{cargo}"
    );

    let tags = git_output(dir, &["tag", "-l"]);
    assert!(
        tags.contains("0.1.0+user-registration.r1"),
        "Tag should include slug. Tags:\n{tags}"
    );
}

#[test]
fn strict_feature_patch_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "feat/my-feature"]);

    fs::write(dir.join("src/lib.rs"), "// feature\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "work on feature"]);

    let result = panrelease(dir, "patch");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("can only produce post-releases"),
        "Expected post-only error, got: {err}"
    );
}

#[test]
fn strict_feature_minor_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "feature/something"]);

    fs::write(dir.join("src/lib.rs"), "// feat\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "work"]);

    let result = panrelease(dir, "minor");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("can only produce post-releases"),
        "Expected post-only error, got: {err}"
    );
}

#[test]
fn strict_feature_multiple_post_releases_increment() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "feat/incremental"]);

    // First post-release
    fs::write(dir.join("src/lib.rs"), "// r1\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "first iteration"]);

    panrelease(dir, "post").expect("first post should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.1.0+incremental.r1\""),
        "First post should be r1. Cargo.toml:\n{cargo}"
    );

    // Second post-release
    fs::write(dir.join("src/lib.rs"), "// r2\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "second iteration"]);

    panrelease(dir, "post").expect("second post should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.1.0+incremental.r2\""),
        "Second post should be r2. Cargo.toml:\n{cargo}"
    );

    // Third post-release
    fs::write(dir.join("src/lib.rs"), "// r3\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "third iteration"]);

    panrelease(dir, "post").expect("third post should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.1.0+incremental.r3\""),
        "Third post should be r3. Cargo.toml:\n{cargo}"
    );
}

// =====================================================================
// Hotfix branch tests
// =====================================================================

#[test]
fn strict_hotfix_post_succeeds_with_fix_prefix_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "hotfix/timeout-error"]);

    fs::write(dir.join("src/lib.rs"), "// hotfix\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "fix timeout"]);

    panrelease(dir, "post").expect("post on hotfix branch should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.1.0+fix-timeout-error.r1\""),
        "Hotfix slug should have fix- prefix. Cargo.toml:\n{cargo}"
    );
}

#[test]
fn strict_fix_branch_post_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "fix/null-pointer"]);

    fs::write(dir.join("src/lib.rs"), "// fix\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "fix npe"]);

    panrelease(dir, "post").expect("post on fix/ branch should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.1.0+fix-null-pointer.r1\""),
        "fix/ branch slug should have fix- prefix. Cargo.toml:\n{cargo}"
    );
}

#[test]
fn strict_hotfix_patch_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "hotfix/critical"]);

    fs::write(dir.join("src/lib.rs"), "// hotfix\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "fix critical"]);

    let result = panrelease(dir, "patch");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("can only produce post-releases"),
        "Expected post-only error, got: {err}"
    );
}

// =====================================================================
// Unknown branch
// =====================================================================

#[test]
fn strict_unknown_branch_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "develop"]);

    fs::write(dir.join("src/lib.rs"), "// dev\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "work on develop"]);

    let result = panrelease(dir, "patch");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("does not match any known pattern"),
        "Expected unknown branch error, got: {err}"
    );
}

#[test]
fn strict_unknown_branch_post_also_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    git(dir, &["checkout", "-b", "release/1.0"]);

    fs::write(dir.join("src/lib.rs"), "// rel\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "release prep"]);

    let result = panrelease(dir, "post");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("does not match any known pattern"),
        "Expected unknown branch error, got: {err}"
    );
}

// =====================================================================
// Version base validation
// =====================================================================

#[test]
fn strict_feature_after_mainline_bump_uses_new_base() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    // Bump mainline to 0.2.0 first
    fs::write(dir.join("src/lib.rs"), "// v0.2\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "new feature"]);
    panrelease(dir, "minor").expect("minor bump should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("version = \"0.2.0\""));

    // Create feature branch from new mainline
    git(dir, &["checkout", "-b", "feat/new-thing"]);

    fs::write(dir.join("src/lib.rs"), "// feature work\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feature work"]);

    panrelease(dir, "post").expect("post on feature from 0.2.0 should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.2.0+new-thing.r1\""),
        "Post should use 0.2.0 as base. Cargo.toml:\n{cargo}"
    );
}

// =====================================================================
// Tag template with strict mode
// =====================================================================

#[test]
fn strict_with_tag_template() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "test@test.com"]);
    git(dir, &["config", "user.name", "Test User"]);

    fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::write(
        dir.join("CHANGELOG.md"),
        "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] 2025-01-01\n",
    )
    .unwrap();

    // Config with v{{version}} tag template
    fs::write(
        dir.join(".panproject.toml"),
        r#"[vcs]
software = "Git"
tag_template = "v{{version}}"

[vcs.strict]
mainline = "main"

[modules.root]
path = "."
packageManager = "Cargo"
main = true
"#,
    )
    .unwrap();

    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "").unwrap();

    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "initial"]);
    git(dir, &["tag", "v0.1.0"]);

    // Mainline patch with tag template
    fs::write(dir.join("src/lib.rs"), "// patch\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "fix"]);

    panrelease(dir, "patch").expect("patch with tag template should succeed");

    let tags = git_output(dir, &["tag", "-l"]);
    assert!(
        tags.contains("v0.1.1"),
        "Tag should use v prefix. Tags:\n{tags}"
    );

    // Feature branch with tag template
    git(dir, &["checkout", "-b", "feat/tagged"]);

    fs::write(dir.join("src/lib.rs"), "// feat\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-m", "feature"]);

    panrelease(dir, "post").expect("post with tag template should succeed");

    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("version = \"0.1.1+tagged.r1\""),
        "Feature should use 0.1.1 as base (from v0.1.1 tag). Cargo.toml:\n{cargo}"
    );

    let tags = git_output(dir, &["tag", "-l"]);
    assert!(
        tags.contains("v0.1.1+tagged.r1"),
        "Post tag should use v prefix. Tags:\n{tags}"
    );
}

// =====================================================================
// Dirty staging area
// =====================================================================

#[test]
fn strict_dirty_staging_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    setup_strict_repo(dir);

    // Modify a tracked file without committing
    fs::write(dir.join("src/lib.rs"), "// uncommitted\n").unwrap();
    git(dir, &["add", "-A"]);

    let result = panrelease(dir, "patch");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("uncommitted changes"),
        "Expected dirty staging error, got: {err}"
    );
    assert!(
        err.contains("lib.rs"),
        "Error should list the dirty file, got: {err}"
    );
}
