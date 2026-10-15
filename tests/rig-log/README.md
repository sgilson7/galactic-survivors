# The rig logbook (T18)

The rig space test (`crates/core/tests/rig_space.rs`) plays a rig for four seconds in one fixed scene and checks it against ten rules. The logbook keeps a line for every rig it has played, so each run plays only rigs that are new or out of date.

    cargo test -p tool_core --test rig_space -- --ignored --nocapture logbook

`RIG_BUDGET_S` sets how long a run may play (120 seconds unless set). `RIG_LENGTHS` sets which rig lengths it covers (1,2,3,4 unless set), and `RIG_LEVELS` which ability levels (1 unless set). Rigs of up to four tokens are logged in number order, all of them. Five and six tokens are far too many to finish, so they are logged in a fixed shuffled order: a run that stops early has logged an even sample, and later runs fill in the rest. Shorter rigs come first, and a run stops after the block it is playing when the budget is spent. The test fails when any rig in the log breaks a rule.

## Where it lives

The full log stays on the computer that runs it, in `rig-log/` at the top of the repository, which git ignores. Every rig of four tokens or fewer comes to about 1.5 GB. This folder holds a smaller version for the repository:

| File | Holds |
| --- | --- |
| `summary.tsv` | One line for each block of 4,096 rig numbers: how many rigs it holds, how many failed or were noted, a fingerprint of their behaviour, its key, and when it was last tested |
| `notable.tsv` | Every rig whose latest result is a failure or a note, in full |
| `changes.tsv` | The rigs whose behaviour changed in the latest run, at most 200; the full list is in `rig-log/changes.tsv` |
| `runs.tsv` | One line for each run: when, from which commit, how many rigs it played, how many changed, and how many of each length are now logged |

## A line in the full log

    number  rig  result  behaviour  fingerprint  key  last tested

- **number**: the rig's place among all rows of its length. It turns into exactly one rig and back.
- **rig**: its tokens, left to right.
- **result**: `ok`, or each `FAIL` and `note` with its rule.
- **behaviour**: how often it fired, the damage it dealt to the six enemies that can't die, its kills, what it made, the fields it left, field reactions, and its deepest trigger.
- **fingerprint**: the world at the end of the scene. A change here means the rig behaves differently.
- **key**: what the line depends on (below).
- **last tested**: the date and the commit (`+` when the tree had uncommitted changes).

## When a line goes stale

A line is played again when its key no longer matches. The key covers:

- the rule code: `world.rs`, `arsenal.rs`, `rig.rs`, `field.rs`, `rune.rs`, `geom.rs`, `fx.rs`, `rng.rs`, `content.rs` and `meta.rs` in `crates/core/src/`;
- the scene and rules: `crates/core/tests/support/rig_rules.rs` (editing `rig_space.rs` itself changes no line);
- the data that isn't an ability or a part: enemies, run settings, fields and runes;
- the rig's level;
- the data of each ability and part in the rig, so a change to one part touches only the rigs that hold it, about 7% of four-token rigs.

Adding, removing or reordering an ability or part renumbers every rig, so the log starts again.

A rig that behaves differently when played again isn't a failure. It is listed in `changes.tsv`, so a balance change shows which rigs it moved.
