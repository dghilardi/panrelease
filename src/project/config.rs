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
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            force_sign: false,
            tag_template: default_tag_template(),
        }
    }
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
}

impl PackageManager {
    pub fn detect<F: FileSystem>(path: &Path) -> Option<Self> {
        if F::is_a_file(&path.join("Cargo.toml")) {
            Some(Self::Cargo)
        } else if F::is_a_file(&path.join("pom.xml")) {
            Some(Self::Maven)
        } else if F::is_a_file(&path.join("package.json")) {
            Some(Self::Npm)
        } else {
            None
        }
    }
}

impl<P> Default for PanProjectConfig<P> {
    fn default() -> Self {
        Self {
            vcs: default_vcs_config(),
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
        let mut conf: PanProjectConfig<F> = toml::from_str(&conf_str)?;

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

    fn validate_module(mod_name: &str, module_conf: &ProjectModule) -> anyhow::Result<()> {
        match module_conf.package_manager {
            PackageManager::Cargo => {
                let cargo_toml_path = module_conf.path.join("Cargo.toml");
                if !F::is_a_file(&cargo_toml_path) {
                    return Err(anyhow!(
                        "Error during {mod_name} module validation. {:?} is not a valid file",
                        cargo_toml_path
                    ));
                }
            }
            PackageManager::Npm => {
                let cargo_toml_path = module_conf.path.join("package.json");
                if !F::is_a_file(&cargo_toml_path) {
                    return Err(anyhow!(
                        "Error during {mod_name} module validation. {:?} is not a valid file",
                        cargo_toml_path
                    ));
                }
            }
            PackageManager::Maven => {
                let cargo_toml_path = module_conf.path.join("pom.xml");
                if !F::is_a_file(&cargo_toml_path) {
                    return Err(anyhow!(
                        "Error during {mod_name} module validation. {:?} is not a valid file",
                        cargo_toml_path
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
            let (name, conf) = self.modules.iter().next().expect("Module not found");

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
                let (name, conf) = main_modules.first().expect("Module not found");
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
}
