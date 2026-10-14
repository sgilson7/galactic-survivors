# Measurements

Stage 4, recorded by the agent. `B2 Require step-by-step justification`. Measured on Sam's Mac (Apple silicon), 14 October 2026.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | rustc 1.99.0, wasm-bindgen 0.2.127 | 14 Oct 2026 |
| Core tests | `./scripts/count-tests.sh` | 115 passed, 0 failed, 1 ignored (the run probe) | 14 Oct 2026 |
| WebAssembly size | printed by `./scripts/build.sh` | 878 KB | 14 Oct 2026 |
| Art | `du -sh web/art` | 932 KB, 57 pictures | 14 Oct 2026 |
| Breaks caught | `python3 scripts/break_tests.py` | 21 of 21, after four tests were sharpened (SECOND-ORDER R1.23) | 14 Oct 2026 |
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

## Not measured

- Frame rate on a phone or an older laptop with many enemies on screen.
- Co-op over the internet, by room code, between two homes.
- How the game feels to a person: whether it is too easy for a first run (the bot wins four of six without upgrades), and whether the screen stays calm in the late game.
