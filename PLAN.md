# Plan

Stage 2, written by the agent from `BRIEF.md`. `B2 Require step-by-step justification`. The brief approves the plan in advance, so this records the decisions in build order; the person's first use of the game (stage 7) is the gate before the website listing is trusted.

The design insights behind each rule are in `notes/design-insights.html` (numbered 1–22; cited below as I1…I22).

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | One `Game` object lives in the WebAssembly module and holds the save, the screen and the run. The page calls `advance(now, input)` every display frame, then draws `draw_list()` and renders `ui()`. Everything else goes through `act(name, value)`. | A realtime game cannot pass its whole world through JSON every frame, as the template's stateless `act` does. The shim still decides nothing. | `shim_decides_nothing`; `api` tests on unknown actions |
| 2 | The run is a fixed 60-tick simulation in integers: positions in 1/256 px, distances in `i64`, angles in turns through `fx.rs`, chance through `rng.rs` from a seed the page passes in. | D2, and lockstep co-op needs the same world bit for bit in both browsers. | `the_same_seed_and_inputs_give_the_same_world`; `online` lagged-copies test |
| 3 | All content and balance in `data/content.json`; all reader-visible text in `data/strings.json`; presentation numbers in `data/display.json`. | D3, D5; the person decides names and balance (BRIEF). | `content_file_loads_and_every_reference_resolves`; `copy.rs` |
| 4 | Abilities are built from eight behaviours (shot, nova, arc, orbit, chain, bomb, beam, turret) with per-level numbers in data. | Seventeen abilities and five evolutions without seventeen code paths. | one test per behaviour on a hand-placed enemy |
| 5 | Auto-aim targets the nearest enemy unless the ability names another rule; bosses get no priority (I4). Pointer-aimed abilities use the pointer, or the nearest enemy when auto-aim is on (I3). | I3, I4 | `default_targeting_picks_the_nearest_enemy_not_the_boss`; `pointer_abilities_follow_auto_aim` |
| 6 | Contact: each enemy can hurt a player once per window, and a hit gives brief invulnerability (I6). Breaker halves contact damage and reflects some. | I6 | `a_crowd_touching_the_player_in_one_tick_deals_one_hit` |
| 7 | Level-up: three cards from abilities and passives the character may take, never a regeneration card; rerolls and banishes; each card shows its evolution partner (I8–I10, I12). | I8–I10, I12 | `no_offer_contains_regeneration`; `offers_hold_only_what_the_character_may_take`; `a_banished_card_is_never_offered_again` |
| 8 | Evolution: a maxed ability plus its partner passive evolves at the next supply cache or rift reward (I9). | I9 | `an_evolution_is_reachable_by_a_third_of_the_run` (scripted run) |
| 9 | Schedule from data: rifts, the Burrower at a third of the run, a corporate raid at a seeded time in the middle third ending with the Strider, the Matriarch at the end (I11, BRIEF). | I11 and the person's raid event | `the_raid_arrives_inside_its_window`; `rift_takes_players_to_a_solo_arena` |
| 10 | No immunity: every starting ability damages every boss (I13). | I13 | `every_starter_damages_every_boss` |
| 11 | Meta layer: gold kept on death, shop ranks and character unlocks with no +XP or +gold item, twenty visible achievements paying gold and +1% XP, some unlocking a character, a banish or an evolution (I15–I20). | I15–I20, the person's "xp and gold: not allowed" | `the_shop_sells_no_xp_or_gold_bonus`; `achievements_are_listed_before_they_are_earned`; `an_achievement_pays_once` |
| 12 | Salvage cache (gacha): modules from tickets earned in play; rarities with a rare-or-better guaranteed within ten pulls; duplicates raise a module's rank; reveal skippable. | the person's "some gacha type system"; transcript's ten-pull guarantee | `ten_pulls_hold_a_rare_or_better`; `a_duplicate_raises_rank_up_to_the_cap` |
| 13 | Save: the core writes and reads a JSON save file; the page downloads it and reads a chosen file, as in Gear Master 2D. No browser storage (D17 unchanged). | the person's "saveable json file like in gear master 2d" | `a_save_round_trips`; `a_save_from_another_game_is_refused_with_a_reason` |
| 14 | Co-op: two seats in lockstep through `kits/online/`, input delay in data; the host sends the seed and both loadouts; XP is shared; a downed player becomes a ghost that drops repair cells and is revived by the partner standing beside it. | the person's "2 player online co-op"; I21 | `online.rs` required tests; `a_downed_player_is_revived_by_standing_beside_the_ghost` |
| 15 | Canvas draw list (kits/canvas, extended with sprites, circles and a ground pattern). Enemy shots one shape with a dark ring; player effects drawn dimmer; effect cap; no screen shake (I7, BRIEF). | I7 and the person's emphasis on readability | `draw.rs` walk tests; `enemy_shots_draw_last` |
| 16 | Touch: an on-screen stick through pointer events; the core turns stick and keys into one move direction. | kits/mobile M1 | browser check on a phone profile |

## Open questions for the person

None block the build. Names, balance and the look are first passes listed in `SECOND-ORDER.md`.

## Status

- [x] Approved in advance by the brief's Autonomy paragraph (14 October 2026).
