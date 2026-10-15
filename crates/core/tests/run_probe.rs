//! Probes (realtime kit): a bot plays whole runs and prints what happened.
//! They measure and print; they do not assert, and check.sh does not run
//! them. Numbers go in MEASUREMENTS.md.
//!
//!   cargo test --release -p tool_core --test run_probe -- --ignored --nocapture

use tool_core::content::Content;
use tool_core::meta::Save;
use tool_core::world::{Input, Phase, World};

fn content() -> Content {
    Content::parse(include_str!("../../../data/content.json")).unwrap()
}

/// A bot that kites the way a new player does: away from nearby enemies,
/// toward nearby shards, back toward the middle near the edge, and takes
/// the first card.
fn bot(w: &World, seat: usize, step: u32) -> Input {
    use tool_core::geom::{px, V};
    let p = &w.players[seat];
    // the world waits while a card is open, so alternate on the call count
    let card = if p.offer.is_some() && step.is_multiple_of(2) {
        pick(w, seat)
    } else {
        0
    };
    let mut push = V::ZERO;
    // a close-range character keeps less distance, or it never lands a hit
    let flee = if p.abilities[0].id == "arc_sweep" || p.abilities[0].id == "shockwave_maul" {
        110
    } else {
        260
    };
    for f in &w.foes {
        let d = p.pos.sub(f.pos);
        let l = d.len();
        if l < px(flee) as i64 && l > 0 {
            // closer enemies push harder
            push = push.add(d.with_len(px(flee) - l as i32));
        }
    }
    for b in &w.bolts {
        let d = p.pos.sub(b.pos);
        let l = d.len();
        if l < px(120) as i64 && l > 0 {
            push = push.add(d.with_len((px(120) - l as i32) * 2));
        }
    }
    if let Some(s) = w
        .pickups
        .iter()
        .filter(|s| s.pos.sub(p.pos).len() < px(320) as i64)
        .min_by_key(|s| s.pos.sub(p.pos).len())
    {
        push = push.add(s.pos.sub(p.pos).with_len(px(90)));
    }
    if let Some(r) = &w.rift {
        push = push.add(r.pos.sub(p.pos).with_len(px(120)));
    }
    // drift around the middle so the bot does not pin itself on the edge
    let home = V::ZERO.sub(p.pos);
    if home.len() > px(900) as i64 {
        push = push.add(home.with_len(px(200)));
    }
    if push.is_zero() {
        let a = tool_core::fx::TURN * (w.tick as i32 % 600) / 600;
        push = tool_core::geom::from_angle(a, 127);
    }
    let v = push.with_len(127);
    Input {
        mx: v.x as i8,
        my: v.y as i8,
        card,
        ..Input::default()
    }
}

/// Which card to take: with FOCUS set, the starter's level or its
/// evolution partner when offered; otherwise the first card.
static mut FOCUS: bool = false;

fn pick(w: &World, seat: usize) -> u8 {
    // SAFETY: the probe runs on one thread
    let focus = unsafe { FOCUS };
    let p = &w.players[seat];
    let Some(offer) = &p.offer else { return 0 };
    if !focus {
        return 1;
    }
    let c = content();
    let starter = &p.abilities[0].id;
    let partner = c
        .ability(starter)
        .map(|a| a.evolves_with.clone())
        .unwrap_or_default();
    for (k, card) in offer.cards.iter().enumerate() {
        if &card.id == starter || card.id == partner || card.kind == tool_core::world::CardKind::Evolve {
            return k as u8 + 1;
        }
    }
    // a planning player keeps passive slots free for the partner
    use tool_core::world::CardKind;
    for (k, card) in offer.cards.iter().enumerate() {
        if matches!(
            card.kind,
            CardKind::NewAbility | CardKind::AbilityUp | CardKind::PassiveUp
        ) {
            return k as u8 + 1;
        }
    }
    1
}

/// A save with every outfitter rank bought and rank 3 gear in every slot:
/// the far end of what permanent progress can give.
fn veteran(c: &Content) -> Save {
    let mut s = Save::new(c, 1);
    s.claimed = c.achievements.iter().map(|a| a.id.clone()).collect();
    for item in &c.shop {
        s.ranks.insert(item.id.clone(), item.ranks);
    }
    let picks = ["kinetic_core", "plating_mod", "magnet_mod", "spare_reroll"];
    for ch in ["pathfinder", "breaker", "vector"] {
        let mut slots = Vec::new();
        for id in picks {
            s.modules.push(tool_core::meta::Piece {
                uid: s.next_uid,
                id: id.into(),
                rank: 3,
            });
            slots.push(s.next_uid);
            s.next_uid += 1;
        }
        s.equipped.insert(ch.into(), slots);
    }
    s
}

static mut VETERAN: bool = false;

fn play(c: &Content, character: &str, seed: u64, coop: bool) -> World {
    // SAFETY: the probe runs on one thread
    let save = if unsafe { VETERAN } {
        veteran(c)
    } else {
        Save::new(c, 1)
    };
    let mut loads = vec![save.loadout(c, character)];
    if coop {
        loads.push(save.loadout(c, character));
    }
    let mut w = World::new(c, seed, &loads);
    let mut last = 0;
    let mut step = 0u32;
    while w.phase == Phase::Playing && w.tick < 60 * 60 * 20 && step < 60 * 60 * 40 {
        step += 1;
        let inputs: Vec<Input> = (0..loads.len()).map(|s| bot(&w, s, step)).collect();
        w.step(c, &inputs);
        if w.seconds() >= last + 60 {
            last = w.seconds();
            let p = &w.players[0];
            println!(
                "  {:>4}s hp {:>4} lvl {:>2} kills {:>5} foes {:>3} abilities {:?}",
                w.seconds(),
                p.hp,
                w.level,
                p.record.kills,
                w.foes.len(),
                p.abilities
                    .iter()
                    .map(|s| format!("{}:{}", s.id, s.level))
                    .collect::<Vec<_>>()
            );
        }
    }
    w
}

#[test]
#[ignore]
fn probe_whole_runs() {
    let c = content();
    for ch in ["pathfinder", "breaker", "vector"] {
        for seed in [1u64, 2] {
            println!("{ch} seed {seed}");
            let w = play(&c, ch, seed, false);
            let p = &w.players[0];
            println!(
                "  end {:?} at {}s, level {}, kills {}, evolved {:?}, bosses {:?}, rifts {}",
                w.phase,
                w.seconds(),
                w.level,
                p.record.kills,
                p.record.evolved,
                w.bosses_killed,
                w.rifts_cleared
            );
        }
    }
    // SAFETY: the probe runs on one thread
    unsafe { FOCUS = true };
    for ch in ["pathfinder", "breaker", "vector"] {
        for seed in [1u64, 2] {
            let save = Save::new(&c, 1);
            let mut w = World::new(&c, seed, &[save.loadout(&c, ch)]);
            let mut step = 0;
            while w.phase == Phase::Playing && w.players[0].record.evolved.is_empty() && step < 60 * 60 * 15 {
                step += 1;
                let input = bot(&w, 0, step);
                w.step(&c, &[input]);
            }
            println!(
                "focused {ch} seed {seed}: evolved {:?} at {}s, level {}, {:?}, held {:?}",
                w.players[0].record.evolved,
                w.seconds(),
                w.level,
                w.phase,
                w.players[0]
                    .abilities
                    .iter()
                    .chain(w.players[0].passives.iter())
                    .map(|s| format!("{}:{}", s.id, s.level))
                    .collect::<Vec<_>>()
            );
        }
    }
    unsafe { FOCUS = false };
    unsafe { VETERAN = true };
    for ch in ["pathfinder", "breaker", "vector"] {
        for seed in [1u64, 2] {
            let w = play(&c, ch, seed, false);
            let p = &w.players[0];
            println!(
                "veteran {ch} seed {seed}: end {:?} at {}s, level {}, evolved {:?}",
                w.phase,
                w.seconds(),
                w.level,
                p.record.evolved
            );
        }
    }
    unsafe { VETERAN = false };
    println!("co-op pathfinder");
    let w = play(&c, "pathfinder", 3, true);
    println!("  end {:?} at {}s, level {}", w.phase, w.seconds(), w.level);
}
