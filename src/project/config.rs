use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PanProjectConfig {
    vcs: VcsConfig,
    modules: HashMap<PathBuf, ProjectModule>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "software")]
pub enum VcsConfig {
    Git,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "packageManager")]
pub enum ProjectModule {
    Cargo,
    Npm,
    Maven,
}

impl PanProjectConfig {
    pub fn load(path: &Path) -> anyhow::Result<PanProjectConfig> {
        let conf_str = fs::read_to_string(path.join(".panproject.toml"))
            .with_context(|| format!("Failed to read .panproject.toml from {:?}", path))?;
        let conf: PanProjectConfig = toml::from_str(&conf_str)?;

        conf.modules.iter()
            .map(|(mod_path, conf)| Self::validate_module(path.join(mod_path), conf))
            .collect::<Result<Vec<()>, _>>()?;

        Ok(conf)
    }

    fn validate_module(path: PathBuf, module_conf: &ProjectModule) -> anyhow::Result<()> {
        match module_conf {
            ProjectModule::Cargo => {
                let cargo_toml_path = path.join("Cargo.toml");
                if !cargo_toml_path.is_file() {
                    return Err(anyhow!("{:?} is not a valid file", cargo_toml_path));
                }
            }
            ProjectModule::Npm => {
                let cargo_toml_path = path.join("package.json");
                if !cargo_toml_path.is_file() {
                    return Err(anyhow!("{:?} is not a valid file", cargo_toml_path));
                }
            }
            ProjectModule::Maven => {
                let cargo_toml_path = path.join("pom.xml");
                if !cargo_toml_path.is_file() {
                    return Err(anyhow!("{:?} is not a valid file", cargo_toml_path));
                }
            }
        }
        Ok(())
    }
}

