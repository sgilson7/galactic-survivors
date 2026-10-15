# Handoff

Stage 9. `C5 Consolidate in your own words`. Last updated after round 3, 15 October 2026. A fresh session starts here.

## Where things are

| What | Where |
| --- | --- |
| The game, live | https://sgilson7.github.io/galactic-survivors/ |
| Repository | https://github.com/sgilson7/galactic-survivors |
| Website listing | https://sgilson7.github.io/projects/galactic-survivors/ and a tile under Games on /projects/ |
| Design insights from the transcript | `notes/design-insights.html` (local only, not in git) and a private artifact |
| Art gallery | a private artifact built from `web/art/` |

## What was built (milestone 1, 14 October 2026)

The MVP the brief describes: one twelve-minute run on one bounded map, three characters (Pathfinder, Breaker, Vector), seventeen abilities, eight passives and five evolutions, four rifts a run, the Burrower, a corporate raid at a seeded time ending with the Strider, and the Matriarch at the end. Between runs: gold kept on death, an outfitter with eight permanent ranks and no XP or gold upgrade, twenty visible achievements, the salvage cache (the gacha) with twelve modules and a ten-pull guarantee, and a JSON save file. Two-player co-op by room code or pasted invitation, with shared levels and a ghost that drops repair cells and is revived by the partner. Art: 57 TikZ pictures in the project style pack.

## What changed in the playtest round (14 October 2026, TRIAGE T1–T12)

The person's friends played the first build and sent twelve notes. All twelve are answered:
- **The build.** A paused solo run shows the build: what each ability does now and what it still needs to evolve. In co-op, I shows it without pausing.
- **The codex.** It lists every ability, passive and evolution, with numbers and recipes.
- **Dash reminder.** Vector sees a dash meter and a prompt.
- **Walk.** All three characters walk.
- **Achievements.** The 20 are now 56 on a Halls of Torment grid with hidden tiles.
- **Outfitter.** Ranks are opened by achievements.
- **Gear window.** Four typed slots and a stash, with merging and scrapping.
- **Harder runs.** Enemies are tankier, levels come slower, and evolutions need the partner passive at level 3.

The save format is now version 2. A version 1 file still loads: its modules keep their ranks, and its outfitter ranks are refunded in gold, so buy them again under the caps.

## What changed in round 3 (15 October 2026, TRIAGE T13–T15)

- **Co-op is rollback.** Your own moves land after 2 ticks instead of 6. The partner is guessed and corrected, a browser runs at most 8 ticks ahead, and inputs travel on a channel that never waits for a lost packet. A level-up in co-op no longer stops both players: the chooser is shielded until they move. A ping, the number of corrections and the time spent waiting show at the top right.
- **Rigs.** Each ability slot is a rig of sockets read left to right. Parts (modifiers, formations, triggers) come from level-up cards. E opens the rig window, which never pauses. Six rare parts are opened by eight new achievements or found in the salvage cache.
- **New content.** 7 abilities, 8 passives, 4 ground fields that react (fire, frost, charge, slick, and steam from fire meeting frost), and 3 runes drawn by walking a loop.
- **A faster tick.** Crowd separation no longer sorts a list for every enemy; a late tick is about half the time it was.

## What changed in round 4 (15 October 2026, TRIAGE T16–T18)

- **The first ability sets a rig's recharge** (T16, R4.1), not the slowest. A rig with no ability before its first trigger is idle.
- **A rig space test** (T17, `crates/core/tests/rig_space.rs`) plays any rig for four seconds in one fixed scene against ten rules. The everyday test plays 4,811 rigs in 0.6 s. The ignored `sweep` plays a whole length or a sample: `RIG_LEN=4 cargo test -p tool_core --test rig_space -- --ignored --nocapture`. Rates and coverage are in MEASUREMENTS and the journal's sixth tab.
- **Three bugs it found, fixed:** a Wormhole shot spent itself on the enemy it came out of (R4.2); a formation dropped Wake Mines while the player stood still (R4.3). Two design questions went to the person (R4.4, R4.5).
- **The logbook** (T18) is designed in the journal's sixth tab, not built (R4.8).

## What changed after the person's answers (15 October 2026, TRIAGE T19–T21)

- **Each copy of a formation on Tether keeps its own line** (R4.4): the partner first, then what the player placed, farthest first; a copy with nothing to join reaches out its own way.
- **Orbits and tethers leave ground fields** (R4.5): an orbiting body where it hits, then not again for one full turn (`fields.orbit_turns`); a tether where it cuts, at most every two seconds (`fields.tether_ms`).
- **The rig logbook** (R4.8–R4.10): `cargo test -p tool_core --test rig_space -- --ignored --nocapture logbook` plays only rigs whose line is missing or stale. The full log is `rig-log/` (local, 1.5 GB); `tests/rig-log/` is the summary in git. Its first run found a panic (R4.10), now fixed and replayed every day from `FOUND`.

## What changed in the co-op lobby (15 October 2026, TRIAGE T23–T24)

- **A gear bug, fixed first** (T23, R4.12): a partner's gear or Outfitter change made after connecting never reached the run. Each player now sends their seat whenever it changes.
- **The new lobby** (T24, R4.13), after Ape Race and PokéSurvivor (journal entry 8). The room is on the left: its code with Copy code and Copy link, both players with name, character and Ready, where the partner is, Start or Ready, Try again after a failed connection, and Leave. Your Character, Gear and Outfitter are tabs on the right. The partner's own loadout change takes back Ready; the host's does not. Players type a name, kept in the save. A room link (`#room=CODE`) joins on opening. The invitation buttons sit behind "Can't connect with a code?". Solo play waits until you leave the room, and the title's co-op button reads "Back to your co-op room".
- **Two older bugs found on the way:** Leave never closed the room (R4.14), and after a co-op run the session stayed "playing", so changes after Results went unsent (R4.15).
- `PROTOCOL` is 3: a tab opened before this change is told to reload.

## What passed

| Check | Result |
| --- | --- |
| `./scripts/check.sh`, after the co-op lobby | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 15 Oct 2026 |
| `./scripts/count-tests.sh`, after the co-op lobby | 194 passed, 0 failed, 3 ignored (the probe, the rig sweep and the logbook) |
| `python3 tests/browser/online.py chromium firefox webkit`, after the co-op lobby | ONLINE CHECK PASSED (now presses Ready) |
| `tests/browser/lobby.py` (new), Chromium | 12 checks passed: Ready gates Start, the partner's row, names, the phone layout, Copy code and Copy link, a room link |
| `python3 scripts/break_tests.py T23 T24 "tethers replace"`, after the co-op lobby | 15 of 15 caught; two older rows had rotted (R4.17) and were re-aimed |
| `./scripts/check.sh`, after round 4 | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 15 Oct 2026, in 1 min 44 s |
| `./scripts/count-tests.sh`, after round 4 | 178 passed, 0 failed, 2 ignored (the probe and the rig sweep) |
| `python3 scripts/break_tests.py`, after round 4 | 87 of 87 planted defects caught |
| Live check after publishing 8a320ea (`BUILDER_ORIGIN=https://sgilson7.github.io/galactic-survivors tests/browser/check.py`) | PASSED in all three engines |
| `./scripts/check.sh` on a clean copy of 0242533, then `scripts/publish.sh`; live check after publishing | BUILDER CHECK PASSED; live PASSED in all three engines, 15 Oct 2026 |
| Rig logbook, first full run from 0242533 | all 8,926,258 rigs of 1–4 tokens at level 1 in 25.7 min; no rule broken; 164 noted (a field after Wormhole that never landed) |
| Rig sweeps, after round 4 (`rig_space` `sweep`) | every rig of 3 tokens at levels 1 and 5, and of 4 tokens at level 1 (8,772,732), and 200,000 each of 5 and 6: no rule broken |
| `./scripts/check.sh`, after round 3 | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 15 Oct 2026 |
| `python3 tests/browser/online.py chromium firefox webkit`, after round 3 (rollback and the fast channel) | ONLINE CHECK PASSED |
| `./scripts/count-tests.sh`, after round 3 | 174 passed, 0 failed, 1 ignored (the probe) |
| `python3 scripts/break_tests.py`, after round 3 | 79 of 79 planted defects caught |
| `python3 tools/art/art.py --check-only`, after round 3 | 144 pictures pass |
| `./scripts/check.sh` | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 14 Oct 2026 |
| `./scripts/check.sh`, after the playtest round | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 14 Oct 2026 |
| `python3 tests/browser/online.py chromium firefox webkit`, after the playtest round | ONLINE CHECK PASSED |
| Live check after publishing be0f02d (`BUILDER_ORIGIN=https://sgilson7.github.io/galactic-survivors tests/browser/check.py`) | PASSED in all three engines |
| `BUILDER_ORIGIN=https://sgilson7.github.io/galactic-survivors tests/browser/check.py` | PASSED in all three engines against the live site |
| Website: `make check`, `make packet`, `make gate`, then `make gate-live` after publishing | all passed; the listing is at /projects/galactic-survivors/ |
| Website listing after round 3 (website commit f15315c, built from a clean worktree): `make check`, `make packet`, the gate, publish, then the gate against the live site | all passed |
| Live check after publishing 5074143 (`BUILDER_ORIGIN=https://sgilson7.github.io/galactic-survivors tests/browser/check.py`) | PASSED in all three engines |
| `./scripts/count-tests.sh` | 132 passed, 0 failed, 1 ignored (the probe), after the playtest round |
| `python3 scripts/break_tests.py` | 45 of 45 planted defects caught, after the playtest round |
| `python3 tests/browser/online.py chromium firefox webkit` | passed: two pages meet by pasted codes and play in step |
| `python3 tools/art/art.py --check-only` | 95 pictures pass, after the playtest round |

## Try these by hand first

After the co-op lobby:
1. Host a room on one computer and send the link to your partner on another. Their page should open in your room. Then try typing the code instead (R4.18).
2. As the partner, press Ready, then equip a module: Ready should go back off, and the host's Start should wait.
3. Both type a name. Play a run, go back to the lobby from Results, change gear, and play again: the change should be in the second run.

After round 3:
1. Play co-op between two homes. Watch the numbers at the top right (ping, corrections, waited) and send them (R3.9).
2. Press E in a run and move a part: pick it, then click a socket. Try Ring, Ignite and the Orbital Lance in one rig.
3. Walk a triangle around a crowd and check the rune strikes the enemies inside.
4. Read the new names in `data/strings.json` (R3.18) and look over the new icons (R3.20).

After the playtest round:
1. Load the save your friends already have (version 1). Their modules should come through, and their outfitter ranks should come back as gold.
2. Play a first run on a fresh save. Is it now too hard (SECOND-ORDER R2.13)?
3. Pause a run, open the codex, and check that the recipe text matches what happens.
4. Open the gear window. Merge two copies, equip one, and scrap one.
5. Look over the achievement grid and what each tile opens (R2.14).

From the first build:

1. Play a solo run as Pathfinder to the end. Is the first run too easy? The probe bot wins four of six without upgrades (SECOND-ORDER R1.25, R1.17).
2. After the run, download the save, reload the page, and load the save. Gold and achievements should come back.
3. With your co-op partner, host a room from one home and join from the other (SECOND-ORDER R1.18). If the room code does not connect, try Host by invitation. Write what happened and the date here.
4. Read `data/strings.json` top to bottom and rename anything (SECOND-ORDER R1.16). Names live only there.

## Rows for the person

- R4.18: the lobby between two computers, by room code and by link.
- R3.9: co-op between two homes, with the new readout.
- R3.17: the numbers for the new content were tuned against a bot that never builds a rig.
- R3.18: the names and texts of the new content were written by the agent.
- R3.20: the weakest new icons.
- R1.16: every name and string in `data/strings.json` was written by the agent.
- R1.17: the balance in `data/content.json` is a first pass against a bot.
- R1.18: co-op over room codes and real networks is untested.
- R1.25: decide whether a first run should be harder so the outfitter matters sooner. (Answered by the friends' T7; see R2.13.)
- R2.13: the playtest round's balance may make a first run too hard.
- R2.14: the achievement grid's layout, goals, rewards and unlocks, and 36 new names, were all chosen by the agent.

## Unfinished

- Glossary hover hints (D15) are not drawn in the menus (R1.14). The codex now covers abilities, passives and evolutions.
- The weakest new pictures are listed in R2.15.
- Damage numbers are not drawn; the setting is hidden (R1.22).
- The relays in `web/net/config.js` were last confirmed in October 2026 by another project (R1.19).
- No sound.

## What the next session must not assume

- That `rig-log/` exists on another computer. It is local; a fresh clone rebuilds it with the `logbook` test (about 20 minutes for every rig of four tokens or fewer).

- That co-op works between two homes. It has been seen working only between two tabs on one machine.
- That the balance is right. The playtest round made runs much harder against a bot; no person has played it yet. Every number is in `data/content.json`; change balance there, not in code.
- That `notes/` is in the repository. It is not, on purpose (R1.24).
