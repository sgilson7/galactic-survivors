//! tool_core: every rule of Galactic Survivors.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.

pub mod api;
pub mod arsenal;
pub mod content;
pub mod draw;
pub mod fx;
pub mod game;
pub mod geom;
pub mod glossary;
pub mod meta;
pub mod offer;
pub mod online;
pub mod render;
pub mod rng;
pub mod text;
pub mod ticks;
pub mod view;
pub mod world;
#[cfg(test)]
mod world_tests;
