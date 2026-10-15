# Glossary

The source of truth for what each word in Galactic Survivors means. Strings, tests, the plan and the README use a term in the sense given here. Each entry is a `### Term` heading, one or two plain sentences, and an optional `Not:` line of words never used for it. A number a rule decides is a `{placeholder}`, so the glossary cannot disagree with the rules. `crates/core/tests/copy.rs` checks the format and the `Not:` words.

### Achievement

A goal on the achievement grid. A few show from the first launch; earning one shows the ones next to it, and a hidden one can still be earned before it shows. Earning one pays gold and tickets once, and many also open an outfitter rank, a gear slot, an ability, an evolution, a character or a banish.

Not: trophy, badge

### Auto-aim

The setting that makes the pointer-aimed abilities fire at the nearest enemy instead of toward the pointer. Every other ability aims itself whatever the setting.

### Banish

Removing one card from the current run for good; a replacement card is drawn in its place. A run starts with a few banishes, and the outfitter and an achievement add more.

### Boss

A large enemy that arrives at a set time or ends the corporate raid: the Burrower, the Strider and the Matriarch. Every character's starting ability can damage every boss.

### Card

One choice offered at a level-up or after a rift: a new ability, a higher level of one, a passive, or an evolution.

### Co-op

Two players in one run, each in their own browser, connected directly to each other. Both players share experience and level up together.

### Rig

A row of sockets that fires on its own, read left to right each time. A player has one rig for each ability slot. The first ability in a rig sets its recharge, and each part adds to it.

### Socket

One place in a rig. It holds an ability, a part, or nothing.

### Part

What goes in a socket beside the abilities: a modifier, a formation or a trigger. Level-up cards offer parts; rare ones are opened by achievements or found in the salvage cache.

Not: relic, artifact, spell

### Payload

An ability as a rig sees it: the thing a rig puts into the world when it fires.

### Modifier

A part that changes the next payload in its rig, such as leaving fire where it lands.

### Formation

A part that fires the next payload in its rig several times at once, arranged around you or side by side.

### Trigger

A part that hands the rest of its rig to an event: where the payload before it hits, ends or defeats an enemy, or when you walk a stride, dash, draw a rune, or every few seconds.

### Stride

A set distance walked, counted by the Each Stride trigger.

### Spare parts

Parts and abilities held but not in a rig. They do nothing until moved into a socket.

### Ground field

A patch of fire, frost, charge or slick that some payloads leave where they land. Fields hurt or slow enemies standing in them, and two kinds that meet react.

### Rune

A shape drawn by walking a closed loop: a circle, a triangle or a square. A rune strikes every enemy inside the loop and gives a passive for a while; several of one shape add up.

### Ping

The time a message takes to reach the other co-op player and come back, in milliseconds. Shown at the top right during a co-op run.

### Correction

In co-op, each browser moves your partner as if they kept doing what they last did, then puts it right when their real input arrives. Each time that happens is one correction, counted at the top right during a co-op run.

Not: rollback, desync

### Elite

A strong single enemy, the Stalker, met inside a rift.

### Evolution

A stronger form of an ability. An ability at its highest level, held together with its partner passive, can evolve; the next level-up offers it as a card, and a supply cache or a rift reward evolves it too.

Not: fusion

### Ghost

What a co-op player becomes when their health runs out while their partner still stands. A ghost drops a repair cell every few seconds and comes back when the partner stays beside it long enough.

### Gold

What runs pay, kept whether the run is won or lost. It buys outfitter ranks and unlocks characters.

Not: coins, credits, money

### Level-up

The moment the shared experience bar fills. In a solo run the field waits while you choose one card. In co-op the run goes on, and a player choosing is shielded until they move after choosing, for a few seconds at most.

### Build

The abilities and passives a player holds in the current run. Pausing a solo run shows it, and in co-op a key shows it without pausing: what each one does now and what it still needs to evolve.

### Codex

The list of every ability, passive and evolution, with their numbers and how they evolve. It opens from the title screen and from the build during a run.

### Module

A part pulled from the salvage cache that a character carries into a run. Every pull is its own piece with a rank, and each module fits one kind of gear slot.

Not: relic, artifact

### Gear

The modules a character wears, one in each open gear slot. The gear window shows the slots beside the stash of every piece the player owns. Some slots start locked, and achievements open them.

### Merge

Combining copies of one module at the same rank into a single piece a rank higher, in the gear window. Abilities never merge; they evolve.

### Scrap

Turning a piece the player is not wearing into gold, in the gear window. Rarer and higher-rank pieces pay more.

### Outfitter

The shop between runs. It sells permanent ranks for gold and never sells extra experience or extra gold. Most ranks start locked, and achievements open them one at a time.

### Raid

An event partway through a run: corporate dropships land drones and troopers for a while, and a Strider ends the raid.

### Reroll

Drawing a whole new hand of cards at a level-up.

### Rift

A portal that opens at set times. Stepping in takes the players to a small arena to fight the elite alone, and clearing it pays a reward card, tickets and gold.

Not: portal room, dungeon

### Run

One landing on the planet, from the start until the players win or are all down.

Not: match, round

### Salvage cache

Where tickets are spent on modules. Each pull has fixed odds by rarity, and a rare or better is certain within a set number of pulls.

Not: loot box

### Save file

A file the player downloads that holds all progress between runs, and loads again next time. The page keeps nothing in the browser.

### Supply cache

A crate an elite or a boss drops. Picking it up evolves an ability that can evolve, or raises an ability one level.

### Ticket

What a pull from the salvage cache costs. Bosses, rifts and achievements give them.
