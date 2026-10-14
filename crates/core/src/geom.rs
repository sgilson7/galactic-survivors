//! Integer geometry for the run. A position is in sub-pixels (`SUB` to a
//! pixel) so slow movement still moves; squared distances are `i64` so a
//! map's width squared never overflows. Angles are turns, through `fx.rs`.

use crate::fx;
use serde::{Deserialize, Serialize};

/// Sub-pixels in a pixel.
pub const SUB: i32 = 256;
/// Ticks in a second. The clock in `ticks.rs` runs at this rate.
pub const TPS: i32 = 60;

/// Pixels to sub-pixels.
pub const fn px(v: i32) -> i32 {
    v * SUB
}

/// A speed in pixels a second, as sub-pixels a tick.
pub const fn per_tick(px_per_s: i32) -> i32 {
    px_per_s * SUB / TPS
}

/// Milliseconds as ticks, at least one for any positive time.
pub fn ticks(ms: i32) -> i32 {
    if ms <= 0 {
        0
    } else {
        (ms * TPS / 1000).max(1)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct V {
    pub x: i32,
    pub y: i32,
}

// saturating add and sub, named for what they do on a point; no operator
// overloads, so every saturating step reads as a call
#[allow(clippy::should_implement_trait, clippy::len_without_is_empty)]
impl V {
    pub const ZERO: V = V { x: 0, y: 0 };

    pub const fn new(x: i32, y: i32) -> V {
        V { x, y }
    }

    pub fn add(self, o: V) -> V {
        V::new(self.x.saturating_add(o.x), self.y.saturating_add(o.y))
    }

    pub fn sub(self, o: V) -> V {
        V::new(self.x.saturating_sub(o.x), self.y.saturating_sub(o.y))
    }

    pub fn is_zero(self) -> bool {
        self.x == 0 && self.y == 0
    }

    pub fn len2(self) -> i64 {
        self.x as i64 * self.x as i64 + self.y as i64 * self.y as i64
    }

    pub fn len(self) -> i64 {
        isqrt(self.len2())
    }

    /// The vector scaled to `length`, or zero for a zero vector.
    pub fn with_len(self, length: i32) -> V {
        let d = self.len();
        if d == 0 {
            return V::ZERO;
        }
        V::new(
            (self.x as i64 * length as i64 / d) as i32,
            (self.y as i64 * length as i64 / d) as i32,
        )
    }

    /// The vector times `num / den`.
    pub fn scale(self, num: i32, den: i32) -> V {
        if den == 0 {
            return V::ZERO;
        }
        V::new(
            (self.x as i64 * num as i64 / den as i64) as i32,
            (self.y as i64 * num as i64 / den as i64) as i32,
        )
    }

    /// The vector turned by `angle` turns.
    pub fn rotate(self, angle: i32) -> V {
        let c = fx::cos(angle) as i64;
        let s = fx::sin(angle) as i64;
        let x = self.x as i64;
        let y = self.y as i64;
        V::new(
            ((x * c - y * s) >> fx::SHIFT) as i32,
            ((x * s + y * c) >> fx::SHIFT) as i32,
        )
    }

    pub fn dot(self, o: V) -> i64 {
        self.x as i64 * o.x as i64 + self.y as i64 * o.y as i64
    }

    pub fn clamp_box(self, half: i32) -> V {
        V::new(self.x.clamp(-half, half), self.y.clamp(-half, half))
    }
}

/// The unit vector at `angle` turns, `length` long.
pub fn from_angle(angle: i32, length: i32) -> V {
    V::new(length, 0).rotate(angle)
}

pub fn dist2(a: V, b: V) -> i64 {
    b.sub(a).len2()
}

/// Whether `a` and `b` are within `r` of each other.
pub fn within(a: V, b: V, r: i32) -> bool {
    dist2(a, b) <= r as i64 * r as i64
}

pub fn isqrt(n: i64) -> i64 {
    if n < 2 {
        return n.max(0);
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Whether the angle between `dir` and `to` is within half of `width`
/// turns. A width of a full turn or more takes every direction.
pub fn in_arc(dir: V, to: V, width: i32) -> bool {
    if width >= fx::TURN {
        return true;
    }
    if to.is_zero() || dir.is_zero() {
        return true;
    }
    // compare cos(angle) with cos(width / 2), both scaled by the lengths
    let limit = fx::cos(width / 2) as i64;
    let lhs = dir.dot(to);
    let rhs = (dir.len() * to.len() * limit) >> fx::SHIFT;
    lhs >= rhs
}

/// The squared distance from `p` to the segment from `a` to `b`.
pub fn seg_dist2(p: V, a: V, b: V) -> i64 {
    let ab = b.sub(a);
    let ap = p.sub(a);
    let l2 = ab.len2();
    if l2 == 0 {
        return ap.len2();
    }
    let t = ap.dot(ab).clamp(0, l2);
    let cx = a.x as i64 + ab.x as i64 * t / l2;
    let cy = a.y as i64 + ab.y as i64 * t / l2;
    let dx = p.x as i64 - cx;
    let dy = p.y as i64 - cy;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vector_scaled_to_a_length_keeps_its_direction() {
        // a 3-4-5 triangle scaled to 10
        assert_eq!(V::new(3, 4).with_len(10), V::new(6, 8));
        assert_eq!(V::ZERO.with_len(10), V::ZERO);
    }

    #[test]
    fn a_quarter_turn_rotates_x_onto_y() {
        let r = V::new(1000, 0).rotate(fx::QUARTER_TURN);
        assert!(r.x.abs() <= 3 && (r.y - 1000).abs() <= 3, "{r:?}");
    }

    #[test]
    fn an_arc_takes_what_is_ahead_and_refuses_what_is_behind() {
        let ahead = V::new(100, 10);
        let behind = V::new(-100, 0);
        let width = fx::TURN / 3; // 120 degrees
        assert!(in_arc(V::new(1, 0), ahead, width));
        assert!(!in_arc(V::new(1, 0), behind, width));
        assert!(in_arc(V::new(1, 0), behind, fx::TURN));
    }

    #[test]
    fn distance_to_a_segment_uses_the_nearest_point() {
        // (5, 3) is 3 from the segment (0,0)-(10,0); (13, 4) is 5 from its end
        assert_eq!(seg_dist2(V::new(5, 3), V::ZERO, V::new(10, 0)), 9);
        assert_eq!(seg_dist2(V::new(13, 4), V::ZERO, V::new(10, 0)), 25);
    }

    #[test]
    fn milliseconds_become_at_least_one_tick() {
        assert_eq!(ticks(1000), 60);
        assert_eq!(ticks(5), 1);
        assert_eq!(ticks(0), 0);
    }
}
