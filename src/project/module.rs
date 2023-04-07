use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::bail;

use crate::package::cargo::CargoPackage;
use crate::package::maven::MavenPackage;
use crate::package::npm::NpmPackage;
use crate::package::PanPackage;
use crate::project::config::{PackageManager, ProjectModule};
use crate::runner::CmdRunner;
use crate::system::FileSystem;

pub struct PanModule<F> {
    name: String,
    conf: ProjectModule,
    package: Box<dyn PanPackage>,
    filesystem: PhantomData<F>,
}

impl <F: FileSystem + 'static> PanModule<F> {
    pub fn new(name: String, conf: ProjectModule) -> anyhow::Result<Self> {
        Ok(Self {
            name,
            package: Self::extract_package(&conf)?,
            conf,
            filesystem: PhantomData
        })
    }

    pub fn detect(path: PathBuf) -> anyhow::Result<Option<Self>> {
        let Some(package_manager) = PackageManager::detect::<F>(&path) else {
            return Ok(None)
        };
        let conf = ProjectModule {
            path,
            main: false,
            package_manager,
            hooks: Default::default(),
        };

        Ok(Some(Self {
            name: String::from("<detected>"),
            package: Self::extract_package(&conf)?,
            conf,
            filesystem: PhantomData
        }))
    }

    fn extract_package(conf: &ProjectModule) -> anyhow::Result<Box<dyn PanPackage>> {
        Ok(match conf.package_manager {
            PackageManager::Cargo => Box::new(CargoPackage::<F>::new(conf.path.clone())?),
            PackageManager::Npm => Box::new(NpmPackage::<F>::new(conf.path.clone())?),
            PackageManager::Maven => Box::new(MavenPackage::<F>::new(conf.path.clone())?),
        })
    }

    pub fn extract_version(&self) -> anyhow::Result<semver::Version> {
        self.package.extract_version()
    }

    pub fn set_version(&mut self, version: &semver::Version) -> anyhow::Result<()> {
        self.package.set_version(version)
    }

    pub fn persist(&self) -> anyhow::Result<()> {
        self.package.persist()
    }

    pub fn hook_after_rel(&mut self) -> anyhow::Result<()> {
        self.package.hook_after_rel()?;
        for (name, full_command) in self.conf.hooks.after_rel.iter() {
            let [command, args @ ..] = full_command.as_slice() else {
                bail!("error reading '{name}' after_rel hook");
            };
            println!("running after_rel hook {name}");
            let mut runner = CmdRunner::build(command, args, &self.conf.path)?;
            runner.run()?;
        }
        Ok(())
    }
}

