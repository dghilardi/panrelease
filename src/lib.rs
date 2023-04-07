mod wasm_utils;
pub mod system;
pub mod engine;
mod project;
mod args;
mod package;
mod runner;
mod parser;
mod utils;
mod git;
pub mod conf;

use anyhow::Context;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use crate::system::NodeJsSystem;
use crate::wasm_utils::set_panic_hook;

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct ReifyRunArgs {
    cli_args: Vec<String>
}

#[wasm_bindgen]
pub fn run(js_args: js_sys::Array) {
    set_panic_hook();

    let args: Vec<String> = js_args.iter()
        .filter_map(|el| el.as_string())
        .collect();

    let result = engine::run::<_, _, NodeJsSystem>(args)
        .context("Error running panrelease");

    if let Err(e) = result {
        wasm_utils::log(&format!("Error happened: {:?}", e));
    }
}