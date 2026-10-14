# Licences

Every file in this repository that is not code written for it: its source, its author, its terms. The code is under the repository's licence (MIT, `LICENSE`).

## Assets

| Path | Source | Terms |
| --- | --- | --- |
| `web/art/` | drawn by `tools/art/art.py` and `tools/art/art_galactic.py` from TikZ in this repository, in the project style pack `styles/galactic/` | this repository's licence |

The look draws on games the person named (Risk of Rain, Metroid, Armored Core 6, Marathon) as a direction only. No picture, logo, name or likeness from those games is used.

## Vendored code

| Path | Project and version | Author | Licence | Checksum |
| --- | --- | --- | --- | --- |
| `web/net/vendor/trystero-nostr-0.25.4.js` | [Trystero](https://github.com/dmotz/trystero) 0.25.4, Nostr strategy, bundled by esm.sh | Dan Motzenbecker | MIT | `web/net/vendor/SHA256SUMS` |

## Code taken from other repositories

| From | What | Licence |
| --- | --- | --- |
| Builder (`sgilson7/builder-setting`) | the template, and the realtime, canvas and online kits (`fx.rs`, `rng.rs`, `ticks.rs`, the lockstep session in `online.rs`, `online_fence.rs`, `tests/browser/online.py`) | MIT, Sam Gilson |
| Raiders of the Lost Mart (`sgilson7`) | `web/net/rtc.js`, by way of Ribbit King, Vagrancy and Floodline; the relay list in `web/net/config.js` | MIT, Sam Gilson |
| Peter's Mandible and Ape Race (GitHub ptonkle) | the draw-list renderer design and the transport design, as rewritten in the Builder kits; no code copied from Peter's repositories | design credit |
