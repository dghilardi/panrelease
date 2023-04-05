use panrelease::engine;
use panrelease::system::NativeSystem;

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    check_version();

    engine::run::<_, _, NativeSystem>(std::env::args())
        .expect("Error executing engine");
}

#[cfg(not(target_arch = "wasm32"))]
fn check_version() {
    use update_informer::Check;
    use update_informer::registry::Crates;

    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(Crates, name, version);
    if let Some(version) = informer.check_version().ok().flatten()  {
        println!("New version is available: {}", version);
    }
}