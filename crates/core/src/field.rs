//! Ground fields (T14): patches of fire, frost, charge and slick that
//! payloads leave where they land, after Noita's materials. A field hurts
//! or slows what stands in it, and two kinds that meet react:
//!
//! * fire on slick flares: a burst over the slick, which then burns;
//! * fire on frost (either way round) makes steam, which slows hard;
//! * charge on slick or frost conducts: everything in the other field is
//!   struck at once.
//!
//! A new field on one of its own kind merges into it, so a stream of hits
//! keeps one patch alive instead of stacking dozens. There are at most
//! `fields.cap`; the oldest goes first.

use crate::content::{Content, FieldDef};
use crate::geom::{px, ticks, within, V};
use crate::world::World;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Fire,
    Frost,
    Charge,
    Slick,
    Steam,
}

impl FieldKind {
    pub fn parse(name: &str) -> Option<FieldKind> {
        Some(match name {
            "fire" => FieldKind::Fire,
            "frost" => FieldKind::Frost,
            "charge" => FieldKind::Charge,
            "slick" => FieldKind::Slick,
            "steam" => FieldKind::Steam,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            FieldKind::Fire => "fire",
            FieldKind::Frost => "frost",
            FieldKind::Charge => "charge",
            FieldKind::Slick => "slick",
            FieldKind::Steam => "steam",
        }
    }

    pub fn def(self, content: &Content) -> &FieldDef {
        let f = &content.fields;
        match self {
            FieldKind::Fire => &f.fire,
            FieldKind::Frost => &f.frost,
            FieldKind::Charge => &f.charge,
            FieldKind::Slick => &f.slick,
            FieldKind::Steam => &f.steam,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub kind: FieldKind,
    pub pos: V,
    /// In pixels.
    pub radius: i32,
    pub left: i32,
    pub full: i32,
    pub cd: i32,
    /// Damage each tick of the field.
    pub damage: i32,
    pub seat: usize,
}

/// The id kills by fields are counted under.
pub const FIELD_SOURCE: &str = "field";

impl World {
    /// Leaves a field. `damage` is the payload's; the field deals its kind's
    /// share of it. `radius` 0 takes the kind's own.
    pub fn add_field(
        &mut self,
        content: &Content,
        kind: FieldKind,
        pos: V,
        radius: i32,
        damage: i32,
        seat: usize,
    ) {
        let def = kind.def(content);
        if def.ms <= 0 {
            return;
        }
        let radius = if radius > 0 {
            radius.min(def.max_radius.max(radius))
        } else {
            def.radius
        };
        let dmg = (damage * def.damage_pct / 100).max(if def.damage_pct > 0 { 1 } else { 0 });
        let full = ticks(def.ms);
        let mut k = 0;
        while k < self.fields.len() {
            let f = &self.fields[k];
            if !within(f.pos, pos, px(f.radius + radius)) {
                k += 1;
                continue;
            }
            match (kind, f.kind) {
                (a, b) if a == b && within(f.pos, pos, px(f.radius)) => {
                    // merge into the patch already burning there
                    let grow = (f.radius + radius / 4).min(def.max_radius.max(f.radius));
                    let f = &mut self.fields[k];
                    f.left = f.left.max(full);
                    f.full = f.full.max(full);
                    f.radius = grow;
                    f.damage = f.damage.max(dmg);
                    return;
                }
                (FieldKind::Fire, FieldKind::Slick) | (FieldKind::Slick, FieldKind::Fire) => {
                    let fire_dmg = if kind == FieldKind::Fire { dmg } else { f.damage };
                    let (at, r) = (f.pos, f.radius.max(radius));
                    let burst = (fire_dmg * content.fields.flare_pct / 100).max(1);
                    self.strike_area(content, at, r, burst, seat);
                    let fire = &content.fields.fire;
                    let f = &mut self.fields[k];
                    f.kind = FieldKind::Fire;
                    f.radius = r.min(fire.max_radius.max(r));
                    f.left = ticks(fire.ms);
                    f.full = f.left;
                    f.damage = fire_dmg;
                    self.reacted(seat);
                    return;
                }
                (FieldKind::Fire, FieldKind::Frost) | (FieldKind::Frost, FieldKind::Fire) => {
                    let at = f.pos.add(pos.sub(f.pos).scale(1, 2));
                    let r = f.radius.max(radius);
                    self.fields.remove(k);
                    let steam = &content.fields.steam;
                    self.fields.push(Field {
                        kind: FieldKind::Steam,
                        pos: at,
                        radius: r,
                        left: ticks(steam.ms),
                        full: ticks(steam.ms),
                        cd: 0,
                        damage: 0,
                        seat,
                    });
                    self.reacted(seat);
                    return;
                }
                (FieldKind::Charge, FieldKind::Slick | FieldKind::Frost)
                | (FieldKind::Slick | FieldKind::Frost, FieldKind::Charge) => {
                    let charge_dmg = if kind == FieldKind::Charge { dmg } else { f.damage };
                    let (at, r) = if kind == FieldKind::Charge {
                        (f.pos, f.radius)
                    } else {
                        (pos, radius)
                    };
                    let hit = (charge_dmg * content.fields.conduct_pct / 100).max(1);
                    self.strike_area(content, at, r, hit, seat);
                    self.reacted(seat);
                    k += 1;
                }
                _ => k += 1,
            }
        }
        if self.fields.len() >= content.fields.cap.max(1) {
            self.fields.remove(0);
        }
        self.fields.push(Field {
            kind,
            pos,
            radius,
            left: full,
            full,
            cd: 0,
            damage: dmg,
            seat,
        });
    }

    fn reacted(&mut self, seat: usize) {
        if let Some(p) = self.players.get_mut(seat) {
            p.record.reactions += 1;
        }
    }

    /// Hurts every living foe within `radius` pixels of `at`; returns how
    /// many fell.
    pub fn strike_area(&mut self, content: &Content, at: V, radius: i32, damage: i32, seat: usize) -> u32 {
        let mut fell = 0;
        for i in self.near(content, at, radius + 40) {
            let f = &self.foes[i];
            if f.hp > 0 && within(f.pos, at, px(radius + content.enemies[f.kind].radius)) {
                let killed = self.hurt_foe(content, i, damage, seat, FIELD_SOURCE, V::ZERO);
                if killed {
                    fell += 1;
                }
            }
        }
        if let Some(p) = self.players.get_mut(seat) {
            p.record.field_kills += fell;
        }
        fell
    }

    /// Whether a player stands in one of their own fire fields (Heat Sink).
    pub fn in_own_fire(&self, seat: usize) -> bool {
        let pos = self.players[seat].pos;
        self.fields
            .iter()
            .any(|f| f.kind == FieldKind::Fire && f.seat == seat && within(f.pos, pos, px(f.radius)))
    }

    /// One tick of every field: burn, chill, zap; then the spent ones go.
    pub fn update_fields(&mut self, content: &Content) {
        for k in 0..self.fields.len() {
            let def = self.fields[k].kind.def(content).clone();
            let f = &mut self.fields[k];
            f.left -= 1;
            f.cd -= 1;
            if f.cd > 0 {
                continue;
            }
            f.cd = ticks(def.tick_ms).max(1);
            let (kind, at, r, dmg, seat) = (f.kind, f.pos, f.radius, f.damage, f.seat);
            match kind {
                FieldKind::Fire => {
                    self.strike_area(content, at, r, dmg, seat);
                }
                FieldKind::Charge => {
                    // one zap at the nearest enemy standing in it
                    let near = self
                        .near(content, at, r + 40)
                        .into_iter()
                        .filter(|&i| self.foes[i].hp > 0 && within(self.foes[i].pos, at, px(r)))
                        .min_by_key(|&i| (crate::geom::dist2(at, self.foes[i].pos), self.foes[i].id));
                    if let Some(i) = near {
                        if self.hurt_foe(content, i, dmg, seat, FIELD_SOURCE, V::ZERO) {
                            if let Some(p) = self.players.get_mut(seat) {
                                p.record.field_kills += 1;
                            }
                        }
                    }
                }
                FieldKind::Frost | FieldKind::Slick | FieldKind::Steam => {
                    let slow = ticks(def.tick_ms) + 2;
                    for i in self.near(content, at, r + 40) {
                        let f = &mut self.foes[i];
                        if f.hp > 0 && within(f.pos, at, px(r)) {
                            f.slow = f.slow.max(slow);
                            f.slow_pct = f.slow_pct.max(def.slow_pct);
                        }
                    }
                }
            }
        }
        self.fields.retain(|f| f.left > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::shipped;
    use crate::meta::Save;

    fn world() -> (Content, World) {
        let mut c = shipped();
        c.waves.clear();
        c.events.minibosses.clear();
        c.events.rifts_s.clear();
        c.events.raid.from_s = 100_000;
        c.events.raid.to_s = 100_000;
        c.events.final_boss.at_s = 100_000;
        let load = Save::new(&c, 1).loadout(&c, "pathfinder");
        let w = World::new(&c, 3, &[load]);
        (c, w)
    }

    fn foe_at(c: &Content, w: &mut World, x: i32) -> usize {
        let k = c.enemy_index("carapace");
        let i = w.spawn_foe(c, k, V::new(px(x), px(300)));
        w.foes[i].hp = 10_000;
        w.foes[i].max_hp = 10_000;
        i
    }

    #[test]
    fn a_fire_field_burns_what_stands_in_it_and_not_what_stands_outside() {
        let (c, mut w) = world();
        let inside = foe_at(&c, &mut w, 0);
        let outside = foe_at(&c, &mut w, 400);
        w.add_field(&c, FieldKind::Fire, V::new(0, px(300)), 60, 100, 0);
        // fire deals damage_pct of 100 on its first tick
        w.update_fields(&c);
        let burn = 100 * c.fields.fire.damage_pct / 100;
        assert_eq!(w.foes[inside].hp, 10_000 - burn);
        assert_eq!(w.foes[outside].hp, 10_000);
    }

    #[test]
    fn a_field_on_its_own_kind_merges_instead_of_stacking() {
        let (c, mut w) = world();
        for _ in 0..10 {
            w.add_field(&c, FieldKind::Fire, V::new(px(5), px(300)), 40, 50, 0);
        }
        assert_eq!(w.fields.len(), 1);
        assert!(w.fields[0].radius <= c.fields.fire.max_radius);
    }

    #[test]
    fn fire_on_slick_flares_and_the_slick_burns() {
        let (c, mut w) = world();
        let i = foe_at(&c, &mut w, 0);
        w.add_field(&c, FieldKind::Slick, V::new(0, px(300)), 80, 100, 0);
        w.add_field(&c, FieldKind::Fire, V::new(px(30), px(300)), 40, 100, 0);
        assert_eq!(w.fields.len(), 1, "the slick became the fire");
        assert_eq!(w.fields[0].kind, FieldKind::Fire);
        // the flare: flare_pct of the fire's damage (damage_pct of 100)
        let fire = 100 * c.fields.fire.damage_pct / 100;
        assert_eq!(w.foes[i].hp, 10_000 - fire * c.fields.flare_pct / 100);
        assert_eq!(w.players[0].record.reactions, 1);
    }

    #[test]
    fn fire_meeting_frost_makes_steam_either_way_round() {
        for (a, b) in [
            (FieldKind::Fire, FieldKind::Frost),
            (FieldKind::Frost, FieldKind::Fire),
        ] {
            let (c, mut w) = world();
            w.add_field(&c, a, V::new(0, px(300)), 60, 100, 0);
            w.add_field(&c, b, V::new(px(20), px(300)), 60, 100, 0);
            assert_eq!(w.fields.len(), 1);
            assert_eq!(w.fields[0].kind, FieldKind::Steam);
        }
    }

    #[test]
    fn charge_on_slick_strikes_everything_in_the_slick() {
        let (c, mut w) = world();
        let a = foe_at(&c, &mut w, -40);
        let b = foe_at(&c, &mut w, 40);
        w.add_field(&c, FieldKind::Slick, V::new(0, px(300)), 90, 100, 0);
        w.add_field(&c, FieldKind::Charge, V::new(px(60), px(300)), 30, 100, 0);
        let charge = 100 * c.fields.charge.damage_pct / 100;
        let hit = charge * c.fields.conduct_pct / 100;
        assert_eq!(w.foes[a].hp, 10_000 - hit);
        assert_eq!(w.foes[b].hp, 10_000 - hit);
        assert_eq!(w.fields.len(), 2, "both fields stay");
    }

    #[test]
    fn there_are_never_more_fields_than_the_cap() {
        let (c, mut w) = world();
        for k in 0..(c.fields.cap as i32 + 10) {
            w.add_field(&c, FieldKind::Frost, V::new(px(k * 300), 0), 30, 10, 0);
        }
        assert_eq!(w.fields.len(), c.fields.cap);
    }

    #[test]
    fn a_field_runs_out() {
        let (c, mut w) = world();
        w.add_field(&c, FieldKind::Frost, V::ZERO, 30, 10, 0);
        for _ in 0..ticks(c.fields.frost.ms) {
            w.update_fields(&c);
        }
        assert!(w.fields.is_empty());
    }
}
