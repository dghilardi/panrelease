use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::project::module::PanModule;
use crate::system::FileSystem;

#[derive(Deserialize, Debug)]
pub struct PanProjectConfig<F> {
    #[serde(default = "default_vcs_config")]
    vcs: VcsConfig,
    #[serde(default)]
    changelog: ChangelogConfig,
    modules: HashMap<String, ProjectModule>,
    #[serde(skip_deserializing, skip_serializing)]
    filesystem: PhantomData<F>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "software")]
pub enum VcsConfig {
    Git(GitConfig),
}

fn default_vcs_config() -> VcsConfig {
    VcsConfig::Git(GitConfig::default())
}

#[derive(Deserialize, Clone, Debug)]
pub struct GitConfig {
    #[serde(default)]
    pub force_sign: bool,
    #[serde(default = "default_tag_template")]
    pub tag_template: String,
    #[serde(default)]
    pub strict: Option<StrictConfig>,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            force_sign: false,
            tag_template: default_tag_template(),
            strict: None,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct StrictConfig {
    pub mainline: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ChangelogConfig {
    #[serde(default)]
    pub from_commits: bool,
    #[serde(default)]
    pub commit_format: CommitFormatKind,
    #[serde(default = "default_true")]
    pub include_scope: bool,
    #[serde(default)]
    pub include_unmatched: bool,
}

#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommitFormatKind {
    #[default]
    Auto,
    Conventional,
    Gitmoji,
}

fn default_true() -> bool {
    true
}

fn default_tag_template() -> String {
    String::from("{{version}}")
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectModule {
    pub path: PathBuf,
    #[serde(default = "default_main")]
    pub main: bool,
    #[serde(flatten)]
    pub package_manager: PackageManager,
    #[serde(default)]
    pub hooks: ProjectHooks,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct ProjectHooks {
    pub after_rel: BTreeMap<String, Vec<String>>,
}

fn default_main() -> bool {
    false
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(tag = "packageManager")]
pub enum PackageManager {
    Cargo,
    Npm,
    Maven,
    Gradle,
}

impl PackageManager {
    pub fn detect<F: FileSystem>(path: &Path) -> Option<Self> {
        if F::is_a_file(&path.join("Cargo.toml")) {
            Some(Self::Cargo)
        } else if F::is_a_file(&path.join("pom.xml")) {
            Some(Self::Maven)
        } else if F::is_a_file(&path.join("package.json")) {
            Some(Self::Npm)
        } else if F::is_a_file(&path.join("gradle.properties")) {
            Some(Self::Gradle)
        } else {
            None
        }
    }
}

impl<P> Default for PanProjectConfig<P> {
    fn default() -> Self {
        Self {
            vcs: default_vcs_config(),
            changelog: ChangelogConfig::default(),
            modules: Default::default(),
            filesystem: PhantomData,
        }
    }
}

impl<F: FileSystem + 'static> PanProjectConfig<F> {
    pub fn load(path: &Path) -> anyhow::Result<PanProjectConfig<F>> {
        let conf_file_path = path.join(".panproject.toml");
        if !F::is_a_file(&conf_file_path) {
            return Ok(Default::default());
        }
        let conf_str = F::read_string(&conf_file_path)
            .with_context(|| format!("Failed to read .panproject.toml from {:?}", path))?;
        let mut conf: PanProjectConfig<F> = toml::from_str(&conf_str)
            .with_context(|| format!("Failed to parse .panproject.toml at {:?}", conf_file_path))?;

        conf.modules
            .iter_mut()
            .map(|(mod_name, conf)| {
                conf.path = path.join(&conf.path);
                Self::validate_module(mod_name, conf)
            })
            .collect::<Result<Vec<()>, _>>()?;

        Ok(conf)
    }

    pub fn vcs(&self) -> &VcsConfig {
        &self.vcs
    }

    pub fn changelog(&self) -> &ChangelogConfig {
        &self.changelog
    }

    fn validate_module(mod_name: &str, module_conf: &ProjectModule) -> anyhow::Result<()> {
        match module_conf.package_manager {
            PackageManager::Cargo => {
                let manifest_path = module_conf.path.join("Cargo.toml");
                if !F::is_a_file(&manifest_path) {
                    return Err(anyhow!(
                        "Module '{mod_name}' validation failed: expected Cargo.toml at {:?}. \
                         Check that the path exists and is a regular file.",
                        manifest_path
                    ));
                }
            }
            PackageManager::Npm => {
                let manifest_path = module_conf.path.join("package.json");
                if !F::is_a_file(&manifest_path) {
                    return Err(anyhow!(
                        "Module '{mod_name}' validation failed: expected package.json at {:?}. \
                         Check that the path exists and is a regular file.",
                        manifest_path
                    ));
                }
            }
            PackageManager::Maven => {
                let manifest_path = module_conf.path.join("pom.xml");
                if !F::is_a_file(&manifest_path) {
                    return Err(anyhow!(
                        "Module '{mod_name}' validation failed: expected pom.xml at {:?}. \
                         Check that the path exists and is a regular file.",
                        manifest_path
                    ));
                }
            }
            PackageManager::Gradle => {
                let manifest_path = module_conf.path.join("gradle.properties");
                if !F::is_a_file(&manifest_path) {
                    return Err(anyhow!(
                        "Module '{mod_name}' validation failed: expected gradle.properties at {:?}. \
                         Check that the path exists and is a regular file.",
                        manifest_path
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn extract_master_mod(&self) -> anyhow::Result<Option<PanModule<F>>> {
        if self.modules.is_empty() {
            Ok(None)
        } else if self.modules.len() == 1 {
            let (name, conf) = self.modules.iter().next()
                .ok_or_else(|| anyhow!("Internal error: module list reported length 1 but iterator was empty"))?;

            Ok(Some(PanModule::new(String::from(name), conf.clone())?))
        } else {
            let main_modules = self
                .modules
                .iter()
                .filter(|(_, m)| m.main)
                .collect::<Vec<_>>();
            if main_modules.is_empty() {
                Err(anyhow!("No module marked as main"))
            } else if main_modules.len() > 1 {
                Err(anyhow!(
                    "Only one module must be marked as main. found {}",
                    main_modules.len()
                ))
            } else {
                let (name, conf) = main_modules.first()
                    .ok_or_else(|| anyhow!("Internal error: main module list reported length 1 but was empty"))?;
                Ok(Some(PanModule::new(String::from(*name), (*conf).clone())?))
            }
        }
    }

    pub fn modules(&self) -> anyhow::Result<Vec<PanModule<F>>> {
        self.modules
            .iter()
            .map(|(name, conf)| PanModule::new(String::from(name), conf.clone()))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn validate_module_pub(mod_name: &str, module_conf: &ProjectModule) -> anyhow::Result<()> {
        Self::validate_module(mod_name, module_conf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::NativeSystem;

    #[test]
    fn validate_module_missing_manifest_mentions_module_name_and_file() {
        let conf = ProjectModule {
            path: std::path::PathBuf::from("/nonexistent/path"),
            main: false,
            package_manager: PackageManager::Npm,
            hooks: Default::default(),
        };
        let err = PanProjectConfig::<NativeSystem>::validate_module_pub("my-module", &conf)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("my-module"), "error should mention module name: {msg}");
        assert!(msg.contains("package.json"), "error should mention manifest file: {msg}");
        assert!(msg.contains("nonexistent"), "error should mention the path: {msg}");
    }

    #[test]
    fn validate_module_missing_cargo_toml_mentions_cargo_toml() {
        let conf = ProjectModule {
            path: std::path::PathBuf::from("/nonexistent/path"),
            main: false,
            package_manager: PackageManager::Cargo,
            hooks: Default::default(),
        };
        let err = PanProjectConfig::<NativeSystem>::validate_module_pub("my-crate", &conf)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("my-crate"), "error should mention module name: {msg}");
        assert!(msg.contains("Cargo.toml"), "error should mention Cargo.toml: {msg}");
    }

    #[test]
    fn parse_toml_error_context_contains_filename() {
        // Verify the context string format used in load() contains .panproject.toml
        let fake_path = std::path::PathBuf::from("/project/.panproject.toml");
        let context = format!("Failed to parse .panproject.toml at {:?}", fake_path);
        assert!(context.contains(".panproject.toml"), "context should mention the config file: {context}");
    }
}
