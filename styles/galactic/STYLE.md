# Galactic style

The style pack for Galactic Survivors: an alien planet seen from above at three-quarter view, dark and calm under the action. The person asked for round Risk of Rain characters, Metroid creatures, Armored Core 6 corporate machines with a little of Marathon's graphic marks, and above all a screen that stays understandable and does not cause motion sickness. Every rule below serves that last request first.

## What a picture looks like

- Flat fills only. No gradients, glow halos, blur, soft shadows or texture. A highlight is a second flat shape (`suit`, `visorhi`, `violethi` ...), never a gradient.
- Every filled shape is outlined in `ink`. `Fig` gives no way to fill without an outline; limbs, tendrils and pipes use `motifs.tube`, which draws an ink line under a narrower fill line so a stroke reads as an outlined shape.
- Smaller pictures get heavier outlines (see the table below), so the outline survives at 32 to 64 px.
- Round, not blocky. Bodies are circles, ellipses and capsules; armour plates have rounded corners (`rect(..., r=...)`). Sharp points are kept for teeth, claws, crests, fins and chevrons, where they read as danger or speed.
- Sprites face right. The game mirrors them to face left and never rotates them. Each standing sprite has a flat `shadow` ellipse under it; a hovering one has the shadow apart from its body.

## The four families

| Family | Shape | Colour |
| --- | --- | --- |
| Players | A round chunky body (circle or capsule), a dark visor with one flat highlight, small limbs, one strong silhouette feature each | `suit` and `suitshade`, with one accent: Pathfinder `amber` (shoulder drone), Breaker `teal` (shoulder dome and arc gauntlet), Vector `coral` (rail-lance, crest fin, thrusters) |
| Alien fauna | Organic, rounded: carapaces, plates, tendrils, mandibles; at least one bright weak-spot eye (`motifs.eye`) | mid-saturation `violet`, `olive`, `faunateal`, with `flesh` and `acid`; eyes and weak spots `weakspot` |
| Corporate raid | Armored Core machines: round-cornered armour plates, sensor eyes, reverse-joint legs, Marathon decals (`chevrons`, `stripes`, `corp_glyph`) | `hazard` and `offwhite` paint over `gunmetal`; sensors `sensor` |
| Ground | Regolith, rock pillars, wreckage | dark low-contrast `ground`, `groundB`, `groundC`, `pebble`, `rock` |

The players are the lightest things on screen, so they pop against the dark ground. The three must stay distinguishable in greyscale: Pathfinder by the drone above its shoulder, Breaker by its width and shoulder dome, Vector by its slim body and long lance.

The corporate glyph is invented and stands for no real company. No real logo, name or character appears.

## The enemy shot

Every enemy projectile uses one shape, `motifs.enemy_shot`: a thin pale rim (`shothi`), a thick dark ring (`ink`), a bright magenta core (`shot`) and a small flat highlight. Light, dark, light from the edge in, so it reads without colour on any ground. Nothing else in the game uses `shot`.

## Pickups

Pickups differ by shape, not only colour: XP is a diamond (`xp`), gold a hexagon (`gold`), health a white capsule with a cross (`heal`), a ticket a notched chip (`ticket`), a cache a crate with hazard bands, a rift a swirling oval (`rift`).

## Icons

One subject on a round badge (`badge`). The rim tells the kind: abilities `badgehi`, passives `teal`, evolutions `gold`. Subjects are bold and simple, readable at 32 px.

## The title picture

A night sky in three flat bands (`motifs.sky_bands`), a ringed planet and the corporate dropship on the right, a ridge across the lower half, and the three players on its crest. The upper half of the left third is sky only, checked by `motifs.calm_left_third`, for the title text.

## Sizes and line weights

Sizes, pixel sizes, transparency and byte caps are in `data/art.json`. Ink widths come from `ink.json`:

| Kind | Size | Ink width |
| --- | --- | --- |
| sprite | 256 px cutout | 3.0 pt |
| boss | 512 px cutout | 3.6 pt |
| small | 64 px cutout | 1.6 pt |
| icon | 128 px cutout | 2.2 pt |
| tile | 512 px seamless | 1.0 pt |
| scene | 1280 x 720 | 1.4 pt |

Interior lines (plate seams, cracks) may be lighter, down to about two thirds of the ink width, and stay `ink`. A cutout keeps every edge clear.

## Palette by role

The values are in `palette.json`. Pictures name roles, never colour values.

| Role | Use |
| --- | --- |
| `ink` | every outline and interior line |
| `shadow` | flat ground shadows, holes |
| `ground`, `groundB`, `groundC`, `pebble` | the regolith tile |
| `rock`, `rockhi`, `rockdark`, `crystal` | rocks, ridges, crystals |
| `skyhi`, `sky`, `skylow`, `haze`, `moon` | the title sky and far hills |
| `suit`, `suitshade`, `visor`, `visorhi` | player suits and visors |
| `amber`, `teal`, `coral` | the three player accents |
| `violet`, `violethi`, `violetdark`, `olive`, `olivehi`, `faunateal`, `faunatealhi`, `flesh`, `acid`, `weakspot` | alien fauna |
| `hazard`, `offwhite`, `gunmetal`, `gunmetaldark`, `sensor` | corporate machines |
| `shot`, `shothi` | the enemy shot only |
| `xp`, `xphi`, `gold`, `goldhi`, `heal`, `ticket`, `rift`, `rifthi` | pickups and effects |
| `badge`, `badgehi` | icon badges |

## What not to do

- No words, letters or digits in any picture; the pipeline refuses TeX that could set text.
- No gradients, glow halos, bloom or soft shadows.
- No busy ground: the tile stays within a few steps of `ground` so it never competes with sprites.
- No second projectile shape for enemies, and no magenta anywhere but the enemy shot.
- No red and green as the only distinction between two things.
- No real person, brand, logo or known character.
