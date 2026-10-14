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

## What passed

| Check | Result |
| --- | --- |
| `./scripts/check.sh` | BUILDER CHECK PASSED (Chromium, Firefox, WebKit), 14 Oct 2026 |
| `BUILDER_ORIGIN=https://sgilson7.github.io/galactic-survivors tests/browser/check.py` | PASSED in all three engines against the live site |
| Website: `make check`, `make packet`, `make gate`, then `make gate-live` after publishing | all passed; the listing is at /projects/galactic-survivors/ |
| `./scripts/count-tests.sh` | 115 passed, 0 failed, 1 ignored (the probe) |
| `python3 scripts/break_tests.py` | 21 of 21 planted defects caught |
| `python3 tests/browser/online.py chromium firefox webkit` | passed: two pages meet by pasted codes and play in step |
| `python3 tools/art/art.py --check-only` | 57 pictures pass |

## Try these by hand first

1. Play a solo run as Pathfinder to the end. Is the first run too easy? The probe bot wins four of six without upgrades (SECOND-ORDER R1.25, R1.17).
2. After the run, download the save, reload the page, and load the save. Gold and achievements should come back.
3. With your co-op partner, host a room from one home and join from the other (SECOND-ORDER R1.18). If the room code does not connect, try Host by invitation. Write what happened and the date here.
4. Read `data/strings.json` top to bottom and rename anything (SECOND-ORDER R1.16). Names live only there.

## Rows for the person

- R1.16: every name and string in `data/strings.json` was written by the agent.
- R1.17: the balance in `data/content.json` is a first pass against a bot.
- R1.18: co-op over room codes and real networks is untested.
- R1.25: decide whether a first run should be harder so the outfitter matters sooner.

## Unfinished

- Glossary hover hints (D15) are not drawn in the menus (R1.14).
- Damage numbers are not drawn; the setting is hidden (R1.22).
- The relays in `web/net/config.js` were last confirmed in October 2026 by another project (R1.19).
- No sound.

## What the next session must not assume

- That co-op works between two homes. It has been seen working only between two tabs on one machine.
- That the balance is right. Every number is in `data/content.json`; change balance there, not in code.
- That `notes/` is in the repository. It is not, on purpose (R1.24).
