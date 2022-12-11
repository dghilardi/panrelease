use crate::package::cargo::CargoPackage;
use crate::package::PanPackage;
use crate::project::config::{PackageManager, ProjectModule};

pub struct PanModule {
    name: String,
    conf: ProjectModule,
    package: Box<dyn PanPackage>,
}

impl PanModule {
    pub fn new(name: String, conf: ProjectModule) -> anyhow::Result<Self> {
        Ok(Self {
            name,
            package: Self::extract_package(&conf)?,
            conf,
        })
    }

    fn extract_package(conf: &ProjectModule) -> anyhow::Result<Box<dyn PanPackage>> {
        Ok(match conf.package_manager {
            PackageManager::Cargo => Box::new(CargoPackage::new(conf.path.clone())?),
            PackageManager::Npm => todo!("package not implemented"),
            PackageManager::Maven => todo!("package not implemented"),
        })
    }

    pub fn extract_version(&self) -> anyhow::Result<semver::Version> {
        self.package.extract_version()
    }
}

