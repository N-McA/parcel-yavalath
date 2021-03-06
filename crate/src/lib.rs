#![feature(use_extern_macros)]
#![feature(uniform_paths)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate cfg_if;

extern crate wasm_bindgen;
use wasm_bindgen::prelude::*;

mod main;

cfg_if! {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        use console_error_panic_hook::set_once as set_panic_hook;
    }
}

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

// Definitions of the functionality available in JS, which wasm-bindgen will
// generate shims for today (and eventually these should be near-0 cost!)
//
// These definitions need to be hand-written today but the current vision is
// that we'll use WebIDL to generate this `extern` block into a crate which you
// can link and import. There's a tracking issue for this at
// https://github.com/rustwasm/wasm-bindgen/issues/42
//
// In the meantime these are written out by hand and correspond to the names and
// signatures documented on MDN, for example
#[wasm_bindgen]
extern "C" {
    type HTMLDocument;
    static document: HTMLDocument;
    #[wasm_bindgen(method)]
    fn createElement(this: &HTMLDocument, tagName: &str) -> Element;
    #[wasm_bindgen(method, getter)]
    fn body(this: &HTMLDocument) -> Element;

    type Element;
    #[wasm_bindgen(method, setter = innerHTML)]
    fn set_inner_html(this: &Element, html: &str);
    #[wasm_bindgen(method, js_name = appendChild)]
    fn append_child(this: &Element, other: Element);
    
    type Date;
    #[wasm_bindgen(static_method_of = Date)]
    pub fn now() -> f64;
}

// Called by our JS entry point to run the example
#[wasm_bindgen]
pub fn run() {
    let val = document.createElement("p");
    val.set_inner_html("Hello from Rust, WebAssembly, and Parcel!");
    document.body().append_child(val);
}

#[wasm_bindgen]
pub fn foo() -> f64 {
    return Date::now();
}

#[wasm_bindgen]
pub fn check_game_outcome(board: &str) -> String {
    main::check_game_string(board.to_string())
}

#[wasm_bindgen]
pub fn pick_move(board: &str, thinking_time: f64) -> i32 {
    let start = Date::now();
    let mut term = || Date::now() - start > thinking_time;
    main::ai_pick_move(board.to_string(), &mut term)
}

