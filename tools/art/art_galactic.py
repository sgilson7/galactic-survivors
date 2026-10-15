"""Galactic Survivors pictures, registered for tools/art/art.py.

Every picture names colour roles from styles/galactic (data/art.json), never
colour values. Sprites face right and are drawn in a unit frame: x and y run
from -1 to 1 across the picture, with a margin left clear at every edge.
"""
import math
from contextlib import contextmanager

from artlib import Fig
import style

M = style.load().motifs()
arc, tube, shadow, eye = M.arc, M.tube, M.shadow, M.eye

PICTURES = {}


def pic(name):
    def reg(fn):
        PICTURES[name] = fn
        return fn
    return reg


@contextmanager
def unit(fig, pad=0.88):
    """A frame where -1..1 spans the picture, shrunk by pad to keep a margin."""
    W, H = fig.size
    with fig.frame(W / 2, H / 2, s=W / 2 * pad) as f:
        yield f


def pill(fig, x0, y0, x1, y1, fill):
    """A rounded rectangle whose ends are half circles."""
    r = min(x1 - x0, y1 - y0) / 2
    fig.rect(x0, y0, x1, y1, fill, r=r * 0.999)


def ring(fig, x, y, r0, r1, a0, a1, fill, n=20):
    """An arc band between radii r0 and r1 from angle a0 to a1."""
    fig.poly(arc(x, y, r1, a0, a1, n) + list(reversed(arc(x, y, r0, a0, a1, n))), fill)


def segment(fig, x, y, r, a0, a1, fill, n=20):
    """The part of a circle cut off by the chord from angle a0 to a1."""
    fig.poly(arc(x, y, r, a0, a1, n), fill)


# ---------------------------------------------------------------- players

def leg(fig, x, top, foot_x, foot_y, fill, w=0.17, boot="gunmetal"):
    tube(fig, [(x, top), (foot_x, foot_y + 0.06)], fill, w, smooth=False)
    fig.ellipse(foot_x + 0.04, foot_y, w * 0.85, w * 0.5, boot)


GROUND = -0.8
WALK_FRAMES = 4


def gait(phase, feet, spread, lift=0.09, bob=0.035):
    """Body lift and the (far, near) foot points for one walk frame.

    phase None is the idle stance: feet where the idle picture has them.
    Frames 0 and 2 are contact poses: feet apart, near foot forward in 0 and
    back in 2. Frames 1 and 3 are passing poses: one foot planted under the
    body, the other lifted beside it, and the body raised by bob.
    """
    if phase is None:
        return 0.0, [(feet[0], GROUND), (feet[1], GROUND)]
    if phase == 0:
        return 0.0, [(-spread, GROUND), (spread, GROUND)]
    if phase == 2:
        return 0.0, [(spread, GROUND), (-spread, GROUND)]
    if phase == 1:
        return bob, [(-0.04, GROUND + lift), (0.04, GROUND)]
    return bob, [(-0.04, GROUND), (0.04, GROUND + lift)]


def legs(fig, phase, hips, top, feet, spread, w=0.17):
    """The far leg (shaded) then the near leg, from hips that ride with the
    body down to feet that follow the gait; returns the body lift."""
    lift, pts = gait(phase, feet, spread)
    if phase is not None:
        # walking, seen from the side: both legs swing from near one hip
        mid = sum(hips) / 2
        hips = [mid + (h - mid) * 0.25 for h in hips]
    for hip, (fx, fy), fill in zip(hips, pts, ("suitshade", "suit")):
        leg(fig, hip, top + lift, fx, fy, fill, w=w)
    return lift


def draw_pathfinder(fig, phase=None):
    shadow(fig, 0, -0.86, 0.56)
    lift, _ = gait(phase, (-0.2, 0.2), 0.3)
    with fig.frame(0, lift):
        # backpack
        fig.rect(-0.66, -0.32, -0.3, 0.22, "suitshade", r=0.12)
        fig.rect(-0.62, -0.1, -0.4, 0.0, "amber", r=0.03)
    legs(fig, phase, (-0.16, 0.14), -0.4, (-0.2, 0.2), 0.3)
    with fig.frame(0, lift):
        # body
        fig.circle(0, 0, 0.52, "suit")
        segment(fig, 0, 0, 0.52, 205, 335, "amber")
        fig.ellipse(-0.22, 0.24, 0.1, 0.06, "suitshade", rot=35)
        # visor
        fig.rect(0.02, 0.0, 0.56, 0.3, "visor", r=0.14)
        fig.rect(0.1, 0.17, 0.28, 0.24, "visorhi", r=0.035)
        # shoulder drone on a mount
        tube(fig, [(-0.28, 0.36), (-0.36, 0.56)], "gunmetal", 0.07, smooth=False)
        fig.rect(-0.25, 0.6, 0.06, 0.7, "gunmetal", r=0.03)
        fig.rect(-0.66, 0.48, -0.18, 0.8, "amber", r=0.14)
        fig.rect(-0.6, 0.48, -0.24, 0.56, "suit", r=0.03)
        fig.circle(-0.3, 0.68, 0.06, "visor")
        # arm and carbine
        tube(fig, [(0.02, -0.14), (0.36, -0.24)], "suit", 0.17, smooth=False)
        fig.rect(0.26, -0.36, 0.84, -0.18, "gunmetal", r=0.06)
        fig.rect(0.7, -0.33, 0.84, -0.21, "amber", r=0.03)
        fig.circle(0.34, -0.26, 0.09, "suit")


def draw_breaker(fig, phase=None):
    shadow(fig, 0, -0.86, 0.66)
    lift = legs(fig, phase, (-0.2, 0.18), -0.4, (-0.24, 0.22), 0.32, w=0.22)
    with fig.frame(0, lift):
        # back shoulder plate
        fig.ellipse(-0.42, 0.2, 0.3, 0.26, "teal")
        # body: very round
        fig.circle(0, -0.02, 0.6, "suit")
        segment(fig, 0, -0.02, 0.6, 210, 330, "suitshade")
        # visor slit
        fig.rect(0.12, 0.12, 0.6, 0.3, "visor", r=0.09)
        fig.rect(0.2, 0.2, 0.34, 0.25, "visorhi", r=0.02)
        # big front shoulder dome
        fig.ellipse(0.02, 0.32, 0.4, 0.27, "teal")
        ring(fig, 0.02, 0.32, 0.4 * 0.0 + 0.24, 0.4, 200, 340, "faunateal")
        fig.ellipse(-0.08, 0.42, 0.12, 0.06, "suit", rot=15)
        # arm and energy-arc gauntlet
        tube(fig, [(0.08, -0.12), (0.4, -0.22)], "suitshade", 0.2, smooth=False)
        zig = [(0.62, -0.02), (0.74, 0.14), (0.66, 0.24), (0.82, 0.42)]
        tube(fig, zig, "teal", 0.07, smooth=False)
        zig2 = [(0.78, -0.24), (0.9, -0.1), (0.82, 0.0), (0.94, 0.12)]
        tube(fig, zig2, "teal", 0.06, smooth=False)
        fig.circle(0.56, -0.24, 0.27, "teal")
        fig.rect(0.56, -0.38, 0.84, -0.1, "suit", r=0.1)
        fig.circle(0.5, -0.17, 0.08, "visorhi")


def draw_vector(fig, phase=None):
    shadow(fig, 0, -0.86, 0.46)
    lift, _ = gait(phase, (-0.2, 0.2), 0.28)
    with fig.frame(0, lift):
        # back thrusters with flat flame shapes
        for dy in (0.18, -0.12):
            fig.poly([(-0.6, dy + 0.05), (-0.92, dy - 0.02), (-0.6, dy - 0.1)], "coral")
            fig.rect(-0.62, dy - 0.13, -0.3, dy + 0.08, "suitshade", r=0.1)
    legs(fig, phase, (-0.1, 0.1), -0.36, (-0.2, 0.2), 0.28, w=0.13)
    with fig.frame(0, lift):
        # slim round body
        fig.ellipse(0, 0.02, 0.38, 0.52, "suit")
        # crest fin
        fig.poly([(-0.26, 0.42), (-0.02, 0.54), (-0.5, 0.74)], "coral")
        # coral collar band
        segment(fig, 0, 0.02, 0.38, 215, 325, "coral")
        # visor
        fig.rect(0.0, 0.08, 0.42, 0.3, "visor", r=0.11)
        fig.rect(0.08, 0.19, 0.22, 0.24, "visorhi", r=0.025)
        # rail-lance along the arm
        tube(fig, [(0.0, -0.12), (0.26, -0.22)], "suit", 0.14, smooth=False)
        fig.rect(0.06, -0.3, 0.98, -0.18, "gunmetal", r=0.06)
        fig.rect(0.3, -0.27, 0.9, -0.21, "coral", r=0.02)
        fig.poly([(0.98, -0.32), (0.98, -0.16), (0.9, -0.24)], "suit")
        fig.circle(0.24, -0.24, 0.08, "suit")


def player(draw, phase=None):
    def mk(rng):
        fig = Fig("sprite")
        with unit(fig):
            draw(fig, phase)
        return fig
    return mk


# The idle picture players/<id>, and walk frames players/<id>_walk0..3 at
# the same size and framing, so the page can swap a frame in place.
for _n, _f in (("pathfinder", draw_pathfinder), ("breaker", draw_breaker), ("vector", draw_vector)):
    PICTURES[f"players/{_n}"] = player(_f)
    for _i in range(WALK_FRAMES):
        PICTURES[f"players/{_n}_walk{_i}"] = player(_f, _i)


# ------------------------------------------------------------ alien fauna

def draw_skitter(fig):
    shadow(fig, 0, -0.68, 0.7)
    # far legs: knees poke above the body
    for x, dx in ((-0.36, -0.42), (-0.04, -0.06), (0.26, 0.34)):
        tube(fig, [(x, -0.06), (x + dx * 0.5, 0.38), (x + dx, -0.56)], "violetdark", 0.07, smooth=False)
    # body: low round carapace
    fig.ellipse(-0.08, -0.08, 0.56, 0.36, "violet")
    ring(fig, -0.08, -0.08, 0.2, 0.36, 50, 130, "violethi", n=12)
    fig.stroke([(-0.32, 0.2), (-0.38, -0.06)], width=2.2)
    fig.stroke([(0.16, 0.2), (0.22, -0.06)], width=2.2)
    # near legs: knees out to the sides, feet down
    for x, dx in ((-0.38, -0.46), (-0.06, -0.08), (0.24, 0.4)):
        tube(fig, [(x, -0.26), (x + dx * 0.6, -0.2), (x + dx, -0.66)], "violet", 0.08, smooth=False)
    # head with mandibles
    fig.poly([(0.6, -0.26), (0.96, -0.38), (0.7, -0.42)], "flesh")
    fig.poly([(0.66, -0.08), (0.96, -0.04), (0.74, -0.2)], "flesh")
    fig.circle(0.5, -0.14, 0.24, "violetdark")
    eye(fig, 0.58, -0.12, 0.12)


def draw_mote(fig):
    shadow(fig, 0, -0.86, 0.36)
    # tendrils
    for i, x in enumerate((-0.32, -0.1, 0.12, 0.32)):
        tube(fig, [(x, -0.1), (x - 0.08, -0.32), (x + 0.04, -0.5), (x - 0.04, -0.66 + 0.06 * (i % 2))],
             "faunateal", 0.07)
    # bell
    fig.blob([(-0.62, -0.12), (-0.5, 0.3), (0, 0.58), (0.5, 0.3), (0.62, -0.12), (0.3, -0.2), (0, -0.14), (-0.3, -0.2)],
             "faunatealhi", tension=0.6)
    fig.blob([(-0.32, 0.22), (0, 0.42), (0.3, 0.22), (0, 0.12)], "faunateal", tension=0.7)
    fig.ellipse(-0.28, 0.32, 0.1, 0.05, "suit", rot=40)
    eye(fig, 0.22, 0.02, 0.13)


def draw_carapace(fig):
    shadow(fig, 0, -0.74, 0.86)
    for x in (-0.55, 0.0, 0.4):
        tube(fig, [(x, -0.3), (x + 0.02, -0.68)], "olive", 0.16, smooth=False)
        fig.ellipse(x + 0.04, -0.7, 0.13, 0.07, "ink")
    # dome shell with segment plates
    fig.blob([(-0.92, -0.36), (-0.82, 0.18), (-0.36, 0.6), (0.2, 0.62), (0.66, 0.26), (0.82, -0.3), (0, -0.46)],
             "olive", tension=0.55)
    fig.blob([(-0.62, 0.1), (-0.3, 0.44), (0.16, 0.46), (0.46, 0.18), (0, 0.06), (-0.36, 0.02)], "olivehi", tension=0.6)
    fig.stroke([(-0.12, 0.46), (-0.2, 0.06)], width=2.4)
    fig.stroke([(0.2, 0.42), (0.16, 0.06)], width=2.4)
    fig.blob([(-0.88, -0.34), (0, -0.46), (0.82, -0.3), (0.76, -0.18), (0, -0.3), (-0.84, -0.2)], "olive", tension=0.4)
    fig.ellipse(-0.42, 0.34, 0.12, 0.06, "suit", rot=35)
    for x, y, rot in ((-0.5, 0.42, 30), (-0.1, 0.6, 0), (0.32, 0.5, -30)):
        with fig.frame(x, y, rot=rot):
            fig.poly([(-0.08, -0.04), (0.0, 0.2), (0.08, -0.04)], "olivehi")
    # head poking out under the shell rim
    fig.ellipse(0.7, -0.2, 0.3, 0.26, "flesh")
    fig.poly([(0.88, -0.3), (1.0, -0.44), (0.8, -0.42)], "suit")
    eye(fig, 0.78, -0.12, 0.1)


def draw_spitter(fig):
    shadow(fig, 0, -0.74, 0.7)
    # stumpy legs
    for x in (-0.4, 0.3):
        fig.ellipse(x, -0.62, 0.18, 0.12, "violetdark")
    # squat body
    fig.blob([(-0.72, -0.5), (-0.7, 0.0), (-0.3, 0.26), (0.3, 0.22), (0.7, -0.08), (0.6, -0.52), (0, -0.62)],
             "olive", tension=0.6)
    fig.blob([(-0.5, -0.5), (0, -0.56), (0.5, -0.46), (0.3, -0.36), (-0.3, -0.4)], "olivehi", tension=0.6)
    # acid sac on its back
    fig.circle(-0.28, 0.32, 0.36, "acid")
    fig.ellipse(-0.4, 0.46, 0.11, 0.07, "suit", rot=30)
    fig.stroke(arc(-0.28, 0.32, 0.22, 200, 300, 10), width=2.4)
    # spout pointing up and right
    tube(fig, [(0.36, 0.06), (0.62, 0.3), (0.74, 0.5)], "flesh", 0.2)
    fig.ellipse(0.75, 0.52, 0.14, 0.09, "violetdark", rot=60)
    fig.circle(0.76, 0.6, 0.06, "acid")
    eye(fig, 0.4, -0.16, 0.1)
    eye(fig, 0.16, -0.08, 0.07)


def draw_stalker(fig):
    shadow(fig, 0, -0.86, 0.78)
    # tail
    tube(fig, [(-0.3, -0.2), (-0.7, -0.3), (-0.86, -0.02), (-0.78, 0.26)], "violetdark", 0.16)
    fig.poly([(-0.86, 0.22), (-0.72, 0.3), (-0.82, 0.48)], "faunatealhi")
    # back legs
    tube(fig, [(-0.24, -0.3), (-0.44, -0.56), (-0.3, -0.82)], "violetdark", 0.17, smooth=False)
    # body: hunched round torso with plates
    fig.ellipse(-0.06, -0.06, 0.48, 0.44, "violet")
    for (x, y, r) in ((-0.24, 0.16, 0.28), (0.0, 0.26, 0.26), (0.22, 0.2, 0.22)):
        fig.circle(x, y, r, "faunateal")
    fig.ellipse(-0.18, 0.3, 0.12, 0.06, "faunatealhi", rot=20)
    # crest spikes
    fig.poly([(-0.36, 0.34), (-0.56, 0.7), (-0.18, 0.44)], "faunatealhi")
    fig.poly([(-0.08, 0.46), (-0.16, 0.86), (0.1, 0.48)], "faunatealhi")
    tube(fig, [(0.06, -0.3), (0.16, -0.56), (0.06, -0.82)], "violet", 0.18, smooth=False)
    # head
    fig.blob([(0.24, 0.26), (0.54, 0.4), (0.9, 0.2), (0.82, 0.0), (0.4, 0.02)], "violetdark", tension=0.55)
    fig.poly([(0.6, 0.06), (0.92, 0.04), (0.7, -0.08)], "suit")
    eye(fig, 0.6, 0.24, 0.08)
    eye(fig, 0.42, 0.22, 0.06)
    # claw arm
    tube(fig, [(0.2, -0.04), (0.46, -0.16), (0.62, -0.06)], "violet", 0.15, smooth=False)
    fig.poly([(0.6, -0.02), (0.94, 0.0), (0.66, -0.14)], "faunatealhi")
    fig.poly([(0.6, -0.12), (0.88, -0.28), (0.58, -0.22)], "faunatealhi")
    # weak spot in the chest
    fig.circle(0.08, -0.16, 0.1, "weakspot")


for _n, _f in (("skitter", draw_skitter), ("mote", draw_mote), ("carapace", draw_carapace),
               ("spitter", draw_spitter), ("stalker", draw_stalker)):
    def _mk(rng, f=_f):
        fig = Fig("sprite")
        with unit(fig):
            f(fig)
        return fig
    PICTURES[f"enemies/{_n}"] = _mk


# ---------------------------------------------------------- corporate raid

def draw_corp_drone(fig):
    shadow(fig, 0, -0.86, 0.4)
    # side rotor pods
    for x in (-0.66, 0.66):
        fig.rect(x - 0.2, 0.06, x + 0.2, 0.18, "gunmetaldark", r=0.05)
        fig.ellipse(x, 0.28, 0.3, 0.07, "gunmetal")
    # round hull
    fig.circle(0, 0, 0.48, "offwhite")
    segment(fig, 0, 0, 0.48, 200, 340, "gunmetal")
    # hazard band
    ring(fig, 0, 0, 0.3, 0.48, 100, 150, "hazard", n=8)
    # sensor eye
    fig.circle(0.22, 0.06, 0.18, "gunmetaldark")
    fig.circle(0.26, 0.06, 0.09, "sensor")
    # underslung gun
    fig.rect(0.0, -0.5, 0.56, -0.38, "gunmetaldark", r=0.04)
    # chevrons decal
    M.chevrons(fig, -0.2, -0.18, 0.08, "hazard")


def draw_corp_trooper(fig):
    shadow(fig, 0, -0.86, 0.56)
    # reverse-joint legs
    for x, fill in ((-0.18, "gunmetaldark"), (0.14, "gunmetal")):
        tube(fig, [(x, -0.3), (x - 0.12, -0.56), (x + 0.04, -0.8)], fill, 0.16, smooth=False)
        fig.rect(x - 0.12, -0.86, x + 0.2, -0.76, "gunmetaldark", r=0.04)
    # backpack with antenna
    fig.rect(-0.62, -0.2, -0.32, 0.36, "gunmetal", r=0.1)
    fig.stroke([(-0.54, 0.36), (-0.6, 0.66)], width=3.2)
    fig.circle(-0.6, 0.68, 0.04, "sensor")
    # torso: round-cornered armour
    fig.rect(-0.4, -0.36, 0.34, 0.34, "offwhite", r=0.2)
    fig.rect(-0.4, -0.36, 0.34, -0.12, "gunmetal", r=0.12)
    M.stripes(fig, -0.34, 0.0, -0.04, 0.12, 3, "hazard", "ink", 0.6)
    # head: helmet with single sensor eye
    fig.rect(-0.16, 0.24, 0.32, 0.62, "offwhite", r=0.16)
    fig.rect(0.06, 0.34, 0.36, 0.5, "gunmetaldark", r=0.07)
    fig.circle(0.24, 0.42, 0.06, "sensor")
    # hazard shoulder
    fig.ellipse(0.02, 0.18, 0.24, 0.18, "hazard")
    M.corp_glyph(fig, -0.12, -0.2, 0.08)
    # rifle
    tube(fig, [(0.0, 0.0), (0.24, -0.1)], "gunmetal", 0.14, smooth=False)
    fig.rect(0.0, -0.22, 0.94, -0.08, "gunmetaldark", r=0.04)
    fig.rect(0.12, -0.34, 0.26, -0.18, "gunmetaldark", r=0.03)
    fig.rect(0.48, -0.06, 0.66, 0.02, "gunmetal", r=0.02)
    fig.circle(0.22, -0.14, 0.08, "offwhite")


for _n, _f in (("corp_drone", draw_corp_drone), ("corp_trooper", draw_corp_trooper)):
    def _mk(rng, f=_f):
        fig = Fig("sprite")
        with unit(fig):
            f(fig)
        return fig
    PICTURES[f"enemies/{_n}"] = _mk


# ---------------------------------------------------------------- bosses

def draw_burrower(fig):
    # broken ground around the hole
    fig.blob([(-0.94, -0.7), (-0.7, -0.5), (-0.3, -0.42), (0.3, -0.44), (0.76, -0.5), (0.96, -0.72), (0.5, -0.9), (-0.5, -0.9)],
             "rockdark", tension=0.5)
    fig.blob([(-0.6, -0.66), (-0.3, -0.52), (0.3, -0.52), (0.6, -0.66), (0.2, -0.76), (-0.3, -0.76)], "shadow", tension=0.6)
    # segmented body curving up and right
    segs = [(-0.1, -0.56, 0.46), (-0.16, -0.3, 0.46), (-0.12, -0.04, 0.46), (0.0, 0.2, 0.46)]
    for x, y, r in segs:
        fig.ellipse(x, y, r, r * 0.62, "violet")
        fig.ellipse(x - 0.06, y + 0.08, r * 0.6, r * 0.22, "violethi")
    # head: wide armoured cowl with a round maw facing right
    fig.blob([(-0.42, 0.26), (-0.36, 0.7), (0.1, 0.9), (0.6, 0.78), (0.86, 0.44), (0.7, 0.08), (0.2, 0.02)],
             "violetdark", tension=0.55)
    fig.blob([(-0.3, 0.46), (-0.16, 0.76), (0.16, 0.84), (0.1, 0.58)], "faunateal", tension=0.6)
    # maw
    fig.ellipse(0.58, 0.4, 0.26, 0.3, "ink")
    fig.ellipse(0.6, 0.4, 0.18, 0.21, "flesh")
    for a in range(0, 360, 45):
        r = math.radians(a)
        cx, cy = 0.58 + 0.26 * math.cos(r), 0.4 + 0.3 * math.sin(r)
        tx, ty = 0.58 + 0.12 * math.cos(r), 0.4 + 0.14 * math.sin(r)
        nx, ny = -math.sin(r) * 0.06, math.cos(r) * 0.06
        fig.poly([(cx + nx, cy + ny), (cx - nx, cy - ny), (tx, ty)], "suit")
    fig.circle(0.6, 0.4, 0.06, "weakspot")
    # eyes along the cowl
    eye(fig, 0.2, 0.58, 0.07)
    eye(fig, 0.34, 0.72, 0.05)
    eye(fig, 0.12, 0.4, 0.05)
    # rocks thrown up at the rim
    for x, y, r in ((-0.78, -0.6, 0.16), (-0.5, -0.84, 0.1), (0.6, -0.8, 0.14), (0.8, -0.52, 0.1), (0.2, -0.86, 0.07)):
        fig.blob([(x - r, y - r * 0.5), (x - r * 0.6, y + r * 0.7), (x + r * 0.7, y + r * 0.6), (x + r, y - r * 0.4)],
                 "rock", tension=0.3)


def draw_strider(fig):
    shadow(fig, 0, -0.88, 0.7)
    # legs: reverse-joint, round-cornered plates
    for x, fill, d in ((-0.26, "gunmetaldark", -0.04), (0.18, "gunmetal", 0.04)):
        tube(fig, [(x, -0.06), (x - 0.22, -0.4), (x + 0.04, -0.82)], fill, 0.14, smooth=False)
        fig.rect(x - 0.32, -0.36, x - 0.06, -0.18, "offwhite", r=0.08)
        fig.rect(x - 0.18, -0.92, x + 0.26, -0.8, "gunmetaldark", r=0.04)
    # back missile pod
    fig.rect(-0.7, 0.24, -0.3, 0.6, "offwhite", r=0.1)
    for i in range(3):
        fig.circle(-0.6 + i * 0.12, 0.48, 0.04, "ink")
    M.stripes(fig, -0.66, 0.28, -0.34, 0.36, 3, "hazard", "ink", 0.6)
    # hip
    fig.rect(-0.36, -0.14, 0.3, 0.06, "gunmetal", r=0.08)
    # core: round-cornered armour
    fig.rect(-0.44, -0.02, 0.4, 0.5, "hazard", r=0.2)
    fig.rect(-0.44, -0.02, 0.4, 0.12, "gunmetal", r=0.06)
    fig.rect(-0.26, 0.26, 0.2, 0.36, "ink", r=0.03)
    M.chevrons(fig, -0.2, 0.31, 0.06, "offwhite", n=3)
    M.corp_glyph(fig, 0.24, 0.3, 0.09, "hazard", "ink")
    # head with sensor cluster
    fig.rect(0.08, 0.46, 0.5, 0.7, "offwhite", r=0.1)
    fig.rect(0.3, 0.52, 0.56, 0.64, "gunmetaldark", r=0.05)
    fig.circle(0.46, 0.58, 0.04, "sensor")
    fig.circle(0.36, 0.58, 0.03, "sensor")
    # shoulder and arm cannon
    fig.ellipse(0.24, 0.32, 0.18, 0.16, "offwhite")
    tube(fig, [(0.26, 0.22), (0.4, 0.0)], "gunmetal", 0.1, smooth=False)
    fig.rect(0.26, -0.08, 0.96, 0.06, "gunmetaldark", r=0.05)
    fig.rect(0.3, -0.12, 0.56, 0.1, "offwhite", r=0.05)
    fig.rect(0.86, -0.06, 0.96, 0.04, "sensor", r=0.02)


def draw_matriarch(fig):
    shadow(fig, -0.05, -0.8, 0.98, 0.2)
    # egg-sac abdomen behind
    fig.blob([(-0.98, -0.4), (-0.94, 0.1), (-0.6, 0.34), (-0.2, 0.16), (-0.2, -0.56), (-0.62, -0.74)], "flesh", tension=0.6)
    for i in range(3):
        fig.stroke(arc(-0.58, -0.2, 0.22 + i * 0.13, 115, 235, 10), width=3.0)
    for x, y in ((-0.72, -0.02), (-0.48, 0.14), (-0.56, -0.44), (-0.84, -0.32)):
        fig.circle(x, y, 0.07, "weakspot")
    # great fanned crest behind the head
    fig.blob([(-0.36, 0.1), (-0.5, 0.56), (-0.2, 0.92), (0.26, 0.96), (0.56, 0.66), (0.4, 0.2)], "faunateal", tension=0.55)
    for a in (110, 80, 50):
        r = math.radians(a)
        fig.stroke([(0.06 + 0.2 * math.cos(r), 0.28 + 0.2 * math.sin(r)),
                    (0.06 + 0.62 * math.cos(r), 0.28 + 0.62 * math.sin(r))], width=3.0)
    fig.poly([(-0.42, 0.66), (-0.62, 0.86), (-0.32, 0.8)], "faunatealhi")
    fig.poly([(0.0, 0.94), (0.06, 1.0), (0.16, 0.95)], "faunatealhi")
    # massive crouched body
    fig.blob([(-0.36, -0.6), (-0.42, -0.04), (-0.12, 0.34), (0.42, 0.36), (0.72, 0.0), (0.66, -0.6), (0.14, -0.72)],
             "violet", tension=0.6)
    fig.blob([(-0.2, 0.14), (0.0, 0.3), (0.3, 0.3), (0.1, 0.16)], "violethi", tension=0.6)
    # chest weak spot
    fig.circle(0.12, -0.2, 0.16, "faunateal")
    fig.circle(0.12, -0.2, 0.09, "weakspot")
    # claws on the ground
    for x in (-0.24, 0.3, 0.58):
        fig.poly([(x - 0.12, -0.6), (x + 0.12, -0.6), (x + 0.2, -0.82), (x + 0.04, -0.76), (x - 0.06, -0.84)], "violetdark")
    # face at the front with many eyes and mandibles
    fig.poly([(0.66, 0.06), (0.98, -0.1), (0.72, -0.08)], "suit")
    fig.poly([(0.7, 0.3), (0.98, 0.34), (0.74, 0.2)], "suit")
    fig.blob([(0.36, 0.12), (0.44, 0.46), (0.72, 0.52), (0.88, 0.26), (0.72, 0.02), (0.5, -0.02)], "violetdark", tension=0.6)
    eye(fig, 0.7, 0.26, 0.08)
    eye(fig, 0.54, 0.34, 0.06)
    eye(fig, 0.6, 0.12, 0.05)
    # tendrils curling forward on the ground
    tube(fig, [(0.5, -0.4), (0.8, -0.44), (0.94, -0.6), (0.82, -0.72)], "faunateal", 0.08)
    tube(fig, [(-0.3, -0.5), (-0.56, -0.7), (-0.82, -0.68)], "faunateal", 0.08)


for _n, _f in (("burrower", draw_burrower), ("strider", draw_strider), ("matriarch", draw_matriarch)):
    def _mk(rng, f=_f):
        fig = Fig("boss")
        with unit(fig, pad=0.9):
            f(fig)
        return fig
    PICTURES[f"bosses/{_n}"] = _mk


# ------------------------------------------------------ pickups and effects

def small(draw):
    def mk(rng):
        fig = Fig("small")
        with unit(fig, pad=0.84):
            draw(fig)
        return fig
    return mk


@pic("fx/enemy_shot")
@small
def enemy_shot(fig):
    M.enemy_shot(fig, 0, 0, 0.82)


@pic("fx/xp_shard")
@small
def xp_shard(fig):
    fig.poly([(0, 0.92), (0.5, 0.1), (0, -0.92), (-0.5, 0.1)], "xp")
    fig.poly([(0, 0.92), (0.0, 0.1), (-0.5, 0.1)], "xphi")


@pic("fx/gold_chip")
@small
def gold_chip(fig):
    fig.poly(arc(0, 0, 0.9, 30, 390, 6)[:-1], "gold")
    fig.poly(arc(0, 0, 0.5, 30, 390, 6)[:-1], "goldhi")


@pic("fx/health_cell")
@small
def health_cell(fig):
    pill(fig, -0.9, -0.62, 0.9, 0.62, "suit")
    fig.poly([(-0.14, 0.42), (0.14, 0.42), (0.14, 0.14), (0.42, 0.14), (0.42, -0.14), (0.14, -0.14),
              (0.14, -0.42), (-0.14, -0.42), (-0.14, -0.14), (-0.42, -0.14), (-0.42, 0.14), (-0.14, 0.14)], "heal")


@pic("fx/ticket")
@small
def ticket(fig):
    fig.poly([(-0.9, -0.55), (0.9, -0.55), (0.9, -0.15), (0.72, 0), (0.9, 0.15), (0.9, 0.55), (-0.9, 0.55),
              (-0.9, 0.15), (-0.72, 0), (-0.9, -0.15)], "ticket")
    fig.rect(-0.45, -0.25, 0.45, 0.25, "suit", r=0.06)


@pic("fx/cache")
def cache(rng):
    fig = Fig("small")
    with unit(fig, pad=0.84):
        fig.rect(-0.9, -0.75, 0.9, 0.45, "offwhite", r=0.14)
        fig.rect(-0.9, 0.2, 0.9, 0.75, "gunmetal", r=0.14)
        fig.rect(-0.2, -0.2, 0.2, 0.36, "hazard", r=0.05)
        fig.rect(-0.9, -0.75, -0.55, 0.45, "hazard", r=0.1)
        fig.rect(0.55, -0.75, 0.9, 0.45, "hazard", r=0.1)
    return fig


@pic("fx/rift")
def rift(rng):
    fig = Fig("sprite")
    with unit(fig, pad=0.9):
        fig.ellipse(0, -0.1, 0.92, 0.8, "rift")
        for i in range(5):
            a0 = i * 72
            pts = [(math.cos(math.radians(a0 + t * 1.6)) * (0.86 - t * 0.0085),
                    -0.1 + math.sin(math.radians(a0 + t * 1.6)) * (0.74 - t * 0.0075)) for t in range(0, 85, 6)]
            M.tube(fig, pts, "rifthi", 0.05)
        fig.ellipse(0, -0.1, 0.24, 0.2, "ink")
        fig.ellipse(-0.06, -0.06, 0.08, 0.05, "rifthi")
    return fig


# ---------------------------------------------------------------- ground

def wrap(fig, x, y, draw, W):
    """Draw a thing at (x, y) and its copies a tile away, so the tile is seamless."""
    for dx in (-W, 0, W):
        for dy in (-W, 0, W):
            if -1.2 < x + dx < W + 1.2 and -1.2 < y + dy < W + 1.2:
                with fig.frame(x + dx, y + dy):
                    draw()


@pic("ground/regolith")
def regolith(rng):
    fig = Fig("tile")
    W, H = fig.size
    fig.band(0, H, "ground")
    for _ in range(9):
        x, y, rx, ry = rng.uniform(0, W), rng.uniform(0, H), rng.uniform(0.7, 1.3), rng.uniform(0.35, 0.6)
        pts = [(math.cos(math.radians(a)) * rx * rng.uniform(0.8, 1.1), math.sin(math.radians(a)) * ry * rng.uniform(0.8, 1.1))
               for a in range(0, 360, 45)]
        fill = rng.choice(["groundB", "groundC"])
        wrap(fig, x, y, lambda pts=pts, fill=fill: fig.blob(pts, fill, tension=0.6), W)
    for _ in range(3):
        x, y, r = rng.uniform(0, W), rng.uniform(0, H), rng.uniform(0.35, 0.55)
        def crater(r=r):
            fig.ellipse(0, 0, r, r * 0.6, "groundC")
            fig.ellipse(0.04, 0.04, r * 0.7, r * 0.4, "shadow")
        wrap(fig, x, y, crater, W)
    for _ in range(26):
        x, y, r = rng.uniform(0, W), rng.uniform(0, H), rng.uniform(0.04, 0.11)
        wrap(fig, x, y, lambda r=r: fig.ellipse(0, 0, r, r * 0.7, "pebble"), W)
    return fig


@pic("ground/rock_pillar")
def rock_pillar(rng):
    fig = Fig("sprite")
    with unit(fig):
        shadow(fig, 0.04, -0.82, 0.62)
        fig.blob([(-0.5, -0.8), (-0.56, -0.2), (-0.42, 0.5), (-0.1, 0.66), (0.38, 0.56), (0.52, -0.1), (0.54, -0.8), (0, -0.9)],
                 "rock", tension=0.35)
        fig.blob([(-0.42, 0.5), (-0.1, 0.72), (0.38, 0.58), (0.06, 0.4)], "rockhi", tension=0.5)
        fig.blob([(0.2, 0.36), (0.5, 0.2), (0.54, -0.8), (0.26, -0.86), (0.3, -0.2)], "rockdark", tension=0.35)
        fig.stroke([(-0.28, 0.24), (-0.12, -0.1), (-0.24, -0.46)], width=2.2)
        # crystal cluster
        for x, h, w in ((-0.62, 0.42, 0.13), (-0.48, 0.3, 0.1), (-0.76, 0.22, 0.09)):
            fig.poly([(x - w, -0.78), (x - w * 0.6, -0.78 + h), (x, -0.78 + h + w), (x + w * 0.6, -0.78 + h), (x + w, -0.78)], "crystal")
    return fig


@pic("ground/wreck")
def wreck(rng):
    fig = Fig("sprite")
    with unit(fig):
        shadow(fig, 0, -0.72, 0.9)
        # hull chunk, tilted into the ground
        with fig.frame(0, -0.1, rot=-14):
            fig.blob([(-0.86, -0.4), (-0.8, 0.22), (-0.4, 0.5), (0.4, 0.5), (0.74, 0.3), (0.8, -0.44)], "suitshade", tension=0.4)
            fig.rect(-0.7, -0.1, 0.62, 0.06, "amber", r=0.03)
            fig.circle(0.1, 0.28, 0.13, "gunmetaldark")
            fig.circle(0.1, 0.28, 0.07, "visor")
            fig.circle(-0.38, 0.28, 0.1, "gunmetaldark")
            # torn edge
            fig.poly([(0.68, 0.34), (0.98, 0.2), (0.82, 0.04), (0.96, -0.14), (0.8, -0.4), (0.7, -0.38)], "gunmetal")
        fig.poly([(-0.9, -0.62), (0.9, -0.62), (0.74, -0.74), (-0.74, -0.76)], "rockdark")
        # landing strut
        tube(fig, [(-0.6, -0.2), (-0.86, 0.36)], "gunmetal", 0.07, smooth=False)
        fig.rect(-0.96, 0.32, -0.74, 0.42, "gunmetaldark", r=0.03)
        for x, y, r in ((0.6, -0.72, 0.08), (-0.4, -0.78, 0.06), (0.2, -0.82, 0.05)):
            fig.rect(x - r, y - r * 0.6, x + r, y + r * 0.6, "gunmetal", r=0.02)
    return fig


# ------------------------------------------------------------------ icons
# One subject per icon on a round badge. The badge rim tells the kind:
# abilities a plain rim, passives a teal rim, evolutions a gold rim.

def zigzag(x0, y0, x1, y1, n=4, amp=0.12):
    pts = []
    for i in range(n + 1):
        t = i / n
        dx, dy = x1 - x0, y1 - y0
        L = math.hypot(dx, dy)
        off = 0 if i in (0, n) else (amp if i % 2 else -amp)
        pts.append((x0 + dx * t - dy / L * off, y0 + dy * t + dx / L * off))
    return pts


def drone(fig, x, y, s, fill="amber"):
    with fig.frame(x, y, s=s):
        fig.rect(-1, -0.6, 1, 0.6, fill, r=0.5)
        fig.circle(0.4, 0.05, 0.28, "visor")


def lance(fig, x, y, s, rot, fill="coral"):
    with fig.frame(x, y, s=s, rot=rot):
        fig.rect(-1, -0.3, 0.7, 0.3, "suitshade", r=0.14)
        fig.poly([(-0.8, 0.3), (-1.15, 0.5), (-1.0, 0.3)], fill)
        fig.poly([(-0.8, -0.3), (-1.15, -0.5), (-1.0, -0.3)], fill)
        fig.rect(-0.66, -0.1, 0.56, 0.1, "xp", r=0.05)
        fig.poly([(0.7, -0.36), (1.15, 0), (0.7, 0.36)], fill)


def missile(fig, x, y, s, rot):
    with fig.frame(x, y, s=s, rot=rot):
        fig.poly([(-1, 0.1), (-1.3, 0.4), (-0.8, 0.2)], "coral")
        fig.poly([(-1, -0.1), (-1.3, -0.4), (-0.8, -0.2)], "coral")
        fig.rect(-1.1, -0.22, 0.6, 0.22, "offwhite", r=0.1)
        fig.poly([(0.55, 0.22), (1.1, 0), (0.55, -0.22)], "sensor")


def bolt_shape(fig, x, y, s, fill="teal", rot=0):
    with fig.frame(x, y, s=s, rot=rot):
        fig.poly([(0.15, 1), (-0.5, -0.1), (0.0, -0.1), (-0.2, -1), (0.55, 0.2), (0.05, 0.2), (0.35, 1)], fill)


def flame(fig, x, y, s):
    with fig.frame(x, y, s=s):
        fig.blob([(-0.6, -0.5), (-0.6, 0.1), (-0.2, 0.5), (0, 1), (0.3, 0.5), (0.6, 0.1), (0.5, -0.5), (0, -0.8)], "coral", tension=0.5)
        fig.blob([(-0.3, -0.5), (-0.3, 0.0), (0, 0.4), (0.25, 0), (0.25, -0.5), (0, -0.62)], "amber", tension=0.5)


def ICONS():
    def seeker_rifle(f):
        tube(f, [(-0.5, -0.2), (-0.1, 0.42), (0.44, 0.5)], "xp", 0.05)
        f.circle(0.5, 0.5, 0.1, "xphi")
        with f.frame(0, -0.1, rot=20):
            f.rect(-0.62, -0.14, 0.62, 0.12, "gunmetal", r=0.06)
            f.rect(-0.3, -0.36, -0.12, -0.1, "gunmetaldark", r=0.04)
            f.rect(0.36, -0.08, 0.62, 0.06, "amber", r=0.03)

    def arc_sweep(f):
        ring(f, -0.3, 0, 0.36, 0.72, -70, 70, "teal")
        ring(f, -0.3, 0, 0.5, 0.58, -45, 45, "xphi", n=12)

    def rail_bolt(f):
        # a glowing rail slug through two coil rings, with a short trail
        with f.frame(0.0, 0.0, rot=30):
            for y in (0.15, -0.15):
                f.poly([(-0.94, y * 0.8), (-0.42, y + 0.09), (-0.42, y - 0.09)], "xp")
            for x in (-0.3, 0.02):
                f.ellipse(x, 0, 0.13, 0.58, "coral")
            f.poly(arc(-0.26, 0, 0.26, 90, 270, 12) + [(0.3, -0.26), (0.86, 0), (0.3, 0.26)], "xp")
            f.rect(-0.3, 0.02, 0.34, 0.15, "xphi", r=0.06)

    def needler(f):
        for rot in (-25, 0, 25):
            with f.frame(-0.4, -0.2, rot=rot + 25):
                f.poly([(0.1, 0), (0.5, 0.07), (1.0, 0), (0.5, -0.07)], "amber")

    def ricochet_disc(f):
        f.stroke([(-0.6, -0.4), (-0.2, 0.4), (0.2, -0.2), (0.45, 0.2)], width=2.2, role="amber")
        f.circle(0.35, 0.28, 0.3, "suit")
        f.circle(0.35, 0.28, 0.12, "amber")

    def ground_slam(f):
        f.ellipse(0, -0.5, 0.66, 0.16, "rock")
        for a in (-1, 1):
            f.stroke([(0.2 * a, -0.42), (0.62 * a, -0.2)], width=2.4, role="teal")
        f.rect(-0.3, -0.36, 0.3, 0.3, "teal", r=0.14)
        f.rect(-0.22, 0.24, 0.22, 0.62, "suit", r=0.08)

    def gravity_hook(f):
        tube(f, [(-0.5, 0.56), (-0.2, 0.3), (0.0, 0.2)], "gunmetal", 0.06)
        tube(f, [(0.0, 0.2), (0.1, -0.2), (0.0, -0.5), (-0.3, -0.5), (-0.36, -0.28)], "teal", 0.13)
        f.circle(0.0, 0.24, 0.1, "teal")

    def mine_layer(f):
        for a in range(0, 360, 60):
            r = math.radians(a)
            f.rect(math.cos(r) * 0.42 - 0.07, math.sin(r) * 0.42 - 0.07, math.cos(r) * 0.42 + 0.07, math.sin(r) * 0.42 + 0.07, "gunmetaldark", r=0.03)
        f.circle(0, 0, 0.42, "gunmetal")
        f.circle(0, 0, 0.16, "coral")

    def cutting_beam(f):
        f.rect(-0.2, -0.16, 0.74, 0.16, "coral", r=0.15)
        f.rect(-0.1, -0.05, 0.64, 0.05, "suit", r=0.04)
        f.rect(-0.7, -0.28, -0.14, 0.28, "suitshade", r=0.12)

    def orbit_drones(f):
        f.stroke(arc(0, 0, 0.5, 0, 360, 40), width=2.4, role="suitshade")
        f.circle(0, 0, 0.18, "suit")
        for a in (90, 210, 330):
            r = math.radians(a)
            drone(f, 0.5 * math.cos(r), 0.5 * math.sin(r), 0.14)

    def pulse_field(f):
        f.circle(0, 0, 0.68, "teal")
        f.circle(0, 0, 0.5, "badge")
        f.circle(0, 0, 0.36, "teal")
        f.circle(0, 0, 0.2, "xphi")

    def mortar(f):
        f.stroke(arc(0.0, -0.3, 0.55, 170, 30, 16), width=2.2, role="amber")
        with f.frame(0.48, 0.0, s=0.22, rot=-60):
            f.rect(-1, -0.5, 0.3, 0.5, "gunmetal", r=0.15)
            f.blob([(0.3, -0.5), (1.0, 0), (0.3, 0.5)], "amber", tension=0.4)
        f.rect(-0.62, -0.62, -0.22, -0.3, "gunmetaldark", r=0.08)

    def chain_arc(f):
        nodes = [(-0.55, -0.3), (0.0, 0.35), (0.55, -0.25)]
        tube(f, zigzag(*nodes[0], *nodes[1], 4, 0.1), "xp", 0.09, smooth=False)
        tube(f, zigzag(*nodes[1], *nodes[2], 4, 0.1), "xp", 0.09, smooth=False)
        for x, y in nodes:
            f.circle(x, y, 0.17, "suit")

    def cryo_burst(f):
        for a in range(0, 180, 60):
            with f.frame(0, 0, rot=a):
                f.poly([(-0.68, 0), (0, 0.12), (0.68, 0), (0, -0.12)], "xp")
        f.circle(0, 0, 0.16, "xphi")

    def missiles(f):
        missile(f, -0.1, 0.22, 0.44, 30)
        missile(f, 0.16, -0.3, 0.44, 30)

    def flamer(f):
        flame(f, 0.0, 0.05, 0.58)

    def sentry(f):
        for a in (-1, 1):
            f.stroke([(0, -0.1), (0.4 * a, -0.6)], width=3.0, role="gunmetal")
        f.rect(-0.08, -0.12, 0.66, 0.06, "gunmetaldark", r=0.04)
        f.circle(0, 0.06, 0.3, "suit")
        f.circle(0.1, 0.1, 0.1, "amber")

    def power_cell(f):
        f.rect(-0.3, -0.56, 0.3, 0.46, "suit", r=0.1)
        f.rect(-0.12, 0.46, 0.12, 0.6, "gunmetal", r=0.03)
        bolt_shape(f, 0, -0.05, 0.3, "amber")

    def coolant_loop(f):
        ring(f, 0, 0, 0.34, 0.52, 60, 330, "xp", n=30)
        f.poly([(0.3, 0.22), (0.62, 0.62), (0.2, 0.62)], "xp")

    def field_emitter(f):
        for r0 in (0.3, 0.5):
            ring(f, -0.3, -0.1, r0, r0 + 0.08, -50, 50, "teal", n=10)
        f.ellipse(-0.36, -0.1, 0.18, 0.34, "gunmetal")
        f.rect(-0.66, -0.5, -0.32, -0.38, "gunmetaldark", r=0.03)

    def split_feed(f):
        for dy in (0.42, 0, -0.42):
            tube(f, [(-0.5, 0), (0.0, 0.0), (0.44, dy)], "amber", 0.08)
            with f.frame(0.48, dy, rot=math.degrees(math.atan2(dy, 0.44))):
                f.poly([(-0.06, 0.14), (0.16, 0), (-0.06, -0.14)], "amber")

    def thrusters(f):
        with f.frame(0, -0.4, s=0.46, rot=180):
            flame(f, 0, 0, 1)
        f.rect(-0.3, -0.1, 0.3, 0.6, "suitshade", r=0.12)
        f.rect(-0.3, 0.1, 0.3, 0.22, "coral", r=0.03)

    def plating(f):
        f.blob([(-0.5, 0.5), (0.5, 0.5), (0.48, -0.1), (0, -0.62), (-0.48, -0.1)], "suitshade", tension=0.2)
        f.rect(-0.3, 0.1, 0.3, 0.3, "teal", r=0.05)

    def hardened_shell(f):
        f.poly(arc(0, 0, 0.62, 90, 450, 6)[:-1], "olive")
        f.poly(arc(0, 0, 0.32, 90, 450, 6)[:-1], "olivehi")

    def magnet_coil(f):
        tube(f, [(-0.3, 0.5), (-0.3, -0.08)] + arc(0, -0.08, 0.3, 180, 360, 10)[1:] + [(0.3, 0.5)], "coral", 0.2, smooth=False)
        f.rect(-0.44, 0.32, -0.16, 0.56, "suit", r=0.03)
        f.rect(0.16, 0.32, 0.44, 0.56, "suit", r=0.03)

    def swarm_battery(f):
        for x, y in ((-0.34, 0.3), (0.34, 0.3), (0.0, -0.3), (-0.4, -0.3), (0.4, -0.3)):
            drone(f, x, y, 0.16)

    def shockwave_maul(f):
        ring(f, 0, -0.35, 0.5, 0.62, 180, 360, "teal", n=20)
        f.rect(-0.07, -0.4, 0.07, 0.3, "gunmetal", r=0.03)
        f.rect(-0.4, 0.2, 0.4, 0.56, "teal", r=0.1)

    def lance_array(f):
        for dy, rot in ((0.42, 12), (0, 0), (-0.42, -12)):
            lance(f, -0.04, dy, 0.48, rot)

    def halo_guard(f):
        f.ellipse(0, 0.48, 0.42, 0.13, "goldhi")
        f.ellipse(0, 0.48, 0.24, 0.05, "badge")
        f.blob([(-0.42, 0.24), (0.42, 0.24), (0.4, -0.2), (0, -0.62), (-0.4, -0.2)], "suit", tension=0.2)

    def storm_lattice(f):
        for x in (-0.35, 0.35):
            f.stroke([(x, -0.55), (x, 0.55)], width=2.0, role="badgehi")
        for y in (-0.35, 0.35):
            f.stroke([(-0.55, y), (0.55, y)], width=2.0, role="badgehi")
        bolt_shape(f, 0, 0, 0.55, "xp")

    abilities = [seeker_rifle, arc_sweep, rail_bolt, needler, ricochet_disc, ground_slam, gravity_hook, mine_layer,
                 cutting_beam, orbit_drones, pulse_field, mortar, chain_arc, cryo_burst, missiles, flamer, sentry]
    passives = [power_cell, coolant_loop, field_emitter, split_feed, thrusters, plating, hardened_shell, magnet_coil]
    evolutions = [swarm_battery, shockwave_maul, lance_array, halo_guard, storm_lattice]
    out = {}
    for group, rim in ((abilities, "badgehi"), (passives, "teal"), (evolutions, "gold")):
        for fn in group:
            out[fn.__name__] = (fn, rim)
    return out


def make_icon(draw, rim):
    def mk(rng):
        fig = Fig("icon")
        with unit(fig, pad=0.9):
            fig.circle(0, 0, 0.96, rim)
            fig.circle(0, 0, 0.8, "badge")
            draw(fig)
        return fig
    return mk


for _id, (_draw, _rim) in ICONS().items():
    PICTURES[f"icons/{_id}"] = make_icon(_draw, _rim)


@pic("icons/module")
def icon_module(rng):
    fig = Fig("icon")
    with unit(fig, pad=0.9):
        for i in range(4):
            y = -0.45 + i * 0.3
            fig.rect(-0.88, y - 0.06, 0.88, y + 0.06, "gunmetal", r=0.03)
            x = -0.45 + i * 0.3
            fig.rect(x - 0.06, -0.88, x + 0.06, 0.88, "gunmetal", r=0.03)
        fig.rect(-0.66, -0.66, 0.66, 0.66, "badge", r=0.14)
        fig.rect(-0.36, -0.36, 0.36, 0.36, "badgehi", r=0.08)
        fig.circle(0, 0, 0.16, "xp")
    return fig


@pic("icons/gold")
def icon_gold(rng):
    fig = Fig("icon")
    with unit(fig, pad=0.9):
        for dx, dy in ((-0.28, -0.3), (0.3, -0.18), (-0.02, 0.24)):
            fig.poly(arc(dx, dy, 0.5, 30, 390, 6)[:-1], "gold")
            fig.poly(arc(dx, dy, 0.26, 30, 390, 6)[:-1], "goldhi")
    return fig


@pic("icons/ticket")
def icon_ticket(rng):
    fig = Fig("icon")
    with unit(fig, pad=0.9):
        with fig.frame(0, 0, rot=12):
            fig.poly([(-0.9, -0.52), (0.9, -0.52), (0.9, -0.15), (0.72, 0), (0.9, 0.15), (0.9, 0.52), (-0.9, 0.52),
                      (-0.9, 0.15), (-0.72, 0), (-0.9, -0.15)], "ticket")
            fig.rect(-0.45, -0.24, 0.45, 0.24, "suit", r=0.06)
            fig.circle(0, 0, 0.1, "ticket")
    return fig


# ---------------------------------------------------------- gear modules
# One icon per gear module: the subject on a chip like icons/module. The
# chip is the same neutral grey for every module; the page draws the rarity
# border, so no icon carries a rarity colour.

def chip(fig):
    for i in (-0.4, 0.0, 0.4):
        for a in (-1, 1):
            fig.rect(i - 0.07, a * 0.7, i + 0.07, a * 0.92, "gunmetal", r=0.03)
            fig.rect(a * 0.7, i - 0.07, a * 0.92, i + 0.07, "gunmetal", r=0.03)
    fig.rect(-0.8, -0.8, 0.8, 0.8, "badge", r=0.2)


def up_arrow(fig, x, y, s, fill):
    with fig.frame(x, y, s=s):
        fig.poly([(-0.28, -1), (0.28, -1), (0.28, 0), (0.7, 0), (0, 1), (-0.7, 0), (-0.28, 0)], fill)


def shield_shape(fig, x, y, s, fill):
    with fig.frame(x, y, s=s):
        fig.blob([(-0.8, 0.8), (0, 0.92), (0.8, 0.8), (0.74, -0.1), (0, -1), (-0.74, -0.1)], fill, tension=0.25)


def cross(fig, x, y, s, fill):
    with fig.frame(x, y, s=s):
        fig.poly([(-0.34, 1), (0.34, 1), (0.34, 0.34), (1, 0.34), (1, -0.34), (0.34, -0.34),
                  (0.34, -1), (-0.34, -1), (-0.34, -0.34), (-1, -0.34), (-1, 0.34), (-0.34, 0.34)], fill)


def MODULES():
    passive = ICONS()

    def kinetic_core(f):
        # an impact burst: damage
        pts = []
        for i in range(16):
            a = math.radians(90 + i * 22.5)
            r = 0.66 if i % 2 == 0 else 0.34
            pts.append((r * math.cos(a), r * math.sin(a)))
        f.poly(pts, "coral")
        f.circle(0, 0, 0.24, "amber")
        f.circle(-0.07, 0.07, 0.07, "goldhi")

    def coolant_mod(f):
        # a refresh arrow around a coolant drop: faster recharge
        ring(f, 0, -0.04, 0.42, 0.6, 70, 340, "xp", n=30)
        f.poly([(0.08, 0.62), (0.3, 0.22), (0.46, 0.68)], "xp")
        f.blob([(0, 0.24), (0.17, -0.06), (0.12, -0.24), (0, -0.28), (-0.12, -0.24), (-0.17, -0.06)], "xphi", tension=0.5)

    def plating_mod(f):
        # a riveted suit plate with a health cross
        f.rect(-0.6, -0.56, 0.6, 0.56, "suit", r=0.16)
        cross(f, 0, 0, 0.34, "heal")
        for x in (-0.44, 0.44):
            for y in (-0.4, 0.4):
                f.circle(x, y, 0.05, "suitshade")

    def magnet_mod(f):
        with f.frame(0, -0.02, s=1.15):
            passive["magnet_coil"][0](f)

    def thruster_mod(f):
        # a boot with a jet from the heel and speed lines: move speed
        for y in (0.44, 0.2):
            f.stroke([(-0.66, y), (-0.4, y)], width=3.0, role="suitshade")
        f.blob([(-0.18, 0.58), (0.18, 0.58), (0.2, 0.02), (0.62, -0.06), (0.64, -0.4), (-0.2, -0.4)], "suit", tension=0.15)
        f.rect(-0.22, -0.48, 0.68, -0.32, "gunmetal", r=0.06)
        f.rect(-0.16, 0.34, 0.16, 0.46, "coral", r=0.03)
        with f.frame(-0.46, -0.24, s=0.3, rot=90):
            flame(f, 0, 0, 1)

    def overclock(f):
        # stacked chevrons pointing up: the weapon starts a level higher
        for i, fill in enumerate(("amber", "amber", "goldhi")):
            y = -0.52 + i * 0.36
            f.poly([(-0.56, y), (0, y + 0.42), (0.56, y), (0.56, y - 0.2), (0, y + 0.2), (-0.56, y - 0.2)], fill)

    def field_lens(f):
        # a lens with four arrows pushing out: larger areas
        f.circle(0, 0, 0.3, "teal")
        f.circle(-0.08, 0.08, 0.09, "xphi")
        for a in (45, 135, 225, 315):
            with f.frame(0, 0, rot=a):
                f.poly([(0.42, 0.1), (0.42, 0.22), (0.68, 0), (0.42, -0.22), (0.42, -0.1)], "teal")

    def spare_reroll(f):
        # two cards fanned with a spare one added
        with f.frame(-0.12, -0.04, rot=16):
            f.rect(-0.3, -0.46, 0.3, 0.46, "suitshade", r=0.08)
        with f.frame(0.08, -0.08, rot=-10):
            f.rect(-0.3, -0.46, 0.3, 0.46, "suit", r=0.08)
            f.poly([(0, 0.2), (0.14, 0), (0, -0.2), (-0.14, 0)], "ticket")
        cross(f, 0.44, 0.44, 0.2, "amber")

    def shield_charge(f):
        # a shield with a charge bolt: a recharging shield
        shield_shape(f, 0, 0.02, 0.66, "xp")
        shield_shape(f, 0, 0.06, 0.44, "badge")
        bolt_shape(f, 0, 0.0, 0.38, "amber")

    def split_chamber(f):
        with f.frame(0, 0, s=1.1):
            passive["split_feed"][0](f)

    def aegis_core(f):
        # a heavy hexagonal armour plate with a round core: armour
        f.poly(arc(0, 0, 0.68, 90, 450, 6)[:-1], "offwhite")
        f.poly(arc(0, 0, 0.44, 90, 450, 6)[:-1], "gunmetal")
        f.circle(0, 0, 0.2, "teal")
        f.circle(-0.06, 0.06, 0.06, "suit")

    def rift_key(f):
        # a key whose bow is a rift ring: more tickets from rifts
        with f.frame(-0.04, 0.04, rot=-35):
            f.rect(-0.1, -0.1, 0.74, 0.1, "suit", r=0.05)
            f.rect(0.42, -0.34, 0.54, -0.06, "suit", r=0.03)
            f.rect(0.62, -0.28, 0.74, -0.06, "suit", r=0.03)
            f.circle(-0.34, 0, 0.36, "rifthi")
            f.circle(-0.34, 0, 0.26, "rift")
            f.circle(-0.34, 0, 0.1, "ink")

    out = {}
    for fn in (kinetic_core, coolant_mod, plating_mod, magnet_mod, thruster_mod, overclock, field_lens,
               spare_reroll, shield_charge, split_chamber, aegis_core, rift_key):
        out[fn.__name__] = fn
    return out


def make_module(draw):
    def mk(rng):
        fig = Fig("icon")
        with unit(fig, pad=0.9):
            chip(fig)
            draw(fig)
        return fig
    return mk


for _id, _draw in MODULES().items():
    PICTURES[f"icons/mod_{_id}"] = make_module(_draw)


# ------------------------------------------------------ empty gear slots
# Faint silhouettes for an empty slot box: one low-contrast fill and an ink
# edge, so they sit back behind whatever fills a slot.

FAINT, FAINT_DETAIL = "badgehi", "badge"


def slot_icon(draw):
    def mk(rng):
        fig = Fig("icon")
        with unit(fig, pad=0.9):
            draw(fig)
        return fig
    return mk


@pic("icons/slot_core")
@slot_icon
def slot_core(f):
    # a weapon core: a carbine body around a round core
    f.rect(-0.7, -0.18, 0.78, 0.2, FAINT, r=0.1)
    f.rect(-0.46, -0.6, -0.18, -0.12, FAINT, r=0.08)
    f.rect(0.5, 0.2, 0.7, 0.3, FAINT, r=0.03)
    f.circle(0.0, 0.0, 0.34, FAINT)
    f.circle(0.0, 0.0, 0.15, FAINT_DETAIL)


@pic("icons/slot_frame")
@slot_icon
def slot_frame(f):
    # a chest armour frame: shoulders and a rounded breastplate
    f.ellipse(-0.5, 0.42, 0.28, 0.2, FAINT)
    f.ellipse(0.5, 0.42, 0.28, 0.2, FAINT)
    f.blob([(-0.46, 0.5), (0, 0.42), (0.46, 0.5), (0.5, -0.1), (0.3, -0.58), (0, -0.66), (-0.3, -0.58), (-0.5, -0.1)],
           FAINT, tension=0.35)
    f.stroke([(0, 0.36), (0, -0.52)], width=1.6)
    f.stroke(arc(0, 0.62, 0.22, 200, 340, 10), width=1.6)


@pic("icons/slot_utility")
@slot_icon
def slot_utility(f):
    # a utility pack: a rounded pack with a flap and two side pouches
    f.rect(-0.7, -0.5, -0.4, 0.1, FAINT, r=0.1)
    f.rect(0.4, -0.5, 0.7, 0.1, FAINT, r=0.1)
    f.rect(-0.46, -0.66, 0.46, 0.5, FAINT, r=0.18)
    f.rect(-0.46, 0.04, 0.46, 0.5, FAINT, r=0.18)
    f.rect(-0.1, -0.06, 0.1, 0.14, FAINT_DETAIL, r=0.03)
    f.stroke(arc(0, 0.5, 0.2, 20, 160, 10), width=3.0, role="ink")


# ------------------------------------------------------- achievements
# Achievement category icons on a plain round badge; ach_hidden is a dim
# badge with a question mark for achievements the player cannot see yet.

def ACHIEVEMENTS():
    def survive(f):
        # an hourglass
        f.rect(-0.42, 0.5, 0.42, 0.64, "suitshade", r=0.05)
        f.rect(-0.42, -0.64, 0.42, -0.5, "suitshade", r=0.05)
        f.poly([(-0.34, 0.5), (0.34, 0.5), (0.06, 0.0), (0.34, -0.5), (-0.34, -0.5), (-0.06, 0.0)], "suit")
        f.poly([(-0.18, 0.3), (0.18, 0.3), (0.0, 0.04)], "amber")
        f.poly([(-0.3, -0.48), (0.3, -0.48), (0.0, -0.22)], "amber")

    def slayer(f):
        # two crossed blades
        for rot in (45, 135):
            with f.frame(0, 0, rot=rot):
                f.poly([(-0.2, -0.09), (0.56, -0.09), (0.74, 0), (0.56, 0.09), (-0.2, 0.09)], "suit")
                f.rect(-0.26, -0.2, -0.16, 0.2, "gunmetal", r=0.03)
                f.rect(-0.62, -0.07, -0.26, 0.07, "amber", r=0.04)

    def boss(f):
        # a big horned alien head
        for a in (-1, 1):
            f.blob([(0.3 * a, 0.2), (0.62 * a, 0.36), (0.66 * a, 0.7), (0.5 * a, 0.42), (0.24 * a, 0.36)], "faunatealhi", tension=0.4)
        f.blob([(-0.48, 0.1), (-0.3, 0.42), (0.3, 0.42), (0.48, 0.1), (0.3, -0.42), (0, -0.56), (-0.3, -0.42)], "violet", tension=0.55)
        eye(f, -0.18, 0.06, 0.1)
        eye(f, 0.18, 0.06, 0.1)
        for x in (-0.12, 0.0, 0.12):
            f.poly([(x - 0.05, -0.28), (x + 0.05, -0.28), (x, -0.42)], "suit")

    def evolve(f):
        # two shapes merging into an upward arrow
        tube(f, [(-0.44, -0.42), (-0.2, -0.16), (0, 0.0)], "teal", 0.1)
        tube(f, [(0.44, -0.42), (0.2, -0.16), (0, 0.0)], "amber", 0.1)
        f.circle(-0.46, -0.44, 0.15, "teal")
        f.rect(0.32, -0.58, 0.6, -0.3, "amber", r=0.05)
        up_arrow(f, 0, 0.26, 0.38, "gold")

    def character(f):
        # a round player helmet
        f.circle(0, -0.02, 0.56, "suit")
        segment(f, 0, -0.02, 0.56, 210, 330, "suitshade")
        f.rect(-0.38, -0.06, 0.38, 0.26, "visor", r=0.15)
        f.rect(-0.24, 0.12, -0.04, 0.2, "visorhi", r=0.035)

    def rift(f):
        # a portal ring
        f.ellipse(0, 0, 0.6, 0.6, "rift")
        ring(f, 0, 0, 0.42, 0.5, 20, 160, "rifthi", n=14)
        ring(f, 0, 0, 0.42, 0.5, 200, 340, "rifthi", n=14)
        f.ellipse(0, 0, 0.28, 0.28, "ink")
        f.ellipse(-0.08, 0.08, 0.07, 0.05, "rifthi")

    def cache(f):
        # a supply crate with hazard bands
        f.rect(-0.6, -0.5, 0.6, 0.3, "offwhite", r=0.1)
        f.rect(-0.6, 0.14, 0.6, 0.5, "gunmetal", r=0.1)
        f.rect(-0.13, -0.12, 0.13, 0.24, "hazard", r=0.04)
        f.rect(-0.6, -0.5, -0.36, 0.3, "hazard", r=0.07)
        f.rect(0.36, -0.5, 0.6, 0.3, "hazard", r=0.07)

    def coop(f):
        # two round heads side by side
        for x, y, acc in ((-0.26, 0.08, "amber"), (0.26, -0.08, "coral")):
            f.circle(x, y, 0.36, "suit")
            segment(f, x, y, 0.36, 210, 330, acc)
            f.rect(x - 0.06, y - 0.02, x + 0.34, y + 0.18, "visor", r=0.09)
            f.rect(x + 0.02, y + 0.09, x + 0.14, y + 0.14, "visorhi", r=0.02)

    def outfit(f):
        # a wrench
        with f.frame(0, 0, rot=-45):
            f.rect(-0.66, -0.14, 0.34, 0.14, "suitshade", r=0.1)
            f.circle(0.42, 0, 0.3, "suitshade")
            f.rect(0.4, -0.11, 0.78, 0.11, "badge")
            f.circle(-0.62, 0, 0.2, "suitshade")
            f.circle(-0.62, 0, 0.08, "badge")

    def weapon(f):
        # a crosshair
        ring(f, 0, 0, 0.38, 0.5, 0, 360, "suit", n=40)
        for a in (0, 90, 180, 270):
            with f.frame(0, 0, rot=a):
                f.rect(0.22, -0.05, 0.68, 0.05, "suit", r=0.03)
        f.circle(0, 0, 0.1, "coral")

    out = {}
    for fn in (survive, slayer, boss, evolve, character, rift, cache, coop, outfit, weapon):
        out[fn.__name__] = fn
    return out


for _id, _draw in ACHIEVEMENTS().items():
    PICTURES[f"icons/ach_{_id}"] = make_icon(_draw, "badgehi")


@pic("icons/ach_hidden")
def ach_hidden(rng):
    fig = Fig("icon")
    with unit(fig, pad=0.9):
        fig.circle(0, 0, 0.96, "badge")
        fig.circle(0, 0, 0.8, "shadow")
        # a question mark drawn as shapes: a hook and a dot
        tube(fig, arc(0, 0.22, 0.24, 160, -60, 14) + [(0.02, -0.08), (0, -0.2)], "badgehi", 0.16)
        fig.circle(0, -0.46, 0.1, "badgehi")
    return fig


# ------------------------------------------------------------------ title

@pic("title")
def title(rng):
    fig = Fig("scene")
    W, H = fig.size
    M.sky_bands(fig, splits=(0.42, 0.72))
    # ringed planet and a small moon, right side
    with fig.frame(15.6, 8.4, rot=-14):
        fig.poly(arc(0, 0, 2.9, 0, 180, 30, ry=0.5) + list(reversed(arc(0, 0, 2.2, 0, 180, 30, ry=0.28))), "moon")
        fig.circle(0, 0, 1.7, "haze")
        fig.blob([(-1.3, 0.7), (-0.4, 1.4), (0.9, 1.3), (0.0, 0.8)], "moon", tension=0.6)
        fig.poly(arc(0, 0, 2.9, 180, 360, 30, ry=0.5) + list(reversed(arc(0, 0, 2.2, 180, 360, 30, ry=0.28))), "moon")
    fig.circle(10.6, 9.6, 0.3, "moon")
    # distant mesas in haze
    fig.poly([(-0.2, 3.6), (1.4, 3.9), (2.0, 4.6), (3.4, 4.7), (4.0, 4.1), (6.2, 4.0), (7.0, 4.9), (8.6, 5.0),
              (9.0, 4.3), (12.0, 4.1), (12.8, 5.2), (14.6, 5.3), (15.2, 4.4), (19.4, 4.2), (19.4, -0.2), (-0.2, -0.2)], "haze")
    # corporate dropship crossing the sky
    with fig.frame(11.6, 7.3, s=1.0, rot=-6):
        fig.poly([(-1.6, 0.1), (-2.3, 0.6), (-1.8, 0.6), (-1.0, 0.3)], "gunmetal")
        fig.rect(-1.8, -0.36, 1.4, 0.36, "offwhite", r=0.3)
        fig.poly([(1.3, 0.3), (2.1, 0.0), (1.3, -0.34)], "offwhite")
        fig.rect(0.9, -0.08, 1.7, 0.12, "gunmetaldark", r=0.08)
        M.stripes(fig, -1.2, -0.22, -0.2, -0.04, 4, "hazard", "ink", 0.6)
        M.corp_glyph(fig, 0.3, 0.06, 0.2)
        for x in (-1.2, 0.6):
            fig.rect(x - 0.3, -0.62, x + 0.3, -0.3, "gunmetal", r=0.12)
            fig.circle(x, -0.62, 0.12, "sensor")
    # ridge in the foreground
    fig.poly([(-0.2, 2.6), (3.0, 2.9), (6.0, 3.3), (8.4, 3.7), (11.0, 3.8), (13.6, 3.2), (16.2, 2.6), (19.4, 2.3),
              (19.4, -0.2), (-0.2, -0.2)], "rock")
    fig.poly([(13.6, 3.2), (16.2, 2.6), (19.4, 2.3), (19.4, -0.2), (14.0, -0.2), (12.4, 1.6)], "rockdark")
    fig.poly([(-0.2, 1.2), (4.0, 1.6), (8.0, 1.2), (12.0, 1.0), (19.4, 0.8), (19.4, -0.2), (-0.2, -0.2)], "ground")
    fig.blob([(5.0, 3.15), (6.2, 3.32), (7.6, 3.62), (6.6, 2.9)], "rockhi", tension=0.4)
    for x, y, r in ((2.0, 0.6, 0.16), (4.4, 1.1, 0.1), (13.2, 0.5, 0.14), (17.6, 0.6, 0.1), (9.6, 0.4, 0.12)):
        fig.ellipse(x, y, r, r * 0.6, "pebble")
    for x in (3.6, 3.9):
        fig.poly([(x - 0.18, 2.86), (x - 0.1, 3.4), (x, 3.6), (x + 0.1, 3.4), (x + 0.18, 2.88)], "crystal")
    # the three players on the ridge crest
    for x, y, draw, s, flip in ((7.2, 4.6, draw_breaker, 1.3, False), (9.9, 4.85, draw_pathfinder, 1.3, False),
                                (12.6, 4.6, draw_vector, 1.3, False)):
        with fig.frame(x, y, s=s, flip=flip):
            draw(fig)
    M.calm_left_third(fig)
    return fig
