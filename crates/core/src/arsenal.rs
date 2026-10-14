//! The players' abilities: when each fires, where it aims, and what its
//! shots and effects do. Every ability is one of eight behaviours
//! (`content::Kind`) with numbers from the content file.

use crate::content::{Aim, Content, Kind, Level};
use crate::geom::{dist2, from_angle, in_arc, per_tick, px, seg_dist2, ticks, within, V};
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
        let area_pct = 100 + s.area_pct;
        let extra = match def.kind {
            Kind::Shot | Kind::Bomb | Kind::Orbit | Kind::Turret => s.projectiles,
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
            Aim::Own => Some((p.facing, None)),
            Aim::Move => Some((p.facing, None)),
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

    pub fn cast(&mut self, content: &Content) {
        for seat in 0..self.players.len() {
            if !self.players[seat].alive {
                continue;
            }
            for k in 0..self.players[seat].abilities.len() {
                let slot = &mut self.players[seat].abilities[k];
                slot.cd -= 1;
                if slot.cd > 0 {
                    continue;
                }
                let (id, level) = (slot.id.clone(), slot.level);
                let Some(s) = self.scaled(content, seat, &id, level) else {
                    continue;
                };
                let fired = self.fire(content, seat, &id, &s);
                // an ability with nothing to aim at tries again soon
                self.players[seat].abilities[k].cd = if fired { s.cooldown } else { ticks(150) };
            }
        }
    }

    fn fire(&mut self, content: &Content, seat: usize, id: &str, s: &Scaled) -> bool {
        let from = self.players[seat].pos;
        match s.kind {
            Kind::Shot => {
                let Some((dir, _)) = self.aim_for(seat, s) else {
                    return false;
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
                    self.add_shot(
                        content, seat, id, from, vel, s.damage, s.radius, s.pierce, s.bounce, s.homing, life,
                    );
                }
                true
            }
            Kind::Arc => {
                let Some((dir, _)) = self.aim_for(seat, s) else {
                    return false;
                };
                let push = if s.knockback > 0 {
                    s.knockback
                } else {
                    content.run.knockback / 2
                };
                for i in self.near(content, from, s.area + 40) {
                    let f = &self.foes[i];
                    let r = content.enemies[f.kind].radius;
                    let to = f.pos.sub(from);
                    if within(f.pos, from, px(s.area + r)) && in_arc(dir, to, s.width) {
                        self.hurt_foe(content, i, s.damage, seat, id, to.with_len(per_tick(push * 3)));
                    }
                }
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Wedge,
                        width: s.width,
                        ..self.effect_base(seat, id, from, dir, s)
                    },
                );
                true
            }
            Kind::Nova => {
                for i in self.near(content, from, s.area + 40) {
                    let f = &self.foes[i];
                    let r = content.enemies[f.kind].radius;
                    if !within(f.pos, from, px(s.area + r)) {
                        continue;
                    }
                    let to = f.pos.sub(from);
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
                let facing = self.players[seat].facing;
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Ring,
                        ..self.effect_base(seat, id, from, facing, s)
                    },
                );
                true
            }
            Kind::Chain => {
                let Some((_, Some(first))) = self.aim_for(seat, s) else {
                    return false;
                };
                let mut points = vec![from];
                let mut hit_ids = Vec::new();
                let mut at = first;
                for _ in 0..s.count {
                    let pos = self.foes[at].pos;
                    points.push(pos);
                    hit_ids.push(self.foes[at].id);
                    self.hurt_foe(content, at, s.damage, seat, id, V::ZERO);
                    let next = self
                        .foes
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| {
                            f.hp > 0 && !hit_ids.contains(&f.id) && within(f.pos, pos, px(s.radius))
                        })
                        .map(|(i, f)| (dist2(pos, f.pos), f.id, i))
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
                        ..self.effect_base(seat, id, from, V::ZERO, s)
                    },
                );
                true
            }
            Kind::Bomb => {
                for _ in 0..s.count {
                    let target = if s.aim == Aim::Own {
                        let jitter = from_angle(self.rng.below(crate::world::geom_turn()) as i32, px(20));
                        Some(from.add(jitter))
                    } else {
                        self.aim_for(seat, s).map(|(dir, i)| match i {
                            Some(i) => self.foes[i].pos,
                            None => from.add(dir),
                        })
                    };
                    let Some(at) = target else {
                        return false;
                    };
                    self.add_effect(
                        content,
                        Effect {
                            kind: EffectKind::Bomb,
                            life: s.fuse,
                            full: s.fuse,
                            armed: s.aim == Aim::Own,
                            ..self.effect_base(seat, id, at, V::ZERO, s)
                        },
                    );
                }
                true
            }
            Kind::Orbit => {
                // one ring of bodies per ability; a new cast replaces the old
                self.effects
                    .retain(|e| !(e.kind == EffectKind::Orbit && e.seat == seat && e.ability == id));
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Orbit,
                        count: s.count,
                        spin: s.speed * crate::geom::TPS / 256 / crate::geom::TPS,
                        ..self.effect_base(seat, id, from, V::ZERO, s)
                    },
                );
                true
            }
            Kind::Beam => {
                let Some((dir, _)) = self.aim_for(seat, s) else {
                    return false;
                };
                self.add_effect(
                    content,
                    Effect {
                        kind: EffectKind::Beam,
                        every: s.every.max(1),
                        ..self.effect_base(seat, id, from, dir, s)
                    },
                );
                true
            }
            Kind::Turret => {
                for k in 0..s.count {
                    let at = from.add(V::new(px(30 * (2 * k - (s.count - 1))), px(-24)));
                    self.add_effect(
                        content,
                        Effect {
                            kind: EffectKind::Turret,
                            every: s.fire.max(1),
                            shot_speed: s.speed,
                            shot_radius: s.radius,
                            shot_range: s.range,
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
        }
    }

    fn add_effect(&mut self, content: &Content, e: Effect) {
        let lasting = matches!(e.kind, EffectKind::Orbit | EffectKind::Turret | EffectKind::Bomb);
        if self.effects.len() >= content.run.effect_cap && !lasting {
            return;
        }
        self.effects.push(e);
    }

    #[allow(clippy::too_many_arguments)]
    fn add_shot(
        &mut self,
        content: &Content,
        seat: usize,
        id: &str,
        pos: V,
        vel: V,
        damage: i32,
        radius: i32,
        pierce: i32,
        bounce: i32,
        homing: bool,
        life: i32,
    ) {
        if self.shots.len() >= content.run.shot_cap {
            return;
        }
        self.shots.push(Shot {
            seat,
            ability: id.to_string(),
            pos,
            vel,
            damage,
            radius,
            pierce,
            bounce,
            homing,
            life,
            hit: Vec::new(),
        });
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
                    self.hurt_foe(content, i, sh.damage, sh.seat, &ability, sh.vel.with_len(push));
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
                self.shots.remove(k);
            } else {
                self.shots[k] = sh;
                k += 1;
            }
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
                    if e.kind != EffectKind::Chain {
                        e.pos = owner.0;
                    }
                }
                EffectKind::Beam => {
                    e.pos = owner.0;
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
                            self.hurt_foe(content, i, e.damage, e.seat, &e.ability, V::ZERO);
                        }
                    }
                }
                EffectKind::Bomb => {
                    let trigger = e.armed
                        && self.foes.iter().any(|f| {
                            f.hp > 0
                                && within(f.pos, e.pos, px(e.radius / 2 + content.enemies[f.kind].radius))
                        });
                    if trigger || e.life <= 0 {
                        for i in self.near(content, e.pos, e.radius + 40) {
                            let f = &self.foes[i];
                            let r = content.enemies[f.kind].radius;
                            if within(f.pos, e.pos, px(e.radius + r)) {
                                let to = f.pos.sub(e.pos).with_len(per_tick(content.run.knockback * 2));
                                self.hurt_foe(content, i, e.damage, e.seat, &e.ability, to);
                            }
                        }
                        let boom = Effect {
                            kind: EffectKind::Ring,
                            life: ticks(220),
                            full: ticks(220),
                            ..e.clone()
                        };
                        self.effects[k] = boom;
                        k += 1;
                        continue;
                    }
                }
                EffectKind::Orbit => {
                    e.pos = owner.0;
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
                                && !struck.iter().any(|(id, _, _)| *id == f.id)
                            {
                                struck.push((f.id, i, f.pos.sub(e.pos)));
                            }
                        }
                    }
                    for (id, i, away) in struck {
                        e.hits.push((id, now + ticks(500) as u32));
                        self.hurt_foe(
                            content,
                            i,
                            e.damage,
                            e.seat,
                            &e.ability,
                            away.with_len(per_tick(200)),
                        );
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
                            self.add_shot(
                                content,
                                e.seat,
                                &ability,
                                e.pos,
                                vel,
                                e.damage,
                                e.shot_radius,
                                1,
                                0,
                                false,
                                life,
                            );
                        }
                    }
                }
            }
            if e.life <= 0 {
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
                && p.passives.iter().any(|q| q.id == def.evolves_with)
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
        let p = &mut self.players[seat];
        p.abilities[slot] = crate::world::Slot {
            id: into.clone(),
            level: 1,
            cd: ticks(300),
        };
        p.record.evolved.push(into);
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
