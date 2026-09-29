#!/usr/bin/env python3
"""Generate the website's feature illustrations as SVG.

Every scene is built from the same posable `person()` and a small set of props,
so the figures share one style and palette. Edit a scene below and re-run:

    python3 website/illustrations/draw.py      # writes website/static/illustrations/*.svg

Conventions: 480x300 canvas, ground at y=250. Arm angles are degrees from
hanging straight down, positive toward the direction the person faces. Instead
of angles, an arm can be given a point to reach (`near_reach=(x, y)`), and the
elbow is solved for, so hands land exactly on props.

Anything that rests on something (feet on the ground, a candle on a table) should
be placed from that surface's y, not by eye: render the scenes and check.
"""

import math
import pathlib

OUT = pathlib.Path(__file__).resolve().parent.parent / "static" / "illustrations"

W, H, GROUND = 480, 300, 250

# Palette: the app's warm paper and ink, plus soft clothing colours.
PAPER, CARD_EDGE, INK, ACCENT, GOLD = "#fbf4e8", "#ead9bf", "#2c1810", "#8b4513", "#c9a96e"
SAGE, ROSE, SLATE, PLUM, SKY = "#8a9a78", "#c27c6b", "#5f6f82", "#7a5c7e", "#dfe9ee"
FLOOR, WOOD, WOOD_DARK = "#f1e4cf", "#a8774d", "#8a5f3b"
SKIN = ["#f3cfb1", "#e0ae84", "#b97a51", "#7a4b2e"]
HAIR = ["#2c1810", "#6b3f23", "#a86b3a", "#1d1d1d", "#d8c6a4"]
FONT = "font-family=\"system-ui, -apple-system, 'Segoe UI', sans-serif\""


def f(n):
    return f"{n:.1f}".rstrip("0").rstrip(".")


# ── Primitives ────────────────────────────────────────────────────────────────

def line(x1, y1, x2, y2, color, width, cap="round", extra=""):
    return (f'<line x1="{f(x1)}" y1="{f(y1)}" x2="{f(x2)}" y2="{f(y2)}" stroke="{color}" '
            f'stroke-width="{f(width)}" stroke-linecap="{cap}" {extra}/>')


def circle(cx, cy, r, fill, extra=""):
    return f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}" fill="{fill}" {extra}/>'


def ellipse(cx, cy, rx, ry, fill, extra=""):
    return f'<ellipse cx="{f(cx)}" cy="{f(cy)}" rx="{f(rx)}" ry="{f(ry)}" fill="{fill}" {extra}/>'


def rect(x, y, w, h, fill, rx=0, extra=""):
    return f'<rect x="{f(x)}" y="{f(y)}" width="{f(w)}" height="{f(h)}" rx="{f(rx)}" fill="{fill}" {extra}/>'


def path(d, fill="none", stroke=None, width=0, extra=""):
    s = f' stroke="{stroke}" stroke-width="{f(width)}" stroke-linecap="round" stroke-linejoin="round"' if stroke else ""
    return f'<path d="{d}" fill="{fill}"{s} {extra}/>'


def text(x, y, s, size=13, color=INK, anchor="middle", weight="600"):
    return (f'<text x="{f(x)}" y="{f(y)}" {FONT} font-size="{size}" font-weight="{weight}" '
            f'fill="{color}" text-anchor="{anchor}">{s}</text>')


def dashed(d, color=ACCENT, width=2.5):
    return path(d, stroke=color, width=width, extra='stroke-dasharray="2 7"')


def limb(p1, p2, w1, w2, color):
    """A tapered segment with rounded ends: w1 wide at p1, w2 wide at p2."""
    (x1, y1), (x2, y2) = p1, p2
    n = math.hypot(x2 - x1, y2 - y1) or 1
    nx, ny = -(y2 - y1) / n, (x2 - x1) / n
    a, b = w1 / 2, w2 / 2
    return path(f"M {f(x1 + nx * a)} {f(y1 + ny * a)} L {f(x2 + nx * b)} {f(y2 + ny * b)} "
                f"A {f(b)} {f(b)} 0 0 0 {f(x2 - nx * b)} {f(y2 - ny * b)} "
                f"L {f(x1 - nx * a)} {f(y1 - ny * a)} A {f(a)} {f(a)} 0 0 0 {f(x1 + nx * a)} {f(y1 + ny * a)} Z",
                fill=color)


def polar(x, y, length, angle, facing):
    """Point `length` away from (x, y) at `angle` degrees from straight down."""
    a = math.radians(angle)
    return x + math.sin(a) * length * facing, y + math.cos(a) * length


def lerp(p, q, t):
    return p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t


def rotate(p, deg, about):
    """Rotate p clockwise on screen by deg around `about` (what SVG's rotate() does)."""
    a = math.radians(deg)
    dx, dy = p[0] - about[0], p[1] - about[1]
    return about[0] + dx * math.cos(a) - dy * math.sin(a), about[1] + dx * math.sin(a) + dy * math.cos(a)


def shade(hex_color, k=0.82):
    r, g, b = (int(hex_color[i:i + 2], 16) for i in (1, 3, 5))
    return "#%02x%02x%02x" % tuple(min(255, int(c * k)) for c in (r, g, b))


# ── People ────────────────────────────────────────────────────────────────────

class Pose:
    """Where a drawn person's hands and head ended up, for placing props."""

    def __init__(self, near_hand, far_hand, head):
        self.near_hand, self.far_hand, self.head = near_hand, far_hand, head


UPPER_ARM, FOREARM = 29, 27  # shoulder→elbow, elbow→centre of the hand


def reach(shoulder, target, fc, s):
    """(shoulder angle, elbow bend) that puts the hand on target, elbow down."""
    ox, oy = shoulder
    l1, l2 = UPPER_ARM * s, FOREARM * s
    d = min(max(math.hypot(target[0] - ox, target[1] - oy), abs(l1 - l2) + 0.01), l1 + l2 - 0.01)
    phi = math.degrees(math.atan2((target[0] - ox) * fc, target[1] - oy))
    alpha = math.degrees(math.acos((l1 * l1 + d * d - l2 * l2) / (2 * l1 * d)))
    best = None
    for a1 in (phi + alpha, phi - alpha):
        ex, ey = polar(ox, oy, l1, a1, fc)
        a2 = math.degrees(math.atan2((target[0] - ex) * fc, target[1] - ey))
        if best is None or ey > best[0]:
            best = (ey, (a1, a2 - a1))
    return best[1]


def shoe(ax, ground, fc, s, color):
    """A shoe whose sole sits exactly on `ground`, toe toward the facing side."""
    def p(dx, dy):
        return f"{f(ax + dx * s * fc)} {f(ground + dy * s)}"
    return (path(f"M {p(-6, 0)} L {p(12, 0)} Q {p(13.5, -6)} {p(5, -7.5)} L {p(-2, -9.5)} "
                 f"Q {p(-7, -9)} {p(-6.5, -4)} Z", fill=color)
            + line(ax - 6 * s * fc, ground - 1.2 * s, ax + 12 * s * fc, ground - 1.2 * s, "#5a4636", 1.8 * s, cap="butt"))


def person(x, *, facing=1, skin=SKIN[0], hair=HAIR[0], hair_style="short",
           shirt=SLATE, pants=INK, near_arm=(10, 0), far_arm=(-8, 0),
           near_reach=None, far_reach=None, sleeves="short", shoes=INK,
           sit=False, lean=0, s=1.0, ground=GROUND):
    """Draw a person standing (or sitting) on the ground at x.

    near_arm / far_arm = (shoulder angle, elbow bend) in degrees; near_reach /
    far_reach = a point for that hand to reach instead. `lean` tips the upper
    body forward (degrees) about the hips. Returns the SVG and a Pose with
    hand/head positions.
    """
    fc = facing
    legs = []

    def leg(hip, knee, ankle, color, shoe_color):
        return (limb(hip, knee, 14.5 * s, 12.5 * s, color) + limb(knee, ankle, 12.5 * s, 10.5 * s, color)
                + shoe(ankle[0], ground, fc, s, shoe_color))

    if sit:
        hip_y = ground - 52 * s
        for far, dx in ((True, -3), (False, 3)):
            hip = (x + dx * s, hip_y)
            knee = (x + (33 + dx) * s * fc, hip_y + 1 * s)
            ankle = (x + (35 + dx) * s * fc, ground - 7 * s)
            legs.append(leg(hip, knee, ankle, shade(pants, 0.8) if far else pants,
                            shade(shoes, 0.8) if far else shoes))
    else:
        hip_y = ground - 80 * s
        for far, hx_, kx, ax in ((True, -5, -5, -6), (False, 5, 6, 7)):
            hip = (x + hx_ * s * fc, hip_y)
            knee = (x + kx * s * fc, hip_y + 38 * s)
            ankle = (x + ax * s * fc, ground - 7 * s)
            legs.append(leg(hip, knee, ankle, shade(pants, 0.8) if far else pants,
                            shade(shoes, 0.8) if far else shoes))

    pivot = (x, hip_y)
    tilt = lean * fc
    sy = hip_y - 60 * s                    # shoulder line
    hx, hy = x + 3 * s * fc, sy - 24 * s   # head centre

    def local(p):
        return rotate(p, -tilt, pivot)

    def world(p):
        return rotate(p, tilt, pivot)

    def arm(angles, target, far):
        shoulder = (x + (-11 if far else 11) * s * fc, sy + 9 * s)
        if target is not None:
            angles = reach(shoulder, local(target), fc, s)
        a1, bend = angles
        elbow = polar(*shoulder, UPPER_ARM * s, a1, fc)
        hand = polar(*elbow, FOREARM * s, a1 + bend, fc)
        wrist = lerp(elbow, hand, 0.8)
        sleeve = shade(shirt, 0.85) if far else shirt
        tone = shade(skin, 0.85) if far else skin
        out = []
        if sleeves == "long":
            out.append(limb(shoulder, elbow, 13 * s, 11 * s, sleeve))
            out.append(limb(elbow, wrist, 11 * s, 9.5 * s, sleeve))
            out.append(limb(lerp(elbow, wrist, 0.85), wrist, 10.5 * s, 10 * s, shade(sleeve, 0.9)))
        else:
            out.append(limb(shoulder, elbow, 10 * s, 9 * s, tone))
            out.append(limb(elbow, wrist, 9 * s, 7.5 * s, tone))
            out.append(limb(shoulder, lerp(shoulder, elbow, 0.62), 13.5 * s, 12.5 * s, sleeve))
        # Hand: a palm and a thumb on the upper side of the forearm.
        dx, dy = hand[0] - elbow[0], hand[1] - elbow[1]
        n = math.hypot(dx, dy) or 1
        side = -1 if dy * fc > 0 or dx * fc < 0 else 1
        thumb = (hand[0] + (-dy / n) * 4 * s * side * fc - dx / n * 1.5 * s,
                 hand[1] + (dx / n) * 4 * s * side * fc - dy / n * 1.5 * s)
        out.append(limb(wrist, hand, 7.5 * s, 10.5 * s, tone))
        out.append(circle(*thumb, 2.4 * s, tone))
        return out, hand

    far_svg, far_hand = arm(far_arm, far_reach, True)
    near_svg, near_hand = arm(near_arm, near_reach, False)

    up = list(far_svg)
    # Neck, then the torso: broad shoulders tapering to a hem below the hips.
    up.append(limb((x + 1 * s * fc, sy + 4 * s), (hx - 1 * s * fc, hy + 12 * s), 12 * s, 9.5 * s, shade(skin, 0.9)))

    def tp(dx, dy):
        return f"{f(x + dx * s * fc)} {f(dy)}"
    hem = hip_y + 10 * s
    up.append(path(f"M {tp(-14, sy)} Q {tp(-21, sy)} {tp(-21, sy + 11 * s)} L {tp(-18, hem - 5 * s)} "
                   f"Q {tp(-18, hem)} {tp(-13, hem)} L {tp(14, hem)} Q {tp(19, hem)} {tp(19, hem - 5 * s)} "
                   f"L {tp(21, sy + 11 * s)} Q {tp(21, sy)} {tp(14, sy)} Z", fill=shirt))
    # Back-side shading for a little depth, a hem line and the neckline.
    up.append(path(f"M {tp(-14, sy)} Q {tp(-21, sy)} {tp(-21, sy + 11 * s)} L {tp(-18, hem - 5 * s)} "
                   f"Q {tp(-18, hem)} {tp(-13, hem)} L {tp(-10, hem)} L {tp(-13, sy + 12 * s)} Q {tp(-12, sy + 2 * s)} {tp(-8, sy)} Z",
                   fill=shade(shirt, 0.9)))
    up.append(line(x - 16 * s, hem - 4 * s, x + 16 * s, hem - 4 * s, shade(shirt, 0.88), 1.6 * s, cap="butt"))
    up.append(path(f"M {tp(-5, sy + 0.5 * s)} Q {tp(1, sy + 8 * s)} {tp(6.5, sy + 0.5 * s)}",
                   fill=shade(skin, 0.9), stroke=shade(shirt, 0.78), width=1.8 * s))

    def hp(dx, dy):
        return f"{f(hx + dx * s * fc)} {f(hy + dy * s)}"

    if hair_style == "long":   # the fall of hair behind the head, down to the shoulders
        up.append(path(f"M {hp(0, -17)} C {hp(-21, -17)} {hp(-23, 12)} {hp(-20, 30)} "
                       f"Q {hp(-12, 33)} {hp(-5, 29)} Q {hp(-6, 14)} {hp(2, 4)} Z", fill=shade(hair, 0.9)))
    # Head, nose and ear.
    up.append(path(f"M {hp(15.2, -1)} Q {hp(20.5, 4.5)} {hp(15, 6.5)} Z", fill=skin))
    up.append(ellipse(hx, hy, 16.5 * s, 17.5 * s, skin))
    ear = (ellipse(hx - 7.5 * s * fc, hy + 2 * s, 3 * s, 4.4 * s, shade(skin, 0.94))
           + path(f"M {hp(-6.5, 0)} q {f(-1.8 * s * fc)} {f(2 * s)} 0 {f(4 * s)}", stroke=shade(skin, 0.8), width=1 * s))
    cap = (f"M {hp(15.5, -5)} C {hp(14, -22)} {hp(-18, -25)} {hp(-17.5, 2)} L {hp(-11, 6)} "
           f"Q {hp(-8, -5)} {hp(2, -8)} Q {hp(10, -9.5)} {hp(15.5, -5)} Z")
    if hair_style == "long":
        up.append(path(f"M {hp(16, -3)} C {hp(14, -23)} {hp(-18.5, -25)} {hp(-18, 6)} L {hp(-9, 8)} "
                       f"Q {hp(-6, -6)} {hp(5, -9)} Q {hp(12, -9)} {hp(16, -3)} Z", fill=hair))
    elif hair_style == "bun":
        up.append(ear)
        up.append(circle(hx - 12 * s * fc, hy - 16 * s, 7.5 * s, hair))
        up.append(path(cap, fill=hair))
        up.append(path(f"M {hp(-8, -19)} Q {hp(-10, -14)} {hp(-15, -12)}", stroke=shade(hair, 1.25), width=1.6 * s))
    elif hair_style == "curly":
        up.append(ear)
        up.append(path(cap, fill=hair))
        for dx, dy, r in ((-14, -8, 7), (-9, -15, 7.5), (0, -18, 7), (8, -16, 6.5), (13, -10, 5),
                          (-17, 0, 6.5), (-15, 7, 5)):
            up.append(circle(hx + dx * s * fc, hy + dy * s, r * s, hair))
    elif hair_style == "short":
        up.append(ear)
        up.append(path(cap, fill=hair))
    else:  # bald
        up.append(ear)
        up.append(ellipse(hx + 2 * s * fc, hy - 11 * s, 5 * s, 2.5 * s, "#ffffff", 'opacity="0.25"'))
    # Face: brow, eye with a catch-light, cheek, smile.
    up.append(path(f"M {hp(4.5, -5.5)} Q {hp(8, -7.5)} {hp(11, -5.5)}", stroke=shade(hair, 0.9), width=1.7 * s))
    up.append(ellipse(hx + 8 * s * fc, hy, 1.8 * s, 2.3 * s, INK))
    up.append(circle(hx + 8.6 * s * fc, hy - 0.8 * s, 0.6 * s, "#ffffff"))
    up.append(circle(hx + 8 * s * fc, hy + 6.5 * s, 3.2 * s, ROSE, 'opacity="0.3"'))
    up.append(path(f"M {hp(5, 9)} q {f(3.5 * s * fc)} {f(3 * s)} {f(7.5 * s * fc)} 0", stroke=INK, width=1.6 * s))
    up += near_svg

    body = "".join(legs)
    if tilt:
        body += f'<g transform="rotate({f(tilt)} {f(pivot[0])} {f(pivot[1])})">{"".join(up)}</g>'
    else:
        body += "".join(up)
    return body, Pose(world(near_hand), world(far_hand), world((hx, hy)))


def shadow(x, w=34, ground=GROUND):
    return ellipse(x, ground + 1, w, 5, INK, 'opacity="0.08"')


# ── Props ─────────────────────────────────────────────────────────────────────

def phone(cx, cy, w=22, h=38, screen="#fdf8f0", tilt=0, content=""):
    g = [rect(cx - w / 2, cy - h / 2, w, h, INK, rx=4),
         rect(cx - w / 2 + 2.5, cy - h / 2 + 3.5, w - 5, h - 8, screen, rx=2),
         rect(cx - 3, cy - h / 2 + 1.3, 6, 1.2, "#5a4636", rx=0.6), content]
    return f'<g transform="rotate({f(tilt)} {f(cx)} {f(cy)})">{"".join(g)}</g>'


QR_PATTERN = [
    "1110101", "1010010", "1110111", "0001000", "1101011", "0101101", "1110110",
]


def qr(cx, cy, size):
    cell = size / 7
    out = [rect(cx - size / 2 - 1.5, cy - size / 2 - 1.5, size + 3, size + 3, "#ffffff", rx=1)]
    for r, row in enumerate(QR_PATTERN):
        for c, bit in enumerate(row):
            if bit == "1":
                out.append(rect(cx - size / 2 + c * cell, cy - size / 2 + r * cell, cell, cell, INK))
    return "".join(out)


def book_open(cx, bottom, w=70, h=34, lines=True):
    """An open book whose lower edge rests on `bottom`."""
    half, top = w / 2, bottom - h
    out = []
    for d in (-1, 1):
        edge = cx + half * d
        # Page block under the top leaf, so the book has some thickness.
        out.append(path(f"M {f(cx)} {f(bottom)} Q {f(cx + half / 2 * d)} {f(bottom - 5)} {f(edge)} {f(bottom)} "
                        f"L {f(edge)} {f(bottom + 3)} L {f(cx)} {f(bottom + 3)} Z", fill=CARD_EDGE))
        out.append(path(f"M {f(cx)} {f(top + 3)} Q {f(cx + half / 2 * d)} {f(top - 3)} {f(edge)} {f(top + 1)} "
                        f"L {f(edge)} {f(bottom)} Q {f(cx + half / 2 * d)} {f(bottom - 5)} {f(cx)} {f(bottom)} Z",
                        fill="#fffaf0", stroke=ACCENT, width=2))
    out.append(line(cx, top + 4, cx, bottom - 1, shade(CARD_EDGE, 0.9), 1.4))
    if lines:
        for i in range(3):
            y = top + 10 + i * 7
            out.append(line(cx - half + 7, y, cx - 6, y + 1, GOLD, 1.6))
            out.append(line(cx + 6, y + 1, cx + half - 7, y, GOLD, 1.6))
    return "".join(out)


def envelope(cx, cy, w=34, h=22, tilt=0):
    g = (rect(cx - w / 2, cy - h / 2, w, h, "#fffaf0", rx=2, extra=f'stroke="{ACCENT}" stroke-width="2"')
         + path(f"M {f(cx - w / 2)} {f(cy - h / 2)} L {f(cx)} {f(cy + 2)} L {f(cx + w / 2)} {f(cy - h / 2)}",
                stroke=ACCENT, width=2))
    return f'<g transform="rotate({f(tilt)} {f(cx)} {f(cy)})">{g}</g>'


def heart(cx, cy, r=7, color=ROSE):
    return path(f"M {f(cx)} {f(cy + r)} C {f(cx - r * 2)} {f(cy - r * 0.2)} {f(cx - r * 0.9)} {f(cy - r * 1.6)} {f(cx)} {f(cy - r * 0.5)} "
                f"C {f(cx + r * 0.9)} {f(cy - r * 1.6)} {f(cx + r * 2)} {f(cy - r * 0.2)} {f(cx)} {f(cy + r)} Z", fill=color)


def sparkle(cx, cy, r=7, color=GOLD):
    return path(f"M {f(cx)} {f(cy - r)} Q {f(cx + r * 0.18)} {f(cy - r * 0.18)} {f(cx + r)} {f(cy)} "
                f"Q {f(cx + r * 0.18)} {f(cy + r * 0.18)} {f(cx)} {f(cy + r)} "
                f"Q {f(cx - r * 0.18)} {f(cy + r * 0.18)} {f(cx - r)} {f(cy)} "
                f"Q {f(cx - r * 0.18)} {f(cy - r * 0.18)} {f(cx)} {f(cy - r)} Z", fill=color)


def sun(cx, cy, r=22):
    out = [circle(cx, cy, r, "#f2c14e", 'opacity="0.9"')]
    for i in range(12):
        a = math.radians(i * 30)
        out.append(line(cx + math.cos(a) * (r + 6), cy + math.sin(a) * (r + 6),
                        cx + math.cos(a) * (r + 15), cy + math.sin(a) * (r + 15), "#f2c14e", 3.5))
    return "".join(out)


def bird(cx, cy, s=1.0, color=INK):
    return path(f"M {f(cx - 12 * s)} {f(cy)} Q {f(cx - 6 * s)} {f(cy - 8 * s)} {f(cx)} {f(cy)} "
                f"Q {f(cx + 6 * s)} {f(cy - 8 * s)} {f(cx + 12 * s)} {f(cy)}", stroke=color, width=2.6 * s)


def tree(x, ground=GROUND, s=1.0):
    return (path(f"M {f(x - 6 * s)} {f(ground)} L {f(x - 4 * s)} {f(ground - 72 * s)} L {f(x + 4 * s)} {f(ground - 72 * s)} "
                 f"L {f(x + 6 * s)} {f(ground)} Z", fill="#7a5a3c")
            + circle(x, ground - 92 * s, 34 * s, SAGE)
            + circle(x - 22 * s, ground - 74 * s, 22 * s, shade(SAGE, 0.92))
            + circle(x + 22 * s, ground - 76 * s, 24 * s, shade(SAGE, 0.88)))


TABLE_TOP = GROUND - 62


def table(x, w=110, top=TABLE_TOP, inset=10):
    """A side-on table; its surface is y=top (put things there)."""
    legs = "".join(line(lx, top + 8, lx, GROUND, WOOD_DARK, 6, cap="butt")
                   for lx in (x - w / 2 + inset, x + w / 2 - inset))
    return (legs + rect(x - w / 2 + 4, top + 7, w - 8, 6, WOOD_DARK)
            + rect(x - w / 2, top, w, 8, WOOD, rx=3))


def chair(x, facing=1, seat=GROUND - 50):
    """A side-on chair; the sitter's hips (x) sit on `seat`, its back behind them."""
    back = x - 21 * facing
    front = x + 14 * facing
    return (line(back, seat - 46, back, GROUND, WOOD_DARK, 6, cap="butt")
            + rect(min(back, back + 6 * facing) - 3, seat - 46, 12, 6, WOOD, rx=2)
            + line(front, seat + 7, front, GROUND, WOOD_DARK, 6, cap="butt")
            + rect(min(back, front) - 3, seat, abs(front - back) + 6, 7, WOOD, rx=3))


def laptop(cx, base_y, facing=1):
    """A laptop sitting on `base_y`, its screen tipped back away from the facing side."""
    fc = facing
    hx = cx + 26 * fc

    def p(dx, dy):
        return f"{f(hx + dx * fc)} {f(base_y + dy)}"
    return (path(f"M {f(cx - 28 * fc)} {f(base_y)} L {p(0, 0)} L {p(-2, -4)} L {f(cx - 24 * fc)} {f(base_y - 4)} Z", fill="#9aa3ad")
            + line(cx - 20 * fc, base_y - 3.2, hx - 10 * fc, base_y - 3.2, "#7d8690", 1.2, cap="butt")
            + path(f"M {p(-2, -3)} L {p(-14, -42)} L {p(-2, -45)} L {p(8, -41)} L {p(1, 0)} Z", fill=INK)
            + path(f"M {p(-3, -7)} L {p(-12, -38)} L {p(-3, -40.5)} L {p(4, -38)} L {p(-1, -5)} Z", fill="#e8eef3")
            + line(hx - 9 * fc, base_y - 32, hx - 2 * fc, base_y - 34, GOLD, 1.4)
            + line(hx - 7 * fc, base_y - 25, hx - 1 * fc, base_y - 27, GOLD, 1.4))


def bubble(cx, cy, w, h, tail_x, tail_y, fill="#ffffff"):
    return (rect(cx - w / 2, cy - h / 2, w, h, fill, rx=12, extra=f'stroke="{CARD_EDGE}" stroke-width="2"')
            + path(f"M {f(cx - 8)} {f(cy + h / 2 - 1)} L {f(tail_x)} {f(tail_y)} L {f(cx + 8)} {f(cy + h / 2 - 1)} Z",
                   fill=fill, stroke=CARD_EDGE, width=2)
            + rect(cx - 9, cy + h / 2 - 3, 18, 4, fill))


def page_card(x, y, w, h, lines=3):
    """A loose page (an entry) floating in the scene."""
    out = [rect(x, y, w, h, "#fffaf0", rx=6, extra=f'stroke="{ACCENT}" stroke-width="2"')]
    for i in range(lines):
        out.append(line(x + 14, y + 16 + i * 10, x + w - 20 - i * 20, y + 16 + i * 10, GOLD, 2))
    return "".join(out)


def svg(title, desc, body, sky=None):
    # Everything is clipped to the rounded card, and the card's edge is drawn
    # last, so no fill can poke past (or cover) the border at the corners.
    bg = [rect(0, 0, W, H, PAPER)]
    if sky:
        bg.append(rect(0, 0, W, GROUND, sky))
    bg.append(rect(0, GROUND, W, H - GROUND, FLOOR))
    bg.append(line(0, GROUND, W, GROUND, CARD_EDGE, 2, cap="butt"))
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
            f'role="img" aria-labelledby="t d">\n<title id="t">{title}</title>\n<desc id="d">{desc}</desc>\n'
            f'<defs><clipPath id="card">{rect(1, 1, W - 2, H - 2, "#000", rx=18)}</clipPath></defs>\n'
            f'<g clip-path="url(#card)">' + "".join(bg) + "\n" + body + "</g>\n"
            + rect(1, 1, W - 2, H - 2, "none", rx=18, extra=f'stroke="{CARD_EDGE}" stroke-width="2"')
            + "\n</svg>\n")


# ── Scenes ────────────────────────────────────────────────────────────────────

def scene_write():
    book_x = 232
    body = [chair(140, facing=1), table(240, w=150, inset=34), shadow(160, 44)]
    body.append(book_open(book_x, TABLE_TOP, w=86, h=30))
    p, pose = person(140, facing=1, sit=True, lean=14, skin=SKIN[1], hair=HAIR[1], hair_style="long",
                     shirt=ROSE, pants=SLATE, sleeves="long",
                     near_reach=(book_x - 20, TABLE_TOP - 16), far_reach=(book_x - 44, TABLE_TOP - 5))
    body.append(p)
    # Pen in hand, its tip on the left-hand page.
    hx, hy = pose.near_hand
    body.append(line(hx - 7, hy - 11, hx + 4, hy + 7, INK, 3))
    # A candle for warmth, standing on the table.
    cx = 300
    body.append(ellipse(cx, TABLE_TOP - 1.5, 10, 2.5, GOLD))
    body.append(rect(cx - 5, TABLE_TOP - 24, 10, 22, "#fffaf0", rx=2, extra=f'stroke="{CARD_EDGE}" stroke-width="1.5"'))
    body.append(line(cx, TABLE_TOP - 24, cx, TABLE_TOP - 28, INK, 1.4))
    body.append(path(f"M {cx} {TABLE_TOP - 42} q 6 8 0 14 q -6 -6 0 -14 Z", fill="#f2c14e"))
    body.append(circle(cx, TABLE_TOP - 33, 16, "#f2c14e", 'opacity="0.12"'))
    body.append(text(240, 34, "Write it down: a prayer, or a thanksgiving", 15))
    return svg("Writing a prayer",
               "Someone sits at a table writing in an open prayer book by candlelight.", "".join(body))


def scene_book():
    body = [shadow(150, 36)]
    p, pose = person(150, facing=1, skin=SKIN[2], hair=HAIR[3], hair_style="short",
                     shirt=SAGE, near_arm=(38, 70), far_arm=(-6, 0))
    body.append(p)
    hx, hy = pose.near_hand
    body.append(phone(hx + 6, hy - 10, tilt=8))
    # A single page, its corner lifting as it turns.
    x0, y0, x1, y1, curl = 270, 70, 420, 220, 46
    body.append(path(f"M {x0 + 6} {y0} L {x1 - 6} {y0} Q {x1} {y0} {x1} {y0 + 6} L {x1} {y1 - curl} "
                     f"L {x1 - curl} {y1} L {x0 + 6} {y1} Q {x0} {y1} {x0} {y1 - 6} L {x0} {y0 + 6} Q {x0} {y0} {x0 + 6} {y0} Z",
                     fill="#fffaf0", stroke=ACCENT, width=2))
    for i in range(6):
        body.append(line(288, 100 + i * 17, 400 - (i % 3) * 18 - (22 if i == 5 else 0), 100 + i * 17, GOLD, 2))
    body.append(path(f"M {x1} {y1 - curl} Q {x1 - 30} {y1 - 30} {x1 - curl} {y1} Q {x1 - 20} {y1 - 18} {x1} {y1 - curl} Z",
                     fill="#efe0c6", stroke=ACCENT, width=2))
    body.append(path(f"M {x1} {y1 - curl} L {x1 - curl} {y1}", stroke=shade(CARD_EDGE, 0.9), width=1.2))
    # The swipe: from the page back toward the phone.
    body.append(path("M 258 152 Q 226 124 196 142", stroke=ACCENT, width=2.5))
    body.append(path("M 205 134 L 195 143 L 208 146", stroke=ACCENT, width=2.5))
    body.append(text(345, 242, "one page at a time", 12, ACCENT, weight="500"))
    body.append(text(240, 34, "A book, not a feed: swipe to turn the page", 15))
    return svg("Paging through the book",
               "Someone swipes on their phone and a single page of the prayer book turns over.",
               "".join(body))


def scene_qr():
    body = [tree(430, s=0.8), shadow(140, 36), shadow(340, 36)]
    left, lp = person(140, facing=1, skin=SKIN[0], hair=HAIR[2], hair_style="bun",
                      shirt=PLUM, pants=SLATE, sleeves="long", near_reach=(196, 142), far_arm=(-8, 0))
    right, rp = person(340, facing=-1, skin=SKIN[3], hair=HAIR[0], hair_style="curly",
                       shirt=GOLD, pants=INK, near_reach=(288, 140), far_arm=(-6, 0))
    body += [left, right]
    # Left holds up a phone showing the group's QR code, screen toward the right.
    lx, ly = lp.near_hand
    qx, qy = lx + 4, ly - 16
    body.append(phone(qx, qy, w=30, h=46, content=qr(qx, qy - 2, 20)))
    # Right points a phone at it; dashed scan lines between the two.
    rx, ry = rp.near_hand
    body.append(phone(rx - 4, ry - 14, w=22, h=36, screen="#e8eef3", tilt=-10))
    for dy in (-12, 0, 12):
        body.append(dashed(f"M {f(rx - 18)} {f(ry - 14 + dy / 2)} L {f(qx + 17)} {f(qy + dy)}", color=ACCENT, width=2))
    body.append(sparkle(242, 82, 8))
    body.append(text(240, 34, "Scan to join: groups start in real space", 15))
    return svg("Joining a group by QR code",
               "Two people stand together outside. One holds up a phone showing the group's QR code; "
               "the other scans it with their own phone to join.", "".join(body), sky=SKY)


def house(hx, door_side):
    wall = shade(PAPER, 0.95)
    dx = hx + 30 * door_side
    return (rect(hx + 22, GROUND - 166, 12, 40, shade(PAPER, 0.86))
            + rect(hx + 20, GROUND - 168, 16, 4, shade(PAPER, 0.82), rx=1)
            + rect(hx - 55, GROUND - 120, 110, 120, wall)
            + path(f"M {hx - 66} {GROUND - 118} L {hx} {GROUND - 164} L {hx + 66} {GROUND - 118} Z", fill=shade(PAPER, 0.9))
            + rect(dx - 13, GROUND - 58, 26, 58, shade(PAPER, 0.88), rx=2)
            + circle(dx + 7 * door_side, GROUND - 28, 1.8, GOLD)
            + rect(hx - 44 if door_side > 0 else hx + 26, GROUND - 78, 18, 18, SKY, rx=2))


def scene_invite():
    body = [house(110, -1), house(375, 1), shadow(110, 34), shadow(375, 34)]
    left, lp = person(110, facing=1, skin=SKIN[1], hair=HAIR[0], hair_style="short",
                      shirt=SLATE, near_arm=(140, 20), far_arm=(-6, 0))
    right, rp = person(375, facing=-1, skin=SKIN[0], hair=HAIR[4], hair_style="long",
                       shirt=ROSE, sleeves="long", near_reach=(340, 138), far_arm=(-6, 0))
    body += [left, right]
    hx, hy = lp.near_hand
    rx, ry = rp.near_hand
    body.append(dashed(f"M {f(hx + 12)} {f(hy - 4)} Q 240 30 {f(rx - 4)} {f(ry - 36)}"))
    body.append(envelope(240, 66, tilt=-6))
    body.append(phone(rx - 4, ry - 12, tilt=-6, content=envelope(rx - 4, ry - 13, w=12, h=8)))
    body.append(text(240, 34, "Invite by email or text", 15))
    return svg("Inviting someone to a group",
               "Someone waves, and an invitation envelope flies across to a friend in another home, "
               "who opens it on their phone.", "".join(body))


def scene_share():
    body = []
    # The rug lies on the floor under the group's feet.
    body.append(ellipse(330, GROUND + 4, 118, 9, GOLD, 'opacity="0.4"'))
    body.append(ellipse(330, GROUND + 4, 104, 6, "none", f'stroke="{GOLD}" stroke-width="1.5" opacity="0.6"'))
    # The group, turned toward the page coming their way.
    group = ((272, SKIN[3], HAIR[3], "short", SAGE, "short", (246, 162)),
             (336, SKIN[1], HAIR[1], "bun", PLUM, "long", None),
             (400, SKIN[0], HAIR[2], "long", SLATE, "short", None))
    for x, sk, hr, st, sh, sl, target in group:
        body.append(person(x, facing=-1, skin=sk, hair=hr, hair_style=st, shirt=sh, sleeves=sl,
                           near_arm=(8, 10), far_arm=(-6, 0), near_reach=target, s=0.86)[0])
    # Someone offers a page to the group.
    body.append(shadow(105, 34))
    p, pose = person(105, facing=1, skin=SKIN[2], hair=HAIR[0], hair_style="curly",
                     shirt=ROSE, near_reach=(166, 126), far_arm=(-6, 0))
    body.append(p)
    hx, hy = pose.near_hand
    body.append(f'<g transform="rotate(-8 {f(hx + 20)} {f(hy - 6)})">'
                + rect(hx - 2, hy - 32, 40, 50, "#fffaf0", rx=3, extra=f'stroke="{ACCENT}" stroke-width="2"')
                + "".join(line(hx + 5, hy - 22 + i * 9, hx + 31, hy - 22 + i * 9, GOLD, 1.6) for i in range(4))
                + "</g>")
    # Re-draw the gripping hand over the page so the page sits in it.
    body.append(circle(hx, hy, 5.2, SKIN[2]))
    body.append(dashed(f"M {f(hx + 30)} {f(hy - 42)} Q 220 60 290 92"))
    body.append(heart(318, 92, 7))
    body.append(text(240, 34, "Share a prayer with your group", 15))
    return svg("Sharing a prayer with a group",
               "Someone hands a written prayer toward three friends gathered together, sharing it with the group.",
               "".join(body))


def room(x0, w):
    """A window-lit wall panel that reaches down to the floor."""
    return (rect(x0, 56, w, GROUND - 56, shade(PAPER, 0.97))
            + rect(x0 + w / 2 - 38, 74, 76, 70, SKY, rx=4, extra=f'stroke="{CARD_EDGE}" stroke-width="4"')
            + line(x0 + w / 2, 74, x0 + w / 2, 144, CARD_EDGE, 3)
            + line(x0 + w / 2 - 38, 109, x0 + w / 2 + 38, 109, CARD_EDGE, 3)
            + path(f"M {x0} {GROUND} L {x0} 60 Q {x0} 56 {x0 + 4} 56 L {x0 + w - 4} 56 Q {x0 + w} 56 {x0 + w} 60 L {x0 + w} {GROUND}",
                   stroke=CARD_EDGE, width=3))


def scene_praying_now():
    body = [room(22, 196), room(262, 196)]
    body.append(line(240, 60, 240, GROUND - 6, CARD_EDGE, 3, extra='stroke-dasharray="4 8"'))
    body.append(shadow(92, 30))
    p, pose = person(92, facing=1, skin=SKIN[3], hair=HAIR[3], hair_style="short",
                     shirt=GOLD, near_reach=(138, 164), far_arm=(-6, 0), s=0.95)
    body.append(p)
    hx, hy = pose.near_hand
    body.append(phone(hx + 4, hy - 12, content=text(hx + 4, hy - 7, "🙏", 12)))
    body.append(shadow(388, 30))
    q, qpose = person(388, facing=-1, skin=SKIN[0], hair=HAIR[2], hair_style="long",
                      shirt=PLUM, sleeves="long", near_reach=(342, 164), far_arm=(-6, 0), s=0.95)
    body.append(q)
    qx, qy = qpose.near_hand
    body.append(phone(qx - 4, qy - 12, content=text(qx - 4, qy - 7, "3", 12, ACCENT)))
    # +1s drifting across.
    for i, (x, y) in enumerate(((178, 176), (212, 190), (282, 186))):
        body.append(text(x, y, "+1", 14 - i, ACCENT, weight="700"))
    body.append(heart(240, 170, 6))
    body.append(text(240, 34, "Praying now: tap 🙏 every time you pray", 15))
    return svg("Praying now",
               "Two people in different places. One taps the praying-now button on their phone, and the "
               "other sees the count of prayers for their request go up.", "".join(body))


def scene_answered():
    body = [sun(418, 84, 20), shadow(170, 36)]
    p, pose = person(170, facing=1, skin=SKIN[1], hair=HAIR[1], hair_style="curly",
                     shirt=SAGE, near_arm=(145, 15), far_arm=(-145, -15))
    body.append(p)
    for x, y in ((112, 78), (236, 70), (254, 124)):
        body.append(sparkle(x, y, 7))
    # The entry: the original prayer, and the answer added after it.
    body.append(page_card(290, 132, 150, 86))
    body.append(line(304, 172, 426, 172, CARD_EDGE, 1.5))
    body.append(text(306, 190, "ANSWERED", 9, ACCENT, anchor="start", weight="700"))
    body.append(line(306, 200, 400, 200, ACCENT, 2))
    body.append(text(240, 34, "Answered: turn a prayer into a thanksgiving", 15))
    return svg("A prayer answered",
               "Someone raises both hands in thanks in the sunshine. In the book, the original prayer "
               "stays as written, with a note about how it was answered added underneath.", "".join(body), sky=SKY)


def scene_release():
    body = [shadow(200, 36)]
    p, pose = person(200, facing=1, skin=SKIN[2], hair=HAIR[4], hair_style="bun",
                     shirt=SLATE, pants=INK, sleeves="long", near_arm=(135, 20), far_arm=(-135, -20))
    body.append(p)
    hx, hy = pose.near_hand
    birds = ((hx + 30, hy - 8, 1.3), (318, 58, 1.0), (366, 42, 0.8), (408, 32, 0.6))
    body.append(dashed(f"M {f(hx + 6)} {f(hy - 4)} Q {f(hx + 60)} {f(hy - 16)} 408 34", color=GOLD, width=2))
    for i, (x, y, s) in enumerate(birds):
        body.append(bird(x, y, s, color=INK if i == 0 else shade(INK, 1.6)))
    body.append(text(240, 278, "It leaves the book, with your reason kept.", 12, ACCENT, weight="500"))
    body.append(text(240, 34, "Release: let a prayer go", 15))
    return svg("Releasing a prayer",
               "Someone opens their hands and a bird flies up and away: a prayer being released.",
               "".join(body), sky=SKY)


def scene_ai():
    lap_x = 214
    body = [chair(146, facing=1), table(240, w=150, inset=36), shadow(166, 44)]
    body.append(laptop(lap_x, TABLE_TOP))
    p, pose = person(146, facing=1, sit=True, lean=16, skin=SKIN[0], hair=HAIR[3], hair_style="short",
                     shirt=PLUM, pants=SLATE, near_reach=(lap_x, TABLE_TOP - 7),
                     far_reach=(lap_x - 16, TABLE_TOP - 5))
    body.append(p)
    # The assistant's reply, with the prayer book it can read and write.
    body.append(bubble(372, 100, 170, 84, 244, 142))
    body.append(sparkle(308, 84, 9))
    body.append(text(386, 84, "Your AI, via", 11, INK, weight="500"))
    body.append(text(386, 100, "MCP", 13, ACCENT, weight="700"))
    body.append(book_open(386, 134, w=54, h=22, lines=False))
    body.append(text(240, 34, "Bring your own AI: it reads and writes as you", 15))
    return svg("Connecting your own AI",
               "Someone at a laptop talks with their AI assistant, which connects to their prayer book "
               "through MCP and acts only as them.", "".join(body))


SCENES = {
    "write": scene_write,
    "book": scene_book,
    "qr-join": scene_qr,
    "invite": scene_invite,
    "share": scene_share,
    "praying-now": scene_praying_now,
    "answered": scene_answered,
    "release": scene_release,
    "ai": scene_ai,
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, scene in SCENES.items():
        (OUT / f"{name}.svg").write_text(scene())
    print(f"wrote {len(SCENES)} illustrations to {OUT}")


if __name__ == "__main__":
    main()
