# Handoff

Stage 9. `C5 Consolidate in your own words`. A fresh session starts here.

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

## What passed

| Check | Result |
| --- | --- |
| `./scripts/check.sh` | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 14 Oct 2026 |
| `./scripts/check.sh`, after the playtest round | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 14 Oct 2026 |
| `python3 tests/browser/online.py chromium firefox webkit`, after the playtest round | ONLINE CHECK PASSED |
| `BUILDER_ORIGIN=https://sgilson7.github.io/galactic-survivors tests/browser/check.py` | PASSED in all three engines against the live site |
| Website: `make check`, `make packet`, `make gate`, then `make gate-live` after publishing | all passed; the listing is at /projects/galactic-survivors/ |
| `./scripts/count-tests.sh` | 132 passed, 0 failed, 1 ignored (the probe), after the playtest round |
| `python3 scripts/break_tests.py` | 45 of 45 planted defects caught, after the playtest round |
| `python3 tests/browser/online.py chromium firefox webkit` | passed: two pages meet by pasted codes and play in step |
| `python3 tools/art/art.py --check-only` | 95 pictures pass, after the playtest round |

## Try these by hand first

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

- R1.16: every name and string in `data/strings.json` was written by the agent.
- R1.17: the balance in `data/content.json` is a first pass against a bot.
- R1.18: co-op over room codes and real networks is untested.
- R1.25: decide whether a first run should be harder so the outfitter matters sooner. (Answered by the friends' T7; see R2.13.)
- R2.13: the playtest round's balance may make a first run too hard.
- R2.14: the achievement grid's layout, goals, rewards and unlocks, and 36 new names, were all chosen by the agent.

## Unfinished

- Glossary hover hints (D15) are not drawn in the menus (R1.14). The codex now covers abilities, passives and evolutions.
- The weakest new pictures are listed in R2.15.
- The website listing still says "all twenty achievements … each adds a little experience". Fixing it needs a website deploy, which needs the person's go-ahead.
- Damage numbers are not drawn; the setting is hidden (R1.22).
- The relays in `web/net/config.js` were last confirmed in October 2026 by another project (R1.19).
- No sound.

## What the next session must not assume

- That co-op works between two homes. It has been seen working only between two tabs on one machine.
- That the balance is right. The playtest round made runs much harder against a bot; no person has played it yet. Every number is in `data/content.json`; change balance there, not in code.
- That `notes/` is in the repository. It is not, on purpose (R1.24).
