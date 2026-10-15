//! Rigs (T15): abilities you build from parts, after Noita's wands.
//!
//! Each player has one rig per ability slot: a row of sockets. A socket
//! holds an ability (a payload), a part, or nothing. When a rig fires it
//! reads its sockets left to right:
//!
//! * a modifier changes the next payload (more damage, a field, homing);
//! * a formation fires the next payload several times, arranged;
//! * a payload fires, with what came before it, and clears them;
//! * a trigger hands the rest of the row to an event. Hit, expire and kill
//!   belong to the payload just before it: the rest fires where that
//!   payload hit, ended or killed. Stride, dash, rune and clock belong to
//!   the player: the rest fires when the player walks far enough, dashes,
//!   draws a rune or every few seconds.
//!
//! A rig never shuffles or misfires (the person's answer 4): the same rig
//! always does the same thing. Each part adds to the rig's recharge, so a
//! long rig fires less often, and triggers nest at most `MAX_DEPTH` deep.
//!
//! The rig window moves one token at a time, as one input (`Input::rig`),
//! so both co-op browsers make the same move on the same tick.

use crate::arsenal::Scaled;
use crate::content::{Arrange, Content, On, Part, PartKind};
use crate::field::FieldKind;
use crate::geom::{px, ticks, V};
use crate::world::{Rig, World};
use serde::{Deserialize, Serialize};

/// The deepest a trigger may fire a trigger.
pub const MAX_DEPTH: u8 = 3;

/// The most payloads one firing of a rig may make, copies and triggers
/// included, before the rest of it is skipped.
pub const MAX_PAYLOADS: u32 = 24;

/// What a trigger left on a payload: the rest of the row, and the event
/// that fires it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Carry {
    pub on: On,
    pub rest: Vec<String>,
    pub depth: u8,
}

/// A payload waiting to fire again (Echo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub at: u32,
    pub seat: usize,
    pub program: Vec<String>,
    pub pos: V,
    pub from_player: bool,
    pub depth: u8,
}

/// A place in the rig window: a rig's socket, or a spare-parts slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spot {
    Socket(usize, usize),
    Spare(usize),
}

const SPARE_BASE: u16 = 64;

impl Spot {
    pub fn code(self) -> u16 {
        match self {
            Spot::Socket(r, s) => (r as u16) * 16 + s as u16,
            Spot::Spare(k) => SPARE_BASE + k as u16,
        }
    }

    pub fn from_code(c: u16) -> Option<Spot> {
        if c < SPARE_BASE {
            Some(Spot::Socket((c / 16) as usize, (c % 16) as usize))
        } else if c < 128 {
            Some(Spot::Spare((c - SPARE_BASE) as usize))
        } else {
            None
        }
    }

    /// The spot as text for an action value, and back.
    pub fn text(self) -> String {
        match self {
            Spot::Socket(r, s) => format!("r{r}s{s}"),
            Spot::Spare(k) => format!("x{k}"),
        }
    }

    pub fn parse(t: &str) -> Option<Spot> {
        if let Some(rest) = t.strip_prefix('x') {
            return rest.parse().ok().filter(|k| *k < 64).map(Spot::Spare);
        }
        let rest = t.strip_prefix('r')?;
        let (r, s) = rest.split_once('s')?;
        let (r, s) = (r.parse().ok()?, s.parse().ok()?);
        if r < 4 && s < 16 {
            Some(Spot::Socket(r, s))
        } else {
            None
        }
    }
}

/// A move in the rig window as one input number: 0 is none.
pub fn pack_move(from: Spot, to: Spot) -> u16 {
    1 + from.code() * 128 + to.code()
}

pub fn unpack_move(v: u16) -> Option<(Spot, Spot)> {
    if v == 0 {
        return None;
    }
    let v = v - 1;
    Some((Spot::from_code(v / 128)?, Spot::from_code(v % 128)?))
}

/// The modifiers gathered for the next payload.
#[derive(Debug, Clone, Default)]
pub struct Mods {
    pub damage_pct: i32,
    pub area_pct: i32,
    pub speed_pct: i32,
    pub count: i32,
    pub bounce: i32,
    pub pierce: i32,
    pub homing: bool,
    pub field: Option<FieldKind>,
    pub slow_pct: i32,
    pub slow_ms: i32,
    pub pull: i32,
    pub echo_ms: i32,
    pub wormhole: bool,
    pub burst_pct: i32,
}

impl Mods {
    pub fn add(&mut self, p: &Part) {
        self.damage_pct += p.damage_pct;
        self.area_pct += p.area_pct;
        self.speed_pct += p.speed_pct;
        self.count += p.count;
        self.bounce += p.bounce;
        self.pierce += p.pierce;
        self.homing |= p.homing;
        if let Some(f) = FieldKind::parse(&p.field) {
            self.field = Some(f);
        }
        self.slow_pct = self.slow_pct.max(p.slow_pct);
        self.slow_ms = self.slow_ms.max(p.slow_ms);
        self.pull += p.pull;
        self.echo_ms = self.echo_ms.max(p.echo_ms);
        self.wormhole |= p.wormhole;
        self.burst_pct += p.burst_pct;
    }

    pub fn apply(&self, s: &mut Scaled) {
        s.damage = (s.damage * (100 + self.damage_pct) / 100).max(1);
        s.area = (s.area * (100 + self.area_pct) / 100).max(1);
        s.radius = (s.radius * (100 + self.area_pct / 2) / 100).max(1);
        s.speed = s.speed * (100 + self.speed_pct).max(20) / 100;
        s.count = (s.count + self.count).max(1);
        s.bounce += self.bounce;
        s.pierce += self.pierce;
        s.homing |= self.homing;
        if self.field.is_some() {
            s.field = self.field;
        }
        if self.slow_pct > 0 {
            s.slow_pct = s.slow_pct.max(self.slow_pct);
            s.slow = s.slow.max(ticks(self.slow_ms));
        }
        s.pull += self.pull;
        s.burst_pct += self.burst_pct;
    }
}

/// Where a firing comes from: a place, and whether that place is the
/// player (who may still aim with the pointer and move the effect along).
#[derive(Debug, Clone, Copy)]
pub struct Origin {
    pub pos: V,
    pub player: bool,
}

impl World {
    /// Rigs and sockets as the player's slots and Extra Socket say. Rigs
    /// only ever grow, so nothing in them is lost.
    pub fn fit_rigs(&mut self, content: &Content, seat: usize) {
        let want =
            content.run.rig_sockets.max(1) as usize + self.stat_total(content, seat).sockets.max(0) as usize;
        let rigs = content.run.ability_slots.max(1);
        let p = &mut self.players[seat];
        while p.rigs.len() < rigs {
            p.rigs.push(Rig::default());
        }
        for r in p.rigs.iter_mut() {
            while r.sockets.len() < want.min(16) {
                r.sockets.push(String::new());
            }
        }
    }

    /// A new ability goes into the first empty rig, else the first empty
    /// socket, else the spare parts.
    pub fn place_ability(&mut self, content: &Content, seat: usize, id: &str) {
        self.fit_rigs(content, seat);
        let p = &mut self.players[seat];
        if let Some(r) = p.rigs.iter_mut().find(|r| r.sockets.iter().all(|s| s.is_empty())) {
            r.sockets[0] = id.to_string();
            r.cd = ticks(500);
            return;
        }
        for r in p.rigs.iter_mut() {
            if let Some(s) = r.sockets.iter_mut().find(|s| s.is_empty()) {
                *s = id.to_string();
                return;
            }
        }
        p.spare.push(id.to_string());
    }

    /// A new part goes where it does something without the player's help:
    /// a modifier or formation just before the first payload of the first
    /// rig with room after it; a trigger to the spare parts, since where it
    /// sits decides what it does.
    pub fn place_part(&mut self, content: &Content, seat: usize, id: &str) {
        self.fit_rigs(content, seat);
        let kind = content.part(id).map(|p| p.kind).unwrap_or(PartKind::Trigger);
        let p = &mut self.players[seat];
        if kind != PartKind::Trigger {
            for r in p.rigs.iter_mut() {
                let Some(first) = r.sockets.iter().position(|s| content.ability(s).is_some()) else {
                    continue;
                };
                let Some(gap) = (first..r.sockets.len()).find(|&k| r.sockets[k].is_empty()) else {
                    continue;
                };
                // shift the payload and what follows it one socket right
                r.sockets.remove(gap);
                r.sockets.insert(first, id.to_string());
                return;
            }
        }
        p.spare.push(id.to_string());
    }

    fn spot_mut(&mut self, seat: usize, at: Spot) -> Option<&mut String> {
        let p = &mut self.players[seat];
        match at {
            Spot::Socket(r, s) => p.rigs.get_mut(r)?.sockets.get_mut(s),
            Spot::Spare(k) => {
                if k == p.spare.len() {
                    p.spare.push(String::new());
                }
                p.spare.get_mut(k)
            }
        }
    }

    /// A move in the rig window: the two places swap what they hold. A move
    /// to the first free spare slot puts the token with the spare parts.
    pub fn rig_move(&mut self, content: &Content, seat: usize, code: u16) {
        let Some((a, b)) = unpack_move(code) else { return };
        self.fit_rigs(content, seat);
        if a == b {
            return;
        }
        let Some(x) = self.spot_mut(seat, a).map(std::mem::take) else {
            self.players[seat].spare.retain(|s| !s.is_empty());
            return;
        };
        match self.spot_mut(seat, b) {
            Some(slot) => {
                let y = std::mem::replace(slot, x);
                if let Some(back) = self.spot_mut(seat, a) {
                    *back = y;
                }
            }
            None => {
                if let Some(back) = self.spot_mut(seat, a) {
                    *back = x;
                }
            }
        }
        self.players[seat].spare.retain(|s| !s.is_empty());
    }

    /// An evolution keeps its place in the rig.
    pub fn rename_token(&mut self, seat: usize, from: &str, to: &str) {
        let p = &mut self.players[seat];
        for s in p
            .rigs
            .iter_mut()
            .flat_map(|r| r.sockets.iter_mut())
            .chain(p.spare.iter_mut())
        {
            if s == from {
                *s = to.to_string();
            }
        }
    }

    /// Where a rig's player trigger sits, if it has one: the sockets before
    /// it fire on the recharge, the sockets after it on the event.
    pub fn split_rig<'a>(&self, content: &'a Content, sockets: &[String]) -> (usize, Option<&'a Part>) {
        for (k, s) in sockets.iter().enumerate() {
            if let Some(p) = content.part(s) {
                if p.kind == PartKind::Trigger && p.on.is_player() {
                    return (k, Some(p));
                }
            }
        }
        (sockets.len(), None)
    }

    /// A rig's recharge in ticks: its slowest payload before any player
    /// trigger, plus every part's load. None when it has no payload there.
    pub fn rig_cooldown(&self, content: &Content, seat: usize, r: usize) -> Option<i32> {
        let rig = self.players[seat].rigs.get(r)?;
        let (cut, _) = self.split_rig(content, &rig.sockets);
        let slowest = rig.sockets[..cut]
            .iter()
            .filter_map(|id| {
                let level = self.players[seat].abilities.iter().find(|a| &a.id == id)?.level;
                Some(self.scaled(content, seat, id, level)?.cooldown)
            })
            .max()?;
        let load: i32 = rig
            .sockets
            .iter()
            .filter_map(|s| content.part(s))
            .map(|p| p.load_pct)
            .sum();
        Some((slowest * (100 + load.max(0)) / 100).max(1))
    }

    /// Every rig's tick: recharge, fire, and the player events.
    pub fn fire_rigs(&mut self, content: &Content) {
        // echoes due now
        let now = self.tick;
        let due: Vec<_> = self.pending.iter().filter(|e| e.at <= now).cloned().collect();
        self.pending.retain(|e| e.at > now);
        for e in due {
            if e.seat < self.players.len() && self.players[e.seat].alive {
                let pos = if e.from_player {
                    self.players[e.seat].pos
                } else {
                    e.pos
                };
                let mut budget = MAX_PAYLOADS;
                self.run_program(
                    content,
                    e.seat,
                    &e.program,
                    Origin {
                        pos,
                        player: e.from_player,
                    },
                    e.depth,
                    &mut budget,
                );
            }
        }
        for seat in 0..self.players.len() {
            if !self.players[seat].alive {
                continue;
            }
            self.fit_rigs(content, seat);
            let stats = self.stat_total(content, seat);
            let hot = stats.heat_sink_pct > 0 && self.in_own_fire(seat);
            for r in 0..self.players[seat].rigs.len() {
                let sockets = self.players[seat].rigs[r].sockets.clone();
                if sockets.iter().all(|s| s.is_empty()) {
                    continue;
                }
                let (cut, trigger) = self.split_rig(content, &sockets);
                let rig = &mut self.players[seat].rigs[r];
                rig.cd -= 1;
                if hot {
                    rig.heat += stats.heat_sink_pct;
                    while rig.heat >= 100 {
                        rig.heat -= 100;
                        rig.cd -= 1;
                    }
                }
                if rig.cd <= 0 {
                    if let Some(cd) = self.rig_cooldown(content, seat, r) {
                        let pos = self.players[seat].pos;
                        let mut budget = MAX_PAYLOADS;
                        let fired = self.run_program(
                            content,
                            seat,
                            &sockets[..cut],
                            Origin { pos, player: true },
                            0,
                            &mut budget,
                        );
                        let rig = &mut self.players[seat].rigs[r];
                        if fired {
                            rig.cd = cd;
                            rig.fired += 1;
                            let parts = sockets.iter().filter(|s| content.part(s).is_some()).count() as u32;
                            let rec = &mut self.players[seat].record;
                            rec.fullest_rig = rec.fullest_rig.max(parts);
                            let every = 6 - stats.double_tap.clamp(0, 3);
                            if stats.double_tap > 0
                                && self.players[seat].rigs[r].fired.is_multiple_of(every as u32)
                            {
                                let mut budget = MAX_PAYLOADS;
                                self.run_program(
                                    content,
                                    seat,
                                    &sockets[..cut],
                                    Origin { pos, player: true },
                                    0,
                                    &mut budget,
                                );
                            }
                        } else {
                            // nothing to aim at: try again soon
                            rig.cd = ticks(150);
                        }
                    }
                }
                let Some(t) = trigger else { continue };
                let rest: Vec<String> = sockets[cut + 1..].to_vec();
                let p = &self.players[seat];
                let mut at = None;
                match t.on {
                    On::Stride => {
                        let walked = p.walked;
                        let rig = &mut self.players[seat].rigs[r];
                        rig.stride += walked;
                        if rig.stride >= t.stride_px {
                            rig.stride -= t.stride_px;
                            at = Some(self.players[seat].pos);
                        }
                    }
                    On::Dash => {
                        if p.dashed {
                            at = Some(p.pos);
                        }
                    }
                    On::Rune => at = p.rune_at,
                    On::Clock => {
                        let rig = &mut self.players[seat].rigs[r];
                        rig.clock += 1;
                        if rig.clock >= ticks(t.every_ms) {
                            rig.clock = 0;
                            at = Some(self.players[seat].pos);
                        }
                    }
                    _ => {}
                }
                if let Some(pos) = at {
                    let mut budget = MAX_PAYLOADS;
                    let from_player = pos == self.players[seat].pos;
                    self.run_program(
                        content,
                        seat,
                        &rest,
                        Origin {
                            pos,
                            player: from_player,
                        },
                        1,
                        &mut budget,
                    );
                    let rec = &mut self.players[seat].record;
                    rec.deepest_trigger = rec.deepest_trigger.max(1);
                }
            }
        }
    }

    /// Reads a row of sockets and fires it. Returns whether anything fired.
    pub fn run_program(
        &mut self,
        content: &Content,
        seat: usize,
        program: &[String],
        from: Origin,
        depth: u8,
        budget: &mut u32,
    ) -> bool {
        let mut mods = Mods::default();
        let mut form: Option<(Arrange, i32)> = None;
        let mut fired = false;
        // what the last payload made, for a trigger after it
        let mut made: Option<Made> = None;
        let mut start = 0;
        let extra = self.stat_total(content, seat).formation_extra;
        for (k, tok) in program.iter().enumerate() {
            if tok.is_empty() {
                continue;
            }
            if let Some(part) = content.part(tok) {
                match part.kind {
                    PartKind::Modifier => mods.add(part),
                    PartKind::Formation => form = Some((part.arrange, part.copies + extra.max(0))),
                    PartKind::Trigger => {
                        if part.on.is_player() {
                            // the rest belongs to the player's event
                            return fired;
                        }
                        if let Some(m) = made.take() {
                            if depth < MAX_DEPTH && k + 1 < program.len() {
                                let carry = Carry {
                                    on: part.on,
                                    rest: program[k + 1..].to_vec(),
                                    depth: depth + 1,
                                };
                                self.hand_on(content, seat, m, carry, budget);
                            }
                        }
                        return fired;
                    }
                }
                continue;
            }
            let Some(level) = self.players[seat]
                .abilities
                .iter()
                .find(|a| &a.id == tok)
                .map(|a| a.level)
            else {
                continue;
            };
            let Some(mut s) = self.scaled(content, seat, tok, level) else {
                continue;
            };
            mods.apply(&mut s);
            let copies = form.map(|(_, n)| n.max(1) as u32).unwrap_or(1);
            if *budget < copies {
                return fired;
            }
            *budget -= copies;
            let before = (self.shots.len(), self.effects.len());
            self.hit_log = Some(Vec::new());
            let ok = self.fire_formed(content, seat, tok, &s, from, form, mods.wormhole);
            let log = self.hit_log.take().unwrap_or_default();
            fired |= ok;
            made = Some(Made {
                shots: before.0..self.shots.len(),
                effects: before.1..self.effects.len(),
                log,
            });
            if mods.echo_ms > 0 && depth < MAX_DEPTH {
                let again: Vec<String> = program[start..=k]
                    .iter()
                    .filter(|t| content.part(t).map(|p| p.echo_ms == 0).unwrap_or(true))
                    .cloned()
                    .collect();
                self.pending.push(Pending {
                    at: self.tick + ticks(mods.echo_ms) as u32,
                    seat,
                    program: again,
                    pos: from.pos,
                    from_player: from.player,
                    depth: depth + 1,
                });
            }
            mods = Mods::default();
            form = None;
            start = k + 1;
        }
        fired
    }

    /// Gives what a payload made the rest of the row. An instant payload (a
    /// nova, a sweep, a chain) has already hit: its hit or kill fires now.
    /// Shots and lasting effects carry it to their event.
    fn hand_on(&mut self, content: &Content, seat: usize, m: Made, carry: Carry, budget: &mut u32) {
        let instant = |k: crate::world::EffectKind| {
            matches!(
                k,
                crate::world::EffectKind::Ring
                    | crate::world::EffectKind::Wedge
                    | crate::world::EffectKind::Chain
            )
        };
        let now = match carry.on {
            On::Hit => m.log.first().map(|(p, _)| *p),
            On::Kill => m.log.iter().find(|(_, k)| *k).map(|(p, _)| *p),
            _ => None,
        };
        if let Some(at) = now {
            self.spring(content, seat, &carry, at, budget);
        }
        for i in m.shots {
            self.shots[i].carry = Some(carry.clone());
        }
        for i in m.effects {
            let e = &mut self.effects[i];
            // an instant one already fired its hit; it can still expire
            if !instant(e.kind) || carry.on == On::Expire {
                e.carry = Some(carry.clone());
            }
        }
    }

    /// Fires the rest of a row from where its event happened.
    pub fn spring(&mut self, content: &Content, seat: usize, carry: &Carry, at: V, budget: &mut u32) {
        if seat >= self.players.len() || !self.players[seat].alive {
            return;
        }
        let rec = &mut self.players[seat].record;
        rec.deepest_trigger = rec.deepest_trigger.max(carry.depth as u32);
        self.run_program(
            content,
            seat,
            &carry.rest,
            Origin {
                pos: at,
                player: false,
            },
            carry.depth,
            budget,
        );
    }

    /// Fires a payload once, or as its formation says.
    #[allow(clippy::too_many_arguments)]
    fn fire_formed(
        &mut self,
        content: &Content,
        seat: usize,
        id: &str,
        s: &Scaled,
        from: Origin,
        form: Option<(Arrange, i32)>,
        wormhole: bool,
    ) -> bool {
        let mut from = from;
        let mut aim: Option<V> = None;
        if wormhole {
            // from the farthest enemy in reach, toward the player
            let me = self.players[seat].pos;
            let far = self
                .foes
                .iter()
                .filter(|f| f.hp > 0 && crate::geom::within(f.pos, me, px(520)))
                .max_by_key(|f| (crate::geom::dist2(f.pos, me), std::cmp::Reverse(f.id)))
                .map(|f| f.pos);
            if let Some(p) = far {
                from = Origin {
                    pos: p,
                    player: false,
                };
                aim = Some(me.sub(p));
            }
        }
        let Some((arrange, n)) = form else {
            return self.fire_one(content, seat, id, s, from, aim);
        };
        let turn = crate::fx::TURN;
        let base = aim
            .or_else(|| self.aim_dir(content, seat, s, from))
            .unwrap_or(self.players[seat].facing);
        let placed = s.places();
        let mut any = false;
        let n = n.max(1);
        let shots: Vec<(V, V)> = match arrange {
            Arrange::Twin => (0..n)
                .map(|k| {
                    let off = turn / 60 * (2 * k - (n - 1));
                    let side = base.rotate(turn / 4).with_len(px(28 * (2 * k - (n - 1))));
                    if placed {
                        (from.pos.add(side), base)
                    } else {
                        (from.pos, base.rotate(off))
                    }
                })
                .collect(),
            Arrange::Fan => (0..n)
                .map(|k| {
                    let off = if n > 1 {
                        turn / 9 * (2 * k - (n - 1)) / (n - 1)
                    } else {
                        0
                    };
                    if placed {
                        (from.pos.add(base.rotate(off).with_len(px(120))), base.rotate(off))
                    } else {
                        (from.pos, base.rotate(off))
                    }
                })
                .collect(),
            Arrange::Ring => (0..n)
                .map(|k| {
                    let dir = base.rotate(turn * k / n);
                    if placed {
                        (from.pos.add(dir.with_len(px(130))), dir)
                    } else {
                        (from.pos, dir)
                    }
                })
                .collect(),
            Arrange::Behind => {
                let back = V::ZERO.sub(self.players[seat].facing);
                (0..n)
                    .map(|k| {
                        let dir = back.rotate(turn / 36 * (2 * k - (n - 1)));
                        if placed {
                            (from.pos.add(dir.with_len(px(90))), dir)
                        } else {
                            (from.pos, dir)
                        }
                    })
                    .collect()
            }
            Arrange::Mirror => {
                let mut v = vec![(from.pos, base)];
                let back = V::ZERO.sub(base);
                for _ in 1..n {
                    v.push(if placed {
                        (from.pos.add(back.with_len(px(140))), back)
                    } else {
                        (from.pos, back)
                    });
                }
                v
            }
        };
        for (pos, dir) in shots {
            let f = Origin {
                pos,
                player: from.player && pos == from.pos,
            };
            any |= self.fire_one(content, seat, id, s, f, Some(dir));
        }
        any
    }
}

/// What one payload made: its shots and effects, and what it hit at once.
struct Made {
    shots: std::ops::Range<usize>,
    effects: std::ops::Range<usize>,
    log: Vec<(V, bool)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_packs_into_one_number_and_back() {
        for (a, b) in [
            (Spot::Socket(0, 0), Spot::Socket(3, 7)),
            (Spot::Spare(5), Spot::Socket(2, 1)),
            (Spot::Socket(1, 15), Spot::Spare(63)),
        ] {
            let v = pack_move(a, b);
            assert_ne!(v, 0);
            assert_eq!(unpack_move(v), Some((a, b)));
            assert_eq!(Spot::parse(&a.text()), Some(a));
        }
        assert_eq!(unpack_move(0), None);
        assert_eq!(Spot::parse("r9s0"), None);
        assert_eq!(Spot::parse("nonsense"), None);
    }
}
