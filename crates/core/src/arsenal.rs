//! The players' abilities: when each fires, where it aims, and what its
//! shots and effects do. Every ability is one of eight behaviours
//! (`content::Kind`) with numbers from the content file.

use crate::content::{Aim, Content, Kind, Level, On};
use crate::field::FieldKind;
use crate::geom::{dist2, from_angle, in_arc, per_tick, px, seg_dist2, ticks, within, V};
use crate::rig::{Carry, Origin};
use crate::world::{Effect, EffectKind, PickupKind, Shot, World};

/// What an ability does at one level, after the player's stats.
#[derive(Debug, Clone)]
pub struct Scaled {
    pub kind: Kind,
    pub aim: Aim,
    pub damage: i32,
    pub cooldown: i32,
    pub count: i32,
    pub speed: i32,
    pub radius: i32,
    pub area: i32,
    pub pierce: i32,
    pub bounce: i32,
    pub homing: bool,
    pub spread: i32,
    pub range: i32,
    pub width: i32,
    pub duration: i32,
    pub knockback: i32,
    pub pull: i32,
    pub slow_pct: i32,
    pub slow: i32,
    pub fuse: i32,
    pub every: i32,
    pub fire: i32,
    /// The ground field it leaves, from its level or a modifier (T14).
    pub field: Option<FieldKind>,
    pub burst_pct: i32,
    pub returns: bool,
    pub sweep: bool,
}

impl Scaled {
    /// Whether the payload is placed somewhere rather than sent in a
    /// direction: a formation spreads these out in space, not in angle.
    pub fn places(&self) -> bool {
        matches!(
            self.kind,
            Kind::Nova | Kind::Bomb | Kind::Strike | Kind::Well | Kind::Turret | Kind::Orbit
        )
    }

    /// A turret's shot, from the turret's numbers.
    fn for_turret_shot(e: &Effect) -> Scaled {
        Scaled {
            kind: Kind::Shot,
            aim: Aim::Nearest,
            damage: e.damage,
            cooldown: 1,
            count: 1,
            speed: e.shot_speed,
            radius: e.shot_radius,
            area: e.shot_radius,
            pierce: 1,
            bounce: 0,
            homing: false,
            spread: 0,
            range: e.shot_range,
            width: 0,
            duration: 0,
            knockback: 0,
            pull: 0,
            slow_pct: e.slow_pct,
            slow: e.slow,
            fuse: 0,
            every: 0,
            fire: 0,
            field: e.field,
            burst_pct: 0,
            returns: false,
            sweep: false,
        }
    }
}

impl World {
    /// The numbers an ability fires with for this player right now.
    pub fn scaled(&self, content: &Content, seat: usize, id: &str, level: i32) -> Option<Scaled> {
        let def = content.ability(id)?;
        let lv: &Level = def
            .levels
            .get((level - 1).clamp(0, def.levels.len() as i32 - 1) as usize)?;
        let s = self.stat_total(content, seat);
        let ch = &content.characters[self.players[seat].character];
        let dmg_pct = 100 + ch.damage_pct + s.damage_pct;
        let cd_pct = (100 - s.cooldown_pct).max(40);
        let fused = matches!(def.kind, Kind::Bomb | Kind::Strike);
        let area_pct = 100 + s.area_pct + if fused { s.fuse_area_pct } else { 0 };
        let extra = match def.kind {
            Kind::Shot | Kind::Bomb | Kind::Orbit | Kind::Turret | Kind::Strike => s.projectiles,
            Kind::Chain => s.projectiles * 2,
            _ => 0,
        };
        Some(Scaled {
            kind: def.kind,
            aim: def.aim,
            damage: (lv.damage * dmg_pct / 100).max(1),
            cooldown: ticks(lv.cooldown_ms * cd_pct / 100),
            count: (lv.count + extra).max(1),
            speed: per_tick(lv.speed),
            radius: lv.radius,
            area: lv.radius * area_pct / 100,
            pierce: lv.pierce.max(1),
            bounce: lv.bounce,
            homing: lv.homing > 0,
            spread: lv.spread,
            range: lv.range,
            width: lv.width,
            duration: ticks(lv.duration_ms),
            knockback: lv.knockback,
            pull: lv.pull,
            slow_pct: lv.slow_pct,
            slow: ticks(lv.slow_ms),
            fuse: ticks(lv.fuse_ms),
            every: ticks(lv.tick_ms),
            fire: ticks(lv.fire_ms),
            field: FieldKind::parse(&lv.field),
            burst_pct: 0,
            returns: lv.returns > 0,
            sweep: lv.sweep > 0,
        })
    }

    // ---- targeting -----------------------------------------------------------

    /// The nearest living foe within `range` pixels. A boss gets no
    /// priority: distance decides (I4).
    pub fn nearest_foe(&self, from: V, range: i32) -> Option<usize> {
        let r2 = px(range) as i64 * px(range) as i64;
        self.foes
            .iter()
            .enumerate()
            .filter(|(_, f)| f.hp > 0)
            .map(|(i, f)| (dist2(from, f.pos), f.id, i))
            .filter(|(d, _, _)| *d <= r2)
            .min()
            .map(|(_, _, i)| i)
    }

    /// Among the eight nearest foes in range, the one with the most others
    /// within 90 pixels of it.
    fn cluster_foe(&self, from: V, range: i32) -> Option<usize> {
        let r2 = px(range) as i64 * px(range) as i64;
        let mut near: Vec<(i64, u32, usize)> = self
            .foes
            .iter()
            .enumerate()
            .filter(|(_, f)| f.hp > 0)
            .map(|(i, f)| (dist2(from, f.pos), f.id, i))
            .filter(|(d, _, _)| *d <= r2)
            .collect();
        near.sort();
        near.truncate(8);
        near.iter()
            .map(|&(d, id, i)| {
                let crowd = self
                    .foes
                    .iter()
                    .filter(|g| within(g.pos, self.foes[i].pos, px(90)))
                    .count();
                (std::cmp::Reverse(crowd), d, id, i)
            })
            .min()
            .map(|(_, _, _, i)| i)
    }

    fn random_foe(&mut self, from: V, range: i32) -> Option<usize> {
        let r2 = px(range) as i64 * px(range) as i64;
        let pool: Vec<usize> = self
            .foes
            .iter()
            .enumerate()
            .filter(|(_, f)| f.hp > 0 && dist2(from, f.pos) <= r2)
            .map(|(i, _)| i)
            .collect();
        if pool.is_empty() {
            return None;
        }
        Some(pool[self.rng.below(pool.len() as u32) as usize])
    }

    /// The direction an ability fires in, and the foe it aims at if any.
    /// None when nothing is in range: an auto-aimed ability holds its fire.
    fn aim_for(&mut self, seat: usize, s: &Scaled) -> Option<(V, Option<usize>)> {
        let p = &self.players[seat];
        let from = p.pos;
        let reach = if s.range > 0 { s.range } else { 500 };
        match s.aim {
            Aim::Own | Aim::Move => Some((p.facing, None)),
            Aim::Behind => Some((V::ZERO.sub(p.facing), None)),
            Aim::Pointer => {
                if !p.load.autoaim && p.aiming {
                    Some((p.aim, None))
                } else {
                    let i = self.nearest_foe(from, reach)?;
                    Some((self.foes[i].pos.sub(from), Some(i)))
                }
            }
            Aim::Nearest => {
                let i = self.nearest_foe(from, reach)?;
                Some((self.foes[i].pos.sub(from), Some(i)))
            }
            Aim::Cluster => {
                let i = self.cluster_foe(from, reach.max(s.area + 60))?;
                Some((self.foes[i].pos.sub(from), Some(i)))
            }
            Aim::Random => {
                let i = self.random_foe(from, reach)?;
                Some((self.foes[i].pos.sub(from), Some(i)))
            }
        }
    }

    // ---- casting -------------------------------------------------------------

    /// Every rig fires through `rig.rs` (T15); this is its old name.
    pub fn cast(&mut self, content: &Content) {
        self.fire_rigs(content);
    }

    /// The direction a payload fired from `from` points, if it has one.
    pub fn aim_dir(&mut self, content: &Content, seat: usize, s: &Scaled, from: Origin) -> Option<V> {
        let _ = content;
        self.aim_from(seat, s, from).map(|(d, _)| d)
    }

    /// Like `aim_for`, from any place: away from the player, the pointer
    /// plays no part and the nearest enemy to that place is the target.
    fn aim_from(&mut self, seat: usize, s: &Scaled, from: Origin) -> Option<(V, Option<usize>)> {
        if from.player {
            return self.aim_for(seat, s);
        }
        let reach = if s.range > 0 { s.range } else { 500 };
        match s.aim {
            Aim::Own | Aim::Move | Aim::Behind => Some((self.players[seat].facing, None)),
            _ => {
                let i = self.nearest_foe(from.pos, reach)?;
                Some((self.foes[i].pos.sub(from.pos), Some(i)))
            }
        }
    }

    /// Where a placed payload (a bomb, a strike, a well) lands.
    fn landing(&mut self, seat: usize, s: &Scaled, from: Origin, dir: Option<V>) -> Option<V> {
        let p = &self.players[seat];
        match s.aim {
            Aim::Own => {
                if from.player {
                    let jitter = from_angle(self.rng.below(crate::world::geom_turn()) as i32, px(20));
                    Some(from.pos.add(jitter))
                } else {
                    Some(from.pos)
                }
            }
            Aim::Behind => {
                if from.player && !p.moving {
                    return None;
                }
                let back = dir
                    .map(|d| d.with_len(px(36)))
                    .unwrap_or(V::ZERO.sub(p.facing.with_len(px(36))));
                Some(from.pos.add(back))
            }
            _ => {
                if let Some(d) = dir {
                    let reach = if s.range > 0 { s.range } else { 160 };
                    return Some(from.pos.add(d.with_len(px(reach.min(260)))));
                }
                let (d, i) = self.aim_from(seat, s, from)?;
                Some(match i {
                    Some(i) => self.foes[i].pos,
                    None => from.pos.add(d),
                })
            }
        }
    }

    /// Fires one copy of a payload from `from`, toward `dir` when a
    /// formation gave one. Returns false when it had nothing to aim at.
    pub fn fire_one(
        &mut self,
        content: &Content,
        seat: usize,
        id: &str,
        s: &Scaled,
        from: Origin,
        dir: Option<V>,
    ) -> bool {
        let pos = from.pos;
        let anchored = !from.player;
        match s.kind {
            Kind::Shot => {
                let dir = match dir {
                    Some(d) => d,
                    None => match self.aim_from(seat, s, from) {
                        Some((d, _)) => d,
                        None => return false,
                    },
                };
                let n = s.count;
                for k in 0..n {
                    let offset = if n > 1 {
                        s.spread * (2 * k - (n - 1)) / 2
                    } else {
                        0
                    };
                    let vel = dir.rotate(offset).with_len(s.speed);
                    let life = if s.speed > 0 {
                        (px(s.range) / s.speed).max(1)
                    } else {
                        1
                    };
                    self.add_shot(content, seat, id, pos, vel, s, life);
                }
                true
            }
            Kind::Arc => {
                let dir = match dir {
                    Some(d) => d,
                    None => match self.aim_from(seat, s, from) {
                        Some((d, _)) => d,
                        None => return false,
                    },
                };
                let push = if s.knockback > 0 {
                    s.knockback
                } else {
                    content.run.knockback / 2
                };
                for i in self.near(content, pos, s.area + 40) {
                    let f = &self.foes[i];
                    let r = content.enemies[f.kind].radius;
                    let to = f.pos.sub(pos);
                    if within(f.pos, pos, px(s.area + r)) && in_arc(dir, to, s.width) {
                        self.hurt_foe(content, i, s.damage, seat, id, to.with_len(per_tick(push * 3)));
                    }
                }
                if let Some(f) = s.field {
                    self.add_field(
                        content,
                        f,
                        pos.add(dir.with_len(px(s.area / 2))),
                        s.area / 2,
                        s.damage,
                        seat,
                    );
                }
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Wedge,
                        width: s.width,
                        anchored,
                        ..self.effect_base(seat, id, pos, dir, s)
                    },
                );
                true
            }
            Kind::Nova => {
                for i in self.near(content, pos, s.area + 40) {
                    let f = &self.foes[i];
                    let r = content.enemies[f.kind].radius;
                    if !within(f.pos, pos, px(s.area + r)) {
                        continue;
                    }
                    let to = f.pos.sub(pos);
                    let push = if s.pull > 0 {
                        V::ZERO.sub(to.with_len(per_tick(s.pull * 4)))
                    } else if s.knockback > 0 {
                        to.with_len(per_tick(s.knockback * 3))
                    } else {
                        V::ZERO
                    };
                    if s.slow_pct > 0 {
                        self.foes[i].slow = s.slow;
                        self.foes[i].slow_pct = s.slow_pct;
                    }
                    self.hurt_foe(content, i, s.damage, seat, id, push);
                }
                if let Some(f) = s.field {
                    self.add_field(content, f, pos, s.area * 2 / 3, s.damage, seat);
                }
                let facing = self.players[seat].facing;
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Ring,
                        anchored,
                        ..self.effect_base(seat, id, pos, facing, s)
                    },
                );
                true
            }
            Kind::Chain => {
                let first = match dir {
                    Some(d) => self.nearest_foe(pos.add(d.with_len(px(60))), s.range.max(200)),
                    None => self.aim_from(seat, s, from).and_then(|(_, i)| i),
                };
                let Some(first) = first else { return false };
                let mut points = vec![pos];
                let mut hit_ids = Vec::new();
                let mut at = first;
                for _ in 0..s.count {
                    let fpos = self.foes[at].pos;
                    points.push(fpos);
                    hit_ids.push(self.foes[at].id);
                    self.hurt_foe(content, at, s.damage, seat, id, V::ZERO);
                    if let Some(f) = s.field {
                        self.add_field(content, f, fpos, 30, s.damage, seat);
                    }
                    let next = self
                        .foes
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| {
                            f.hp > 0 && !hit_ids.contains(&f.id) && within(f.pos, fpos, px(s.radius))
                        })
                        .map(|(i, f)| (dist2(fpos, f.pos), f.id, i))
                        .min()
                        .map(|(_, _, i)| i);
                    match next {
                        Some(n) => at = n,
                        None => break,
                    }
                }
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Chain,
                        points,
                        life: ticks(180),
                        full: ticks(180),
                        anchored: true,
                        ..self.effect_base(seat, id, pos, V::ZERO, s)
                    },
                );
                true
            }
            Kind::Bomb => {
                let mut any = false;
                for k in 0..s.count {
                    let d = dir.map(|d| d.rotate(crate::fx::TURN / 24 * k));
                    let Some(at) = self.landing(seat, s, from, d) else {
                        return any;
                    };
                    any = true;
                    self.add_effect(
                        content,
                        Effect {
                            kind: EffectKind::Bomb,
                            life: s.fuse,
                            full: s.fuse,
                            armed: matches!(s.aim, Aim::Own | Aim::Behind),
                            anchored: true,
                            ..self.effect_base(seat, id, at, V::ZERO, s)
                        },
                    );
                }
                any
            }
            Kind::Strike => {
                let centre = match s.aim {
                    Aim::Own => pos,
                    _ => match self.landing(seat, s, from, dir) {
                        Some(c) => c,
                        None => return false,
                    },
                };
                let n = s.count.max(1);
                let turn = crate::fx::TURN;
                let start = (self.tick as i32 * 97) % turn;
                for k in 0..n {
                    let at = centre.add(from_angle(start + turn * k / n, px(s.range)));
                    let fuse = s.fuse + if s.sweep { s.every.max(1) * k } else { 0 };
                    self.add_effect(
                        content,
                        Effect {
                            kind: EffectKind::Strike,
                            life: fuse.max(1),
                            full: fuse.max(1),
                            anchored: true,
                            ..self.effect_base(seat, id, at, V::ZERO, s)
                        },
                    );
                }
                true
            }
            Kind::Well => {
                let Some(at) = self.landing(seat, s, from, dir) else {
                    return false;
                };
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Well,
                        every: s.every.max(1),
                        anchored: true,
                        ..self.effect_base(seat, id, at, V::ZERO, s)
                    },
                );
                if let Some(f) = s.field {
                    self.add_field(content, f, at, s.area, s.damage, seat);
                }
                true
            }
            Kind::Tether => {
                // one set of lines per player and ability: a new firing
                // replaces the old one, but the copies of one firing (a
                // formation's, all made this tick, so still at full life)
                // each keep their own line (the person's answer, R4.4)
                // The old ones are ended, not removed: a trigger can fire a
                // tether while the effects are being walked (T18).
                let same = |e: &Effect| e.kind == EffectKind::Tether && e.seat == seat && e.ability == id;
                for e in self.effects.iter_mut().filter(|e| same(e) && e.life != e.full) {
                    e.life = 0;
                }
                let index = self
                    .effects
                    .iter()
                    .filter(|e| same(e) && e.life == e.full)
                    .count() as i32;
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Tether,
                        every: s.every.max(1),
                        width: s.width,
                        count: index,
                        // a copy with nothing to join reaches out its own way
                        armed: dir.is_some(),
                        ..self.effect_base(seat, id, pos, dir.unwrap_or(V::ZERO), s)
                    },
                );
                true
            }
            Kind::Orbit => {
                if !anchored {
                    // one ring of bodies per ability on the player; a new cast replaces it
                    self.effects.retain(|e| {
                        !(e.kind == EffectKind::Orbit && e.seat == seat && e.ability == id && !e.anchored)
                    });
                }
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Orbit,
                        count: s.count,
                        spin: s.speed * crate::geom::TPS / 256 / crate::geom::TPS,
                        anchored,
                        ..self.effect_base(seat, id, pos, V::ZERO, s)
                    },
                );
                true
            }
            Kind::Beam => {
                let dir = match dir {
                    Some(d) => d,
                    None => match self.aim_from(seat, s, from) {
                        Some((d, _)) => d,
                        None => return false,
                    },
                };
                if let Some(f) = s.field {
                    self.add_field(content, f, pos.add(dir.with_len(px(s.range))), 50, s.damage, seat);
                }
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Beam,
                        every: s.every.max(1),
                        anchored,
                        ..self.effect_base(seat, id, pos, dir, s)
                    },
                );
                true
            }
            Kind::Turret => {
                for k in 0..s.count {
                    let at = pos.add(V::new(px(30 * (2 * k - (s.count - 1))), px(-24)));
                    self.add_effect(
                        content,
                        Effect {
                            kind: EffectKind::Turret,
                            every: s.fire.max(1),
                            // a seed pod grows before it fires
                            cd: s.fuse,
                            shot_speed: s.speed,
                            shot_radius: s.radius,
                            shot_range: s.range,
                            anchored: true,
                            ..self.effect_base(seat, id, at, V::ZERO, s)
                        },
                    );
                }
                true
            }
        }
    }

    fn effect_base(&self, seat: usize, id: &str, pos: V, dir: V, s: &Scaled) -> Effect {
        Effect {
            kind: EffectKind::Ring,
            seat,
            ability: id.to_string(),
            pos,
            dir,
            radius: s.area,
            width: 0,
            damage: s.damage,
            life: s.duration.max(1),
            full: s.duration.max(1),
            every: 0,
            cd: 0,
            count: 0,
            angle: 0,
            spin: 0,
            armed: false,
            points: Vec::new(),
            hits: Vec::new(),
            shot_speed: 0,
            shot_radius: 0,
            shot_range: s.range,
            anchored: false,
            carry: None,
            field: s.field,
            marks: Vec::new(),
            burst_pct: s.burst_pct,
            pull: s.pull,
            slow_pct: s.slow_pct,
            slow: s.slow,
        }
    }

    fn add_effect(&mut self, content: &Content, e: Effect) {
        let lasting = matches!(
            e.kind,
            EffectKind::Orbit | EffectKind::Turret | EffectKind::Bomb | EffectKind::Strike | EffectKind::Well
        );
        if self.effects.len() >= content.run.effect_cap && !lasting {
            return;
        }
        self.effects.push(e);
    }

    #[allow(clippy::too_many_arguments)]
    fn add_shot(&mut self, content: &Content, seat: usize, id: &str, pos: V, vel: V, s: &Scaled, life: i32) {
        if self.shots.len() >= content.run.shot_cap {
            return;
        }
        self.shots.push(Shot {
            seat,
            ability: id.to_string(),
            pos,
            vel,
            damage: s.damage,
            radius: s.radius,
            pierce: s.pierce,
            bounce: s.bounce,
            homing: s.homing,
            life,
            hit: Vec::new(),
            carry: None,
            field: s.field,
            burst_pct: s.burst_pct,
            returns: s.returns,
            full_life: life,
            slow_pct: s.slow_pct,
            slow: s.slow,
            pull: s.pull,
        });
    }

    /// A trigger left on a shot or effect fires: once, then it is spent.
    fn spring_carry(&mut self, content: &Content, seat: usize, carry: &mut Option<Carry>, on: On, at: V) {
        if carry.as_ref().map(|c| c.on) != Some(on) {
            return;
        }
        let Some(c) = carry.take() else { return };
        let mut budget = crate::rig::MAX_PAYLOADS;
        self.spring(content, seat, &c, at, &mut budget);
    }

    /// Where a payload ends: its burst and its field.
    fn ending(&mut self, content: &Content, seat: usize, ability: &str, at: V, damage: i32, burst_pct: i32) {
        if burst_pct > 0 {
            let burst = (damage * burst_pct / 100).max(1);
            for i in self.near(content, at, 70) {
                let f = &self.foes[i];
                if f.hp > 0 && within(f.pos, at, px(50 + content.enemies[f.kind].radius)) {
                    self.hurt_foe(content, i, burst, seat, ability, V::ZERO);
                }
            }
        }
    }

    // ---- shots and effects ---------------------------------------------------

    pub fn move_shots(&mut self, content: &Content) {
        let push = per_tick(content.run.knockback);
        let mut k = 0;
        while k < self.shots.len() {
            let mut sh = self.shots[k].clone();
            if sh.homing {
                if let Some(i) = self.nearest_foe(sh.pos, 300) {
                    let speed = sh.vel.len() as i32;
                    let want = self.foes[i].pos.sub(sh.pos).with_len(speed);
                    sh.vel = sh.vel.scale(3, 4).add(want.scale(1, 4)).with_len(speed);
                }
            }
            if sh.returns && sh.life == sh.full_life / 2 {
                // halfway: turn for home, and every foe may be hit again
                if let Some(p) = self.players.get(sh.seat) {
                    let speed = sh.vel.len() as i32;
                    sh.vel = p.pos.sub(sh.pos).with_len(speed);
                }
                sh.hit.clear();
            }
            sh.pos = sh.pos.add(sh.vel);
            sh.life -= 1;
            let mut spent = sh.life <= 0;
            if !spent {
                for i in self.near(content, sh.pos, sh.radius + 64) {
                    let f = &self.foes[i];
                    if f.hp <= 0 || sh.hit.contains(&f.id) {
                        continue;
                    }
                    let r = content.enemies[f.kind].radius;
                    if !within(sh.pos, f.pos, px(sh.radius + r)) {
                        continue;
                    }
                    sh.hit.push(f.id);
                    let ability = sh.ability.clone();
                    if sh.slow_pct > 0 {
                        self.foes[i].slow = self.foes[i].slow.max(sh.slow);
                        self.foes[i].slow_pct = self.foes[i].slow_pct.max(sh.slow_pct);
                    }
                    let shove = if sh.pull > 0 {
                        V::ZERO.sub(sh.vel.with_len(per_tick(sh.pull)))
                    } else {
                        sh.vel.with_len(push)
                    };
                    let killed = self.hurt_foe(content, i, sh.damage, sh.seat, &ability, shove);
                    let at = sh.pos;
                    if let Some(fk) = sh.field {
                        self.add_field(content, fk, at, 40, sh.damage, sh.seat);
                    }
                    self.spring_carry(content, sh.seat, &mut sh.carry, On::Hit, at);
                    if killed {
                        self.spring_carry(content, sh.seat, &mut sh.carry, On::Kill, at);
                    }
                    if sh.bounce > 0 {
                        sh.bounce -= 1;
                        let here = sh.pos;
                        let next = self
                            .foes
                            .iter()
                            .filter(|g| g.hp > 0 && !sh.hit.contains(&g.id) && within(g.pos, here, px(300)))
                            .map(|g| (dist2(here, g.pos), g.id, g.pos))
                            .min();
                        if let Some((_, _, at)) = next {
                            let speed = sh.vel.len() as i32;
                            sh.vel = at.sub(here).with_len(speed);
                            sh.life = sh.life.max(ticks(800));
                            continue;
                        }
                    }
                    sh.pierce -= 1;
                    if sh.pierce <= 0 {
                        spent = true;
                        break;
                    }
                }
            }
            if spent {
                let at = sh.pos;
                self.ending(content, sh.seat, &sh.ability.clone(), at, sh.damage, sh.burst_pct);
                self.spring_carry(content, sh.seat, &mut sh.carry, On::Expire, at);
                self.shots.remove(k);
            } else {
                self.shots[k] = sh;
                k += 1;
            }
        }
    }

    /// The far end of a tether: the partner, or the farthest thing this
    /// player has placed within reach.
    /// Where a tether's line ends: the partner first, then what the player
    /// placed, farthest first. Copy k of a formation takes the k-th; a copy
    /// left over reaches out in its own direction.
    fn tether_end(&self, e: &Effect) -> Option<V> {
        let me = self.players.get(e.seat)?;
        let mut ends = Vec::new();
        if let Some(q) = self.players.iter().find(|q| q.seat != e.seat && q.alive) {
            if within(q.pos, me.pos, px(e.shot_range.max(200) * 2)) {
                ends.push(q.pos);
            }
        }
        let mut placed: Vec<(i64, usize)> = self
            .effects
            .iter()
            .enumerate()
            .filter(|(_, x)| {
                x.seat == e.seat
                    && matches!(
                        x.kind,
                        EffectKind::Bomb | EffectKind::Well | EffectKind::Turret | EffectKind::Strike
                    )
                    && within(x.pos, me.pos, px(e.shot_range.max(200)))
            })
            .map(|(i, x)| (dist2(x.pos, me.pos), i))
            .collect();
        placed.sort_by_key(|(d, i)| (std::cmp::Reverse(*d), std::cmp::Reverse(*i)));
        ends.extend(placed.into_iter().map(|(_, i)| self.effects[i].pos));
        match ends.get(e.count.max(0) as usize) {
            Some(end) => Some(*end),
            None if e.armed && e.dir != V::ZERO => {
                Some(me.pos.add(e.dir.with_len(px(e.shot_range.max(200)))))
            }
            None => None,
        }
    }

    pub fn update_effects(&mut self, content: &Content) {
        let now = self.tick;
        let mut k = 0;
        while k < self.effects.len() {
            let mut e = self.effects[k].clone();
            e.life -= 1;
            let owner = self
                .players
                .get(e.seat)
                .map(|p| (p.pos, p.alive))
                .unwrap_or((e.pos, false));
            match e.kind {
                EffectKind::Ring | EffectKind::Wedge | EffectKind::Chain => {
                    if e.kind != EffectKind::Chain && !e.anchored {
                        e.pos = owner.0;
                    }
                }
                EffectKind::Beam => {
                    if !e.anchored {
                        e.pos = owner.0;
                    }
                    e.cd -= 1;
                    if e.cd <= 0 {
                        e.cd = e.every;
                        let end = e.pos.add(e.dir.with_len(px(e.shot_range)));
                        let reach = px(e.radius);
                        let hit: Vec<usize> = self
                            .foes
                            .iter()
                            .enumerate()
                            .filter(|(_, f)| {
                                let r = px(content.enemies[f.kind].radius) + reach;
                                f.hp > 0 && seg_dist2(f.pos, e.pos, end) <= r as i64 * r as i64
                            })
                            .map(|(i, _)| i)
                            .collect();
                        for i in hit {
                            let at = self.foes[i].pos;
                            let killed = self.hurt_foe(content, i, e.damage, e.seat, &e.ability, V::ZERO);
                            self.spring_carry(content, e.seat, &mut e.carry, On::Hit, at);
                            if killed {
                                self.spring_carry(content, e.seat, &mut e.carry, On::Kill, at);
                            }
                        }
                    }
                }
                EffectKind::Tether => {
                    e.pos = owner.0;
                    if !owner.1 {
                        e.life = 0;
                    }
                    e.cd -= 1;
                    let end = self.tether_end(&e);
                    e.points = end.map(|b| vec![e.pos, b]).unwrap_or_default();
                    if e.cd <= 0 {
                        e.cd = e.every;
                        if let Some(b) = end {
                            let reach = px(e.width.max(4));
                            let hit: Vec<usize> = self
                                .foes
                                .iter()
                                .enumerate()
                                .filter(|(_, f)| {
                                    let r = px(content.enemies[f.kind].radius) + reach;
                                    f.hp > 0 && seg_dist2(f.pos, e.pos, b) <= r as i64 * r as i64
                                })
                                .map(|(i, _)| i)
                                .collect();
                            let gap = ticks(content.fields.tether_ms);
                            for i in hit {
                                let at = self.foes[i].pos;
                                if let Some(fk) = e.field {
                                    let last = e.marks.first().copied().flatten();
                                    if last.map(|t| now as i32 - t >= gap).unwrap_or(true) {
                                        e.marks = vec![Some(now as i32)];
                                        self.add_field(content, fk, at, 30, e.damage, e.seat);
                                    }
                                }
                                let killed = self.hurt_foe(content, i, e.damage, e.seat, &e.ability, V::ZERO);
                                self.spring_carry(content, e.seat, &mut e.carry, On::Hit, at);
                                if killed {
                                    self.spring_carry(content, e.seat, &mut e.carry, On::Kill, at);
                                }
                            }
                        }
                    }
                }
                EffectKind::Well => {
                    for i in self.near(content, e.pos, e.radius + 40) {
                        let f = &mut self.foes[i];
                        if f.hp > 0 && within(f.pos, e.pos, px(e.radius)) {
                            let to = e.pos.sub(f.pos);
                            f.push = f.push.add(to.with_len(per_tick(e.pull.max(40))).scale(1, 4));
                        }
                    }
                    e.cd -= 1;
                    if e.cd <= 0 {
                        e.cd = e.every;
                        let inside: Vec<usize> = self
                            .near(content, e.pos, e.radius + 40)
                            .into_iter()
                            .filter(|&i| self.foes[i].hp > 0 && within(self.foes[i].pos, e.pos, px(e.radius)))
                            .collect();
                        for i in inside {
                            let at = self.foes[i].pos;
                            let killed = self.hurt_foe(content, i, e.damage, e.seat, &e.ability, V::ZERO);
                            self.spring_carry(content, e.seat, &mut e.carry, On::Hit, at);
                            if killed {
                                self.spring_carry(content, e.seat, &mut e.carry, On::Kill, at);
                            }
                        }
                    }
                }
                EffectKind::Bomb | EffectKind::Strike => {
                    let trigger = e.kind == EffectKind::Bomb
                        && e.armed
                        && self.foes.iter().any(|f| {
                            f.hp > 0
                                && within(f.pos, e.pos, px(e.radius / 2 + content.enemies[f.kind].radius))
                        });
                    if trigger || e.life <= 0 {
                        let mut first: Option<(V, bool)> = None;
                        for i in self.near(content, e.pos, e.radius + 40) {
                            let f = &self.foes[i];
                            let r = content.enemies[f.kind].radius;
                            if f.hp > 0 && within(f.pos, e.pos, px(e.radius + r)) {
                                let at = f.pos;
                                let to = f.pos.sub(e.pos).with_len(per_tick(content.run.knockback * 2));
                                let to = if e.pull > 0 { V::ZERO.sub(to) } else { to };
                                if e.slow_pct > 0 {
                                    self.foes[i].slow = self.foes[i].slow.max(e.slow);
                                    self.foes[i].slow_pct = self.foes[i].slow_pct.max(e.slow_pct);
                                }
                                let killed = self.hurt_foe(content, i, e.damage, e.seat, &e.ability, to);
                                if first.is_none() || (killed && !first.map(|f| f.1).unwrap_or(false)) {
                                    first = Some((at, killed));
                                }
                            }
                        }
                        if let Some(fk) = e.field {
                            self.add_field(content, fk, e.pos, e.radius, e.damage, e.seat);
                        }
                        let at = e.pos;
                        let ability = e.ability.clone();
                        self.ending(content, e.seat, &ability, at, e.damage, e.burst_pct);
                        if let Some((p, killed)) = first {
                            self.spring_carry(content, e.seat, &mut e.carry, On::Hit, p);
                            if killed {
                                self.spring_carry(content, e.seat, &mut e.carry, On::Kill, p);
                            }
                        }
                        self.spring_carry(content, e.seat, &mut e.carry, On::Expire, at);
                        let boom = Effect {
                            kind: EffectKind::Ring,
                            life: ticks(220),
                            full: ticks(220),
                            anchored: true,
                            carry: None,
                            ..e.clone()
                        };
                        self.effects[k] = boom;
                        k += 1;
                        continue;
                    }
                }
                EffectKind::Orbit => {
                    if !e.anchored {
                        e.pos = owner.0;
                    }
                    e.angle += e.spin.max(1);
                    if !owner.1 {
                        e.life = 0;
                    }
                    e.hits.retain(|(_, until)| *until > now);
                    let n = e.count.max(1);
                    let turn = crate::fx::TURN;
                    let mut struck = Vec::new();
                    for b in 0..n {
                        let at = e.pos.add(from_angle(e.angle + turn * b / n, px(e.shot_range)));
                        for i in self.near(content, at, e.radius + 64) {
                            let f = &self.foes[i];
                            let r = content.enemies[f.kind].radius;
                            if f.hp > 0
                                && within(f.pos, at, px(e.radius + r))
                                && !e.hits.iter().any(|(id, _)| *id == f.id)
                                && !struck.iter().any(|(id, _, _, _)| *id == f.id)
                            {
                                struck.push((f.id, i, f.pos.sub(e.pos), b));
                            }
                        }
                    }
                    let turns = content.fields.orbit_turns.max(0) * turn;
                    for (id, i, away, b) in struck {
                        e.hits.push((id, now + ticks(500) as u32));
                        let at = self.foes[i].pos;
                        // each body leaves a field, then not again until it
                        // has gone round `orbit_turns` times (R4.5)
                        if let Some(fk) = e.field {
                            let b = b as usize;
                            if e.marks.len() <= b {
                                e.marks.resize(b + 1, None);
                            }
                            if e.marks[b].map(|a| e.angle - a >= turns).unwrap_or(true) {
                                e.marks[b] = Some(e.angle);
                                self.add_field(content, fk, at, 30, e.damage, e.seat);
                            }
                        }
                        let killed = self.hurt_foe(
                            content,
                            i,
                            e.damage,
                            e.seat,
                            &e.ability,
                            away.with_len(per_tick(200)),
                        );
                        self.spring_carry(content, e.seat, &mut e.carry, On::Hit, at);
                        if killed {
                            self.spring_carry(content, e.seat, &mut e.carry, On::Kill, at);
                        }
                    }
                }
                EffectKind::Turret => {
                    e.cd -= 1;
                    if e.cd <= 0 {
                        e.cd = e.every;
                        if let Some(i) = self.nearest_foe(e.pos, e.shot_range) {
                            let vel = self.foes[i].pos.sub(e.pos).with_len(e.shot_speed);
                            let life = (px(e.shot_range) / e.shot_speed.max(1)).max(1);
                            let ability = e.ability.clone();
                            let s = Scaled::for_turret_shot(&e);
                            self.add_shot(content, e.seat, &ability, e.pos, vel, &s, life);
                        }
                    }
                }
            }
            if e.life <= 0 {
                let at = e.pos;
                self.spring_carry(content, e.seat, &mut e.carry, On::Expire, at);
                self.effects.remove(k);
            } else {
                self.effects[k] = e;
                k += 1;
            }
        }
    }

    // ---- caches and evolutions -----------------------------------------------

    /// The first ability this player could evolve now: at its highest level,
    /// holding its partner passive, and with the evolution unlocked (I9).
    pub fn evolvable(&self, content: &Content, seat: usize) -> Option<usize> {
        let p = &self.players[seat];
        p.abilities.iter().position(|slot| {
            let Some(def) = content.ability(&slot.id) else {
                return false;
            };
            !def.evolution.is_empty()
                && slot.level >= content.run.max_level
                && p.passives
                    .iter()
                    .any(|q| q.id == def.evolves_with && q.level >= content.partner_level(&def.evolves_with))
                && !p.load.locked.contains(&def.evolution)
        })
    }

    pub fn evolve(&mut self, content: &Content, seat: usize, slot: usize) {
        let Some(def) = content.ability(&self.players[seat].abilities[slot].id) else {
            return;
        };
        let into = def.evolution.clone();
        let old = def.id.clone();
        self.effects.retain(|e| !(e.seat == seat && e.ability == old));
        self.rename_token(seat, &old, &into);
        let p = &mut self.players[seat];
        p.abilities[slot] = crate::world::Slot {
            id: into.clone(),
            level: 1,
            cd: ticks(300),
        };
        p.record.evolved.push(into);
        if p.record.first_evolve == 0 {
            p.record.first_evolve = (self.tick as i32).max(1);
        }
        self.banner = Some(crate::world::Banner {
            key: "banner.evolved".into(),
            left: ticks(2500),
        });
    }

    /// A supply cache evolves what can evolve; otherwise it raises a level.
    pub fn open_cache(&mut self, content: &Content, seat: usize) {
        if let Some(slot) = self.evolvable(content, seat) {
            self.evolve(content, seat, slot);
            return;
        }
        let max = content.run.max_level;
        let p = &self.players[seat];
        let ups: Vec<usize> = p
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, s)| !content.is_evolution(&s.id) && s.level < max)
            .map(|(k, _)| k)
            .collect();
        if ups.is_empty() {
            self.drop(content, PickupKind::Gold, p.pos, content.run.gold_chip * 10);
            return;
        }
        let k = ups[self.rng.below(ups.len() as u32) as usize];
        self.players[seat].abilities[k].level += 1;
    }
}
