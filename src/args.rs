use std::str::FromStr;
use clap::{Args, Parser, Subcommand, ValueEnum};
use semver::Prerelease;

/// Simple program release and tag software versions
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct PanReleaseArgs {
    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Release a new version
    Release(RelArgs),
}

#[derive(Args, Debug)]
pub struct RelArgs {
    /// Either bump by LEVEL or set the VERSION for all selected packages
    #[arg(value_name = "LEVEL|VERSION", help_heading = "Version")]
    pub level_or_version: TargetVersion,
}

#[derive(Clone, Debug)]
pub enum TargetVersion {
    Relative(BumpLevel),
    Absolute(semver::Version),
}

impl TargetVersion {
    pub fn apply(
        &self,
        current: semver::Version,
    ) -> semver::Version {
        match self {
            TargetVersion::Relative(bump_level) => {
                bump_level.apply(current)
            }
            TargetVersion::Absolute(version) => {
                version.to_owned()
            }
        }
    }
}

impl clap::builder::ValueParserFactory for TargetVersion {
    type Parser = TargetVersionParser;

    fn value_parser() -> Self::Parser {
        TargetVersionParser
    }
}

impl std::str::FromStr for TargetVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(bump_level) = BumpLevel::from_str(s, false) {
            Ok(TargetVersion::Relative(bump_level))
        } else {
            Ok(TargetVersion::Absolute(
                semver::Version::parse(s).map_err(|e| e.to_string())?,
            ))
        }
    }
}

#[derive(Copy, Clone)]
pub struct TargetVersionParser;

impl clap::builder::TypedValueParser for TargetVersionParser {
    type Value = TargetVersion;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let inner_parser = TargetVersion::from_str;
        inner_parser.parse_ref(cmd, arg, value)
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        let inner_parser = clap::builder::EnumValueParser::<BumpLevel>::new();
        #[allow(clippy::needless_collect)] // Erasing a lifetime
        inner_parser.possible_values().map(|ps| {
            let ps = ps.collect::<Vec<_>>();
            let ps: Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_> =
                Box::new(ps.into_iter());
            ps
        })
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
#[value(rename_all = "kebab-case")]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
    Pre,
}

impl BumpLevel {
    fn apply(
        &self,
        current: semver::Version,
    ) -> semver::Version {
        match self {
            BumpLevel::Major => {
                semver::Version {
                    major: current.major + 1,
                    minor: 0,
                    patch: 0,
                    pre: Default::default(),
                    build: Default::default(),
                }
            }
            BumpLevel::Minor => {
                semver::Version {
                    major: current.major,
                    minor: current.minor + 1,
                    patch: 0,
                    pre: Default::default(),
                    build: Default::default(),
                }
            }
            BumpLevel::Patch => {
                semver::Version {
                    major: current.major,
                    minor: current.minor,
                    patch: current.patch + 1,
                    pre: Default::default(),
                    build: Default::default(),
                }
            }
            BumpLevel::Pre => {
                let pre = current.pre.rfind('.').and_then(|sep_idx| {
                    current.pre.as_str()[sep_idx+1..]
                        .parse::<u64>()
                        .ok()
                        .map(|current_pre_v| Prerelease::new(&format!("{}.{}", &current.pre.as_str()[0..sep_idx], current_pre_v + 1)))
                }).unwrap_or_else(|| if current.pre.is_empty() {
                    Prerelease::new("pre.1")
                } else {
                    Prerelease::new(&format!("{}.1", current.pre.as_str()))
                }).expect("Error constructing prerelease slug");

                semver::Version {
                    major: current.major,
                    minor: current.minor,
                    patch: current.patch,
                    pre,
                    build: Default::default(),
                }
            }
        }
    }
}