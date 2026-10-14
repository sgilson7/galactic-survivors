# Galactic Survivors

A survivors-like you play in the browser: you land on an alien planet, steer through waves of native creatures for twelve minutes, and fight the hive's Matriarch at the end. You only move. Your abilities fire on their own, and each level-up asks you to pick one of three cards. Partway through, a corporate raid may drop in with drones, troopers and a walking mech. Two players can play the same run together, browser to browser.

**Play it:** https://sgilson7.github.io/galactic-survivors/

## What the game is built from

The game started from a conversation between two players of Vampire Survivors, Halls of Torment, Megabonk and a Pokémon survivors-like, recorded while they played. I read the transcript for what they said makes these games work and turned each point into a rule the game's core enforces and a test checks. A few examples:

- **Moving is the whole job.** Both players agreed that, moment to moment, the skill in these games is steering. So nothing needs a fire button or a reload, and the camera never turns or shakes.
- **Contact damage should not punish melee.** One player described a crowd of small enemies landing many hits at once, which made close-range builds feel bad. Here an enemy can hurt you at most once per short window, and any hit gives a moment of invulnerability. The test puts twenty enemies on the player at once and expects exactly one hit.
- **Evolutions should come earlier.** When an ability can evolve, the next level-up offers the evolution as a card, so it never waits for a boss to drop a crate. In the run probe, a player who builds toward it evolves at about three to four minutes.
- **No character should be useless against a boss.** Both players remembered a character whose attacks could not hurt most bosses. Here no enemy is immune to anything, and a test fires every character's starting ability at every boss.
- **A wall should always be grindable.** Gold is kept when you lose, and the outfitter sells permanent upgrades. At the person's request it sells no extra experience or extra gold, since those are the upgrade everyone has to buy first.
- **Achievements should be visible and worth something.** All twenty are listed from the start with your progress. Each pays gold and adds one percent experience to every later run, and some unlock a character or an evolution.

## How to play

- **Move** with WASD or the arrow keys. On a touch screen, drag anywhere to steer.
- **Aim** with the mouse. Only the Rail Bolt and the Cutting Beam use it, and Auto-aim in Settings turns that off.
- **Choose cards** with 1, 2 or 3, or click one. R draws a new hand. A banish removes a card for the rest of the run.
- **Dash** with Space, as Vector.
- **Pause** with Esc in a solo run.

Three characters: Pathfinder (balanced; its drone does the aiming), Breaker (a heavy close-range fighter who takes half damage from contact) and Vector (fast and fragile, aims with the pointer, and can dash). Breaker costs gold; Vector unlocks when you defeat the Burrower.

**Your progress is a file.** The page keeps nothing in your browser. After a run, choose Download save, and next time choose Load save and pick that file. Without it, gold, unlocks and achievements start over.

**The salvage cache** turns tickets, which bosses, rifts and achievements give, into modules a character carries into a run. The odds are on the screen, and a rare or better is certain within ten pulls. There is no real money anywhere in the game.

## Playing together

One player opens Play co-op and hosts a room; the other types the room code. If a room code cannot connect, the host can make an invitation instead: copy it to your partner, who pastes it and sends back a reply. The two browsers then talk directly to each other. Both players share experience and level up together. A player who goes down becomes a ghost that drops repair cells, and comes back when the partner stands beside it long enough.

Room codes are introduced over public Nostr relays listed in `web/net/config.js`; invitations use no server at all. Two players who are both on strict networks may not be able to connect, because the game has no relay server of its own for game traffic.

## How it is built

The game uses [Builder](https://github.com/sgilson7/builder-setting), the setting I use for browser tools and games. Every rule lives in a Rust core with no input or output (`crates/core`): the run, the abilities, the cards, the save, the shop, the salvage cache and the co-op session. A WebAssembly shim passes values to the page and decides nothing, and the page draws the list of shapes and pictures the core sends. Positions are integers and chance comes from a seed, so two browsers given the same inputs hold the same world, which is what lets co-op work without a server.

All content and balance are in `data/content.json`, and every word on screen is in `data/strings.json`, so changing a name or a number does not touch a rule. The art is TikZ drawn by Python (`tools/art/`), in a style pack for this game (`styles/galactic/`).

```bash
./scripts/setup.sh      # once: Playwright and its browsers
./scripts/check.sh      # format, lint, tests, build, browser check
./scripts/build.sh      # build into dist/
python3 scripts/serve.py   # serve dist/ locally
```

`PLAN.md` lists each decision with the test that shows it holds, `SECOND-ORDER.md` records what changed along the way, and `MEASUREMENTS.md` has the probe results.

## Licence

MIT. See `LICENSES.md` for the vendored Trystero bundle and the code taken from my other projects.
