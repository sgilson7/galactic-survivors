//! Every picture the core names exists under web/art, so the page never asks
//! for a file that is not there.
use std::path::Path;
use tool_core::draw::{GROUND, SPRITES};

#[test]
fn every_sprite_name_has_a_picture_in_web_art() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/art");
    for name in SPRITES.iter().chain(std::iter::once(&GROUND)) {
        assert!(root.join(format!("{name}.png")).exists(), "{name}");
    }
}
