use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use crate::project::module::PanModule;

#[derive(Deserialize, Debug)]
pub struct PanProjectConfig {
    pub vcs: VcsConfig,
    pub modules: HashMap<String, ProjectModule>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "software")]
pub enum VcsConfig {
    Git,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectModule {
    pub path: PathBuf,
    #[serde(default = "default_main")]
    pub main: bool,
    #[serde(flatten)]
    pub package_manager: PackageManager,
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

impl PanProjectConfig {
    pub fn load(path: &Path) -> anyhow::Result<PanProjectConfig> {
        let conf_str = fs::read_to_string(path.join(".panproject.toml"))
            .with_context(|| format!("Failed to read .panproject.toml from {:?}", path))?;
        let mut conf: PanProjectConfig = toml::from_str(&conf_str)?;

        conf.modules.iter_mut()
            .map(|(mod_name, conf)| {
                conf.path = path.join(&conf.path);
                Self::validate_module(mod_name, conf)
            })
            .collect::<Result<Vec<()>, _>>()?;

        Ok(conf)
    }

    fn validate_module(mod_name: &str, module_conf: &ProjectModule) -> anyhow::Result<()> {
        match module_conf.package_manager {
            PackageManager::Cargo => {
                let cargo_toml_path = module_conf.path.join("Cargo.toml");
                if !cargo_toml_path.is_file() {
                    return Err(anyhow!("Error during {mod_name} module validation. {:?} is not a valid file", cargo_toml_path));
                }
            }
            PackageManager::Npm => {
                let cargo_toml_path = module_conf.path.join("package.json");
                if !cargo_toml_path.is_file() {
                    return Err(anyhow!("Error during {mod_name} module validation. {:?} is not a valid file", cargo_toml_path));
                }
            }
            PackageManager::Maven => {
                let cargo_toml_path = module_conf.path.join("pom.xml");
                if !cargo_toml_path.is_file() {
                    return Err(anyhow!("Error during {mod_name} module validation. {:?} is not a valid file", cargo_toml_path));
                }
            }
        }
        Ok(())
    }

    pub fn extract_master_mod(&self) -> anyhow::Result<Option<PanModule>> {
        if self.modules.is_empty() {
            Ok(None)
        } else if self.modules.len() == 1 {
            let (name, conf) = self.modules.iter()
                .next()
                .expect("Module not found");

            Ok(Some(PanModule::new(String::from(name), conf.clone())?))
        } else {
            let main_modules = self.modules.iter().filter(|(_, m)| m.main).collect::<Vec<_>>();
            if main_modules.is_empty() {
                Err(anyhow!("No module marked as main"))
            } else if main_modules.len() > 1 {
                Err(anyhow!("Only one module must be marked as main. found {}", main_modules.len()))
            } else {
                let (name, conf) = main_modules.first().expect("Module not found");
                Ok(Some(PanModule::new(String::from(*name), (*conf).clone())?))
            }
        }
    }
}

