//! The draw list: a flat list of integers the page replays onto a canvas
//! (after the canvas kit and Peter's renderer for Mandible). The core never
//! sends a colour, only a palette role whose value lives in the stylesheet,
//! and never a float: alpha is in thousandths, angles in turns.

use serde::Serialize;

/// Thousandths: alpha, and the unit the renderer divides by.
pub const UNIT: i32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Op {
    Clear = 0,
    Rect = 1,
    Circle = 2,
    Sprite = 3,
    Line = 4,
    Text = 5,
    Ground = 6,
    Wedge = 7,
}

pub const OPS: [(Op, &str); 8] = [
    (Op::Clear, "clear"),
    (Op::Rect, "rect"),
    (Op::Circle, "circle"),
    (Op::Sprite, "sprite"),
    (Op::Line, "line"),
    (Op::Text, "text"),
    (Op::Ground, "ground"),
    (Op::Wedge, "wedge"),
];

/// Palette roles. The stylesheet defines `--<role>` for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Color {
    None = 0,
    Ink,
    Ground,
    Panel,
    Text,
    Muted,
    Hp,
    HpBack,
    Xp,
    Shot,
    ShotRim,
    Bolt,
    Ring,
    Gold,
    Heal,
    Rift,
    Raid,
    Boss,
    Flash,
    Shadow,
    Seat0,
    Seat1,
    Ticket,
    Cache,
    Rock,
}

pub const PALETTE: [&str; 25] = [
    "none", "ink", "ground", "panel", "text", "muted", "hp", "hp-back", "xp", "shot", "shot-rim", "bolt",
    "ring", "gold", "heal", "rift", "raid", "boss", "flash", "shadow", "seat0", "seat1", "ticket", "cache",
    "rock",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Align {
    Left = 0,
    Center = 1,
    Right = 2,
}

pub const ALIGNS: [&str; 3] = ["left", "center", "right"];

/// Pictures the renderer loads from `art/<name>.png`, by index.
pub const SPRITES: [&str; 22] = [
    "players/pathfinder",
    "players/breaker",
    "players/vector",
    "enemies/skitter",
    "enemies/mote",
    "enemies/carapace",
    "enemies/spitter",
    "enemies/corp_drone",
    "enemies/corp_trooper",
    "enemies/stalker",
    "bosses/burrower",
    "bosses/strider",
    "bosses/matriarch",
    "fx/enemy_shot",
    "fx/xp_shard",
    "fx/gold_chip",
    "fx/health_cell",
    "fx/ticket",
    "fx/cache",
    "fx/rift",
    "ground/rock_pillar",
    "ground/wreck",
];

/// The ground tile, drawn as a repeating pattern.
pub const GROUND: &str = "ground/regolith";

pub fn sprite(name: &str) -> i32 {
    SPRITES
        .iter()
        .position(|s| *s == name)
        .map(|i| i as i32)
        .unwrap_or(-1)
}

#[derive(Debug, Default, Clone)]
pub struct DrawList {
    list: Vec<i32>,
}

impl DrawList {
    pub fn new() -> DrawList {
        DrawList::default()
    }

    fn op(&mut self, op: Op) {
        self.list.push(op as i32);
    }

    pub fn clear(&mut self, color: Color) {
        self.op(Op::Clear);
        self.list.push(color as i32);
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, fill: Color, alpha: i32) {
        self.op(Op::Rect);
        self.list.extend([x, y, w, h, fill as i32, alpha]);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn circle(&mut self, x: i32, y: i32, r: i32, fill: Color, stroke: Color, width: i32, alpha: i32) {
        self.op(Op::Circle);
        self.list
            .extend([x, y, r.max(1), fill as i32, stroke as i32, width, alpha]);
    }

    /// A picture centred on (x, y), `size` pixels wide, mirrored when
    /// `flip`. The list carries its top-left corner, so the page divides
    /// nothing.
    pub fn sprite(&mut self, index: i32, x: i32, y: i32, size: i32, flip: bool, alpha: i32) {
        if index < 0 {
            return;
        }
        self.op(Op::Sprite);
        self.list
            .extend([index, x - size / 2, y - size / 2, size, flip as i32, alpha]);
    }

    pub fn line(&mut self, points: &[(i32, i32)], stroke: Color, width: i32, alpha: i32) {
        self.op(Op::Line);
        self.list
            .extend([stroke as i32, width, alpha, points.len() as i32]);
        for (x, y) in points {
            self.list.extend([*x, *y]);
        }
    }

    pub fn text(&mut self, x: i32, y: i32, size: i32, fill: Color, align: Align, words: &str) {
        self.op(Op::Text);
        let codes: Vec<i32> = words.chars().map(|c| c as i32).collect();
        self.list
            .extend([x, y, size, fill as i32, align as i32, codes.len() as i32]);
        self.list.extend(codes);
    }

    /// The ground tile over the whole canvas, shifted by (dx, dy) and drawn
    /// `size` pixels a tile.
    pub fn ground(&mut self, dx: i32, dy: i32, size: i32) {
        self.op(Op::Ground);
        self.list.extend([dx, dy, size]);
    }

    /// A filled pie slice: centre, radius, start and end angle in turns.
    #[allow(clippy::too_many_arguments)]
    pub fn wedge(&mut self, x: i32, y: i32, r: i32, from: i32, to: i32, fill: Color, alpha: i32) {
        self.op(Op::Wedge);
        self.list.extend([x, y, r.max(1), from, to, fill as i32, alpha]);
    }

    pub fn finish(self) -> Vec<i32> {
        self.list
    }
}

#[derive(Serialize)]
struct Assets<'a> {
    ops: std::collections::BTreeMap<&'a str, i32>,
    palette: &'a [&'a str],
    aligns: &'a [&'a str],
    sprites: &'a [&'a str],
    ground: &'a str,
    unit: i32,
    turn: i32,
    /// A full turn in radians, in thousandths.
    tau: i32,
}

/// Everything the renderer needs once: op codes, palette roles, alignments,
/// picture names, and the units its numbers are in.
pub fn assets() -> String {
    let a = Assets {
        ops: OPS.iter().map(|(op, name)| (*name, *op as i32)).collect(),
        palette: &PALETTE,
        aligns: &ALIGNS,
        sprites: &SPRITES,
        ground: GROUND,
        unit: UNIT,
        turn: crate::fx::TURN,
        tau: 6283,
    };
    serde_json::to_string(&a).expect("assets serialise")
}

/// How many arguments each op takes before its variable part.
fn arity(code: i32) -> Option<usize> {
    Some(match code {
        0 => 1,
        1 => 6,
        2 => 7,
        3 => 6,
        4 => 4,
        5 => 6,
        6 => 3,
        7 => 7,
        _ => return None,
    })
}

/// Reads a list as the renderer does; the number of ops, or None when a
/// code is unknown or the list stops in the middle of an op.
pub fn walk(list: &[i32]) -> Option<usize> {
    let mut at = 0;
    let mut ops = 0;
    while at < list.len() {
        let code = list[at];
        let n = arity(code)?;
        let head = at + 1;
        if head + n > list.len() {
            return None;
        }
        let extra = match code {
            4 => list[head + 3] as usize * 2,
            5 => list[head + 5] as usize,
            _ => 0,
        };
        at = head + n + extra;
        if at > list.len() {
            return None;
        }
        ops += 1;
    }
    Some(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_op_writes_its_code_then_its_arguments() {
        let mut d = DrawList::new();
        d.clear(Color::Ground);
        d.circle(10, 20, 5, Color::Shot, Color::Ink, 2, 1000);
        d.sprite(3, 1, 2, 48, true, 900);
        d.text(0, 0, 12, Color::Text, Align::Center, "Hi");
        assert_eq!(
            d.finish(),
            vec![
                0, 2, 2, 10, 20, 5, 9, 1, 2, 1000, 3, 3, -23, -22, 48, 1, 900, 5, 0, 0, 12, 4, 1, 2, 72, 105
            ]
        );
    }

    #[test]
    fn a_built_list_walks_to_its_end_and_a_cut_one_does_not() {
        let mut d = DrawList::new();
        d.clear(Color::Ground);
        d.ground(1, 2, 256);
        d.rect(0, 0, 4, 4, Color::Panel, 500);
        d.line(&[(0, 0), (5, 5), (9, 1)], Color::Bolt, 2, 800);
        d.wedge(0, 0, 50, 0, 100, Color::Ring, 300);
        d.text(1, 1, 10, Color::Text, Align::Left, "abc");
        let list = d.finish();
        assert_eq!(walk(&list), Some(6));
        assert_eq!(walk(&list[..list.len() - 1]), None);
        assert_eq!(walk(&[99]), None);
    }

    #[test]
    fn the_palette_has_one_role_per_colour() {
        assert_eq!(PALETTE.len(), Color::Rock as usize + 1);
        assert_eq!(PALETTE[Color::Shot as usize], "shot");
        assert_eq!(PALETTE[Color::Seat1 as usize], "seat1");
    }
}
