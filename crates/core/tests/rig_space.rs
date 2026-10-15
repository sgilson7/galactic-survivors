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
//!   somewhere in the scene (or reacted with another field); orbits and
//!   tethers never land and are left out, and so is a mine still waiting;
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
    // a player has one tether an ability: each copy replaces the last
    if c.ability(&row[i])
        .map(|a| a.kind == Kind::Tether)
        .unwrap_or(false)
    {
        return Ok(());
    }
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

fn check(c: &Content, t: &Tokens, row: &[usize], level: i32, deep: bool, tally: &mut Tally) {
    let row: Vec<String> = row.iter().map(|k| t.all[*k].clone()).collect();
    let name = row.join(" ");
    tally.played += 1;
    let mut local = Tally::default();
    let run = catch_unwind(AssertUnwindSafe(
        || -> Result<(Vec<i64>, World, Vec<String>, Tally), String> {
            let mut lt = Tally::default();
            formation(c, &row, level)?;
            let mut w = scene(c, &row, level, false);
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
                let mut g = scene(c, &row, level, true);
                play(c, &mut g, 0, SCENE_TICKS, &mut Vec::new(), &mut Tally::default())?;
                if print(&g) != end {
                    return Err("gap: an empty socket in the middle changes the world".into());
                }
            }
            Ok((end, w, seen, lt))
        },
    ));
    let (w, seen) = match run {
        Err(e) => {
            let why = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            tally.failures.push(format!("panic: [{name}] L{level}: {why}"));
            return;
        }
        Ok(Err(why)) => {
            tally.failures.push(format!("[{name}] L{level}: {why}"));
            return;
        }
        Ok(Ok((_, w, seen, lt))) => {
            local.add(lt);
            (w, seen)
        }
    };
    let p = &w.players[0];
    let fired = p.rigs[0].fired as i32;
    if let Some(cd) = expected_recharge(c, &row, level) {
        let most = (SCENE_TICKS as i32 + cd - 1) / cd;
        if fired != most {
            tally.failures.push(format!(
                "recharge: [{name}] L{level}: fired {fired} times; a recharge of {cd} ticks fires {most}"
            ));
        }
    } else if fired > 0 {
        tally.failures.push(format!(
            "recharge: [{name}] L{level}: fired {fired} times with no payload before its first trigger"
        ));
    }
    if p.record.deepest_trigger > 3 {
        tally
            .failures
            .push(format!("depth: [{name}] L{level}: {}", p.record.deepest_trigger));
    }
    if let Some(s) = seen
        .iter()
        .find(|s| *s != "rune" && !s.starts_with("field:") && !row.contains(s))
    {
        tally
            .failures
            .push(format!("stray: [{name}] L{level}: a {s} that is not in the rig"));
    }
    // the field rule: the last field modifier before the first payload
    // leaves its field, or reacted with another. Orbits and tethers
    // never land, so they leave none (SECOND-ORDER R4.5, the person's call).
    let first = on_recharge(c, &row);
    let lands = |id: &str| {
        c.ability(id)
            .map(|a| !matches!(a.kind, Kind::Orbit | Kind::Tether))
            .unwrap_or(false)
    };
    if let Some(p) = first
        .first()
        .filter(|p| lands(p))
        .and_then(|p| row.iter().position(|id| id == p))
    {
        if let Some(kind) = row[..p]
            .iter()
            .rev()
            .find_map(|id| c.part(id).filter(|q| !q.field.is_empty()))
        {
            let want = format!("field:{}", kind.field);
            // a mine that has not gone off has not landed yet
            let waiting = w
                .effects
                .iter()
                .any(|e| e.seat == 0 && e.ability == row[p] && e.kind == EffectKind::Bomb);
            if seen.contains(&row[p])
                && !waiting
                && !seen.contains(&want)
                && !seen.iter().any(|s| s == "field:steam")
                && w.players[0].record.reactions == 0
            {
                let why = format!(
                    "field: [{name}] L{level}: {} left no {} field",
                    row[p], kind.field
                );
                // out of a wormhole a payload flies at the player and may hit
                // nothing, and nothing hit is nothing landed on
                if row[..p].iter().any(|id| id == "wormhole") {
                    local.notes.push(why);
                } else {
                    tally.failures.push(why);
                }
            }
        }
    }
    for id in first {
        if !seen.contains(&id) {
            local.notes.push(format!("never fired: [{name}] L{level}: {id}"));
            *local.never_fired.entry(id).or_default() += 1;
        }
    }
    local.played = 0;
    tally.add(local);
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
    assert!(
        all.failures.is_empty(),
        "{} rigs broke a rule:\n{}",
        all.failures.len(),
        all.failures.join("\n")
    );
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
