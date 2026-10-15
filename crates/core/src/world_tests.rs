//! Tests of the run's rules, each tied to a design insight (I-numbers in
//! notes/design-insights.html) or a line of the brief. Expected values are
//! worked out from data/content.json by hand.

use crate::content::{shipped, Content};
use crate::geom::{px, V};
use crate::meta::Save;
use crate::world::{CardKind, Input, Loadout, OfferKind, Phase, World};

fn load(c: &Content, ch: &str) -> Loadout {
    Save::new(c, 1).loadout(c, ch)
}

fn solo(c: &Content, ch: &str) -> World {
    World::new(c, 5, &[load(c, ch)])
}

/// A world with nothing scheduled: waves, bosses, rifts and the raid are
/// pushed past the end, so a test places every enemy itself.
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

#[test]
fn inputs_pack_and_unpack() {
    let i = Input {
        mx: -127,
        my: 64,
        ax: 0,
        ay: -1,
        flags: 3,
        card: 7,
    };
    assert_eq!(Input::unpack(i.pack()), i);
}

#[test]
fn the_same_seed_and_inputs_give_the_same_world() {
    let c = shipped();
    let run = |seed| {
        let mut w = World::new(&c, seed, &[load(&c, "pathfinder")]);
        for t in 0..3000u32 {
            let card = if t % 7 == 0 { 1 } else { 0 };
            let input = Input {
                mx: ((t / 90) % 3) as i8 * 60 - 60,
                my: ((t / 130) % 3) as i8 * 60 - 60,
                card,
                ..Input::default()
            };
            w.step(&c, &[input]);
        }
        w
    };
    let a = run(9);
    let b = run(9);
    assert_eq!(a.sum(), b.sum());
    assert_eq!(a, b);
    assert_ne!(a.sum(), run(10).sum(), "a different seed gives a different run");
}

#[test]
fn default_targeting_picks_the_nearest_enemy_not_the_boss() {
    let c = quiet();
    let mut w = solo(&c, "pathfinder");
    let boss = c.enemy_index("burrower");
    let skitter = c.enemy_index("skitter");
    // the boss is closer to the player than its own width, the skitter
    // closer still
    w.spawn_foe(&c, boss, V::new(px(300), 0));
    let near = w.spawn_foe(&c, skitter, V::new(px(-120), 0));
    assert_eq!(w.nearest_foe(w.players[0].pos, 600), Some(near));
    // moved past the boss, the skitter is no longer the target
    w.foes[near].pos = V::new(px(-500), 0);
    assert_eq!(w.nearest_foe(w.players[0].pos, 600), Some(0));
}

#[test]
fn pointer_abilities_follow_the_pointer_unless_auto_aim_is_on() {
    let c = quiet();
    let skitter = c.enemy_index("skitter");
    for autoaim in [false, true] {
        let mut l = load(&c, "vector");
        l.autoaim = autoaim;
        let mut w = World::new(&c, 5, &[l]);
        // an enemy straight below; the pointer straight above
        w.spawn_foe(&c, skitter, V::new(px(-40), px(200)));
        let up = Input {
            ax: 0,
            ay: -127,
            flags: crate::world::AIM,
            ..Input::default()
        };
        for _ in 0..60 {
            w.step(&c, &[up]);
            if !w.shots.is_empty() {
                break;
            }
        }
        let shot = w.shots.first().expect("the rail bolt fired");
        if autoaim {
            assert!(shot.vel.y > 0, "auto-aim fires at the enemy below");
        } else {
            assert!(shot.vel.y < 0, "the bolt follows the pointer up");
        }
    }
}

#[test]
fn a_crowd_touching_the_player_in_one_tick_deals_one_hit() {
    let c = quiet();
    let mut w = solo(&c, "pathfinder");
    let skitter = c.enemy_index("skitter");
    for k in 0..20 {
        w.spawn_foe(&c, skitter, V::new(px(k % 5 - 2), px(k / 5 - 2)));
    }
    let before = w.players[0].hp;
    w.step(&c, &[Input::default()]);
    // a skitter deals 8 at minute 0, and Pathfinder has no armor
    assert_eq!(
        before - w.players[0].hp,
        8,
        "twenty touching enemies landed one hit"
    );
}

#[test]
fn breaker_takes_half_contact_damage_and_hurts_what_touches_it() {
    let c = quiet();
    let mut w = solo(&c, "breaker");
    let carapace = c.enemy_index("carapace");
    let i = w.spawn_foe(&c, carapace, V::new(px(5), 0));
    let foe_hp = w.foes[i].hp;
    let before = w.players[0].hp;
    w.players[0].abilities.clear();
    w.step(&c, &[Input::default()]);
    // carapace 16, halved to 8, less Breaker's 2 armor: 6
    assert_eq!(before - w.players[0].hp, 6);
    // Breaker reflects 12
    assert_eq!(foe_hp - w.foes[0].hp, 12);
}

#[test]
fn offers_hold_only_what_the_character_may_take_and_never_regeneration() {
    let c = shipped();
    let mut w = solo(&c, "pathfinder");
    let breaker_only = [
        "arc_sweep",
        "ground_slam",
        "gravity_hook",
        "rail_bolt",
        "mine_layer",
        "cutting_beam",
    ];
    for _ in 0..200 {
        let offer = w.level_offer(&c, 0);
        assert_eq!(offer.cards.len(), c.run.card_count);
        for card in &offer.cards {
            assert!(
                !breaker_only.contains(&card.id.as_str()),
                "{} offered to Pathfinder",
                card.id
            );
            let stat = c.passive(&card.id).map(|p| p.stat.clone()).unwrap_or_default();
            assert!(!stat.contains("regen"), "a regeneration card was offered");
        }
    }
    // the content has no regeneration stat at all
    assert!(!crate::content::STATS.iter().any(|s| s.contains("regen")));
}

#[test]
fn a_banished_card_is_never_offered_again() {
    let c = shipped();
    let mut w = solo(&c, "pathfinder");
    w.players[0].banishes = 1;
    // banish a card for an ability the player does not have yet
    let (offer, k) = loop {
        let offer = w.level_offer(&c, 0);
        if let Some(k) = offer
            .cards
            .iter()
            .position(|card| card.kind == CardKind::NewAbility)
        {
            break (offer, k);
        }
    };
    let gone = offer.cards[k].id.clone();
    w.players[0].offer = Some(offer);
    w.card_action(&c, 0, 4 + k as u8);
    assert_eq!(w.players[0].banishes, 0);
    for _ in 0..300 {
        let offer = w.level_offer(&c, 0);
        assert!(offer.cards.iter().all(|card| card.id != gone));
    }
}

#[test]
fn a_reroll_draws_a_new_hand_and_costs_one() {
    let c = shipped();
    let mut w = solo(&c, "pathfinder");
    w.players[0].offer = Some(w.level_offer(&c, 0));
    let rerolls = w.players[0].rerolls;
    w.card_action(&c, 0, crate::world::CARD_REROLL);
    assert_eq!(w.players[0].rerolls, rerolls - 1);
    assert!(w.players[0].offer.is_some());
}

#[test]
fn an_evolution_is_offered_once_possible_and_needs_its_partner() {
    let c = shipped();
    let mut w = solo(&c, "pathfinder");
    w.players[0].abilities[0].level = c.run.max_level;
    let offer = w.level_offer(&c, 0);
    assert!(
        offer.cards.iter().all(|card| card.kind != CardKind::Evolve),
        "no partner yet"
    );
    w.players[0].passives.push(crate::world::Slot {
        id: "split_feed".into(),
        level: 1,
        cd: 0,
    });
    let offer = w.level_offer(&c, 0);
    assert!(
        offer.cards.iter().all(|card| card.kind != CardKind::Evolve),
        "the partner is below its level (split_feed tops out at 2, under the rule's 3)"
    );
    w.players[0].passives[0].level = 2;
    let offer = w.level_offer(&c, 0);
    assert_eq!(offer.cards[0].kind, CardKind::Evolve);
    assert_eq!(offer.cards[0].id, "swarm_battery");
    w.players[0].offer = Some(offer);
    w.card_action(&c, 0, 1);
    assert_eq!(w.players[0].abilities[0].id, "swarm_battery");
}

#[test]
fn a_locked_evolution_is_not_offered() {
    let c = shipped();
    let mut l = load(&c, "pathfinder");
    assert!(l.locked.contains(&"storm_lattice".to_string()));
    let mut w = World::new(&c, 1, std::slice::from_ref(&l));
    w.players[0].abilities.push(crate::world::Slot {
        id: "chain_arc".into(),
        level: c.run.max_level,
        cd: 0,
    });
    w.players[0].passives.push(crate::world::Slot {
        id: "power_cell".into(),
        level: 3,
        cd: 0,
    });
    assert_eq!(w.evolvable(&c, 0), None);
    l.locked.clear();
    w.players[0].load = l;
    assert_eq!(w.evolvable(&c, 0), Some(1));
}

#[test]
fn the_raid_arrives_inside_its_window() {
    let c = shipped();
    let raid = &c.events.raid;
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..200 {
        let w = World::new(&c, seed, &[load(&c, "pathfinder")]);
        let s = w.raid_at / crate::geom::TPS;
        assert!(s >= raid.from_s && s <= raid.to_s, "seed {seed}: {s}");
        seen.insert(s);
    }
    assert!(seen.len() > 20, "the raid's time varies by seed");
}

#[test]
fn a_rift_takes_the_players_to_a_solo_arena_and_pays_on_the_way_out() {
    let mut c = quiet();
    c.events.rifts_s = vec![1];
    let mut w = solo(&c, "pathfinder");
    let skitter = c.enemy_index("skitter");
    w.spawn_foe(&c, skitter, V::new(px(600), px(600)));
    for _ in 0..70 {
        w.step(&c, &[Input::default()]);
    }
    let rift = w.rift.clone().expect("the rift opened at one second");
    w.players[0].pos = rift.pos;
    w.step(&c, &[Input::default()]);
    assert!(w.in_rift);
    assert_eq!(w.foes.len(), 1, "only the elite is in the arena");
    assert_eq!(c.enemies[w.foes[0].kind].id, "stalker");
    // defeat it
    let hp = w.foes[0].hp;
    w.hurt_foe(&c, 0, hp, 0, "seeker_rifle", V::ZERO);
    w.step(&c, &[Input::default()]);
    assert!(!w.in_rift);
    assert_eq!(w.rifts_cleared, 1);
    assert_eq!(w.foes.len(), 1, "the field's skitter is back");
    let offer = w.players[0].offer.as_ref().expect("a reward");
    assert_eq!(offer.kind, OfferKind::Reward);
    assert_eq!(w.players[0].record.tickets, c.rift.tickets);
}

#[test]
fn every_starter_damages_every_boss() {
    let c = quiet();
    for ch in &c.characters {
        for boss in c.enemies.iter().filter(|e| e.boss || e.elite) {
            let mut l = load(&c, &ch.id);
            l.autoaim = true;
            let mut w = World::new(&c, 3, &[l]);
            let kind = c.enemy_index(&boss.id);
            // close enough for the sweep, inside every starter's range
            let i = w.spawn_foe(&c, kind, V::new(px(boss.radius + 40), 0));
            let start = w.foes[i].hp;
            w.players[0].invulnerable = 10_000;
            for _ in 0..180 {
                w.foes[0].pos = V::new(px(boss.radius + 40), 0);
                w.step(&c, &[Input::default()]);
            }
            assert!(
                w.foes[0].hp < start,
                "{}'s {} did nothing to {}",
                ch.id,
                ch.starter,
                boss.id
            );
        }
    }
}

#[test]
fn a_downed_player_drops_cells_and_is_revived_by_standing_beside_the_ghost() {
    let c = quiet();
    let mut w = World::new(&c, 2, &[load(&c, "pathfinder"), load(&c, "breaker")]);
    w.players[1].hp = 1;
    w.hurt_player(&c, 1, 50);
    assert!(!w.players[1].alive);
    assert_eq!(w.phase, Phase::Playing, "one player still stands");
    // the ghost drops a repair cell after its interval
    w.players[0].pos = V::new(px(1500), px(1500));
    for _ in 0..crate::geom::ticks(c.run.ghost_drop_ms) + 1 {
        w.step(&c, &[Input::default(), Input::default()]);
    }
    assert!(w
        .pickups
        .iter()
        .any(|p| p.kind == crate::world::PickupKind::Health));
    // the partner stands beside the ghost for the revive time
    w.players[0].pos = w.players[1].pos.add(V::new(px(20), 0));
    for _ in 0..crate::geom::ticks(c.run.revive_ms) + 1 {
        w.step(&c, &[Input::default(), Input::default()]);
    }
    assert!(w.players[1].alive);
    let max = w.max_hp(&c, 1);
    assert_eq!(w.players[1].hp, max * c.run.revive_hp_pct / 100);
}

#[test]
fn the_run_is_lost_when_every_player_is_down_and_won_when_the_final_boss_falls() {
    let c = quiet();
    let mut w = solo(&c, "pathfinder");
    w.hurt_player(&c, 0, 10_000);
    w.step(&c, &[Input::default()]);
    assert_eq!(w.phase, Phase::Lost);

    let mut w = solo(&c, "pathfinder");
    let i = w.spawn_foe(&c, c.enemy_index("matriarch"), V::new(px(900), 0));
    let hp = w.foes[i].hp;
    w.hurt_foe(&c, i, hp, 0, "", V::ZERO);
    assert_eq!(w.phase, Phase::Won);
    assert_eq!(w.bosses_killed, vec!["matriarch".to_string()]);
}

#[test]
fn experience_is_shared_and_both_players_level_together() {
    let c = quiet();
    let mut w = World::new(&c, 2, &[load(&c, "pathfinder"), load(&c, "vector")]);
    // level 1 needs xp_first (6 in content.json)
    w.gain_xp(&c, 5);
    assert_eq!(w.level, 1);
    w.gain_xp(&c, 1);
    assert_eq!(w.level, 2);
    assert_eq!(w.players[0].pending, 1);
    assert_eq!(w.players[1].pending, 1);
    w.step(&c, &[Input::default(), Input::default()]);
    assert!(w.players[0].offer.is_some() && w.players[1].offer.is_some());
    assert!(w.waiting(), "the field waits while a card is open");
}

#[test]
fn a_locked_ability_is_never_offered_until_unlocked() {
    let c = shipped();
    let mut w = solo(&c, "pathfinder");
    let pool: Vec<String> = w.card_pool(&c, 0).into_iter().map(|k| k.id).collect();
    for id in ["mortar", "missiles", "cryo_burst", "sentry"] {
        assert!(!pool.contains(&id.to_string()), "{id} is locked");
    }
    assert!(pool.contains(&"chain_arc".to_string()));
    w.players[0].load.locked.retain(|x| x != "mortar");
    let pool: Vec<String> = w.card_pool(&c, 0).into_iter().map(|k| k.id).collect();
    assert!(pool.contains(&"mortar".to_string()));
}

#[test]
fn a_run_records_kills_by_enemy_and_the_first_hit() {
    let c = shipped();
    let mut w = solo(&c, "pathfinder");
    let kind = c.enemy_index("skitter");
    let i = w.spawn_foe(&c, kind, V::new(px(300), 0));
    w.hurt_foe(&c, i, 10_000, 0, "seeker_rifle", V::ZERO);
    assert_eq!(w.players[0].record.kills_of.get("skitter"), Some(&1));
    assert_eq!(w.players[0].record.first_hit, 0);
    for _ in 0..30 {
        w.step(&c, &[Input::default()]);
    }
    w.hurt_player(&c, 0, 5);
    let at = w.players[0].record.first_hit;
    assert!(at > 0);
    for _ in 0..30 {
        w.step(&c, &[Input::default()]);
    }
    w.players[0].invulnerable = 0;
    w.hurt_player(&c, 0, 5);
    assert!(w.tick as i32 > at);
    assert_eq!(w.players[0].record.first_hit, at, "only the first hit counts");
}

#[test]
fn a_dash_is_counted() {
    let c = shipped();
    let mut w = solo(&c, "vector");
    let dash = Input {
        mx: 127,
        flags: crate::world::DASH,
        ..Input::default()
    };
    w.step(&c, &[dash]);
    assert_eq!(w.players[0].record.dashes, 1);
    // still recharging: no second dash
    w.step(&c, &[dash]);
    assert_eq!(w.players[0].record.dashes, 1);
}

#[test]
fn an_evolution_waits_for_its_partner_to_reach_level_three() {
    let c = shipped();
    let mut w = solo(&c, "pathfinder");
    w.players[0].load.locked.clear();
    w.players[0].abilities.push(crate::world::Slot {
        id: "chain_arc".into(),
        level: c.run.max_level,
        cd: 0,
    });
    // power_cell has five levels, so the rule's level 3 applies
    w.players[0].passives.push(crate::world::Slot {
        id: "power_cell".into(),
        level: 2,
        cd: 0,
    });
    assert_eq!(w.evolvable(&c, 0), None);
    w.players[0].passives[0].level = 3;
    assert_eq!(w.evolvable(&c, 0), Some(1));
}
