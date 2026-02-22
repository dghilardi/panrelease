use std::path::PathBuf;
use anyhow::{anyhow, bail, Context};
use crate::args::InitArgs;
use crate::git::GitRepo;
use crate::project::config::PackageManager;
use crate::system::FileSystem;

// ── value structs ─────────────────────────────────────────────────────────────

struct VcsOpts {
    tag_template: String,
    force_sign: bool,
    strict_mainline: Option<String>,
}

impl Default for VcsOpts {
    fn default() -> Self {
        Self {
            tag_template: String::from("{{version}}"),
            force_sign: false,
            strict_mainline: None,
        }
    }
}

struct ChangelogOpts {
    from_commits: bool,
    commit_format: String,
    include_scope: bool,
    include_unmatched: bool,
}

impl Default for ChangelogOpts {
    fn default() -> Self {
        Self {
            from_commits: false,
            commit_format: String::from("auto"),
            include_scope: true,
            include_unmatched: false,
        }
    }
}

struct DetectedModule {
    name: String,
    rel_path: String,
    package_manager: PackageManager,
}

struct InitConfig {
    vcs: VcsOpts,
    changelog: Option<ChangelogOpts>,
    modules: Vec<DetectedModule>,
}

// ── entry point ───────────────────────────────────────────────────────────────

pub fn run<F: FileSystem + 'static>(path: Option<PathBuf>, args: InitArgs) -> anyhow::Result<()> {
    let cwd = path.map(Ok).unwrap_or_else(F::current_dir)?;
    let git_root = GitRepo::find_git_root::<F>(&cwd)
        .context("Could not find git repository root")?
        .to_path_buf();

    let config_path = git_root.join(".panproject.toml");
    if F::is_a_file(&config_path) {
        bail!(
            ".panproject.toml already exists at {}. \
             Remove it first if you want to re-initialize.",
            config_path.display()
        );
    }

    let modules = detect_modules::<F>(&git_root)?;
    if modules.is_empty() {
        bail!(
            "Could not detect a supported package manager in {}. \
             Supported: Cargo.toml, package.json, pom.xml, gradle.properties",
            git_root.display()
        );
    }

    let vcs_opts = if args.interactive {
        prompt_vcs()?
    } else {
        VcsOpts::default()
    };

    let changelog_opts = if args.interactive {
        prompt_changelog()?
    } else {
        None
    };

    let config = InitConfig {
        vcs: vcs_opts,
        changelog: changelog_opts,
        modules,
    };

    let toml_content = render_toml(&config);
    F::write_string(&config_path, &toml_content)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    println!("Created {}", config_path.display());
    Ok(())
}

// ── detection ─────────────────────────────────────────────────────────────────

fn detect_modules<F: FileSystem>(root: &std::path::Path) -> anyhow::Result<Vec<DetectedModule>> {
    let mut modules = Vec::new();
    if let Some(pm) = PackageManager::detect::<F>(root) {
        modules.push(DetectedModule {
            name: String::from("root"),
            rel_path: String::from("."),
            package_manager: pm,
        });
    }
    Ok(modules)
}

// ── interactive prompts ───────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
fn prompt(question: &str, default: &str) -> anyhow::Result<String> {
    use std::io::{self, Write};
    if default.is_empty() {
        print!("{question}: ");
    } else {
        print!("{question} [{default}]: ");
    }
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let trimmed = buf.trim().to_string();
    Ok(if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed
    })
}

#[cfg(target_arch = "wasm32")]
fn prompt(_question: &str, default: &str) -> anyhow::Result<String> {
    Ok(default.to_string())
}

fn prompt_bool(question: &str, default: bool) -> anyhow::Result<bool> {
    let hint = if default { "Y/n" } else { "y/N" };
    let answer = prompt(&format!("{question} ({hint})"), "")?;
    match answer.to_lowercase().as_str() {
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        "" => Ok(default),
        other => Err(anyhow!("Expected y/n, got '{other}'")),
    }
}

fn prompt_vcs() -> anyhow::Result<VcsOpts> {
    let tag_template = prompt("Tag template", "{{version}}")?;
    let force_sign = prompt_bool("Force GPG sign commits and tags?", false)?;
    let mainline_raw = prompt("Strict mode: mainline branch (blank to disable)", "")?;
    let strict_mainline = if mainline_raw.is_empty() {
        None
    } else {
        Some(mainline_raw)
    };
    Ok(VcsOpts { tag_template, force_sign, strict_mainline })
}

fn prompt_changelog() -> anyhow::Result<Option<ChangelogOpts>> {
    let from_commits = prompt_bool("Generate changelog entries from commit messages?", false)?;
    if !from_commits {
        return Ok(None);
    }

    let format_raw = prompt("Commit format (auto/conventional/gitmoji)", "auto")?;
    let commit_format = match format_raw.to_lowercase().as_str() {
        "conventional" | "gitmoji" | "auto" => format_raw.to_lowercase(),
        other => {
            return Err(anyhow!(
                "Unknown commit format '{other}'. Choose auto, conventional, or gitmoji."
            ))
        }
    };
    let include_scope = prompt_bool("Include commit scope in changelog?", true)?;
    let include_unmatched = prompt_bool("Include unmatched commits in changelog?", false)?;

    Ok(Some(ChangelogOpts {
        from_commits: true,
        commit_format,
        include_scope,
        include_unmatched,
    }))
}

// ── TOML rendering ────────────────────────────────────────────────────────────

fn render_toml(config: &InitConfig) -> String {
    let mut out = String::new();

    // [vcs]
    out.push_str("[vcs]\n");
    out.push_str("software = \"Git\"\n");
    if config.vcs.force_sign {
        out.push_str("force_sign = true\n");
    }
    if config.vcs.tag_template != "{{version}}" {
        out.push_str(&format!("tag_template = \"{}\"\n", config.vcs.tag_template));
    }

    if let Some(ref mainline) = config.vcs.strict_mainline {
        out.push('\n');
        out.push_str("[vcs.strict]\n");
        out.push_str(&format!("mainline = \"{mainline}\"\n"));
    }

    // [changelog] (only when from_commits or non-defaults)
    if let Some(ref cl) = config.changelog {
        out.push('\n');
        out.push_str("[changelog]\n");
        out.push_str(&format!("from_commits = {}\n", cl.from_commits));
        if cl.commit_format != "auto" {
            out.push_str(&format!("commit_format = \"{}\"\n", cl.commit_format));
        }
        if !cl.include_scope {
            out.push_str("include_scope = false\n");
        }
        if cl.include_unmatched {
            out.push_str("include_unmatched = true\n");
        }
    }

    // [modules.*]
    let has_multiple = config.modules.len() > 1;
    for (i, module) in config.modules.iter().enumerate() {
        out.push('\n');
        out.push_str(&format!("[modules.{}]\n", module.name));
        out.push_str(&format!("path = \"{}\"\n", module.rel_path));
        out.push_str(&format!(
            "packageManager = \"{}\"\n",
            package_manager_name(module.package_manager)
        ));
        // Mark first module as main when there are multiple
        if i == 0 && has_multiple {
            out.push_str("main = true\n");
        }
    }

    out
}

fn package_manager_name(pm: PackageManager) -> &'static str {
    match pm {
        PackageManager::Cargo => "Cargo",
        PackageManager::Npm => "Npm",
        PackageManager::Maven => "Maven",
        PackageManager::Gradle => "Gradle",
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_minimal_cargo_config() {
        let config = InitConfig {
            vcs: VcsOpts::default(),
            changelog: None,
            modules: vec![DetectedModule {
                name: String::from("root"),
                rel_path: String::from("."),
                package_manager: PackageManager::Cargo,
            }],
        };
        let toml = render_toml(&config);
        assert!(toml.contains("[vcs]"), "should have vcs section");
        assert!(toml.contains("software = \"Git\""), "should declare git");
        assert!(toml.contains("[modules.root]"), "should have root module");
        assert!(toml.contains("packageManager = \"Cargo\""), "should declare Cargo");
        assert!(!toml.contains("[changelog]"), "should omit changelog section");
        assert!(!toml.contains("[vcs.strict]"), "should omit strict section");
    }

    #[test]
    fn render_with_strict_and_changelog() {
        let config = InitConfig {
            vcs: VcsOpts {
                strict_mainline: Some(String::from("main")),
                force_sign: true,
                tag_template: String::from("{{version}}"),
            },
            changelog: Some(ChangelogOpts {
                from_commits: true,
                commit_format: String::from("conventional"),
                include_scope: true,
                include_unmatched: false,
            }),
            modules: vec![DetectedModule {
                name: String::from("root"),
                rel_path: String::from("."),
                package_manager: PackageManager::Cargo,
            }],
        };
        let toml = render_toml(&config);
        assert!(toml.contains("force_sign = true"));
        assert!(toml.contains("[vcs.strict]"));
        assert!(toml.contains("mainline = \"main\""));
        assert!(toml.contains("[changelog]"));
        assert!(toml.contains("from_commits = true"));
        assert!(toml.contains("commit_format = \"conventional\""));
    }

    #[test]
    fn render_non_default_tag_template() {
        let config = InitConfig {
            vcs: VcsOpts {
                tag_template: String::from("v{{version}}"),
                ..VcsOpts::default()
            },
            changelog: None,
            modules: vec![DetectedModule {
                name: String::from("root"),
                rel_path: String::from("."),
                package_manager: PackageManager::Npm,
            }],
        };
        let toml = render_toml(&config);
        assert!(toml.contains("tag_template = \"v{{version}}\""));
    }

    #[test]
    fn render_multiple_modules_marks_first_as_main() {
        let config = InitConfig {
            vcs: VcsOpts::default(),
            changelog: None,
            modules: vec![
                DetectedModule {
                    name: String::from("backend"),
                    rel_path: String::from("."),
                    package_manager: PackageManager::Cargo,
                },
                DetectedModule {
                    name: String::from("frontend"),
                    rel_path: String::from("./frontend"),
                    package_manager: PackageManager::Npm,
                },
            ],
        };
        let toml = render_toml(&config);
        // The first module section should have main = true
        let backend_pos = toml.find("[modules.backend]").unwrap();
        let frontend_pos = toml.find("[modules.frontend]").unwrap();
        let main_pos = toml.find("main = true").unwrap();
        assert!(main_pos > backend_pos, "main = true should appear after [modules.backend]");
        assert!(main_pos < frontend_pos, "main = true should appear before [modules.frontend]");
    }
}
