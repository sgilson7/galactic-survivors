# Measurements

Stage 4, recorded by the agent. `B2 Require step-by-step justification`. Measured on Sam's Mac (Apple silicon), 14 October 2026.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | rustc 1.99.0, wasm-bindgen 0.2.127 | 14 Oct 2026 |
| Core tests | `./scripts/count-tests.sh` | 132 passed, 0 failed, 1 ignored (the run probe), after the playtest round (T1–T12) | 14 Oct 2026 |
| WebAssembly size | `ls -l dist/pkg/*.wasm` | 1,065,810 bytes after the playtest round (878 KB before) | 14 Oct 2026 |
| Art | `du -sh web/art` | 1.4 MB, 95 pictures after the playtest round (932 KB, 57 before) | 14 Oct 2026 |
| Breaks caught | `python3 scripts/break_tests.py` | 45 of 45 after the playtest round; two new tests were sharpened first (SECOND-ORDER R2.12) | 14 Oct 2026 |
| Browser check | `./scripts/check.sh` | passed in Chromium, Firefox and WebKit | 14 Oct 2026 |
| Co-op check | `python3 tests/browser/online.py chromium firefox webkit` | passed: two pages, pasted codes, both at tick 249 after 4 s, no desync | 14 Oct 2026 |

## The run probe

`cargo test --release -p tool_core --test run_probe -- --ignored --nocapture`. A bot with no outfitter ranks or modules kites away from enemies, collects shards and takes the first card offered. The focused bot takes the starting ability's levels and its evolution partner when offered, and stops at the first evolution.

| Run | Result |
| --- | --- |
| Pathfinder, seeds 1 and 2 | won at 12:23; won at 13:19 |
| Breaker, seeds 1 and 2 | won at 12:51; lost at 12:14 |
| Vector, seeds 1 and 2 | won at 12:23; lost at 7:08 |
| Co-op, two Pathfinders | won at 12:24 |
| Focused Pathfinder: first evolution | 4:11 and 2:51 |
| Focused Breaker: first evolution | none by 12:27 (lost); Halo Guard at 9:10 |
| Focused Vector: first evolution | 2:45 and 4:09 |

The bot knows where every enemy is and never misses a shard, so it plays better than a new player in some ways and worse in others (it never steers into a crowd for the Breaker's sweep). A run takes about three seconds of computer time in a release build.

The balance history behind these numbers is SECOND-ORDER R1.6 and R1.8.

### After the playtest round (T3, T7, T12)

Same command, on the final numbers (`xp_first` 6, `xp_step` 6, `xp_late_level` 12, `xp_late_step` 22, partner passive at level 3). The focused bot now also keeps its passive slots free for the partner, as a player planning an evolution does; without that it filled them with other passives and could never evolve. The veteran bot holds every outfitter rank and rank 3 gear in all four slots: the far end of what permanent progress gives.

| Run | Result |
| --- | --- |
| Pathfinder, seeds 1 and 2 | lost at 0:53; lost at 2:10 |
| Breaker, seeds 1 and 2 | lost at 7:28; lost at 7:36 |
| Vector, seeds 1 and 2 | lost at 6:09; won at 15:31 |
| Focused Pathfinder: first evolution | 5:22; none (lost at 10:15) |
| Focused Breaker: first evolution | 3:18 and 6:20 |
| Focused Vector: first evolution | 5:36 and 3:12 |
| Veteran, all three characters, seeds 1 and 2 | won all six, between 12:28 and 16:55 |
| Co-op, two Pathfinders | won at 12:58 |

A new character now wins one run in six against the bot's play, down from four in six, and a fully upgraded one wins all six, so the outfitter and gear now decide whether a run is won. Focused evolutions moved from 2:45–4:11 to 3:12–6:20. Two steeper curves were tried first: `xp_step` 4 let a Breaker evolve at 1:29, and `xp_step` 7 lost all six plain runs and the co-op run. The bot takes the first card and never aims, so its early losses (Pathfinder at 0:53) say more about the bot than about a person; how hard a first run feels is the person's to judge (SECOND-ORDER R2.13).

## Cost of one tick, for rollback (15 October 2026)

A throwaway probe (not kept) ran the run bot natively in release and timed `World::step`, `World::clone` and fourteen re-run ticks. Rollback keeps one copy of the world a tick and re-runs ticks when a guess about the partner's input was wrong, so these decide how far back a re-run can reach inside a 16 ms frame.

| Run | Peak enemies | Mean tick | Worst tick | Worst copy of the world | 14 ticks re-run (with the bot) |
| --- | --- | --- | --- | --- | --- |
| Solo Pathfinder, seed 3 (lost at 7:20) | 320 | 370 µs | 957 µs | 25 µs | 7.1–8.3 ms |
| Co-op Pathfinder and Breaker, seed 3, to 13:00 | 320 | 345 µs | 1,001 µs | 24 µs | up to 10.4 ms |

The world as JSON is 84–138 KB late in a run, too large to send every tick, which rules out sending the world instead of inputs. These are native numbers; the browser (WebAssembly) is not yet measured and is likely slower.

### In the browser, and after speeding up the tick (15 October 2026)

A throwaway WebAssembly harness (not kept) ran a co-op Pathfinder and Breaker with every outfitter rank, by a kiting bot, to 12:00, and timed 600 ticks from a kept copy at each two-minute mark (320 enemies from 4:00 on). Times are per tick, from 4:00 to 12:00.

| Engine | Before: mean tick | After: mean tick | After: worst single tick | Copy of the world | After: 8 ticks re-run |
| --- | --- | --- | --- | --- | --- |
| Chromium | 691–771 µs | 360–396 µs | 500 µs | ≤ 8 µs | 3.2 ms |
| Firefox | 805–892 µs | 380–418 µs | 2,000 µs (timer step) | ≤ 15 µs | 3.4 ms |
| WebKit | 737–818 µs | 385–423 µs | 1,000 µs (timer step) | ≤ 10 µs | 3.4 ms |

The system sampler put about two thirds of a late tick in `World::near` called from crowd separation: a list built and sorted for each of 320 enemies. Separation adds pushes, so the order does not matter, and it now walks the grid cells directly (`each_near`). The same bot run natively went from 645 µs to 287 µs a tick. Firefox and WebKit round `performance.now` to a millisecond, so their worst single tick is a timer step.

## Round 3 balance probe (15 October 2026)

`cargo test --release -p tool_core --test run_probe -- --ignored --nocapture`, after T14 and T15 added 7 abilities, 8 passives and 27 parts to the card pool. The bot takes the first card and never moves a part, so a trigger it takes sits in its spare parts.

| Setting | Plain bot, 6 runs | Focused bots: evolved | Veteran, 6 runs | Co-op |
| --- | --- | --- | --- | --- |
| No upgrade card (first try) | won 0 | 1 of 6, at 9:39 | won 5 | not run |
| `up_card_pct` 100, first build | won 1 | 5 of 6, 1:09 to 6:15 | won 5 | won at 14:55 |
| `up_card_pct` 50 | won 0 | 2 of 6 | won 3 | lost at 9:36 |
| `up_card_pct` 75 | won 0 | 1 of 6 | won 4 | lost at 6:43 |
| `up_card_pct` 100, shipped | won 1 | 4 of 6, 1:29 to 11:52 | won 5 | won at 12:56 |

Six runs a setting is a small sample: one seed's run changes with any change to the order the generator is drawn in. The shipped setting is the one closest to the playtest round's targets (plain 1 of 6, veteran 6 of 6, focused evolutions three to six minutes in).

## Test times and the rig space (15 October 2026, T17)

On this machine (12 cores), with the test profile (`opt-level = 2`, overflow checks on).

| What runs | Time |
| --- | --- |
| `cargo test`, nothing changed | 1.4 s |
| `cargo test` after touching a core file | about 6 s, almost all compiling |
| The core's rule tests, one at a time (`--exact`, process start taken off) | 0.21 s in all; slowest `every_screen_finds_all_its_strings` 36 ms, `the_same_seed_and_inputs_give_the_same_world` 34 ms, `a_laggy_lossy_co_op_run_stays_in_step_and_keeps_moving` 34 ms; the rig tests about 1 ms in all |
| `python3 scripts/break_tests.py` (87 breaks, each recompiles) | 8 min 24 s |
| `./scripts/check.sh` | 1 min 44 s |

The rig space (`crates/core/tests/rig_space.rs`): 29 payloads (24 abilities, 5 evolutions) and 27 parts; a rig is 1–6 tokens with at least one payload and no payload twice. Rigs: 29 of one token, 2,378 of two, 151,119 of three, 8,772,732 of four, 487,914,705 of five, 26,484,087,078 of six. With each payload at its own level (1–5), four tokens are 752,447,340.

Rate, rigs a second, each played for 240 ticks (four-token sample):

| Gap and rollback rules | 1 thread | 12 threads |
| --- | --- | --- |
| none | 1,292 | 11,090 |
| 1 rig in 4 (default) | 938 | 8,056–8,330 |
| every rig | 514 | 4,459 |

From one token to six the default rate stayed between 7,710 and 8,537 a second.

Sweeps run, every one with no rule broken:

| Sweep | Rigs | Time |
| --- | --- | --- |
| Everyday test (lengths 1–2 whole, 400 each of 3 and 4; half again at level 5) | 4,811 plays | 0.6 s |
| Length 3, level 1 | 151,119 | 18.5 s |
| Length 3, level 5 | 151,119 | 18.1 s |
| Length 4, level 1 | 8,772,732 | 18 min 34 s (7,878 a second; most 300 shots, 540 effects, 17 echoes at once) |
| Length 5, sample of 200,000 | 200,000 | 24.5 s |
| Length 6, sample of 200,000 | 200,000 | 24.9 s |

At 8,200 a second: all of length 5 takes 16.5 hours, all of length 6 37 days, and all four-token rigs at every level 1.1 days. A tweak to one part's numbers touches 6.7% of four-token rigs, one ability's 7.4%.

## The rig logbook's first run (15 October 2026, T21)

From commit 0242533, `RIG_BUDGET_S=7200`: every rig of one to four tokens at level 1, 8,926,258 in all, in 1,544 s (25.7 minutes, about 5,800 a second with writing the log). No rule broken; 164 rigs noted, all a field after Wormhole that never landed. The local log `rig-log/` is 1.5 GB; the repository's `tests/rig-log/` is 204 KB (2,446 block lines, 164 notable rigs).

## Not measured

- Frame rate on a phone or an older laptop with many enemies on screen.
- Co-op over the internet, by room code, between two homes.
- How the game feels to a person: whether a first run is now too hard (the bot wins one of six without upgrades), how many runs it takes to open and buy the outfitter, and whether the screen stays calm in the late game.
