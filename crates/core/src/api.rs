//! The door into the core. The shim holds one `Api` and forwards each call;
//! every function takes and returns ordinary values (strings, integers, a
//! list of integers), and `act` answers `{"ok": ui}` or `{"error": "..."}`
//! with a message the page shows as it is (R4).

use crate::game::{Game, Raw};
use crate::{draw, render, view};

pub struct Api {
    game: Option<Game>,
    error: String,
}

fn ok_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error_json(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

impl Api {
    /// Starts the game from the content and strings files, a seed from the
    /// page (two 32-bit halves) and the build's name. A file that cannot be
    /// read leaves the reason in `error()` instead of panicking.
    pub fn new(content_json: &str, strings_json: &str, seed_hi: u32, seed_lo: u32, build: &str) -> Api {
        let seed = (seed_hi as u64) << 32 | seed_lo as u64;
        match Game::new(content_json, strings_json, seed, build) {
            Ok(g) => Api {
                game: Some(g),
                error: String::new(),
            },
            Err(e) => Api { game: None, error: e },
        }
    }

    /// Empty when the game started; otherwise why it could not.
    pub fn error(&self) -> String {
        self.error.clone()
    }

    /// The draw list's vocabulary and the input bits, read once by the page.
    pub fn assets(&self) -> String {
        let assets: serde_json::Value = serde_json::from_str(&draw::assets()).expect("assets are JSON");
        let bits: serde_json::Value = serde_json::from_str(&Game::bits_json()).expect("bits are JSON");
        serde_json::json!({ "draw": assets, "bits": bits }).to_string()
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        if let Some(g) = self.game.as_mut() {
            g.resize(w, h);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn advance(
        &mut self,
        now_us: i64,
        bits: u32,
        stick_x: i32,
        stick_y: i32,
        px: i32,
        py: i32,
        pointer: bool,
    ) {
        if let Some(g) = self.game.as_mut() {
            g.advance(
                now_us,
                Raw {
                    bits,
                    stick_x,
                    stick_y,
                    pointer_x: px,
                    pointer_y: py,
                    pointer_on: pointer,
                },
            );
        }
    }

    pub fn draw_list(&self) -> Vec<i32> {
        self.game.as_ref().map(render::frame).unwrap_or_default()
    }

    pub fn ui(&self) -> String {
        match self.game.as_ref() {
            Some(g) => ok_json(&view::build(g)),
            None => error_json(&self.error),
        }
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        let Some(g) = self.game.as_mut() else {
            return error_json(&self.error);
        };
        match g.act(name, value) {
            Ok(()) => ok_json(&view::build(g)),
            Err(e) => error_json(&e),
        }
    }

    pub fn save_text(&self) -> String {
        self.game.as_ref().map(|g| g.save_text()).unwrap_or_default()
    }

    /// The save to store in this browser, or empty when it has not changed.
    pub fn take_autosave(&mut self) -> String {
        self.game
            .as_mut()
            .and_then(|g| g.take_autosave())
            .unwrap_or_default()
    }

    /// Restores the save this browser kept; returns a save to set aside
    /// because it could not be read, or empty.
    pub fn restore(&mut self, text: &str) -> String {
        self.game.as_mut().map(|g| g.restore(text)).unwrap_or_default()
    }

    pub fn storage_ok(&mut self, ok: bool) {
        if let Some(g) = self.game.as_mut() {
            g.storage_ok(ok);
        }
    }

    /// The file name for a downloaded save.
    pub fn save_name(&self) -> String {
        "galactic-survivors-save.json".into()
    }

    pub fn host(&mut self, n_hi: u32, n_lo: u32, app_id: &str) -> String {
        let n = (n_hi as u64) << 32 | n_lo as u64;
        self.game.as_mut().map(|g| g.host(n, app_id)).unwrap_or_default()
    }

    /// The room name for a typed code, or empty when it cannot be a code.
    pub fn join(&mut self, code: &str, app_id: &str) -> String {
        self.game
            .as_mut()
            .and_then(|g| g.join(code, app_id))
            .unwrap_or_default()
    }

    pub fn net_peer(&mut self, peer: u32) {
        if let Some(g) = self.game.as_mut() {
            g.net_peer(peer);
        }
    }

    pub fn net_left(&mut self, peer: u32) {
        if let Some(g) = self.game.as_mut() {
            g.net_left(peer);
        }
    }

    pub fn net_message(&mut self, peer: u32, text: &str) {
        if let Some(g) = self.game.as_mut() {
            g.net_message(peer, text);
        }
    }

    /// Everything to send, as a JSON list of [peer, text, fast]: fast
    /// messages may take the channel that drops and reorders.
    pub fn net_outbox(&mut self) -> String {
        let out = self.game.as_mut().map(|g| g.net_outbox()).unwrap_or_default();
        serde_json::to_string(&out).expect("outbox serialises")
    }

    pub fn net_error(&mut self, key: &str) {
        if let Some(g) = self.game.as_mut() {
            g.set_net_error(key);
        }
    }

    /// Whether the transport should stay open.
    pub fn net_active(&self) -> bool {
        self.game.as_ref().map(|g| g.session.is_some()).unwrap_or(false)
    }

    /// The co-op session's status key, or empty without one.
    pub fn net_status(&self) -> String {
        self.game
            .as_ref()
            .and_then(|g| g.session.as_ref())
            .map(|s| s.status.to_string())
            .unwrap_or_default()
    }

    /// The run's tick, or 0 outside a run.
    pub fn tick(&self) -> u32 {
        self.game
            .as_ref()
            .and_then(|g| g.world.as_ref())
            .map(|w| w.tick)
            .unwrap_or(0)
    }

    /// A string from data/strings.json, for the page's own prompts.
    pub fn say(&self, key: &str) -> String {
        self.game.as_ref().map(|g| g.text.get(key)).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api() -> Api {
        Api::new(
            include_str!("../../../data/content.json"),
            include_str!("../../../data/strings.json"),
            0,
            5,
            "t",
        )
    }

    #[test]
    fn an_unknown_action_answers_with_an_error() {
        let mut a = api();
        assert!(a.error().is_empty());
        assert!(a.act("no.such", "").contains("\"error\""));
        assert!(a.act("screen.show", "shop").contains("\"ok\""));
    }

    #[test]
    fn a_broken_content_file_is_reported_not_panicked() {
        let a = Api::new("{", include_str!("../../../data/strings.json"), 0, 1, "t");
        assert!(a.error().contains("content.json"));
        assert!(a.ui().contains("\"error\""));
    }

    #[test]
    fn the_assets_name_ops_and_bits() {
        let v: serde_json::Value = serde_json::from_str(&api().assets()).unwrap();
        assert_eq!(v["bits"]["dash"], 16);
        assert_eq!(v["draw"]["ops"]["sprite"], 3);
    }
}
