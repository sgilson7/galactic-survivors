//! The scene and the rules every rig is judged by (T17). This file is the
//! logbook's key: change it, and every line in `rig-log/` is played again.
//! Bookkeeping lives in `rig_space.rs`, and changing that changes no line.
//!
//! The rules:
//!
//! * panic: playing the rig does not panic (overflow checks are on);
//! * recharge: the rig fires exactly as often as its first payload's
//!   recharge with every part's load added allows, worked out here from
//!   the data (T16); a rig with no payload before its first trigger never
//!   fires on its recharge;
//! * stray: every shot and effect belongs to a payload in the rig or a rune;
//! * depth: no trigger fires deeper than three;
//! * bounds: shots, effects, echoes and fields stay under fixed ceilings;
//! * health: no enemy has more health than its most;
//! * gap: the same rig with an empty socket in the middle ends in the same
//!   world;
//! * rollback: a copy taken halfway and played on ends in the same world;
//! * field: a payload that fired after a field modifier left that field
//!   somewhere in the scene (or reacted with another field); a tether is
//!   left out (alone it joins nothing), and so is a mine still waiting;
//!   after a Wormhole a miss is only noted, since the payload may hit nothing,
//!   and so is a miss by a payload that hit nothing because another payload
//!   in the same firing killed or knocked away its targets first;
//! * formation: on the first tick, a payload after a formation makes the
//!   formation's copies times what it makes alone.
//!
//! One more is noted but not failed: a payload before the first trigger
//! that was never seen. A shot that hits on the tick it is fired is gone
//! before it can be seen.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use tool_core::content::{Content, Kind, PartKind};
use tool_core::geom::{px, ticks};
use tool_core::meta::Save;
use tool_core::world::{EffectKind, Input, Slot, World, DASH};

/// Ticks a rig is played for: four seconds.
pub const SCENE_TICKS: u32 = 240;
pub const MAX_SHOTS: usize = 2048;
pub const MAX_EFFECTS: usize = 1024;
pub const MAX_ECHOES: usize = 512;

pub fn quiet() -> Content {
    let mut c = Content::parse(include_str!("../../../../data/content.json")).unwrap();
    c.waves.clear();
    c.events.minibosses.clear();
    c.events.rifts_s.clear();
    c.events.raid.from_s = 100_000;
    c.events.raid.to_s = 100_000;
    c.events.final_boss.at_s = 100_000;
    c
}

#[derive(Default, Clone)]
pub struct Tally {
    pub played: u64,
    pub failures: Vec<String>,
    pub never_fired: BTreeMap<String, u64>,
    /// A few of the rigs behind the counts above.
    pub notes: Vec<String>,
    pub most_shots: usize,
    pub most_effects: usize,
    pub most_echoes: usize,
}

impl Tally {
    pub fn add(&mut self, o: Tally) {
        self.played += o.played;
        self.failures.extend(o.failures);
        self.notes.extend(o.notes);
        for (k, v) in o.never_fired {
            *self.never_fired.entry(k).or_default() += v;
        }
        self.most_shots = self.most_shots.max(o.most_shots);
        self.most_effects = self.most_effects.max(o.most_effects);
        self.most_echoes = self.most_echoes.max(o.most_echoes);
    }
}

/// The world with the rig in rig 1, every payload held at `level`, and
/// six enemies that do not die around the player.
pub fn scene(c: &Content, row: &[String], level: i32, gap: bool) -> World {
    let load = Save::new(c, 1).loadout(c, "pathfinder");
    let mut w = World::new(c, 5, &[load]);
    let p = &mut w.players[0];
    p.abilities.clear();
    for r in p.rigs.iter_mut() {
        r.sockets.iter_mut().for_each(|s| s.clear());
    }
    for id in row {
        if c.ability(id).is_some() {
            p.abilities.push(Slot {
                id: id.clone(),
                level,
                cd: 0,
            });
        }
    }
    let mut sockets = row.to_vec();
    if gap {
        sockets.insert(row.len() / 2, String::new());
    }
    let width = sockets.len().max(p.rigs[0].sockets.len());
    sockets.resize(width, String::new());
    p.rigs[0].sockets = sockets;
    p.rigs[0].cd = 0;
    let carapace = c.enemy_index("carapace");
    for k in 0..6 {
        let at = tool_core::geom::from_angle(tool_core::fx::TURN * k / 6, px(220));
        let i = w.spawn_foe(c, carapace, at);
        w.foes[i].hp = 100_000;
        w.foes[i].max_hp = 100_000;
    }
    w
}

/// The player walks a square, dashes once, and an enemy that dies easily
/// appears near them every 40 ticks.
pub fn play(
    c: &Content,
    w: &mut World,
    from: u32,
    to: u32,
    seen: &mut Vec<String>,
    t: &mut Tally,
) -> Result<(), String> {
    let carapace = c.enemy_index("carapace");
    let cap = c.fields.cap;
    for k in from..to {
        let side = (k / 60) % 4;
        let (mx, my) = [(1, 0), (0, 1), (-1, 0), (0, -1)][side as usize];
        let flags = if k == 130 { DASH } else { 0 };
        let input = Input {
            mx,
            my,
            flags,
            ..Input::default()
        };
        if k % 40 == 0 {
            let near = w.players[0].pos.add(tool_core::geom::from_angle(
                tool_core::fx::TURN * (k as i32 / 40) / 5,
                px(140),
            ));
            let i = w.spawn_foe(c, carapace, near);
            w.foes[i].hp = 20;
            w.foes[i].max_hp = 20;
        }
        let p = &mut w.players[0];
        p.invulnerable = 1_000;
        p.offer = None;
        p.pending = 0;
        w.step(c, &[input]);
        for s in w.shots.iter().filter(|s| s.seat == 0) {
            if !seen.contains(&s.ability) {
                seen.push(s.ability.clone());
            }
            if !s.hit.is_empty() {
                let hit = format!("hit:{}", s.ability);
                if !seen.contains(&hit) {
                    seen.push(hit);
                }
            }
        }
        for f in w.fields.iter().filter(|f| f.seat == 0) {
            let id = format!("field:{}", f.kind.name());
            if !seen.contains(&id) {
                seen.push(id);
            }
        }
        for e in w.effects.iter().filter(|e| e.seat == 0) {
            if !seen.contains(&e.ability) {
                seen.push(e.ability.clone());
            }
            if !e.hits.is_empty() {
                let hit = format!("hit:{}", e.ability);
                if !seen.contains(&hit) {
                    seen.push(hit);
                }
            }
        }
        t.most_shots = t.most_shots.max(w.shots.len());
        t.most_effects = t.most_effects.max(w.effects.len());
        t.most_echoes = t.most_echoes.max(w.pending.len());
        if w.shots.len() > MAX_SHOTS
            || w.effects.len() > MAX_EFFECTS
            || w.pending.len() > MAX_ECHOES
            || w.fields.len() > cap
        {
            return Err(format!(
                "bounds: tick {k}: {} shots, {} effects, {} echoes, {} fields",
                w.shots.len(),
                w.effects.len(),
                w.pending.len(),
                w.fields.len()
            ));
        }
        if let Some(f) = w.foes.iter().find(|f| f.hp > f.max_hp) {
            return Err(format!("health: tick {k}: an enemy has {} of {}", f.hp, f.max_hp));
        }
    }
    Ok(())
}

/// More than `World::sum`: the effects, echoes and fields count too.
pub fn print(w: &World) -> Vec<i64> {
    let mut v = vec![
        w.sum() as i64,
        w.effects.len() as i64,
        w.pending.len() as i64,
        w.fields.len() as i64,
    ];
    for e in &w.effects {
        v.extend([e.pos.x as i64, e.pos.y as i64, e.radius as i64]);
    }
    for s in &w.shots {
        v.extend([s.pos.x as i64, s.pos.y as i64]);
    }
    v
}

/// The recharge worked out from the data: the first payload before any
/// trigger, at its level, with every part's load.
pub fn expected_recharge(c: &Content, row: &[String], level: i32) -> Option<i32> {
    let mut first = None;
    for id in row {
        if let Some(p) = c.part(id) {
            if p.kind == PartKind::Trigger {
                break;
            }
            continue;
        }
        let a = c.ability(id)?;
        let lv = &a.levels[(level as usize - 1).min(a.levels.len() - 1)];
        first = Some(ticks(lv.cooldown_ms));
        break;
    }
    let load: i32 = row.iter().filter_map(|id| c.part(id)).map(|p| p.load_pct).sum();
    first.map(|cd| (cd * (100 + load.max(0)) / 100).max(1))
}

/// The payloads that fire on the recharge: those before the first trigger.
pub fn on_recharge(c: &Content, row: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for id in row {
        match c.part(id) {
            Some(p) if p.kind == PartKind::Trigger => break,
            Some(_) => {}
            None => out.push(id.clone()),
        }
    }
    out
}

/// How many shots and effects of `id` the player has.
pub fn made(w: &World, id: &str) -> usize {
    w.shots.iter().filter(|s| s.seat == 0 && s.ability == id).count()
        + w.effects
            .iter()
            .filter(|e| e.seat == 0 && e.ability == id)
            .count()
}

/// The formation rule: on the first tick, a payload after a formation makes
/// `copies` times what it makes without the formation.
pub fn formation(c: &Content, row: &[String], level: i32) -> Result<(), String> {
    let Some(i) = row.iter().position(|id| c.part(id).is_none()) else {
        return Ok(());
    };
    if row[..i]
        .iter()
        .any(|id| c.part(id).map(|p| p.kind == PartKind::Trigger).unwrap_or(false))
    {
        return Ok(());
    }
    // the last formation before the payload is the one that counts
    let Some(f) = row[..i]
        .iter()
        .rev()
        .find_map(|id| c.part(id).filter(|p| p.kind == PartKind::Formation))
    else {
        return Ok(());
    };
    let bare: Vec<String> = row
        .iter()
        .enumerate()
        .filter(|(k, id)| *k >= i || c.part(id).map(|p| p.kind != PartKind::Formation).unwrap_or(true))
        .map(|(_, id)| id.clone())
        .collect();
    let mut a = scene(c, row, level, false);
    a.step(c, &[Input::default()]);
    let mut b = scene(c, &bare, level, false);
    b.step(c, &[Input::default()]);
    let (with, without) = (made(&a, &row[i]), made(&b, &row[i]));
    if with != f.copies as usize * without {
        return Err(format!(
            "formation: {} makes {with} where {} copies of {without} were due",
            f.id, f.copies
        ));
    }
    Ok(())
}

/// What one rig did in the scene, and which rules it broke.
#[derive(Default, Clone)]
pub struct Played {
    pub failures: Vec<String>,
    pub notes: Vec<String>,
    pub fired: i32,
    /// Damage done to the six enemies that do not die.
    pub dealt: i64,
    pub kills: u32,
    /// The payloads seen, and the fields left, in the order first seen.
    pub payloads: Vec<String>,
    pub fields: Vec<String>,
    pub reactions: u32,
    pub depth: u32,
    /// A fingerprint of the world at the end.
    pub print: u64,
    pub most_shots: usize,
    pub most_effects: usize,
    pub most_echoes: usize,
}

pub fn fnv(h: u64, bytes: &[u8]) -> u64 {
    let mut h = h;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

pub const FNV: u64 = 0xcbf2_9ce4_8422_2325;

/// Plays one rig and judges it against every rule.
pub fn judge(c: &Content, row: &[String], level: i32, deep: bool) -> Played {
    let run = catch_unwind(AssertUnwindSafe(
        || -> Result<(Vec<i64>, World, Vec<String>, Tally), String> {
            let mut lt = Tally::default();
            formation(c, row, level)?;
            let mut w = scene(c, row, level, false);
            let mut seen = Vec::new();
            play(c, &mut w, 0, SCENE_TICKS / 2, &mut seen, &mut lt)?;
            let half = w.clone();
            play(c, &mut w, SCENE_TICKS / 2, SCENE_TICKS, &mut seen, &mut lt)?;
            let end = print(&w);
            if deep {
                let mut again = half;
                let mut s2 = Vec::new();
                play(
                    c,
                    &mut again,
                    SCENE_TICKS / 2,
                    SCENE_TICKS,
                    &mut s2,
                    &mut Tally::default(),
                )?;
                if print(&again) != end {
                    return Err("rollback: a copy taken halfway ends in a different world".into());
                }
                let mut g = scene(c, row, level, true);
                play(c, &mut g, 0, SCENE_TICKS, &mut Vec::new(), &mut Tally::default())?;
                if print(&g) != end {
                    return Err("gap: an empty socket in the middle changes the world".into());
                }
            }
            Ok((end, w, seen, lt))
        },
    ));
    let mut out = Played::default();
    let (end, w, seen) = match run {
        Err(e) => {
            let why = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            out.failures.push(format!("panic: {why}"));
            return out;
        }
        Ok(Err(why)) => {
            out.failures.push(why);
            return out;
        }
        Ok(Ok((end, w, seen, lt))) => {
            out.most_shots = lt.most_shots;
            out.most_effects = lt.most_effects;
            out.most_echoes = lt.most_echoes;
            (end, w, seen)
        }
    };
    let p = &w.players[0];
    let fired = p.rigs[0].fired as i32;
    if let Some(cd) = expected_recharge(c, row, level) {
        let most = (SCENE_TICKS as i32 + cd - 1) / cd;
        if fired != most {
            out.failures.push(format!(
                "recharge: fired {fired} times; a recharge of {cd} ticks fires {most}"
            ));
        }
    } else if fired > 0 {
        out.failures.push(format!(
            "recharge: fired {fired} times with no payload before its first trigger"
        ));
    }
    if p.record.deepest_trigger > 3 {
        out.failures.push(format!("depth: {}", p.record.deepest_trigger));
    }
    if let Some(s) = seen
        .iter()
        .find(|s| *s != "rune" && !s.starts_with("field:") && !s.starts_with("hit:") && !row.contains(s))
    {
        out.failures.push(format!("stray: a {s} that is not in the rig"));
    }
    // the field rule: the last field modifier before the first payload
    // leaves its field, or reacted with another. A lone tether in a solo
    // scene joins nothing and so cuts nothing.
    let first = on_recharge(c, row);
    let lands = |id: &str| c.ability(id).map(|a| a.kind != Kind::Tether).unwrap_or(false);
    if let Some(at) = first
        .first()
        .filter(|p| lands(p))
        .and_then(|p| row.iter().position(|id| id == p))
    {
        if let Some(kind) = row[..at]
            .iter()
            .rev()
            .find_map(|id| c.part(id).filter(|q| !q.field.is_empty()))
        {
            let want = format!("field:{}", kind.field);
            // a mine that has not gone off has not landed yet
            let waiting = w
                .effects
                .iter()
                .any(|e| e.seat == 0 && e.ability == row[at] && e.kind == EffectKind::Bomb);
            if seen.contains(&row[at])
                && !waiting
                && !seen.contains(&want)
                && !seen.iter().any(|s| s == "field:steam")
                && p.record.reactions == 0
            {
                let why = format!("field: {} left no {} field", row[at], kind.field);
                // out of a wormhole a payload flies at the player and may hit
                // nothing, and nothing hit is nothing landed on. So may a
                // payload whose targets another payload in the same firing
                // killed or knocked away first (found by the logbook: Swarm
                // Battery clearing a Boomerang Disc's path).
                let crowded = first.len() > 1 && !seen.contains(&format!("hit:{}", row[at]));
                if crowded || row[..at].iter().any(|id| id == "wormhole") {
                    out.notes.push(why);
                } else {
                    out.failures.push(why);
                }
            }
        }
    }
    for id in first {
        if !seen.contains(&id) {
            out.notes.push(format!("never fired: {id}"));
        }
    }
    out.fired = fired;
    out.dealt = w
        .foes
        .iter()
        .filter(|f| f.max_hp == 100_000)
        .map(|f| (f.max_hp - f.hp) as i64)
        .sum();
    out.kills = p.record.kills;
    out.payloads = seen
        .iter()
        .filter(|s| !s.starts_with("field:") && !s.starts_with("hit:"))
        .cloned()
        .collect();
    out.fields = seen
        .iter()
        .filter_map(|s| s.strip_prefix("field:").map(|f| f.to_string()))
        .collect();
    out.reactions = p.record.reactions;
    out.depth = p.record.deepest_trigger;
    out.print = end.iter().fold(FNV, |h, v| fnv(h, &v.to_le_bytes()));
    out
}
