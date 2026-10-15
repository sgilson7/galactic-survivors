//! Runes (T14): shapes drawn by walking. The core keeps a point of each
//! player's trail every `runes.sample_px` pixels. When the newest step
//! crosses an older one, the trail between them is a closed loop. Counting
//! its sharp corners names the shape: none is a circle, three a triangle,
//! four a square; anything else is no rune. A rune strikes every enemy
//! inside the loop and gives a stacking passive for a while.
//!
//! The geometry is whole pixels in i64, so a long trail cannot overflow and
//! two browsers agree.

use crate::content::Content;
use crate::geom::{isqrt, px, ticks, V};
use crate::world::{Effect, EffectKind, RuneStack, World};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Circle,
    Triangle,
    Square,
}

impl Shape {
    pub fn name(self) -> &'static str {
        match self {
            Shape::Circle => "circle",
            Shape::Triangle => "triangle",
            Shape::Square => "square",
        }
    }

    pub const ALL: [Shape; 3] = [Shape::Circle, Shape::Triangle, Shape::Square];
}

/// The id a rune's strike is counted under.
pub const RUNE_SOURCE: &str = "rune";

type P = (i64, i64);

fn whole(v: V) -> P {
    let sub = crate::geom::SUB as i64;
    ((v.x as i64).div_euclid(sub), (v.y as i64).div_euclid(sub))
}

/// Twice the loop's area, always positive (the shoelace sum).
pub fn area2(pts: &[P]) -> i64 {
    let n = pts.len();
    let mut s = 0i64;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        s += a.0 * b.1 - b.0 * a.1;
    }
    s.abs()
}

/// Whether a point is inside the loop (even-odd rule).
pub fn inside(pts: &[P], q: P) -> bool {
    let n = pts.len();
    let mut c = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (pts[i], pts[j]);
        if (a.1 > q.1) != (b.1 > q.1) {
            // x of the edge at q's height, compared without division
            let lhs = (q.0 - a.0) * (b.1 - a.1);
            let rhs = (b.0 - a.0) * (q.1 - a.1);
            if (b.1 - a.1 > 0 && lhs < rhs) || (b.1 - a.1 < 0 && lhs > rhs) {
                c = !c;
            }
        }
        j = i;
    }
    c
}

fn orient(a: P, b: P, c: P) -> i64 {
    ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).signum()
}

/// Whether segments ab and cd cross (touching ends count).
pub fn crosses(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
    o1 != o2 && o3 != o4
}

/// The shape of a closed loop, if it is one. A point is a corner when the
/// direction two points back and two points on differ by more than the
/// corner angle; corners next to each other count once.
pub fn classify(pts: &[P], corner_cos: i64) -> Option<Shape> {
    let n = pts.len();
    if n < 6 {
        return None;
    }
    let sharp: Vec<bool> = (0..n)
        .map(|i| {
            let back = pts[(i + n - 2) % n];
            let here = pts[i];
            let on = pts[(i + 2) % n];
            let a = (here.0 - back.0, here.1 - back.1);
            let b = (on.0 - here.0, on.1 - here.1);
            let dot = a.0 * b.0 + a.1 * b.1;
            let la = isqrt(a.0 * a.0 + a.1 * a.1);
            let lb = isqrt(b.0 * b.0 + b.1 * b.1);
            la > 0 && lb > 0 && dot * 1000 < corner_cos * la * lb
        })
        .collect();
    // count runs of sharp points around the loop
    let mut corners = 0;
    for i in 0..n {
        if sharp[i] && !sharp[(i + n - 1) % n] {
            corners += 1;
        }
    }
    if corners == 0 && sharp.iter().all(|s| *s) {
        return None;
    }
    match corners {
        0 => Some(Shape::Circle),
        3 => Some(Shape::Triangle),
        4 => Some(Shape::Square),
        _ => None,
    }
}

impl World {
    /// Adds the player's new position to their trail and draws a rune if the
    /// trail just closed on itself. Called once a tick after moving.
    pub fn trace(&mut self, content: &Content, seat: usize) {
        let r = &content.runes;
        if r.sample_px <= 0 {
            return;
        }
        let p = &mut self.players[seat];
        p.rune_at = None;
        if !p.alive {
            p.trail.clear();
            return;
        }
        let here = whole(p.pos);
        let Some(&last) = p.trail.last() else {
            p.trail.push(p.pos);
            return;
        };
        let lp = whole(last);
        let d2 = (here.0 - lp.0).pow(2) + (here.1 - lp.1).pow(2);
        let step = r.sample_px as i64;
        if d2 > (step * 6) * (step * 6) {
            // a jump (a rift, a revive) is not a stroke
            p.trail.clear();
            p.trail.push(p.pos);
            return;
        }
        if d2 < step * step {
            return;
        }
        p.trail.push(p.pos);
        if p.trail.len() > r.max_points {
            p.trail.remove(0);
        }
        let pts: Vec<P> = p.trail.iter().map(|v| whole(*v)).collect();
        let n = pts.len();
        if n < 4 {
            return;
        }
        let (c, d) = (pts[n - 2], pts[n - 1]);
        let Some(j) = (0..n.saturating_sub(3))
            .rev()
            .find(|&j| crosses(pts[j], pts[j + 1], c, d))
        else {
            return;
        };
        let ring = &pts[j + 1..n - 1];
        let shape = if ring.len() >= r.min_points && area2(ring) >= 2 * r.min_area {
            classify(ring, r.corner_cos)
        } else {
            None
        };
        let p = &mut self.players[seat];
        let Some(shape) = shape else {
            // an untidy loop: forget it so it is not found again
            p.trail.drain(..=j);
            return;
        };
        let ring: Vec<P> = ring.to_vec();
        p.trail.clear();
        p.trail.push(p.pos);
        self.draw_rune(content, seat, shape, &ring);
    }

    fn draw_rune(&mut self, content: &Content, seat: usize, shape: Shape, ring: &[P]) {
        let r = &content.runes;
        let level = self.stat_total(content, seat).rune_level;
        let n = ring.len() as i64;
        let cx = ring.iter().map(|q| q.0).sum::<i64>() / n;
        let cy = ring.iter().map(|q| q.1).sum::<i64>() / n;
        let centre = V::new(px(cx as i32), px(cy as i32));
        // strike everything inside the loop
        let damage = r.damage + 2 * self.level;
        let hits: Vec<usize> = (0..self.foes.len())
            .filter(|&i| self.foes[i].hp > 0 && inside(ring, whole(self.foes[i].pos)))
            .collect();
        for i in hits {
            self.hurt_foe(content, i, damage, seat, RUNE_SOURCE, V::ZERO);
        }
        let most = r.stacks + level;
        let left = ticks(r.ms) * (100 + 25 * level) / 100;
        let p = &mut self.players[seat];
        p.rune_stacks.push(RuneStack { shape, left });
        while p.rune_stacks.iter().filter(|s| s.shape == shape).count() as i32 > most {
            let k = p.rune_stacks.iter().position(|s| s.shape == shape).unwrap_or(0);
            p.rune_stacks.remove(k);
        }
        *p.record.runes.entry(shape.name().to_string()).or_insert(0) += 1;
        p.rune_at = Some(centre);
        let size = isqrt(area2(ring) / 6) as i32;
        let effect = Effect {
            kind: EffectKind::Ring,
            seat,
            ability: RUNE_SOURCE.into(),
            pos: centre,
            dir: V::ZERO,
            radius: size.max(30),
            width: 0,
            damage: 0,
            life: ticks(450),
            full: ticks(450),
            every: 0,
            cd: 0,
            count: 0,
            angle: 0,
            spin: 0,
            armed: false,
            points: ring
                .iter()
                .map(|q| V::new(px(q.0 as i32), px(q.1 as i32)))
                .collect(),
            hits: Vec::new(),
            shot_speed: 0,
            shot_radius: 0,
            shot_range: 0,
            anchored: true,
            carry: None,
            field: None,
            burst_pct: 0,
            pull: 0,
            slow_pct: 0,
            slow: 0,
        };
        self.effects.push(effect);
    }

    /// Rune stacks wear off.
    pub fn wear_runes(&mut self, seat: usize) {
        let p = &mut self.players[seat];
        for s in p.rune_stacks.iter_mut() {
            s.left -= 1;
        }
        p.rune_stacks.retain(|s| s.left > 0);
    }

    pub fn rune_count(&self, seat: usize, shape: Shape) -> i32 {
        self.players[seat]
            .rune_stacks
            .iter()
            .filter(|s| s.shape == shape)
            .count() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A walked outline: the corners joined by points every `step` pixels.
    fn walk(corners: &[P], step: i64) -> Vec<P> {
        let mut out = Vec::new();
        for k in 0..corners.len() {
            let (a, b) = (corners[k], corners[(k + 1) % corners.len()]);
            let len = isqrt((b.0 - a.0).pow(2) + (b.1 - a.1).pow(2));
            let n = (len / step).max(1);
            for s in 0..n {
                out.push((a.0 + (b.0 - a.0) * s / n, a.1 + (b.1 - a.1) * s / n));
            }
        }
        out
    }

    fn circle(r: i32, n: i32) -> Vec<P> {
        (0..n)
            .map(|k| whole(crate::geom::from_angle(crate::fx::TURN * k / n, px(r))))
            .collect()
    }

    const COS: i64 = 500; // 60 degrees

    #[test]
    fn a_walked_triangle_square_and_circle_are_told_apart() {
        let tri = walk(&[(0, 0), (240, 0), (120, -200)], 24);
        let sq = walk(&[(0, 0), (200, 0), (200, 200), (0, 200)], 24);
        // eight-way keys draw a circle as an octagon
        let oct = walk(
            &[
                (100, 0),
                (240, 0),
                (340, 100),
                (340, 240),
                (240, 340),
                (100, 340),
                (0, 240),
                (0, 100),
            ],
            24,
        );
        assert_eq!(classify(&tri, COS), Some(Shape::Triangle));
        assert_eq!(classify(&sq, COS), Some(Shape::Square));
        assert_eq!(
            classify(&oct, COS),
            Some(Shape::Circle),
            "45 degree turns are not corners"
        );
        assert_eq!(classify(&circle(150, 40), COS), Some(Shape::Circle));
    }

    #[test]
    fn a_pentagon_and_a_two_cornered_loop_are_no_rune() {
        // a half circle closed by its diameter: two corners
        // sampled every 24 pixels, as a walked trail is
        let arc: Vec<P> = (0..=20)
            .map(|k| whole(crate::geom::from_angle(crate::fx::TURN / 2 * k / 20, px(150))))
            .collect();
        let diameter = (1..12).map(|k| (-150 + 25 * k, 0));
        let half: Vec<P> = arc.into_iter().chain(diameter).collect();
        assert_eq!(classify(&half, COS), None);
        let star = walk(&[(0, -200), (120, 160), (-190, -60), (190, -60), (-120, 160)], 24);
        assert_eq!(classify(&star, COS), None);
    }

    #[test]
    fn area_and_inside_work_on_whole_pixels() {
        let sq = [(0, 0), (100, 0), (100, 100), (0, 100)];
        assert_eq!(area2(&sq), 20_000);
        assert!(inside(&sq, (50, 50)));
        assert!(!inside(&sq, (150, 50)));
        assert!(!inside(&sq, (-1, 50)));
        assert!(crosses((0, 0), (10, 10), (0, 10), (10, 0)));
        assert!(!crosses((0, 0), (10, 0), (0, 5), (10, 5)));
    }
}
