"""Galactic Survivors rig icons, registered for tools/art/art.py: the new
payload abilities and passives, the rig parts, the runes and the ground
fields.

Abilities sit on the round badge with the plain rim and passives on the teal
rim, like icons/<id> in art_galactic.py. Each part kind has its own frame, the
same for every part of that kind, so a player can tell the kind at a glance:

    modifier   a rounded square with a coral rim
    formation  a hexagon with an xp rim
    trigger    a circle with a notch at the top, violethi rim
    rare       an eight-pointed gold star with a goldhi rim and a violetdark face

Runes sit on the teal passive badge. Ground fields have no frame: each is a
flat patch seen from above at three-quarter view.

Colours are role names from styles/galactic; this file holds no colour value.
"""
import math

from artlib import Fig
import style
from art_galactic import arc, tube, ring, segment, unit, make_icon, flame, bolt_shape, zigzag, M

PICTURES = {}
INK_PT = style.load().ink_for("icon")


# ---------------------------------------------------------------- helpers

def loop(f, pts, fill, w, smooth=False):
    """A closed outlined line, like motifs.tube but joined end to end."""
    fill_pt = w * f.k() * 28.4528
    f.stroke(pts, width=fill_pt + 2 * INK_PT, role="ink", smooth=smooth, closed=True)
    f.stroke(pts, width=fill_pt, role=fill, smooth=smooth, closed=True)


def head(f, x, y, rot, s=0.2, fill="amber"):
    """An arrowhead with its tip at (x, y), pointing rot degrees."""
    with f.frame(x, y, s=s, rot=rot):
        f.poly([(-1.1, 0.75), (0, 0), (-1.1, -0.75), (-0.8, 0)], fill)


def arrow(f, pts, fill, w=0.08, hs=0.2, smooth=True):
    """A tube along pts ending in an arrowhead."""
    (x0, y0), (x1, y1) = pts[-2], pts[-1]
    rot = math.degrees(math.atan2(y1 - y0, x1 - x0))
    tube(f, pts[:-1] + [(x1 - math.cos(math.radians(rot)) * hs * 0.7, y1 - math.sin(math.radians(rot)) * hs * 0.7)],
         fill, w, smooth=smooth)
    head(f, x1, y1, rot, hs, fill)


def shot(f, x, y, s, rot, fill="amber", hi="goldhi"):
    """A shot flying rot degrees: a round head with a tapered tail, outlined
    as one shape, its head centred at (x, y); s is the head's diameter."""
    with f.frame(x, y, s=s, rot=rot):
        with f.union(fill):
            f.circle(0, 0, 0.5)
            f.poly([(0, 0.38), (-2.2, 0), (0, -0.38)])
        if s >= 0.3:
            f.circle(0.1, 0.12, 0.17, hi)


def trail(f, x, y, rot, lengths=(0.36, 0.5, 0.36), gap=0.14, start=0.12, role="suitshade", width=2.6):
    """Short speed lines behind a point, leaving in direction rot+180."""
    c, s = math.cos(math.radians(rot)), math.sin(math.radians(rot))
    n = len(lengths)
    for i, L in enumerate(lengths):
        off = (i - (n - 1) / 2) * gap
        bx, by = x - c * start - s * off, y - s * start + c * off
        f.stroke([(bx, by), (bx - c * L, by - s * L)], width=width, role=role)


def burst(f, x, y, r0, r1, fill, n=8, rot=90):
    pts = []
    for i in range(2 * n):
        a = math.radians(rot + i * 180 / n)
        r = r1 if i % 2 == 0 else r0
        pts.append((x + r * math.cos(a), y + r * math.sin(a)))
    f.poly(pts, fill)


def spark(f, x, y, r, fill="goldhi"):
    burst(f, x, y, r * 0.38, r, fill, n=4, rot=90)


def wobble(cx, cy, rx, ry, bumps, n=None):
    """Points round an irregular puddle outline: bumps scale the radius."""
    n = n or len(bumps)
    return [(cx + rx * bumps[i] * math.cos(2 * math.pi * i / n), cy + ry * bumps[i] * math.sin(2 * math.pi * i / n))
            for i in range(n)]


# ------------------------------------------------- abilities and passives

def ABILITIES():
    def orbital_lance(f):
        # a satellite overhead sends three beams down onto points on a ring
        f.stroke(arc(0, -0.38, 0.58, 0, 360, 40, ry=0.2), width=2.6, role="suitshade")
        pts = [(0.58 * math.cos(math.radians(a)), -0.38 + 0.2 * math.sin(math.radians(a))) for a in (200, 270, 340)]
        for px, py in pts:
            tx = px * 0.18
            f.poly([(tx - 0.04, 0.42), (tx + 0.04, 0.42), (px + 0.1, py), (px - 0.1, py)], "xp")
            f.ellipse(px, py, 0.16, 0.07, "xphi")
        f.rect(-0.42, 0.46, -0.18, 0.62, "suitshade", r=0.03)
        f.rect(0.18, 0.46, 0.42, 0.62, "suitshade", r=0.03)
        f.circle(0, 0.54, 0.16, "suit")
        f.circle(0, 0.5, 0.07, "xp")

    def sweep_cannon(f):
        # one beam from above that has traced a full ring of flame
        loop(f, arc(0, -0.28, 0.56, 0, 360, 40, ry=0.26), "coral", 0.12, smooth=False)
        for a in (215, 270, 325):
            r = math.radians(a)
            flame(f, 0.56 * math.cos(r), -0.28 + 0.26 * math.sin(r) + 0.14, 0.22)
        bx, by = 0.56 * math.cos(math.radians(25)), -0.28 + 0.26 * math.sin(math.radians(25))
        f.poly([(0.08, 0.66), (0.2, 0.68), (bx + 0.08, by), (bx - 0.08, by)], "xp")
        f.ellipse(bx, by, 0.15, 0.07, "xphi")

    def wake_mine(f):
        # a mover leaves a trail; a mine sits dropped on it behind
        tube(f, [(-0.68, -0.44), (-0.3, -0.38), (0.02, -0.1), (0.26, 0.2)], "suitshade", 0.12)
        for a in range(0, 360, 60):
            with f.frame(-0.32, -0.36, rot=a):
                f.rect(0.22, -0.07, 0.36, 0.07, "gunmetaldark", r=0.02)
        f.circle(-0.32, -0.36, 0.27, "gunmetal")
        f.circle(-0.32, -0.36, 0.11, "coral")
        trail(f, 0.38, 0.34, 40, lengths=(0.2, 0.2), gap=0.44, start=0.3)
        f.circle(0.38, 0.34, 0.24, "suit")
        segment(f, 0.38, 0.34, 0.24, 200, 340, "suitshade")

    def gravity_well(f):
        # a swirling pit pulling rocks inward
        f.ellipse(0, -0.08, 0.66, 0.42, "violetdark")
        f.ellipse(0, -0.08, 0.44, 0.27, "violet")
        f.ellipse(0, -0.08, 0.2, 0.12, "ink")
        for a in (0, 180):
            pts = [(0.62 * math.cos(math.radians(a + t * 2.4)) * (1 - t / 330),
                    -0.08 + 0.38 * math.sin(math.radians(a + t * 2.4)) * (1 - t / 330)) for t in range(0, 230, 23)]
            f.stroke(pts, width=2.6, role="violethi", smooth=True)
        for x, y, r in ((-0.5, 0.42, 0.11), (0.52, 0.36, 0.09), (0.3, -0.62, 0.08)):
            f.circle(x, y, r, "rockhi")

    def tether(f):
        # two posts joined by a line that crackles
        tube(f, [(-0.46, -0.3), (0.46, 0.3)], "goldhi", 0.09)
        spark(f, -0.12, 0.2, 0.17, "amber")
        spark(f, 0.14, -0.18, 0.14, "amber")
        for x, y in ((-0.5, -0.33), (0.5, 0.33)):
            f.poly([(x, y + 0.26), (x + 0.22, y), (x, y - 0.26), (x - 0.22, y)], "gunmetal")
            f.circle(x, y, 0.09, "xp")

    def boomerang_disc(f):
        # a curved disc and an arrow coming back round
        arrow(f, arc(0.0, 0.02, 0.6, 70, -70, 16), "xp", 0.07, 0.2)
        tube(f, [(0.0, 0.46), (-0.36, 0.2), (-0.42, 0.0), (-0.36, -0.2), (0.0, -0.46)], "amber", 0.2)
        f.circle(-0.38, 0.0, 0.07, "goldhi")

    def seed_pod(f):
        # a cracked pod with a stem growing a small gun turret
        tube(f, [(0, -0.3), (-0.04, -0.06), (0, 0.14)], "olivehi", 0.08)
        f.blob([(-0.04, -0.12), (-0.3, 0.02), (-0.5, -0.02), (-0.32, -0.16)], "olivehi", tension=0.5)
        with f.frame(0.0, 0.34, rot=24):
            f.rect(0.0, -0.07, 0.6, 0.07, "gunmetaldark", r=0.035)
        f.rect(-0.26, 0.14, 0.2, 0.5, "gunmetal", r=0.12)
        f.rect(-0.16, 0.28, 0.04, 0.38, "amber", r=0.03)
        f.poly(arc(0, -0.3, 0.5, 180, 360, 16) + [(0.5, -0.3), (0.32, -0.18), (0.16, -0.3), (0, -0.2),
                                                   (-0.16, -0.3), (-0.32, -0.18)], "olive")

    return ABILITIES_LIST(orbital_lance, sweep_cannon, wake_mine, gravity_well, tether, boomerang_disc, seed_pod)


def ABILITIES_LIST(*fns):
    return {fn.__name__: fn for fn in fns}


def PASSIVES():
    def momentum(f):
        # a rolling ball with speed lines behind
        trail(f, 0.2, -0.02, 0, lengths=(0.34, 0.5, 0.34), gap=0.22, start=0.42, width=3.0)
        f.circle(0.24, -0.02, 0.4, "coral")
        ring(f, 0.24, -0.02, 0.18, 0.28, 100, 210, "suit", n=10)
        f.circle(0.24, -0.02, 0.07, "amber")

    def anchor(f):
        # a planted anchor inside a wide ring
        f.stroke(arc(0, -0.44, 0.66, 0, 180, 20, ry=0.2), width=3.0, role="xp")
        f.rect(-0.07, -0.5, 0.07, 0.4, "suitshade", r=0.03)
        f.rect(-0.28, 0.16, 0.28, 0.28, "suitshade", r=0.05)
        f.circle(0, 0.52, 0.14, "suitshade")
        f.circle(0, 0.52, 0.06, "badge")
        tube(f, arc(0, -0.1, 0.38, 200, 340, 14), "suitshade", 0.11)
        for a, rot in ((200, 120), (340, 60)):
            x, y = 0.38 * math.cos(math.radians(a)), -0.1 + 0.38 * math.sin(math.radians(a))
            head(f, x, y + 0.12, rot, 0.17, "suitshade")
        f.stroke(arc(0, -0.44, 0.66, 180, 360, 20, ry=0.2), width=3.0, role="xp")

    def glyph_memory(f):
        # a glowing rune inside a looped trail of steps
        for i in range(14):
            a = math.radians(i * 360 / 14 + 10)
            f.circle(0.62 * math.cos(a), 0.52 * math.sin(a), 0.055, "teal")
        tube(f, [(-0.24, -0.4), (0, -0.1), (0.24, -0.4)], "weakspot", 0.09, smooth=False)
        loop(f, [(0, -0.1), (0.24, 0.14), (0, 0.38), (-0.24, 0.14)], "weakspot", 0.09)

    def heat_sink(f):
        # cooling fins on a plate over a flame
        flame(f, 0, -0.42, 0.3)
        for x in (-0.4, -0.2, 0.0, 0.2, 0.4):
            f.rect(x - 0.06, -0.12, x + 0.06, 0.58, "xp", r=0.03)
        f.rect(-0.54, -0.24, 0.54, -0.06, "gunmetal", r=0.05)

    def double_tap(f):
        # two identical rounds side by side
        for x in (-0.22, 0.22):
            f.poly([(x - 0.15, 0.08), (x + 0.15, 0.08)] + arc(x, 0.08, 0.15, 0, 180, 12, ry=0.34)[1:-1], "suitshade")
            f.rect(x - 0.15, -0.5, x + 0.15, 0.12, "gold", r=0.03)
            f.rect(x - 0.18, -0.6, x + 0.18, -0.48, "amber", r=0.03)
            f.rect(x - 0.08, -0.36, x - 0.02, 0.02, "goldhi", r=0.02)

    def long_fuse(f):
        # a round bomb with a long coiled fuse ending in a spark
        pts = []
        for i in range(29):
            t = i / 28
            a = math.radians(-90 + t * 720 * 1.25)
            pts.append((-0.02 + t * 0.56 + 0.1 * math.cos(a), 0.12 + t * 0.42 + 0.1 * math.sin(a)))
        tube(f, pts, "offwhite", 0.05)
        spark(f, pts[-1][0], pts[-1][1], 0.16, "goldhi")
        with f.frame(-0.06, 0.06, rot=-40):
            f.rect(-0.12, -0.02, 0.12, 0.16, "gunmetaldark", r=0.03)
        f.circle(-0.22, -0.2, 0.4, "gunmetal")
        f.circle(-0.36, -0.06, 0.1, "suitshade")

    def extra_socket(f):
        # a row of sockets with one new, lit socket at the end
        f.rect(-0.72, -0.22, 0.72, 0.22, "gunmetaldark", r=0.2)
        for x in (-0.5, -0.17, 0.16):
            f.circle(x, 0, 0.12, "badge")
        f.circle(0.5, 0, 0.17, "xp")
        f.circle(0.5, 0, 0.07, "xphi")
        cross_plus(f, 0.5, 0.48, 0.17, "amber")

    def wide_formation(f):
        # five shots fanning wide from one point
        for i, a in enumerate((165, 128, 90, 52, 15)):
            r = math.radians(a)
            shot(f, 0.6 * math.cos(r), -0.38 + 0.7 * math.sin(r), 0.22, a)
        f.circle(0, -0.42, 0.16, "gunmetal")

    return ABILITIES_LIST(momentum, anchor, glyph_memory, heat_sink, double_tap, long_fuse, extra_socket,
                          wide_formation)


def cross_plus(f, x, y, s, fill):
    with f.frame(x, y, s=s):
        f.poly([(-0.34, 1), (0.34, 1), (0.34, 0.34), (1, 0.34), (1, -0.34), (0.34, -0.34),
                (0.34, -1), (-0.34, -1), (-0.34, -0.34), (-1, -0.34), (-1, 0.34), (-0.34, 0.34)], fill)


for _id, _draw in ABILITIES().items():
    PICTURES[f"icons/{_id}"] = make_icon(_draw, "badgehi")
for _id, _draw in PASSIVES().items():
    PICTURES[f"icons/{_id}"] = make_icon(_draw, "teal")


# ------------------------------------------------------------------- parts
# One frame per part kind. The subject is drawn about the origin within a
# radius of about 0.68 and scaled to fit its frame.

def frame_modifier(f):
    f.rect(-0.86, -0.86, 0.86, 0.86, "coral", r=0.26)
    f.rect(-0.7, -0.7, 0.7, 0.7, "badge", r=0.16)


def frame_formation(f):
    f.poly(arc(0, 0, 0.98, 90, 450, 6)[:-1], "xp")
    f.poly(arc(0, 0, 0.8, 90, 450, 6)[:-1], "badge")


def frame_trigger(f):
    gap = 16
    f.poly(arc(0, 0, 0.96, 90 + gap, 450 - gap, 40) + [(0, 0.72)], "violethi")
    f.circle(0, 0, 0.78, "badge")


def frame_rare(f):
    burst(f, 0, 0, 0.84, 0.99, "gold", n=8, rot=90)
    f.circle(0, 0, 0.74, "goldhi")
    f.circle(0, 0, 0.64, "violetdark")


FRAMES = {"modifier": (frame_modifier, 0.98), "formation": (frame_formation, 0.94),
          "trigger": (frame_trigger, 1.0), "rare": (frame_rare, 0.86)}


def make_part(kind, draw):
    frame, s = FRAMES[kind]

    def mk(rng):
        fig = Fig("icon")
        with unit(fig, pad=0.9):
            frame(fig)
            with fig.frame(0, 0, s=s):
                draw(fig)
        return fig
    return mk


def MODIFIERS():
    def ignite(f):
        flame(f, 0.0, 0.0, 0.58)
        spark(f, -0.48, 0.42, 0.14, "amber")
        spark(f, 0.46, 0.5, 0.11, "amber")

    def frost(f):
        for a in (90, 150, 210):
            with f.frame(0, 0, rot=a):
                tube(f, [(-0.6, 0), (0.6, 0)], "xp", 0.09, smooth=False)
                for e in (-1, 1):
                    tube(f, [(e * 0.32, 0), (e * 0.5, 0.16)], "xp", 0.07, smooth=False)
                    tube(f, [(e * 0.32, 0), (e * 0.5, -0.16)], "xp", 0.07, smooth=False)
        f.circle(0, 0, 0.13, "xphi")

    def shock(f):
        bolt_shape(f, 0, 0, 0.62, "hazard")
        for x, y, rot in ((-0.5, 0.3, 160), (0.5, -0.3, -20)):
            with f.frame(x, y, rot=rot):
                f.stroke([(0, 0.12), (0.16, 0.2)], width=2.6, role="goldhi")
                f.stroke([(0, -0.08), (0.18, -0.12)], width=2.6, role="goldhi")

    def oil(f):
        f.ellipse(0, -0.48, 0.5, 0.14, "violetdark")
        f.blob([(0, 0.62), (0.3, 0.12), (0.36, -0.12), (0.2, -0.36), (0, -0.42), (-0.2, -0.36), (-0.36, -0.12),
                (-0.3, 0.12)], "violet", tension=0.5)
        f.blob([(-0.2, -0.04), (-0.12, 0.16), (-0.06, -0.06), (-0.12, -0.22)], "violethi", tension=0.5)
        f.ellipse(0.12, -0.22, 0.1, 0.05, "teal", rot=-30)

    def scatter(f):
        shot(f, -0.06, 0, 0.3, 0)
        for a in (38, 0, -38):
            r = math.radians(a)
            shot(f, 0.1 + 0.5 * math.cos(r), 0.5 * math.sin(r), 0.2, a)

    def bounce(f):
        f.rect(-0.62, -0.6, 0.62, -0.46, "rockhi", r=0.05)
        arrow(f, [(-0.56, 0.5), (-0.08, -0.36), (0.06, -0.36), (0.52, 0.42)], "amber", 0.1, 0.22, smooth=False)

    def homing(f):
        f.circle(0.3, 0.3, 0.3, "coral")
        f.circle(0.3, 0.3, 0.19, "suit")
        f.circle(0.3, 0.3, 0.08, "coral")
        arrow(f, [(-0.6, -0.42), (-0.1, -0.5), (0.12, -0.2), (0.16, 0.08)], "amber", 0.08, 0.2)

    def heavy(f):
        # a kettle weight: the heavy round
        f.stroke(arc(0, 0.28, 0.22, 10, 170, 12), width=8.0, role="ink")
        f.stroke(arc(0, 0.28, 0.22, 10, 170, 12), width=8.0 - 2 * INK_PT, role="gunmetal")
        f.poly([(-0.32, 0.26), (0.32, 0.26), (0.56, -0.5), (-0.56, -0.5)], "gunmetal")
        f.rect(-0.44, -0.2, 0.44, -0.04, "amber", r=0.03)
        f.rect(-0.62, -0.62, 0.62, -0.48, "gunmetaldark", r=0.05)

    def pierce(f):
        tube(f, [(-0.66, 0), (0.4, 0)], "suit", 0.08, smooth=False)
        for x in (-0.26, 0.24):
            f.ellipse(x, 0, 0.17, 0.32, "flesh")
            f.circle(x, 0.14, 0.07, "weakspot")
        f.stroke([(-0.44, 0), (0.4, 0)], width=0.08 * f.k() * 28.4528, role="suit")
        head(f, 0.66, 0, 0, 0.24, "suit")
        for y in (0.12, -0.12):
            f.poly([(-0.66, 0.0), (-0.56, y * 1.4), (-0.48, y * 1.4), (-0.56, 0)], "coral")

    def gravity(f):
        pts = []
        for i in range(40):
            t = i / 39
            a = math.radians(90 + t * 600)
            r = 0.62 * (1 - t) + 0.1
            pts.append((r * math.cos(a), r * math.sin(a)))
        tube(f, pts, "rifthi", 0.09)
        f.circle(0, 0, 0.13, "rift")
        head(f, pts[0][0] - 0.01, pts[0][1] - 0.02, 180, 0.2, "rifthi")

    def echo(f):
        for dx, fill in ((-0.36, "badgehi"), (-0.18, "faunateal"), (0.0, "xp")):
            with f.frame(dx + 0.1, 0, s=0.5):
                f.poly([(-0.5, 0.8), (0.4, 0.8), (1.0, 0), (0.4, -0.8), (-0.5, -0.8), (0.1, 0)], fill)

    return ABILITIES_LIST(ignite, frost, shock, oil, scatter, bounce, homing, heavy, pierce, gravity, echo)


def FORMATIONS():
    def twin(f):
        for d in (0.2, -0.2):
            with f.frame(0, 0, rot=45):
                shot(f, 0.3, d * 1.25, 0.34, 0)

    def fan(f):
        f.circle(0, -0.5, 0.16, "gunmetal")
        for a in (130, 90, 50):
            r = math.radians(a)
            shot(f, 0.74 * math.cos(r), -0.5 + 0.74 * math.sin(r), 0.3, a)

    def ring_(f):
        for a in range(0, 360, 60):
            r = math.radians(a + 30)
            shot(f, 0.52 * math.cos(r), 0.52 * math.sin(r), 0.24, a + 30)
        f.circle(0, 0, 0.16, "suit")

    def behind(f):
        shot(f, -0.4, 0.04, 0.32, 180)
        f.rect(0.0, -0.5, 0.52, -0.04, "suitshade", r=0.16)
        f.circle(0.26, 0.16, 0.28, "suit")
        f.rect(0.2, 0.1, 0.44, 0.26, "visor", r=0.07)

    out = ABILITIES_LIST(twin, fan, behind)
    out["ring"] = ring_
    return {k: out[k] for k in ("twin", "fan", "ring", "behind")}


def TRIGGERS():
    def on_hit(f):
        trail(f, -0.22, 0, 0, lengths=(0.24, 0.34, 0.24), gap=0.14, start=0.1)
        burst(f, 0.2, 0, 0.22, 0.52, "goldhi", n=8, rot=90)
        f.circle(0.2, 0, 0.15, "amber")

    def on_expire(f):
        # a shot fading out as its countdown ring runs down
        ring(f, 0, 0, 0.44, 0.6, 150, 450, "badgehi", n=30)
        ring(f, 0, 0, 0.44, 0.6, 90, 150, "amber", n=8)
        f.circle(0, 0, 0.22, "amber")
        f.circle(-0.06, 0.06, 0.08, "goldhi")
        for a in (20, 200, 290):
            r = math.radians(a)
            f.circle(0.32 * math.cos(r), 0.32 * math.sin(r), 0.05, "amber")

    def on_kill(f):
        for a in (-1, 1):
            for dy in (0.08, -0.16):
                tube(f, [(0.2 * a, dy), (0.52 * a, dy - 0.16), (0.6 * a, dy - 0.34)], "violetdark", 0.07)
        f.ellipse(0, 0, 0.36, 0.32, "violet")
        M.eye(f, -0.12, 0.06, 0.08)
        M.eye(f, 0.12, 0.06, 0.08)
        for rot in (45, -45):
            with f.frame(0, 0, rot=rot):
                f.rect(-0.62, -0.07, 0.62, 0.07, "sensor", r=0.06)

    def each_stride(f):
        for i, (x, y) in enumerate(((-0.22, -0.5), (0.2, -0.16), (-0.18, 0.18), (0.22, 0.5))):
            with f.frame(x, y, s=0.95, rot=-8 if i % 2 else 8):
                f.ellipse(0, 0.04, 0.12, 0.16, "suit")
                f.ellipse(0, -0.18, 0.09, 0.07, "suit")

    def on_dash(f):
        for y, L in ((0.26, 0.4), (0, 0.6), (-0.26, 0.4)):
            f.stroke([(-0.68, y), (-0.68 + L, y)], width=3.0, role="suitshade")
        M.chevrons(f, -0.02, 0, 0.42, "xp", n=2)

    def on_rune(f):
        pts = [(0, 0.52), (0.5, -0.36), (-0.5, -0.36)]
        loop(f, pts, "amber", 0.06)
        for i in range(3):
            (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % 3]
            head(f, (x0 + x1) / 2 + (x1 - x0) * 0.08, (y0 + y1) / 2 + (y1 - y0) * 0.08,
                 math.degrees(math.atan2(y1 - y0, x1 - x0)), 0.17, "amber")
        spark(f, 0, 0.52, 0.22, "goldhi")

    return ABILITIES_LIST(on_hit, on_expire, on_kill, each_stride, on_dash, on_rune)


def RARES():
    def wormhole(f):
        for (dx, dy, rx, ry, fill) in ((0, 0, 0.56, 0.5, "xphi"), (0.08, 0.05, 0.44, 0.39, "xp"),
                                       (0.15, 0.09, 0.31, 0.27, "faunateal"), (0.2, 0.12, 0.18, 0.16, "visor")):
            f.ellipse(dx, dy, rx, ry, fill, rot=-20)
        f.ellipse(0.23, 0.14, 0.07, 0.06, "ink")

    def clockwork(f):
        pts = []
        for i in range(40):
            a = math.radians(i * 9)
            r = 0.58 if (i % 5) in (1, 2) else 0.46
            pts.append((r * math.cos(a), r * math.sin(a)))
        f.poly(pts, "suitshade")
        f.circle(0, 0, 0.34, "offwhite")
        f.stroke([(0, 0), (0, 0.24)], width=3.4, role="ink")
        f.stroke([(0, 0), (0.18, -0.1)], width=3.4, role="ink")
        f.circle(0, 0, 0.07, "amber")

    def overcharge(f):
        with f.frame(-0.04, -0.06, rot=0):
            f.rect(-0.5, -0.3, 0.38, 0.3, "suit", r=0.07)
            f.rect(0.38, -0.12, 0.5, 0.12, "gunmetal", r=0.03)
            for x in (-0.38, -0.14, 0.1):
                f.rect(x, -0.18, x + 0.18, 0.18, "acid", r=0.03)
        bolt_shape(f, 0.06, 0.12, 0.52, "hazard")
        spark(f, -0.5, 0.42, 0.14, "goldhi")
        spark(f, 0.5, 0.4, 0.12, "goldhi")

    def afterburst(f):
        burst(f, -0.3, -0.04, 0.2, 0.42, "coral", n=7, rot=10)
        f.circle(-0.3, -0.04, 0.15, "goldhi")
        shot(f, 0.4, -0.04, 0.3, 0)

    def mirror(f):
        f.stroke([(-0.52, -0.52), (0.52, 0.52)], width=1.8, role="goldhi")
        shot(f, -0.3, 0.3, 0.32, 135, "amber")
        shot(f, 0.3, -0.3, 0.32, -45, "xp", "xphi")
        f.circle(0, 0, 0.11, "goldhi")

    def phantom_shot(f):
        # a pale round passing through a brick wall; the bricks show through it
        f.rect(-0.18, -0.58, 0.18, 0.58, "rock", r=0.03)
        seams = [[(-0.18, 0.2), (0.18, 0.2)], [(-0.18, -0.2), (0.18, -0.2)], [(0, 0.2), (0, 0.58)], [(0, -0.2), (0, -0.58)]]
        for s in seams:
            f.stroke(s, width=1.5)
        shot(f, 0.26, 0, 0.42, 0, "rifthi", "suit")
        for s in seams[:2]:
            f.stroke(s, width=1.5)
        for x in (-0.18, 0.18):
            f.stroke([(x, -0.58), (x, 0.58)], width=1.5)

    return ABILITIES_LIST(wormhole, clockwork, overcharge, afterburst, mirror, phantom_shot)


for _kind, _group in (("modifier", MODIFIERS), ("formation", FORMATIONS), ("trigger", TRIGGERS), ("rare", RARES)):
    for _id, _draw in _group().items():
        PICTURES[f"icons/{_id}"] = make_part(_kind, _draw)


# -------------------------------------------------------------------- runes
# The shape as a walked trail that glows: a wide rift band with a bright
# core, and the walker as a small suit dot at one corner.

def rune(pts, walker, smooth=False):
    def draw(f):
        loop(f, pts, "rift", 0.16, smooth=smooth)
        f.stroke(pts, width=0.05 * f.k() * 28.4528, role="rifthi", smooth=smooth, closed=True)
        f.circle(walker[0], walker[1], 0.13, "suit")
    return draw


_TRI = [(0, 0.54), (0.52, -0.36), (-0.52, -0.36)]
PICTURES["icons/rune_circle"] = make_icon(rune(arc(0, 0, 0.5, 0, 360, 36)[:-1], (0.5, 0)), "teal")
PICTURES["icons/rune_triangle"] = make_icon(rune(_TRI, _TRI[1]), "teal")
PICTURES["icons/rune_square"] = make_icon(rune([(-0.44, -0.44), (0.44, -0.44), (0.44, 0.44), (-0.44, 0.44)], (0.44, -0.44)), "teal")


# ------------------------------------------------------------- ground fields
# A flat patch on the ground, three-quarter view, with no frame.

def field(draw):
    def mk(rng):
        fig = Fig("icon")
        with unit(fig, pad=0.9):
            draw(fig)
        return fig
    return mk


PATCH = (1.0, 0.92, 1.04, 0.95, 1.0, 0.9, 1.02, 0.96, 1.0, 0.93, 1.05, 0.94)


@field
def field_fire(f):
    f.blob(wobble(0, -0.32, 0.9, 0.5, PATCH), "coral", tension=0.6)
    f.blob(wobble(0, -0.32, 0.58, 0.3, PATCH[3:] + PATCH[:3]), "amber", tension=0.6)
    for x, y, s in ((-0.44, -0.08, 0.3), (0.14, 0.06, 0.4), (0.5, -0.32, 0.26), (-0.12, -0.5, 0.24)):
        flame(f, x, y + s * 0.6, s)


@field
def field_frost(f):
    f.blob(wobble(0, -0.32, 0.9, 0.5, PATCH), "xp", tension=0.6)
    f.blob(wobble(-0.1, -0.28, 0.5, 0.24, PATCH[5:] + PATCH[:5]), "xphi", tension=0.6)
    for x, y, h in ((-0.5, -0.2, 0.42), (-0.32, -0.3, 0.6), (0.3, -0.12, 0.5), (0.5, -0.36, 0.34), (0.06, -0.52, 0.36)):
        f.poly([(x - 0.08, y), (x, y + h), (x + 0.08, y), (x, y - 0.06)], "suit")


@field
def field_charge(f):
    f.blob(wobble(0, -0.32, 0.9, 0.5, PATCH), "violetdark", tension=0.6)
    f.blob(wobble(0, -0.32, 0.6, 0.3, PATCH[7:] + PATCH[:7]), "violet", tension=0.6)
    for x0, y0, x1, y1 in ((-0.62, -0.36, -0.3, 0.3), (-0.06, -0.62, 0.06, 0.4), (0.34, -0.42, 0.62, 0.16)):
        tube(f, zigzag(x0, y0, x1, y1, 4, 0.08), "hazard", 0.06, smooth=False)
    for x, y in ((-0.36, -0.6), (0.36, 0.0), (-0.28, 0.0)):
        spark(f, x, y, 0.1, "goldhi")


@field
def field_slick(f):
    f.blob(wobble(0, -0.32, 0.9, 0.5, PATCH), "gunmetal", tension=0.6)
    for (cx, cy, r, a0, a1, fill) in ((-0.1, -0.3, 0.52, 110, 200, "violethi"), (-0.1, -0.3, 0.38, 100, 190, "teal"),
                                      (0.1, -0.36, 0.5, -60, 20, "acid"), (0.1, -0.36, 0.36, -70, 10, "violethi")):
        tube(f, arc(cx, cy, r, a0, a1, 10, ry=r * 0.55), fill, 0.07)
    f.ellipse(-0.42, -0.12, 0.1, 0.05, "violethi")


PICTURES["icons/field_fire"] = field_fire
PICTURES["icons/field_frost"] = field_frost
PICTURES["icons/field_charge"] = field_charge
PICTURES["icons/field_slick"] = field_slick
