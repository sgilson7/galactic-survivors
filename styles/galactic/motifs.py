"""Motifs for the Galactic Survivors style pack. Each takes an artlib.Fig and
role names, so a pack with other values for the same roles draws the same
shapes in its own colours.

    import style
    m = style.load().motifs()
    m.tube(fig, [(0, 0), (1, 0.5)], "violet", 0.12)   # an outlined limb
    m.shadow(fig, 0, -0.85, 0.5)                       # flat ground shadow
    m.chevrons(fig, 0, 0, 0.2, "hazard")               # corporate decal
    m.corp_glyph(fig, 0, 0, 0.2)                       # the raiders' mark
    m.enemy_shot(fig, 0, 0, 0.3)                       # the one enemy shot
"""
import json
import math
from pathlib import Path

_INK = {k: float(v) for k, v in json.loads((Path(__file__).parent / "ink.json").read_text()).items() if not k.startswith("_")}


def arc(cx, cy, r, a0, a1, n=24, ry=None):
    """Points on a circle (or ellipse with ry) from angle a0 to a1 degrees."""
    ry = r if ry is None else ry
    return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
             cy + ry * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]


def tube(fig, pts, fill, w, smooth=True):
    """An outlined limb, tendril or pipe: a fat ink line with a narrower fill
    line on top, so the stroke reads as a filled shape with an ink edge.
    w is the visible fill width in cm of the frame's scale."""
    fill_pt = w * fig.k() * 28.4528          # cm to pt
    ink_pt = _INK.get(fig.kind, _INK["default"])
    sm = smooth and len(pts) > 2
    fig.stroke(pts, width=fill_pt + 2 * ink_pt, role="ink", smooth=sm)
    fig.stroke(pts, width=fill_pt, role=fill, smooth=sm)


def shadow(fig, x, y, rx, ry=None):
    """The flat ground shadow under a standing or hovering thing."""
    fig.ellipse(x, y, rx, ry if ry is not None else rx * 0.24, "shadow")


def chevrons(fig, x, y, s, fill="hazard", n=2, rot=0):
    """n bold chevrons pointing right, Marathon-style decal."""
    with fig.frame(x, y, s=s, rot=rot):
        for i in range(n):
            dx = i * 0.75
            fig.poly([(dx - 0.5, 0.6), (dx, 0.6), (dx + 0.5, 0), (dx, -0.6), (dx - 0.5, -0.6), (dx, 0)], fill)


def stripes(fig, x0, y0, x1, y1, n, fill="hazard", back="ink", slant=0.5):
    """Hazard stripes in a box: a back plate with n slanted bars."""
    fig.rect(x0, y0, x1, y1, back)
    w = (x1 - x0) / (2 * n)
    h = y1 - y0
    for i in range(n):
        a = x0 + (2 * i + 0.5) * w
        pts = [(a, y0), (a + w, y0), (min(a + w + slant * h, x1), y1), (min(a + slant * h, x1), y1)]
        fig.poly(pts, fill)


def corp_glyph(fig, x, y, s, fill="hazard", back="gunmetaldark"):
    """The raiders' corporate mark: a round-cornered square holding a split
    ring and a bar. Invented; it stands for no real company."""
    with fig.frame(x, y, s=s):
        fig.rect(-1, -1, 1, 1, back, r=0.3)
        fig.poly(arc(0, 0, 0.7, 30, 330, 16) + list(reversed(arc(0, 0, 0.38, 30, 330, 16))), fill)
        fig.rect(0.05, -0.12, 0.8, 0.12, fill)


def eye(fig, x, y, r, fill="weakspot", pupil="ink"):
    """A glowing weak-spot eye: a bright flat disc and a dark slit pupil, with
    a small flat highlight. No glow halo."""
    fig.circle(x, y, r, fill)
    fig.ellipse(x + r * 0.15, y, r * 0.28, r * 0.62, pupil)


def enemy_shot(fig, x, y, r):
    """The single enemy projectile shape: a bright magenta core inside a thick
    dark ring, inside a thin pale rim, with a flat pale highlight. Light, dark,
    light from the edge in: it reads without colour on any ground."""
    fig.circle(x, y, r, "shothi")
    fig.circle(x, y, r * 0.76, "ink")
    fig.circle(x, y, r * 0.54, "shot")
    fig.circle(x - r * 0.16, y + r * 0.16, r * 0.16, "shothi")


def sky_bands(fig, splits=(0.35, 0.7)):
    """Three flat night-sky bands, no outline: skylow, sky, skyhi upward."""
    W, H = fig.size
    fig.band(0, H, "sky")
    fig.band(H * splits[1], H, "skyhi")
    fig.band(0, H * splits[0], "skylow")


def calm_left_third(fig, top=0.5, roles=("skyhi", "sky", "skylow", "ink"), min_share=0.95):
    """Mark the upper part of the left third as calm so the title can sit on it."""
    W, H = fig.size
    fig.keep_calm((0, H * (1 - top), W / 3, H), roles, min_share)
