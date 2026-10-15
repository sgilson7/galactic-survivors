# Triage

Later requests in the person's own words, one numbered row each (T1, T2 …). The commit that answers a row cites it: "(T3)". `C4 Verify against hand-computed cases`.

Playtest notes from the person's friends, 14 October 2026, split into one row per request (the words are theirs, quoted in order):

| # | The request, word for word | What was done | Commit | Status |
| --- | --- | --- | --- | --- |
| T1 | "you can't inspect your gear and tell what is what during the game" | Pausing a solo run shows the build: each ability's current damage and recharge, each passive, and how each evolves. In co-op, I shows it without pausing (SECOND-ORDER R2.17). | d355ae2 | done |
| T2 | "no way to know what is about to evolve what" | The build and the codex give each recipe and its state ("Now level 3 of 5. Split Feed is level 1."); the strip under the run marks an ability "Ready to evolve". Card text fixed (R2.6). | d355ae2 | done |
| T3 | "First game I was unstoppable, I bought all the permanent character upgrades, next run took different items, and felt like 60% as strong, you shouldn't be able to get so many permanent character upgrades" | Outfitter ranks mostly start locked and achievements open them one at a time; payouts cut and prices raised; old saves get their ranks refunded under the caps (R2.2–R2.4). | d355ae2 | done |
| T4 | "the achievements should be laid out as a grid like halls of torment, and you can only see adjacent achievements, but can unlock them at any time even if they are hidden" | An 8 by 7 grid; four tiles show at the start; earning one shows its neighbours; hidden ones are earned all the same (R2.1). | d355ae2 | done |
| T5 | "there should be a lot more achievements that are harder to get" | 56 achievements, up from 20, many long or hard (40,000 kills, two hours played, win with no gear, evolve within 3:30) (R2.1, R2.14). | d355ae2 | done |
| T6 | "the main character should have a movement animation" | A four-frame walk for all three characters (R2.16). | d355ae2 | done |
| T7 | "generally the game should harder so the permanent upgrades are more useful" | Plain probe wins fell from four in six to one in six; a fully upgraded bot wins six of six (MEASUREMENTS; R2.13). | d355ae2 | done |
| T8 | "the modules you equip from the gatcha should be mergeable / level up able" | Every pull is its own piece; two copies at one rank merge a rank higher, up to rank 5; Merge everything button (R2.4). | d355ae2 | done |
| T9 | "the window to equip them should look more like a traditional gear window like diablo" | A gear window: four typed slots around the character beside a stash grid with rarity colours and rank marks, and a detail panel to equip, merge or scrap (R2.4). | d355ae2 | done |
| T10 | "there should be a glossary for all the moves / upgrades / weapons you get during gameplay" | A codex of every ability, passive and evolution with numbers and recipes, from the title screen and from the pause screen. | d355ae2 | done |
| T11 | "there should be a reminder you can dash on vector" | Vector sees a dash meter and "Dash ready (Space)", and a prompt under the player until the first dash (R2.18). | d355ae2 | done |
| T12 | "enemies should be a little tankier given how fast you can evolve" | Enemy health ×1.4 (but the two first-minute enemies), faster growth per minute, a steeper experience curve, and evolutions need the partner passive at level 3. Focused probes evolve at 3:12–6:20, was 2:45–4:11 (R2.7, R2.11). | d355ae2 | done |

Requests from the person, 15 October 2026 (research and plan in the private design journal; no code yet):

| # | The request, word for word | What was done | Commit | Status |
| --- | --- | --- | --- | --- |
| T13 | "the networking right now is real tough, we are constantly lagging out pretty bad … is there roll back netcode like raiders of the lost mart? im thinking we try and do as much as humanely possible as client side calculations / updates" | Rollback after Raiders: own input after 2 ticks, the partner guessed and corrected, at most 8 ticks ahead; inputs on an unordered channel that never resends; a co-op level-up shields the chooser instead of stopping both; a ping and correction readout; the tick twice as fast (R3.1–R3.10). | 5dd2430 | done |
| T14 | "the designer wants a lot more abilities and passives added to the game, stuff like orbital cannons firing around you that draw circles that get lit on fire and deal damage, you leave mines behind you, you can draw shapes with your movement to draw runes that give stacking passives, etc." | 7 abilities (Orbital Lance, Sweep Cannon, Wake Mine, Gravity Well, Tether, Boomerang Disc, Seed Pod), 8 passives, 4 ground fields that react, and 3 runes drawn by walking (R3.16–R3.19). | 390814f | done |
| T15 | "the designer said that the abilities should be in some way combinable and very open ended and free-form like the game noita" | Rigs: sockets read left to right with 21 common and 6 rare parts; a rig window that never pauses; rare parts from achievements and the salvage cache (R3.11–R3.15). | 390814f | done |

Requests from the person, 15 October 2026, after round 3:

| # | The request, word for word | What was done | Commit | Status |
| --- | --- | --- | --- | --- |
| T16 | "the cooldown of the weapon should be controlled by the first ability, not the highest cooldown ability in the rig" | The first payload before the first trigger sets a rig's recharge; parts still add load (R4.1). | 1879220 | done |
| T17 | "what is the state of testing over rigs? … I want to test and make sure as many permutations of realistic rigs are tested to ensure they have the proper behavior. first do a pass over test run times and test setup, then figure out how many permutations we can test per second, then give figures for our coverage of the sample space given how much time we spend testing" | A rig space test plays rigs in a fixed scene against ten rules; about 8,200 rigs a second on 12 cores. Everyday: 4,811 plays in 0.6 s. Swept: every rig of up to four sockets. It found three bugs, now fixed (R4.2, R4.3) or the person's (R4.4, R4.5). Figures in MEASUREMENTS and the journal's sixth tab (R4.6–R4.8). | 1879220 | done |
| T18 | "keep permutations that have been tested in a logbook, noting its behavior when played, last time tested, and conditions for needing to be retested, so we can try to overtime build a log with essentially every permutation as a log line" | Built: the ignored `logbook` test keeps a line per rig in `rig-log/` (local) and a summary in `tests/rig-log/` (R4.8–R4.10). | 0242533 | done |

Answers from the person to the journal's sixth tab, 15 October 2026:

| # | The request, word for word | What was done | Commit | Status |
| --- | --- | --- | --- | --- |
| T19 | "yes" (formations on Tether: one tether for each copy) | Each copy joins the next end, or reaches out its own way (R4.4). | 0242533 | done |
| T20 | "after x full rotations of an orb, it can leave a field again, i leave x up to you based on gameplay feel" | One turn (`orbit_turns`); a tether, every two seconds (R4.5). | 0242533 | done |
| T21 | "yes build the logbook, keep it locally, publish a scoped down one to the repo" | `rig-log/` stays local; `tests/rig-log/` holds the summary (R4.8). | 0242533 | done |
| T22 | "yes do the shuffled order for 5 and 6 token rigs. do 1 hour of both" | Five and six tokens are logged in a fixed shuffled order; an hour of each was run (R4.11). | pending | done |
| T23 | "4. yes" (fix the co-op gear bug first, on its own) | Gear and Outfitter changes made in the lobby now reach the run (R4.12). | pending | done |
| T24 | "1. yes 2. yes, they can write their own names in 3. yes" (journal entry 8: a partner's own change takes back Ready; players type their names; a room link joins on opening) | The new co-op lobby (R4.13–R4.16). | pending | done |
