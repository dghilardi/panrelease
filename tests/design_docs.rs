use std::fs;
use std::path::Path;
use std::process::Command;

use panrelease::engine;
use panrelease::system::NativeSystem;

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn commit(repo: &Path, message: &str) {
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", message]);
}

fn repo(config: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path();
    git(repo, &["init", "-qb", "main"]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "Test"]);
    git(repo, &["config", "core.autocrlf", "false"]);
    fs::write(repo.join("version.json"), r#"{"version":"1.0.0"}"#).unwrap();
    fs::write(
        repo.join(".panproject.toml"),
        format!(
            r#"
[modules.app]
path = "."
main = true
packageManager = "Generic"
format = "json"
file = "version.json"
versionField = "version"
{config}
"#
        ),
    )
    .unwrap();
    temp
}

fn release(repo: &Path, version: &str) {
    engine::run::<_, _, NativeSystem>([
        "panrelease",
        "--path",
        repo.to_str().unwrap(),
        "release",
        version,
    ])
    .unwrap();
}

fn write(repo: &Path, path: &str, content: &str) {
    let path = repo.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn design(state: &str) -> String {
    format!(
        r#"---
title: "Pianificazione — esempio"
status: accepted
delivery:
  service: my-service
  tracking: EXAMPLE-123
  owner: core
  implementation: {state}
  released_in: null # first release
  available_from: null
---

Example implementation.
```yaml
released_in: null
```
"#
    )
}

const ENABLED: &str = "[design_docs]\nservice = \"my-service\"";

#[test]
fn stamps_the_tagged_commit_and_preserves_the_document_on_later_releases() {
    let temp = repo(ENABLED);
    let repo = temp.path();
    let path = "docs/design/subdir/my design.mdx";
    write(repo, path, &design("in_progress"));
    commit(repo, "plan the change");
    git(repo, &["tag", "-a", "1.0.0", "-m", "initial release"]);

    git(repo, &["switch", "-qc", "feat/design"]);
    let completed = design("implemented");
    let completed = format!("\u{feff}{}", completed.replace('\n', "\r\n"));
    write(repo, path, &completed);
    commit(repo, "implement the design");
    git(repo, &["switch", "-q", "main"]);
    git(repo, &["merge", "-q", "--no-ff", "feat/design"]);
    let completed = completed.replace("Example implementation.", "Updated design text.");
    write(repo, path, &completed);
    commit(repo, "edit design wording");

    let expected = completed.replacen("released_in: null", "released_in: \"1.1.0\"", 1);
    release(repo, "1.1.0");
    assert_eq!(git(repo, &["show", &format!("1.1.0:{path}")]), expected);
    release(repo, "1.1.1");
    assert_eq!(git(repo, &["show", &format!("1.1.1:{path}")]), expected);
}

#[test]
fn only_stamps_eligible_designs_with_a_verifiable_transition() {
    let temp = repo(ENABLED);
    let repo = temp.path();
    let cases = [
        ("eligible", design("implemented"), true),
        ("previous-release", design("implemented"), false),
        ("new-already-implemented", design("implemented"), false),
        (
            "wrong-service",
            design("implemented").replace("my-service", "other-service"),
            false,
        ),
        (
            "shared",
            design("implemented").replace("my-service", "null"),
            false,
        ),
        (
            "proposed",
            design("implemented").replace("accepted", "proposed"),
            false,
        ),
        ("in-progress", design("in_progress"), false),
        ("reverted", design("in_progress"), false),
        (
            "already-stamped",
            design("implemented").replace("null # first", "\"0.9.0\" # first"),
            false,
        ),
        (
            "quoted-null",
            design("implemented").replace("null # first", "\"null\" # first"),
            false,
        ),
        (
            "missing-field",
            design("implemented").replace("  released_in: null # first release\n", ""),
            false,
        ),
        (
            "ambiguous",
            design("implemented").replace(
                "  available_from:",
                "  released_in: null\n  available_from:",
            ),
            false,
        ),
        (
            "different-tracking",
            design("implemented").replace("EXAMPLE-123", "EXAMPLE-456"),
            false,
        ),
    ];

    for (name, _, _) in &cases {
        let baseline = match *name {
            "previous-release" => design("implemented"),
            "new-already-implemented" => continue,
            _ => design("in_progress"),
        };
        write(repo, &format!("docs/design/{name}.md"), &baseline);
    }
    commit(repo, "record design state for previous release");
    git(repo, &["tag", "1.0.0"]);
    write(repo, "docs/design/reverted.md", &design("implemented"));
    commit(repo, "implement a design");
    git(repo, &["revert", "--no-edit", "HEAD"]);

    for (name, current, _) in &cases {
        write(repo, &format!("docs/design/{name}.md"), current);
    }
    commit(repo, "complete designs");
    write(repo, "docs/design/untracked.md", &design("implemented"));
    release(repo, "1.1.0");
    for (name, current, stamped) in cases {
        let expected = if stamped {
            current.replacen("released_in: null", "released_in: \"1.1.0\"", 1)
        } else {
            current
        };
        assert_eq!(
            fs::read_to_string(repo.join(format!("docs/design/{name}.md"))).unwrap(),
            expected,
            "{name}"
        );
    }
    assert_eq!(
        fs::read_to_string(repo.join("docs/design/untracked.md")).unwrap(),
        design("implemented")
    );
}

#[test]
fn uses_custom_directory_and_waits_for_stable_release() {
    let temp = repo(
        r#"
[vcs]
software = "Git"
tag_template = "service-v{{version}}"

[design_docs]
service = "my-service"
path = "design [records]"
format = "cloud-design-v1"
"#,
    );
    let repo = temp.path();
    let path = "design [records]/accepted.md";
    let legacy = "design [records]/old.md";
    write(repo, path, &design("not_started"));
    write(repo, legacy, &design("implemented"));
    commit(repo, "start project");
    git(repo, &["tag", "service-v1.0.0"]);

    let completed = design("implemented").replace("released_in: null", "released_in: ~");
    write(repo, path, &completed);
    write(repo, "docs/design/ignored.md", &completed);
    commit(repo, "deliver design");
    release(repo, "1.0.0+design.r1");
    assert_eq!(fs::read_to_string(repo.join(path)).unwrap(), completed);
    release(repo, "1.1.0-rc.1");
    assert_eq!(fs::read_to_string(repo.join(path)).unwrap(), completed);
    release(repo, "1.1.0");
    assert_eq!(
        fs::read_to_string(repo.join(path)).unwrap(),
        completed.replacen("released_in: ~", "released_in: \"1.1.0\"", 1),
    );
    assert_eq!(
        fs::read_to_string(repo.join(legacy)).unwrap(),
        design("implemented")
    );
    assert_eq!(
        fs::read_to_string(repo.join("docs/design/ignored.md")).unwrap(),
        completed
    );
}

#[test]
fn leaves_design_documents_alone_without_opt_in() {
    let temp = repo("");
    let repo = temp.path();
    let path = "docs/design/design.md";
    write(repo, path, &design("not_started"));
    commit(repo, "propose a design");
    let completed = design("implemented");
    write(repo, path, &completed);
    commit(repo, "implement a design");
    release(repo, "1.1.0");
    assert_eq!(fs::read_to_string(repo.join(path)).unwrap(), completed);
}

#[test]
fn verifies_first_release_with_full_history_and_skips_shallow_history() {
    let source = repo(ENABLED);
    let repo = source.path();
    let path = "docs/design/example.md";
    write(repo, path, &design("not_started"));
    commit(repo, "propose a design");
    let completed = design("implemented");
    write(repo, path, &completed);
    commit(repo, "implement a design");

    let checkout_parent = tempfile::tempdir().unwrap();
    let url = format!("file://{}", repo.display());
    git(
        checkout_parent.path(),
        &["clone", "-q", "--depth=1", &url, "shallow"],
    );
    let shallow = checkout_parent.path().join("shallow");
    git(&shallow, &["config", "user.email", "test@example.com"]);
    git(&shallow, &["config", "user.name", "Test"]);
    release(&shallow, "1.1.0");
    assert_eq!(fs::read_to_string(shallow.join(path)).unwrap(), completed);

    release(repo, "1.1.0");
    assert_eq!(
        fs::read_to_string(repo.join(path)).unwrap(),
        completed.replacen("released_in: null", "released_in: \"1.1.0\"", 1),
    );
}
