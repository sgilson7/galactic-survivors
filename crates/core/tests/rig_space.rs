//! Rig space (T17): rigs played in one short fixed scene and checked
//! against rules that hold for every rig, whatever its parts.
//!
//! A realistic rig is a row of 1 to `rig_sockets` tokens (6 with two Extra
//! Sockets) holding at least one payload, with no payload twice; a part may
//! appear more than once. Empty sockets are left out: the gap rule below
//! checks that a gap changes nothing.
//!
//! The everyday test plays every rig of one and two tokens and a fixed
//! sample of longer ones. The ignored sweep plays a whole length or a
//! random sample of any size:
//!
//!   RIG_LEN=3 cargo test -p tool_core --test rig_space -- --ignored --nocapture
//!   RIG_LEN=4 RIG_N=200000 RIG_SEED=7 cargo test ... (a sample)
//!   RIG_LEN=4 RIG_SHARD=0/8 cargo test ...            (one eighth of all)
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
//!   after a Wormhole a miss is only noted, since the payload may hit nothing;
//! * formation: on the first tick, a payload after a formation makes the
//!   formation's copies times what it makes alone.
//!
//! One more is counted but not failed: a payload before the first trigger
//! that was never seen. A shot that hits on the tick it is fired is gone
//! before it can be seen (Wormhole starts it inside an enemy).

use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use tool_core::content::{Content, Kind, PartKind};
use tool_core::geom::{px, ticks};
use tool_core::meta::Save;
use tool_core::world::{EffectKind, Input, Slot, World, DASH};

/// Ticks a rig is played for: four seconds.
const SCENE_TICKS: u32 = 240;
const MAX_SHOTS: usize = 2048;
const MAX_EFFECTS: usize = 1024;
const MAX_ECHOES: usize = 512;

fn quiet() -> Content {
    let mut c = Content::parse(include_str!("../../../data/content.json")).unwrap();
    c.waves.clear();
    c.events.minibosses.clear();
    c.events.rifts_s.clear();
    c.events.raid.from_s = 100_000;
    c.events.raid.to_s = 100_000;
    c.events.final_boss.at_s = 100_000;
    c
}

/// Every token a socket can hold: payloads first, then parts.
struct Tokens {
    all: Vec<String>,
    payloads: usize,
}

fn tokens(c: &Content) -> Tokens {
    let mut all: Vec<String> = c
        .abilities
        .iter()
        .chain(c.evolutions.iter())
        .map(|a| a.id.clone())
        .collect();
    let payloads = all.len();
    all.extend(c.parts.iter().map(|p| p.id.clone()));
    Tokens { all, payloads }
}

/// The rig with index `i` among all rows of `len` tokens, or None when it
/// holds no payload or a payload twice.
fn rig_at(t: &Tokens, len: usize, mut i: u64) -> Option<Vec<usize>> {
    let n = t.all.len() as u64;
    let mut row = Vec::with_capacity(len);
    for _ in 0..len {
        row.push((i % n) as usize);
        i /= n;
    }
    let pays: Vec<usize> = row.iter().copied().filter(|k| *k < t.payloads).collect();
    let mut seen = pays.clone();
    seen.sort_unstable();
    seen.dedup();
    if pays.is_empty() || seen.len() != pays.len() {
        return None;
    }
    Some(row)
}

/// How many realistic rigs of `len` tokens there are: k payloads, all
/// different, in any k of the places, and parts in the rest.
fn space(t: &Tokens, len: usize) -> u64 {
    let (p, q) = (t.payloads as u64, (t.all.len() - t.payloads) as u64);
    (1..=len as u64)
        .map(|k| {
            let choose = (0..k).fold(1u64, |a, i| a * (len as u64 - i) / (i + 1));
            let perm = (0..k).fold(1u64, |a, i| a * (p - i));
            choose * perm * q.pow((len as u64 - k) as u32)
        })
        .sum()
}

fn rows(t: &Tokens, len: usize) -> u64 {
    (t.all.len() as u64).pow(len as u32)
}

/// A small fixed generator for picking samples (splitmix64).
struct Mix(u64);
impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

fn sample(t: &Tokens, len: usize, n: usize, seed: u64) -> Vec<Vec<usize>> {
    let mut m = Mix(seed ^ (len as u64) << 32);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        if let Some(r) = rig_at(t, len, m.next() % rows(t, len)) {
            out.push(r);
        }
    }
    out
}

#[derive(Default, Clone)]
struct Tally {
    played: u64,
    failures: Vec<String>,
    never_fired: BTreeMap<String, u64>,
    /// A few of the rigs behind the counts above.
    notes: Vec<String>,
    most_shots: usize,
    most_effects: usize,
    most_echoes: usize,
}

impl Tally {
    fn add(&mut self, o: Tally) {
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
fn scene(c: &Content, row: &[String], level: i32, gap: bool) -> World {
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
fn play(
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
fn print(w: &World) -> Vec<i64> {
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
fn expected_recharge(c: &Content, row: &[String], level: i32) -> Option<i32> {
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
fn on_recharge(c: &Content, row: &[String]) -> Vec<String> {
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
fn made(w: &World, id: &str) -> usize {
    w.shots.iter().filter(|s| s.seat == 0 && s.ability == id).count()
        + w.effects
            .iter()
            .filter(|e| e.seat == 0 && e.ability == id)
            .count()
}

/// The formation rule: on the first tick, a payload after a formation makes
/// `copies` times what it makes without the formation.
fn formation(c: &Content, row: &[String], level: i32) -> Result<(), String> {
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
struct Played {
    failures: Vec<String>,
    notes: Vec<String>,
    fired: i32,
    /// Damage done to the six enemies that do not die.
    dealt: i64,
    kills: u32,
    /// The payloads seen, and the fields left, in the order first seen.
    payloads: Vec<String>,
    fields: Vec<String>,
    reactions: u32,
    depth: u32,
    /// A fingerprint of the world at the end.
    print: u64,
    most_shots: usize,
    most_effects: usize,
    most_echoes: usize,
}

fn fnv(h: u64, bytes: &[u8]) -> u64 {
    let mut h = h;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

const FNV: u64 = 0xcbf2_9ce4_8422_2325;

/// Plays one rig and judges it against every rule.
fn judge(c: &Content, row: &[String], level: i32, deep: bool) -> Played {
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
        .find(|s| *s != "rune" && !s.starts_with("field:") && !row.contains(s))
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
                // nothing, and nothing hit is nothing landed on
                if row[..at].iter().any(|id| id == "wormhole") {
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
        .filter(|s| !s.starts_with("field:"))
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

fn check(c: &Content, t: &Tokens, row: &[usize], level: i32, deep: bool, tally: &mut Tally) {
    let row: Vec<String> = row.iter().map(|k| t.all[*k].clone()).collect();
    let name = row.join(" ");
    tally.played += 1;
    let p = judge(c, &row, level, deep);
    for f in p.failures {
        tally.failures.push(format!("[{name}] L{level}: {f}"));
    }
    for n in p.notes {
        if let Some(id) = n.strip_prefix("never fired: ") {
            *tally.never_fired.entry(id.to_string()).or_default() += 1;
        }
        tally.notes.push(format!("[{name}] L{level}: {n}"));
    }
    tally.most_shots = tally.most_shots.max(p.most_shots);
    tally.most_effects = tally.most_effects.max(p.most_effects);
    tally.most_echoes = tally.most_echoes.max(p.most_echoes);
}

fn run_all(c: &Content, t: &Tokens, rigs: &[Vec<usize>], level: i32, deep_every: usize) -> Tally {
    let threads = std::env::var("RIG_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1));
    let chunk = rigs.len().div_ceil(threads).max(1);
    let mut total = Tally::default();
    std::thread::scope(|s| {
        let jobs: Vec<_> = rigs
            .chunks(chunk)
            .enumerate()
            .map(|(j, part)| {
                s.spawn(move || {
                    let mut tally = Tally::default();
                    for (i, row) in part.iter().enumerate() {
                        let deep = deep_every > 0 && (j * chunk + i).is_multiple_of(deep_every);
                        check(c, t, row, level, deep, &mut tally);
                    }
                    tally
                })
            })
            .collect();
        for j in jobs {
            total.add(j.join().unwrap());
        }
    });
    total
}

fn report(what: &str, n: usize, secs: f64, t: &Tally) {
    println!(
        "{what}: {n} rigs in {secs:.2} s, {:.0} a second; most shots {}, effects {}, echoes {}",
        n as f64 / secs,
        t.most_shots,
        t.most_effects,
        t.most_echoes
    );
    if !t.never_fired.is_empty() {
        println!("  never fired on the recharge (rigs): {:?}", t.never_fired);
    }
    if std::env::var("RIG_NOTES").is_ok() {
        for n in &t.notes {
            println!("  note {n}");
        }
    }
    for f in t.failures.iter().take(40) {
        println!("  FAIL {f}");
    }
    if t.failures.len() > 40 {
        println!("  ... {} failures in all", t.failures.len());
    }
}

fn quiet_hook() {
    std::panic::set_hook(Box::new(|_| {}));
}

/// Everyday: every rig of one and two tokens, and 400 each of three and
/// four, at level 1; half of each again at level 5.
#[test]
fn every_short_rig_and_a_sample_of_long_ones_keep_every_rule() {
    let c = quiet();
    let t = tokens(&c);
    let mut rigs: Vec<Vec<usize>> = Vec::new();
    for len in 1..=2 {
        rigs.extend((0..rows(&t, len)).filter_map(|i| rig_at(&t, len, i)));
    }
    let short = rigs.len();
    rigs.extend(sample(&t, 3, 400, 1));
    rigs.extend(sample(&t, 4, 400, 1));
    quiet_hook();
    let start = Instant::now();
    let one = run_all(&c, &t, &rigs, 1, 4);
    let top: Vec<Vec<usize>> = rigs.iter().step_by(2).cloned().collect();
    let five = run_all(&c, &t, &top, 5, 4);
    let _ = std::panic::take_hook();
    let secs = start.elapsed().as_secs_f64();
    report("everyday", rigs.len() + top.len(), secs, &one);
    let mut all = one;
    all.add(five);
    assert_eq!(short, 29 + 2378, "the space counted in MEASUREMENTS.md");
    assert_eq!(space(&t, 2), 2378);
    assert_eq!(space(&t, 4), 8_772_732);
    assert!(
        all.failures.is_empty(),
        "{} rigs broke a rule:\n{}",
        all.failures.len(),
        all.failures.join("\n")
    );
}

/// Rigs the logbook found breaking a rule, kept here once fixed.
const FOUND: &[(&str, i32)] = &[
    // a tether fired by a hit during the effects update removed the old
    // tether from the list being walked: a panic (T18)
    ("orbital_lance on_hit tether", 1),
];

#[test]
fn rigs_the_logbook_found_keep_every_rule() {
    let c = quiet();
    quiet_hook();
    let bad: Vec<String> = FOUND
        .iter()
        .flat_map(|(rig, level)| {
            let row: Vec<String> = rig.split(' ').map(|s| s.to_string()).collect();
            judge(&c, &row, *level, true)
                .failures
                .into_iter()
                .map(move |f| format!("[{rig}] L{level}: {f}"))
        })
        .collect();
    let _ = std::panic::take_hook();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// A larger sweep: RIG_LEN (1-6), RIG_N (a sample; all when unset),
/// RIG_SHARD (k/n of all), RIG_LEVEL, RIG_SEED, RIG_DEEP (1 in n get the
/// gap and rollback rules; 0 for none), RIG_THREADS.
#[test]
#[ignore]
fn sweep() {
    let get = |k: &str, d: u64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let c = quiet();
    let t = tokens(&c);
    let len = get("RIG_LEN", 3) as usize;
    let level = get("RIG_LEVEL", 1) as i32;
    let deep = get("RIG_DEEP", 4) as usize;
    let rigs: Vec<Vec<usize>> = match std::env::var("RIG_N").ok().and_then(|v| v.parse().ok()) {
        Some(n) => sample(&t, len, n, get("RIG_SEED", 1)),
        None => {
            let (k, n) = std::env::var("RIG_SHARD")
                .ok()
                .and_then(|s| {
                    let (a, b) = s.split_once('/')?;
                    Some((a.parse::<u64>().ok()?, b.parse::<u64>().ok()?))
                })
                .unwrap_or((0, 1));
            let all = rows(&t, len);
            let (from, to) = (all * k / n, all * (k + 1) / n);
            (from..to).filter_map(|i| rig_at(&t, len, i)).collect()
        }
    };
    quiet_hook();
    let start = Instant::now();
    let tally = run_all(&c, &t, &rigs, level, deep);
    let _ = std::panic::take_hook();
    report(
        &format!("sweep len {len} level {level}"),
        rigs.len(),
        start.elapsed().as_secs_f64(),
        &tally,
    );
    assert!(
        tally.failures.is_empty(),
        "{} rigs broke a rule",
        tally.failures.len()
    );
}

// ---- the logbook (T18) -------------------------------------------------------
//
// Everything above this line is the scene and the rules; its text is part of
// every log line's key, so changing it makes every line stale. Everything
// below is bookkeeping and may change freely.
//
// The full log stays on this computer, in `rig-log/` (not in git): one line
// for every rig played, in files of 4,096 rig numbers. The repository keeps
// a summary in `tests/rig-log/`: one line for each file, every rig that
// failed or was noted, and what changed run by run.
//
//   cargo test -p tool_core --test rig_space -- --ignored --nocapture logbook
//
// RIG_BUDGET_S (seconds, default 120), RIG_LENGTHS (default 1,2,3,4),
// RIG_LEVELS (default 1). Each run plays only rigs whose line is missing or
// stale, shortest rigs first, until the budget is spent.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const BLOCK: u64 = 4096;
/// The files a rig's play depends on; a change to any makes every line stale.
const RULE_FILES: &[&str] = &[
    "crates/core/src/world.rs",
    "crates/core/src/arsenal.rs",
    "crates/core/src/rig.rs",
    "crates/core/src/field.rs",
    "crates/core/src/rune.rs",
    "crates/core/src/geom.rs",
    "crates/core/src/fx.rs",
    "crates/core/src/rng.rs",
    "crates/core/src/content.rs",
    "crates/core/src/meta.rs",
];
const MARK: &str = "// ---- the logbook (T18)";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What every line depends on: the rule code, the scene and rules above,
/// the order of the tokens, and the data that is not an ability or a part.
/// Each token's own data is keyed separately.
struct Keys {
    code: u64,
    token: Vec<u64>,
}

fn keys(t: &Tokens) -> Keys {
    let r = root();
    let mut code = FNV;
    for f in RULE_FILES {
        code = fnv(code, &std::fs::read(r.join(f)).unwrap());
    }
    let me = std::fs::read_to_string(r.join("crates/core/tests/rig_space.rs")).unwrap();
    code = fnv(code, &me.as_bytes()[..me.find(MARK).unwrap()]);
    code = fnv(code, t.all.join(",").as_bytes());
    let text = std::fs::read_to_string(r.join("data/content.json")).unwrap();
    let mut data: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut entry = BTreeMap::new();
    for list in ["abilities", "evolutions", "parts"] {
        for e in data[list].as_array().unwrap() {
            entry.insert(e["id"].as_str().unwrap().to_string(), e.to_string());
        }
        data[list] = serde_json::Value::Null;
    }
    code = fnv(code, data.to_string().as_bytes());
    let token = t
        .all
        .iter()
        .map(|id| fnv(FNV, entry.get(id).unwrap().as_bytes()))
        .collect();
    Keys { code, token }
}

fn rig_key(k: &Keys, level: i32, row: &[usize]) -> u64 {
    let mut h = fnv(k.code, &level.to_le_bytes());
    for i in row {
        h = fnv(h, &k.token[*i].to_le_bytes());
    }
    h
}

/// One line of the log.
#[derive(Clone)]
struct Line {
    number: u64,
    rig: String,
    result: String,
    behaviour: String,
    print: u64,
    key: u64,
    tested: String,
}

impl Line {
    fn text(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{:016x}\t{:016x}\t{}",
            self.number, self.rig, self.result, self.behaviour, self.print, self.key, self.tested
        )
    }

    fn parse(s: &str) -> Option<Line> {
        let f: Vec<&str> = s.split('\t').collect();
        if f.len() != 7 {
            return None;
        }
        Some(Line {
            number: f[0].parse().ok()?,
            rig: f[1].to_string(),
            result: f[2].to_string(),
            behaviour: f[3].to_string(),
            print: u64::from_str_radix(f[4], 16).ok()?,
            key: u64::from_str_radix(f[5], 16).ok()?,
            tested: f[6].to_string(),
        })
    }
}

fn result_text(p: &Played) -> String {
    let mut r: Vec<String> = p.failures.iter().map(|f| format!("FAIL {f}")).collect();
    r.extend(p.notes.iter().map(|n| format!("note {n}")));
    if r.is_empty() {
        "ok".into()
    } else {
        r.join("; ").replace('\t', " ")
    }
}

fn behaviour_text(p: &Played) -> String {
    let list = |v: &[String]| {
        if v.is_empty() {
            "-".to_string()
        } else {
            v.join(",")
        }
    };
    format!(
        "fired {} dealt {} kills {} made {} fields {} reactions {} depth {}",
        p.fired,
        p.dealt,
        p.kills,
        list(&p.payloads),
        list(&p.fields),
        p.reactions,
        p.depth
    )
}

/// Today's date from the system clock (UTC), as 2026-10-15.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // days to a civil date (Howard Hinnant's algorithm)
    let z = secs / 86_400 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

/// The commit the rules were read from, with + when the tree has changes.
fn commit() -> String {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(root())
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };
    let head = git(&["rev-parse", "--short", "HEAD"]);
    let dirty = !git(&["status", "--porcelain", "--untracked-files=no"]).is_empty();
    format!(
        "{}{}",
        if head.is_empty() { "unknown" } else { &head },
        if dirty { "+" } else { "" }
    )
}

/// The index of the local log: for each file of 4,096 rig numbers, the key
/// it was last played under, how many rigs it holds, a fingerprint of their
/// behaviour, and the results.
#[derive(Clone, Default)]
struct Block {
    key: u64,
    rigs: u64,
    failed: u64,
    noted: u64,
    print: u64,
    tested: String,
}

type Index = BTreeMap<(usize, i32, u64), Block>;

fn read_index(p: &Path, tokens: u64) -> Index {
    let mut out = Index::new();
    let Ok(f) = std::fs::File::open(p) else { return out };
    let mut lines = std::io::BufReader::new(f).lines().map_while(Result::ok);
    // a different list of tokens numbers rigs differently: start again
    if lines.next() != Some(format!("tokens\t{tokens:016x}")) {
        return out;
    }
    for l in lines {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() != 9 {
            continue;
        }
        let (Ok(len), Ok(level), Ok(b)) = (f[0].parse(), f[1].parse(), f[2].parse()) else {
            continue;
        };
        out.insert(
            (len, level, b),
            Block {
                key: u64::from_str_radix(f[3], 16).unwrap_or(0),
                rigs: f[4].parse().unwrap_or(0),
                failed: f[5].parse().unwrap_or(0),
                noted: f[6].parse().unwrap_or(0),
                print: u64::from_str_radix(f[7], 16).unwrap_or(0),
                tested: f[8].to_string(),
            },
        );
    }
    out
}

fn write_index(p: &Path, tokens: u64, index: &Index) {
    let mut s = format!("tokens\t{tokens:016x}\n");
    for ((len, level, b), e) in index {
        s += &format!(
            "{len}\t{level}\t{b}\t{:016x}\t{}\t{}\t{}\t{:016x}\t{}\n",
            e.key, e.rigs, e.failed, e.noted, e.print, e.tested
        );
    }
    std::fs::write(p, s).unwrap();
}

fn block_path(dir: &Path, len: usize, level: i32, b: u64) -> PathBuf {
    dir.join(format!("L{len}-level{level}"))
        .join(format!("{b:06}.tsv"))
}

fn read_block(p: &Path) -> BTreeMap<u64, Line> {
    let Ok(f) = std::fs::File::open(p) else {
        return BTreeMap::new();
    };
    std::io::BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| Line::parse(&l))
        .map(|l| (l.number, l))
        .collect()
}

fn write_block(p: &Path, lines: &BTreeMap<u64, Line>) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut f = std::io::BufWriter::new(std::fs::File::create(p).unwrap());
    for l in lines.values() {
        writeln!(f, "{}", l.text()).unwrap();
    }
}

/// Plays rigs on every core and returns what each did.
fn play_all(c: &Content, rows: &[(u64, Vec<String>)], level: i32) -> Vec<Played> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let chunk = rows.len().div_ceil(threads).max(1);
    let mut out = Vec::with_capacity(rows.len());
    std::thread::scope(|s| {
        let jobs: Vec<_> = rows
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    part.iter()
                        .map(|(n, row)| judge(c, row, level, n % 4 == 0))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for j in jobs {
            out.extend(j.join().unwrap());
        }
    });
    out
}

#[test]
#[ignore]
fn logbook() {
    let get = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
    let budget = get("RIG_BUDGET_S", "120").parse::<f64>().unwrap_or(120.0);
    let lengths: Vec<usize> = get("RIG_LENGTHS", "1,2,3,4")
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect();
    let levels: Vec<i32> = get("RIG_LEVELS", "1")
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect();
    let c = quiet();
    let t = tokens(&c);
    let k = keys(&t);
    let token_list = fnv(FNV, t.all.join(",").as_bytes());
    let dir = root().join("rig-log");
    let repo = root().join("tests/rig-log");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&repo).unwrap();
    let mut index = read_index(&dir.join("index.tsv"), token_list);
    if index.is_empty() {
        // a new list of tokens renumbers every rig: the old files go
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            if e.path().is_dir() {
                std::fs::remove_dir_all(e.path()).unwrap();
            }
        }
    }
    let (date, at) = (today(), commit());
    let tested = format!("{date} {at}");
    let start = Instant::now();
    let (mut played, mut fresh, mut changed) = (0u64, 0u64, Vec::new());
    let mut notable: BTreeMap<String, String> = std::fs::read_to_string(dir.join("notable.tsv"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.splitn(4, '\t').collect();
            (f.len() == 4).then(|| (format!("{}\t{}\t{}", f[0], f[1], f[2]), f[3].to_string()))
        })
        .collect();
    quiet_hook();
    'all: for &level in &levels {
        for &len in &lengths {
            let blocks = rows(&t, len).div_ceil(BLOCK);
            for b in 0..blocks {
                if start.elapsed().as_secs_f64() > budget {
                    break 'all;
                }
                let span = b * BLOCK..((b + 1) * BLOCK).min(rows(&t, len));
                let want: Vec<(u64, Vec<usize>, u64)> = span
                    .filter_map(|n| rig_at(&t, len, n).map(|r| (n, r)))
                    .map(|(n, r)| {
                        let key = rig_key(&k, level, &r);
                        (n, r, key)
                    })
                    .collect();
                let block_key = want.iter().fold(FNV, |h, (_, _, key)| fnv(h, &key.to_le_bytes()));
                let entry = index.get(&(len, level, b));
                if entry.map(|e| e.key == block_key && e.rigs == want.len() as u64) == Some(true) {
                    fresh += want.len() as u64;
                    continue;
                }
                let path = block_path(&dir, len, level, b);
                let mut lines = read_block(&path);
                let todo: Vec<(u64, Vec<String>)> = want
                    .iter()
                    .filter(|(n, _, key)| lines.get(n).map(|l| l.key != *key).unwrap_or(true))
                    .map(|(n, r, _)| (*n, r.iter().map(|i| t.all[*i].clone()).collect()))
                    .collect();
                let keyed: BTreeMap<u64, u64> = want.iter().map(|(n, _, key)| (*n, *key)).collect();
                for ((n, row), p) in todo.iter().zip(play_all(&c, &todo, level)) {
                    let line = Line {
                        number: *n,
                        rig: row.join(" "),
                        result: result_text(&p),
                        behaviour: behaviour_text(&p),
                        print: p.print,
                        key: keyed[n],
                        tested: tested.clone(),
                    };
                    let id = format!("{len}\t{level}\t{n}");
                    if let Some(old) = lines.get(n) {
                        if old.print != line.print {
                            changed.push(format!(
                                "{id}\t{}\twas: {} | {}\tnow: {} | {}",
                                line.rig, old.result, old.behaviour, line.result, line.behaviour
                            ));
                        }
                    }
                    if line.result == "ok" {
                        notable.remove(&id);
                    } else {
                        notable.insert(id, format!("{}\t{}\t{}", line.rig, line.result, tested));
                    }
                    lines.insert(*n, line);
                    played += 1;
                }
                // a rig that is no longer valid leaves the log
                lines.retain(|n, _| keyed.contains_key(n));
                write_block(&path, &lines);
                index.insert(
                    (len, level, b),
                    Block {
                        key: block_key,
                        rigs: lines.len() as u64,
                        failed: lines.values().filter(|l| l.result.contains("FAIL")).count() as u64,
                        noted: lines
                            .values()
                            .filter(|l| l.result != "ok" && !l.result.contains("FAIL"))
                            .count() as u64,
                        print: lines.values().fold(FNV, |h, l| fnv(h, &l.print.to_le_bytes())),
                        tested: tested.clone(),
                    },
                );
            }
        }
    }
    let _ = std::panic::take_hook();
    let secs = start.elapsed().as_secs_f64();
    write_index(&dir.join("index.tsv"), token_list, &index);
    let notable_text: String = notable.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect();
    std::fs::write(dir.join("notable.tsv"), &notable_text).unwrap();
    // every change goes to the local log; the repository keeps the first 200
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("changes.tsv"))
        .unwrap();
    for l in &changed {
        writeln!(log, "{date}\t{at}\t{l}").unwrap();
    }
    write_summary(&repo, &t, &index, &notable_text, &changed, &tested, played, secs);
    let logged: u64 = index.values().map(|e| e.rigs).sum();
    println!(
        "logbook: played {played} rigs in {secs:.1} s, {fresh} already fresh; {} changed behaviour; \
         {logged} rigs in the log; {} failing or noted",
        changed.len(),
        notable.len()
    );
    let failing = notable.values().filter(|v| v.contains("FAIL")).count();
    assert_eq!(
        failing, 0,
        "rigs in the log break a rule: see rig-log/notable.tsv"
    );
}

/// The repository's copy: a line for each file of the log, every failing or
/// noted rig, and this run's changes (at most 200 of them).
#[allow(clippy::too_many_arguments)]
fn write_summary(
    repo: &Path,
    t: &Tokens,
    index: &Index,
    notable: &str,
    changed: &[String],
    tested: &str,
    played: u64,
    secs: f64,
) {
    let mut s = String::from(
        "# One line for each file of 4,096 rig numbers in the local log (rig-log/).\n\
         # length\tlevel\tblock\trigs\tfailed\tnoted\tbehaviour\tkey\tlast tested\n",
    );
    let mut by_len: BTreeMap<(usize, i32), (u64, u64)> = BTreeMap::new();
    for ((len, level, b), e) in index {
        s += &format!(
            "{len}\t{level}\t{b}\t{}\t{}\t{}\t{:016x}\t{:016x}\t{}\n",
            e.rigs, e.failed, e.noted, e.print, e.key, e.tested
        );
        let x = by_len.entry((*len, *level)).or_default();
        x.0 += e.rigs;
        x.1 += e.failed;
    }
    std::fs::write(repo.join("summary.tsv"), s).unwrap();
    std::fs::write(
        repo.join("notable.tsv"),
        format!("# length\tlevel\trig number\trig\tresult\tlast tested\n{notable}"),
    )
    .unwrap();
    let mut runs = std::fs::read_to_string(repo.join("runs.tsv")).unwrap_or_else(|_| {
        "# date and commit\tplayed\tseconds\tchanged\tlogged by length and level\n".into()
    });
    let cover: Vec<String> = by_len
        .iter()
        .map(|((len, level), (n, _))| format!("L{len} level {level}: {n} of {}", space(t, *len)))
        .collect();
    runs += &format!(
        "{tested}\t{played}\t{secs:.0}\t{}\t{}\n",
        changed.len(),
        cover.join(", ")
    );
    std::fs::write(repo.join("runs.tsv"), runs).unwrap();
    let mut ch = String::from(
        "# Rigs whose behaviour changed in the latest run (at most 200; all are in rig-log/changes.tsv)\n",
    );
    for l in changed.iter().take(200) {
        ch += l;
        ch += "\n";
    }
    std::fs::write(repo.join("changes.tsv"), ch).unwrap();
}
