# Decisions

Dated entries. Each quotes the person's request word for word, then says what the agent decided and where it is recorded.

## 2026-10-14

> I approve the creation of a public github repo, and a public github pages to host the tool. I approve deployment to the website in order to update the tools section.

> repo name: Galactic Survivors

The agent decided: the repository is `sgilson7/galactic-survivors`, published from its `gh-pages` branch with `./scripts/publish.sh` once `./scripts/check.sh` passes, and listed in the tools section of sgilson7.github.io. The grant covers this project until withdrawn.

> saving progress: saveable json file like in gear master 2d

The agent decided: the core writes and reads the save as JSON; the page downloads it and reads a chosen file. D17 (no browser storage) is unchanged. `meta.rs`.

> xp and gold: not allowed

The agent decided: no outfitter rank or module may raise experience or gold; the content file is refused at load if one does (`content::FORBIDDEN_STATS`). The achievement XP bonus stays, because it is earned, not bought.

> the game should also support 2 player online co-op so my friend and i who discussed this can play together.

The agent decided: lockstep co-op through Builder's online kit, by room code or by pasted invitation. `online.rs`, `web/online.js`, `web/net/`.

> there should be some gacha type system in the game too.

The agent decided: the salvage cache, paid with tickets earned in play, odds and pity printed on screen. SECOND-ORDER R1.20.

> the modules you equip from the gatcha should be mergeable / level up able, and the window to equip them should look more like a traditional gear window like diablo

(from the person's friends, TRIAGE T8 and T9) The agent decided: each pull is its own piece; two at one rank merge a rank higher; four typed slots (core, frame, two utility) beside a stash grid. SECOND-ORDER R2.4.

> the achievements should be laid out as a grid like halls of torment, and you can only see adjacent achievements, but can unlock them at any time even if they are hidden

(TRIAGE T4) The agent decided: an 8 by 7 grid, four starting tiles, four-way neighbours. The +1% XP per achievement was dropped (SECOND-ORDER R2.1).

The updated game was published to its own GitHub Pages site under the brief's grant: "I approve the creation of a public github repo, and a public github pages to host the tool." The website listing was not redeployed; the person's grant for the website covered adding the listing.
