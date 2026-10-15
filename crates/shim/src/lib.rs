//! tool_shim: moves values between the page and `tool_core::api::Api`.
//!
//! Builder requirement: the shim decides nothing. Each method is one call
//! into the core. `crates/core/tests/boundary.rs` fails if a branch, a loop or
//! the `?` operator appears here; if a feature seems to need one, the rule
//! belongs in the core. The page's timestamp is milliseconds with a
//! fraction; it becomes whole microseconds here by one multiplication and a
//! cast, which is a conversion and not a decision.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Galaxy {
    inner: tool_core::api::Api,
}

#[wasm_bindgen]
impl Galaxy {
    #[wasm_bindgen(constructor)]
    pub fn new(content: &str, strings: &str, seed_hi: u32, seed_lo: u32, build: &str) -> Galaxy {
        Galaxy {
            inner: tool_core::api::Api::new(content, strings, seed_hi, seed_lo, build),
        }
    }

    pub fn error(&self) -> String {
        self.inner.error()
    }

    pub fn assets(&self) -> String {
        self.inner.assets()
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.inner.resize(w, h)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn advance(&mut self, now_ms: f64, bits: u32, sx: i32, sy: i32, px: i32, py: i32, pointer: bool) {
        self.inner
            .advance((now_ms * 1000.0) as i64, bits, sx, sy, px, py, pointer)
    }

    pub fn draw_list(&self) -> Vec<i32> {
        self.inner.draw_list()
    }

    pub fn ui(&self) -> String {
        self.inner.ui()
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.inner.act(name, value)
    }

    pub fn save_text(&self) -> String {
        self.inner.save_text()
    }

    pub fn save_name(&self) -> String {
        self.inner.save_name()
    }

    pub fn take_autosave(&mut self) -> String {
        self.inner.take_autosave()
    }

    pub fn restore(&mut self, text: &str) -> String {
        self.inner.restore(text)
    }

    pub fn storage_ok(&mut self, ok: bool) {
        self.inner.storage_ok(ok)
    }

    pub fn host(&mut self, n_hi: u32, n_lo: u32, app_id: &str) -> String {
        self.inner.host(n_hi, n_lo, app_id)
    }

    pub fn join(&mut self, code: &str, app_id: &str) -> String {
        self.inner.join(code, app_id)
    }

    pub fn net_peer(&mut self, peer: u32) {
        self.inner.net_peer(peer)
    }

    pub fn net_left(&mut self, peer: u32) {
        self.inner.net_left(peer)
    }

    pub fn net_message(&mut self, peer: u32, text: &str) {
        self.inner.net_message(peer, text)
    }

    pub fn net_outbox(&mut self) -> String {
        self.inner.net_outbox()
    }

    pub fn net_error(&mut self, key: &str) {
        self.inner.net_error(key)
    }

    pub fn net_active(&self) -> bool {
        self.inner.net_active()
    }

    pub fn net_status(&self) -> String {
        self.inner.net_status()
    }

    pub fn tick(&self) -> u32 {
        self.inner.tick()
    }

    pub fn say(&self, key: &str) -> String {
        self.inner.say(key)
    }
}
