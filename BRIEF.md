# Brief

Stage 1. The person's own words come first; the rest is written from them for the person to read. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

The first request (14 October 2026):

> Use sgilson7.github.io builder setting, which can be found one directory up. I approve the creation of a public github repo, and a public github pages to host the tool. I approve deployment to the website in order to update the tools section.  Use the shared technical setting within to create a game for me, based on the popular vampire survivor type of game. I want you to start by analyzing the following transcript, and derive all of the design insights related to this genre. then use those insights to build an artifact of what you derived, and how you will use it to build a game. give me a description of the mvp you intend to build, which should involve 3 distinct characters and 1 map alongside the other design insights we spoke of; intentionally leave the theming out of it for now, as I will provide that later. I used the mp4 to transcript to generate the transcript you will analyze. let me know if its like unintelligble and I can try and provide the mp4 directly

The transcript is `notes/discussion-transcript.txt`, and the design insights drawn from it are `notes/design-insights.html` (also a private artifact for the person). Both stay on the person's machine: the recording is a private conversation, so `notes/` is not in the public repository.

The second request (14 October 2026), answering the five questions the proposal asked:

> theme: i want it to be roughly space / exploring alient planet themed. I want the characters to have a vague risk of rain design, so I want them to be circular instead of blocky (long time coming the main character is too blocky for my liking). the character and enemy design should be a mix of risk of rain, metroid, armored core 6 (in case the planet you are exploring is raided by a corporate force like an event that can occur during gameplay), with a lil bit of marathon. emphasis on the art style to make the screen understandable and not too cluttered, so no motion sickness can occur. saving progress: saveable json file like in gear master 2d run length: 10-15 minutes xp and gold: not allowed repo name: Galactic Survivors. start working on the mvp; use my astra v2 style guide for writing human facing prose. I give you approval to check the directory above this one for other of sgilson7's work, and you can copy down ptonkles work if you need too in order to get more inspirations. the game should also support 2 player online co-op so my friend and i who discussed this can play together. there should be some gacha type system in the game too. ok please start building the mvp using all the design insignts and my comments here, and create the assets for the characters / enemies to show me in an artifact in parallel while you work.

## The game

Galactic Survivors. A survivors-like: the player lands on an alien planet and survives a run of about twelve minutes against waves of native creatures, with a corporate raid that can arrive partway through. The player only steers; abilities fire on their own. At each level-up the player picks one of three cards. Between runs, gold buys permanent upgrades and unlocks, achievements pay out, and a salvage cache (the gacha) gives modules that change how a run starts. Two players can play one run together online. Played in a desktop browser with a keyboard and mouse, or on a touch screen with an on-screen stick.

## Done means

A Builder project whose `./scripts/check.sh` passes, deployed as a public repository `galactic-survivors` on GitHub Pages and listed in the tools section of sgilson7.github.io, in which:

- one run on one map lasts between 10 and 15 minutes (the length is a number in `data/`), ending at a final boss;
- three characters differ by starting ability, trait, stats and abilities only they can be offered;
- abilities aim themselves; the pointer-aimed ones switch to auto-aim with a setting; nothing needs reloading;
- level-up offers three cards, with rerolls and banishes; evolutions are reachable by about a third of the way through a run, and each card names its evolution partner;
- an enemy can hurt the player at most once per short window, and a hit grants brief invulnerability;
- every character's starting ability damages every boss; there is no regeneration card;
- rifts open on a schedule and take the player into a solo arena against one elite;
- a corporate raid event can arrive during a run, with its own enemies;
- gold is kept on death; a shop sells permanent ranks and character unlocks, with no +XP or +gold upgrade;
- every achievement is visible from the first launch with its progress, and each pays gold plus a small permanent XP bonus;
- a salvage cache (gacha) gives modules from tickets earned in play, with a guaranteed rare within ten pulls and a skippable reveal;
- progress is saved by downloading a JSON save file and restored by loading it, as in Gear Master 2D;
- two players can play one run together online (peer to peer), and a downed player has something to do;
- the screen stays readable: a calm ground, bright players, one consistent shape for enemy shots, few simultaneous effects, no screen shake by default;
- the characters are round, and the art mixes Risk of Rain, Metroid and Armored Core 6, with a little of Marathon, drawn with the TikZ pipeline.

## Decisions that stay with the person

- The names of the game, the characters, the enemies, the abilities and the modules: first-pass names are written by the agent and listed in `SECOND-ORDER.md` for the person to read.
- Balance: every number lives in `data/*.json` and is a first pass.
- The look of the art, judged in the art artifact.
- Whether the gacha's odds and the pity count are right.

## Accessibility

Enemy shots share one shape and an outline, so they read without colour. Players are marked by a ring and a number in co-op. Pickups differ by shape. Colour pairs avoid red/green as the only distinction. Screen shake is off by default; reduced motion turns off hit freeze and knockback flashes. Input: keyboard (WASD or arrows), mouse for aiming, and touch with an on-screen stick.

## Style

A project style pack, `styles/galactic/`: an alien planet seen from above, dark and calm under the action. The person named Risk of Rain (round characters), Metroid (alien creatures, suits), Armored Core 6 (the corporate raiders and their machines) and a little of Marathon (graphic, high-contrast interface marks). The person's priority is a screen that stays understandable and does not cause motion sickness.

## Words

The person's terms, from the transcript and the requests: run, level-up, evolution, rift, reroll, banish, gold, achievement, gacha (shown in the game as the salvage cache), co-op, boss, elite. They go into `GLOSSARY.md` before a string uses them.

## Writing

Human-facing prose (the README, the website listing, the help text) follows Sam's voice guide v2 (`website/Sam_Website_Voice_Revision_2026-10-08/VOICE_AND_STYLE_GUIDE_V2.md`) and `TONE.md`.

## Autonomy

The plan is approved in advance ("start working on the mvp"; "please start building the mvp"). Where this brief is silent, the agent decides, records the decision in `SECOND-ORDER.md`, and continues. The agent may not loosen a Builder requirement or a boundary test. The online co-op is a declared off-origin feature (R7) through `kits/online/`, loaded only when a player hosts or joins. The person granted, in their words: "I approve the creation of a public github repo, and a public github pages to host the tool. I approve deployment to the website in order to update the tools section." The repository is named from "repo name: Galactic Survivors". The agent may look through the person's other projects in the parent directory and may copy from Peter's (ptonkle) public work for inspiration, crediting it in `LICENSES.md`.

## Allowed core dependencies

`serde`, `serde_json` (reference default).
