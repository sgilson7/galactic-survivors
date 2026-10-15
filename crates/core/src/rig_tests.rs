//! Tests of rigs, parts, triggers, strides and runes in a real world
//! (T14, T15). Expected values are worked out from data/content.json by
//! hand.

use crate::content::{shipped, Content};
use crate::field::FieldKind;
use crate::geom::{px, ticks, V};
use crate::meta::Save;
use crate::rig::{pack_move, Spot};
use crate::rune::Shape;
use crate::world::{EffectKind, Input, Slot, World};

fn quiet() -> Content {
    let mut c = shipped();
    c.waves.clear();
    c.events.minibosses.clear();
    c.events.rifts_s.clear();
    c.events.raid.from_s = 100_000;
    c.events.raid.to_s = 100_000;
    c.events.final_boss.at_s = 100_000;
    c
}

fn solo(c: &Content) -> World {
    let load = Save::new(c, 1).loadout(c, "pathfinder");
    World::new(c, 5, &[load])
}

/// Gives the player an ability without placing it, and sets rig 1.
fn rig(c: &Content, w: &mut World, sockets: &[&str]) {
    for id in sockets {
        if c.ability(id).is_some() && !w.players[0].abilities.iter().any(|a| a.id == *id) {
            w.players[0].abilities.push(Slot {
                id: id.to_string(),
                level: 1,
                cd: 0,
            });
        }
    }
    w.fit_rigs(c, 0);
    let mut s: Vec<String> = sockets.iter().map(|x| x.to_string()).collect();
    s.resize(w.players[0].rigs[0].sockets.len().max(s.len()), String::new());
    w.players[0].rigs[0].sockets = s;
    w.players[0].rigs[0].cd = 0;
}

/// A foe that does not die or move, `x` pixels right of the player.
fn post(c: &Content, w: &mut World, x: i32, y: i32) -> usize {
    let k = c.enemy_index("carapace");
    let i = w.spawn_foe(c, k, V::new(px(x), px(y)));
    w.foes[i].hp = 100_000;
    w.foes[i].max_hp = 100_000;
    i
}

fn rings(w: &World, id: &str) -> Vec<V> {
    w.effects
        .iter()
        .filter(|e| e.kind == EffectKind::Ring && e.ability == id)
        .map(|e| e.pos)
        .collect()
}

#[test]
fn a_run_starts_with_the_starter_in_the_first_rig() {
    let c = quiet();
    let w = solo(&c);
    let p = &w.players[0];
    assert_eq!(p.rigs.len(), c.run.ability_slots);
    assert_eq!(p.rigs[0].sockets.len(), c.run.rig_sockets as usize);
    assert_eq!(p.rigs[0].sockets[0], "seeker_rifle");
    assert!(p.rigs[0].sockets[1..].iter().all(|s| s.is_empty()));
}

#[test]
fn a_rig_reads_left_to_right_so_a_modifier_after_its_payload_does_nothing() {
    let c = quiet();
    let mut w = solo(&c);
    post(&c, &mut w, 200, 0);
    rig(&c, &mut w, &["ignite", "seeker_rifle"]);
    w.fire_rigs(&c);
    assert!(!w.shots.is_empty());
    assert!(w.shots.iter().all(|s| s.field == Some(FieldKind::Fire)));

    let mut w = solo(&c);
    post(&c, &mut w, 200, 0);
    rig(&c, &mut w, &["seeker_rifle", "ignite"]);
    w.fire_rigs(&c);
    assert!(!w.shots.is_empty());
    assert!(
        w.shots.iter().all(|s| s.field.is_none()),
        "ignite after the payload changes nothing"
    );
}

#[test]
fn a_ring_fires_six_copies_and_twin_fires_two() {
    let c = quiet();
    for (part, n) in [("ring", 6), ("twin", 2), ("fan", 3)] {
        let mut w = solo(&c);
        post(&c, &mut w, 200, 0);
        rig(&c, &mut w, &[part, "seeker_rifle"]);
        w.fire_rigs(&c);
        // the Seeker Rifle fires one shot at level 1
        assert_eq!(w.shots.len(), n, "{part}");
    }
}

#[test]
fn on_hit_fires_the_rest_of_the_rig_where_the_payload_hit() {
    let c = quiet();
    let mut w = solo(&c);
    // inside the Pulse Field's 85 px
    let f = post(&c, &mut w, 60, 0);
    let at = w.foes[f].pos;
    rig(&c, &mut w, &["pulse_field", "on_hit", "pulse_field"]);
    w.fire_rigs(&c);
    let r = rings(&w, "pulse_field");
    assert_eq!(r.len(), 2, "the payload and the one its hit set off");
    assert!(r.contains(&at), "the second fired from the foe it hit");
    assert_eq!(w.players[0].record.deepest_trigger, 1);
}

#[test]
fn triggers_nest_at_most_three_deep() {
    let c = quiet();
    let mut w = solo(&c);
    post(&c, &mut w, 60, 0);
    let mut s = vec!["pulse_field"];
    for _ in 0..5 {
        s.push("on_hit");
        s.push("pulse_field");
    }
    w.fit_rigs(&c, 0);
    // room for the whole chain
    for r in w.players[0].rigs.iter_mut() {
        r.sockets.resize(12, String::new());
    }
    rig(&c, &mut w, &s);
    w.fire_rigs(&c);
    // the payload at depth 0, then depths 1, 2 and 3
    assert_eq!(rings(&w, "pulse_field").len(), 4);
    assert_eq!(w.players[0].record.deepest_trigger, crate::rig::MAX_DEPTH as u32);
}

#[test]
fn a_trigger_after_nothing_and_a_payload_trigger_at_the_end_do_nothing() {
    let c = quiet();
    let mut w = solo(&c);
    post(&c, &mut w, 60, 0);
    rig(&c, &mut w, &["on_hit", "pulse_field"]);
    w.fire_rigs(&c);
    assert!(
        rings(&w, "pulse_field").is_empty(),
        "nothing before the trigger, so nothing fires"
    );
    let mut w = solo(&c);
    post(&c, &mut w, 60, 0);
    rig(&c, &mut w, &["pulse_field", "on_hit"]);
    w.fire_rigs(&c);
    assert_eq!(rings(&w, "pulse_field").len(), 1);
}

#[test]
fn every_part_adds_its_load_to_the_recharge() {
    let c = quiet();
    let mut w = solo(&c);
    rig(&c, &mut w, &["seeker_rifle"]);
    let plain = w.rig_cooldown(&c, 0, 0).unwrap();
    // Seeker Rifle level 1: 600 ms is 36 ticks
    assert_eq!(plain, 36);
    rig(&c, &mut w, &["heavy", "ignite", "seeker_rifle"]);
    // heavy 35% and ignite 15%: 36 * 150 / 100
    assert_eq!(w.rig_cooldown(&c, 0, 0), Some(54));
    rig(&c, &mut w, &["ignite"]);
    assert_eq!(
        w.rig_cooldown(&c, 0, 0),
        None,
        "a rig with no payload never fires"
    );
}

#[test]
fn a_wormhole_shot_passes_the_enemy_it_came_out_of() {
    let c = quiet();
    let mut w = solo(&c);
    let far = post(&c, &mut w, 400, 0);
    let near = post(&c, &mut w, 150, 0);
    rig(&c, &mut w, &["wormhole", "seeker_rifle"]);
    w.players[0].invulnerable = 1_000;
    for _ in 0..30 {
        w.step(&c, &[Input::default()]);
    }
    assert_eq!(
        w.foes[far].hp, w.foes[far].max_hp,
        "the shot starts inside it and passes it"
    );
    assert!(
        w.foes[near].hp < w.foes[near].max_hp,
        "and flies on toward the player"
    );
}

#[test]
fn the_first_payload_sets_the_recharge_not_the_slowest() {
    let c = quiet();
    let mut w = solo(&c);
    // Seeker Rifle level 1: 600 ms is 36 ticks; Ricochet Disc 1600 ms is 96
    rig(&c, &mut w, &["seeker_rifle", "ricochet_disc"]);
    assert_eq!(w.rig_cooldown(&c, 0, 0), Some(36));
    rig(&c, &mut w, &["ricochet_disc", "seeker_rifle"]);
    assert_eq!(w.rig_cooldown(&c, 0, 0), Some(96));
    // a modifier before the first payload still adds its load: ignite 15%
    rig(&c, &mut w, &["ignite", "seeker_rifle", "ricochet_disc"]);
    assert_eq!(w.rig_cooldown(&c, 0, 0), Some(36 * 115 / 100));
    // no payload before the first trigger: the rig never fires, so no recharge
    rig(&c, &mut w, &["ignite", "on_hit", "seeker_rifle"]);
    assert_eq!(w.rig_cooldown(&c, 0, 0), None);
}

#[test]
fn a_new_part_goes_where_it_works_and_a_trigger_to_the_spare_parts() {
    let c = quiet();
    let mut w = solo(&c);
    w.place_part(&c, 0, "ignite");
    assert_eq!(w.players[0].rigs[0].sockets[..3], ["ignite", "seeker_rifle", ""]);
    w.place_part(&c, 0, "on_hit");
    assert_eq!(w.players[0].spare, vec!["on_hit".to_string()]);
    w.place_ability(&c, 0, "pulse_field");
    assert_eq!(
        w.players[0].rigs[1].sockets[0], "pulse_field",
        "a new ability gets its own rig"
    );
}

#[test]
fn a_rig_move_swaps_two_places_and_reaches_the_world_as_an_input() {
    let c = quiet();
    let mut w = solo(&c);
    w.players[0].spare.push("ignite".into());
    let mv = pack_move(Spot::Spare(0), Spot::Socket(0, 1));
    w.step(
        &c,
        &[Input {
            rig: mv,
            ..Input::default()
        }],
    );
    assert_eq!(w.players[0].rigs[0].sockets[1], "ignite");
    assert!(w.players[0].spare.is_empty());
    // a payload moved to the first free spare place leaves the rig
    w.rig_move(&c, 0, pack_move(Spot::Socket(0, 0), Spot::Spare(0)));
    assert_eq!(w.players[0].spare, vec!["seeker_rifle".to_string()]);
    assert!(w.players[0].rigs[0].sockets[0].is_empty());
    // a move to a place that cannot exist changes nothing
    w.rig_move(&c, 0, pack_move(Spot::Socket(0, 1), Spot::Socket(3, 15)));
    assert_eq!(w.players[0].rigs[0].sockets[1], "ignite");
}

#[test]
fn each_stride_fires_the_rest_of_the_rig_by_distance_walked() {
    let c = quiet();
    let mut w = solo(&c);
    rig(&c, &mut w, &["each_stride", "pulse_field"]);
    // each_stride is 160 px
    w.players[0].walked = 100;
    w.fire_rigs(&c);
    assert!(rings(&w, "pulse_field").is_empty());
    w.players[0].walked = 100;
    w.fire_rigs(&c);
    assert_eq!(rings(&w, "pulse_field").len(), 1);
    assert_eq!(
        w.players[0].rigs[0].stride, 40,
        "the rest of the stride carries over"
    );
}

#[test]
fn the_wake_mine_drops_only_while_moving_and_behind_the_player() {
    let c = quiet();
    let mut w = solo(&c);
    rig(&c, &mut w, &["wake_mine"]);
    w.players[0].moving = false;
    w.fire_rigs(&c);
    assert!(w.effects.iter().all(|e| e.kind != EffectKind::Bomb));
    w.players[0].moving = true;
    w.players[0].facing = V::new(px(1), 0);
    w.players[0].rigs[0].cd = 0;
    w.fire_rigs(&c);
    let mine = w
        .effects
        .iter()
        .find(|e| e.kind == EffectKind::Bomb)
        .expect("a mine");
    assert!(mine.pos.x < w.players[0].pos.x, "behind a player facing right");
}

#[test]
fn a_formation_drops_no_wake_mine_while_the_player_stands_still() {
    let c = quiet();
    for form in ["twin", "fan", "ring", "behind", "mirror"] {
        let mut w = solo(&c);
        rig(&c, &mut w, &[form, "wake_mine"]);
        w.players[0].moving = false;
        w.fire_rigs(&c);
        assert!(w.effects.iter().all(|e| e.kind != EffectKind::Bomb), "{form}");
        w.players[0].moving = true;
        w.players[0].rigs[0].cd = 0;
        w.fire_rigs(&c);
        assert!(
            w.effects.iter().any(|e| e.kind == EffectKind::Bomb),
            "{form} while moving"
        );
    }
}

#[test]
fn echo_fires_its_payload_again_a_second_later() {
    let c = quiet();
    let mut w = solo(&c);
    rig(&c, &mut w, &["echo", "pulse_field"]);
    w.fire_rigs(&c);
    assert_eq!(rings(&w, "pulse_field").len(), 1);
    assert_eq!(w.pending.len(), 1);
    w.effects.clear();
    w.players[0].rigs[0].cd = 1_000;
    w.tick += ticks(1000) as u32;
    w.fire_rigs(&c);
    assert_eq!(rings(&w, "pulse_field").len(), 1, "the echo");
    assert!(w.pending.is_empty(), "an echo does not echo");
}

#[test]
fn an_evolution_keeps_its_place_in_the_rig() {
    let c = quiet();
    let mut w = solo(&c);
    w.players[0].rigs[0].sockets[1] = "seeker_rifle".into();
    w.players[0].rigs[0].sockets[0] = "ignite".into();
    w.evolve(&c, 0, 0);
    let into = c.ability("seeker_rifle").unwrap().evolution.clone();
    assert_eq!(w.players[0].rigs[0].sockets[1], into);
}

/// Walks the player along points every `step` pixels, tracing as it goes.
fn walk(c: &Content, w: &mut World, corners: &[(i32, i32)], step: i32) {
    for k in 0..corners.len() - 1 {
        let (a, b) = (corners[k], corners[k + 1]);
        let len = crate::geom::isqrt(((b.0 - a.0) as i64).pow(2) + ((b.1 - a.1) as i64).pow(2)) as i32;
        let n = (len / step).max(1);
        for s in 1..=n {
            let x = a.0 + (b.0 - a.0) * s / n;
            let y = a.1 + (b.1 - a.1) * s / n;
            w.players[0].pos = V::new(px(x), px(y));
            w.trace(c, 0);
        }
    }
}

#[test]
fn walking_a_triangle_draws_a_rune_that_strikes_what_is_inside() {
    let c = quiet();
    let mut w = solo(&c);
    let inside = post(&c, &mut w, 150, -80);
    let outside = post(&c, &mut w, 600, 0);
    w.players[0].pos = V::new(px(-40), 0);
    w.trace(&c, 0);
    // right, up to the apex, and back down across the start
    walk(&c, &mut w, &[(-40, 0), (300, 0), (150, -260), (0, 40)], 10);
    assert_eq!(w.rune_count(0, Shape::Triangle), 1);
    // rune damage 30, plus 2 a level at level 1
    assert_eq!(w.foes[inside].hp, 100_000 - 32);
    assert_eq!(w.foes[outside].hp, 100_000);
    // and the stack raises damage by triangle_damage_pct
    let base = World::new(&c, 5, &[Save::new(&c, 1).loadout(&c, "pathfinder")])
        .stat_total(&c, 0)
        .damage_pct;
    assert_eq!(w.stat_total(&c, 0).damage_pct, base + c.runes.triangle_damage_pct);
}

#[test]
fn walking_a_square_draws_a_square_and_a_wandering_line_draws_nothing() {
    let c = quiet();
    let mut w = solo(&c);
    w.players[0].pos = V::new(px(-30), 0);
    w.trace(&c, 0);
    walk(
        &c,
        &mut w,
        &[(-30, 0), (200, 0), (200, -200), (0, -200), (0, 30)],
        10,
    );
    assert_eq!(w.rune_count(0, Shape::Square), 1);
    let mut w = solo(&c);
    walk(&c, &mut w, &[(0, 0), (400, 0), (400, 400), (800, 400)], 10);
    assert!(w.players[0].rune_stacks.is_empty());
}

#[test]
fn rune_stacks_hold_at_most_the_limit_and_wear_off() {
    let c = quiet();
    let mut w = solo(&c);
    for _ in 0..5 {
        w.players[0].trail.clear();
        w.players[0].pos = V::new(px(-30), 0);
        w.trace(&c, 0);
        walk(
            &c,
            &mut w,
            &[(-30, 0), (200, 0), (200, -200), (0, -200), (0, 30)],
            10,
        );
    }
    assert_eq!(w.rune_count(0, Shape::Square), c.runes.stacks);
    for _ in 0..ticks(c.runes.ms) {
        w.wear_runes(0);
    }
    assert_eq!(w.rune_count(0, Shape::Square), 0);
}

#[test]
fn on_rune_fires_the_rest_at_the_rune_centre() {
    let c = quiet();
    let mut w = solo(&c);
    rig(&c, &mut w, &["on_rune", "pulse_field"]);
    w.players[0].rune_at = Some(V::new(px(100), px(-100)));
    w.fire_rigs(&c);
    assert_eq!(rings(&w, "pulse_field"), vec![V::new(px(100), px(-100))]);
}

#[test]
fn a_part_card_can_be_offered_and_a_rare_part_only_once_opened() {
    let c = quiet();
    let w = solo(&c);
    let pool = w.part_pool(&c, 0);
    assert!(pool.iter().any(|p| p.id == "ignite"));
    assert!(
        !pool.iter().any(|p| p.id == "overcharge"),
        "a rare part waits for its unlock"
    );
}
