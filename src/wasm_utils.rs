use wasm_bindgen::prelude::*;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}

#[wasm_bindgen(module = "fs")]
extern "C" {
    #[wasm_bindgen(js_name = readFileSync, catch)]
    pub fn read_file(path: &str, encoding: &str) -> Result<String, JsValue>;

    #[wasm_bindgen(js_name = writeFileSync, catch)]
    pub fn write_file(path: &str, content: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = lstatSync, catch)]
    pub fn lstat(path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = existsSync, catch)]
    pub fn exists(path: &str) -> Result<bool, JsValue>;
}

#[wasm_bindgen(module = "child_process")]
extern "C" {
    #[wasm_bindgen(js_name = execSync, catch)]
    pub fn exec(command: String, opts: JsValue) -> Result<Vec<u8>, JsValue>;
}

pub mod process {

    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(module = process)]
    extern "C" {

        #[wasm_bindgen(js_name = arch)]
        pub static ARCH: String;

        #[wasm_bindgen(js_name = env)]
        pub static ENV: JsValue;

        pub fn hrtime() -> js_sys::Array;

        pub fn cwd() -> String;

        #[wasm_bindgen(js_name = platform)]
        pub static PLATFORM: String;
    }
}