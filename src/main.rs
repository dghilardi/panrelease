mod project;
use crate::project::core::PanProject;

fn main() {
    let cwd = std::env::current_dir()
        .expect("Error loading current directory");

    let project = PanProject::load(cwd.as_path())
        .expect("Error loading project");
}
