#!/usr/bin/env python3
"""Generate the website's feature illustrations as SVG.

Every scene is built from the same posable `person()` and a small set of props,
so the figures share one style and palette. Edit a scene below and re-run:

    python3 website/illustrations/draw.py      # writes website/static/illustrations/*.svg

Conventions: 480x300 canvas, ground at y=250. Arm angles are degrees from
hanging straight down, positive toward the direction the person faces.
"""

import math
import pathlib

OUT = pathlib.Path(__file__).resolve().parent.parent / "static" / "illustrations"

W, H, GROUND = 480, 300, 250

# Palette: the app's warm paper and ink, plus soft clothing colours.
PAPER, CARD_EDGE, INK, ACCENT, GOLD = "#fbf4e8", "#ead9bf", "#2c1810", "#8b4513", "#c9a96e"
SAGE, ROSE, SLATE, PLUM, SKY = "#8a9a78", "#c27c6b", "#5f6f82", "#7a5c7e", "#dfe9ee"
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


def polar(x, y, length, angle, facing):
    """Point `length` away from (x, y) at `angle` degrees from straight down."""
    a = math.radians(angle)
    return x + math.sin(a) * length * facing, y + math.cos(a) * length


# ── People ────────────────────────────────────────────────────────────────────

class Pose:
    """Where a drawn person's hands and head ended up, for placing props."""

    def __init__(self, near_hand, far_hand, head):
        self.near_hand, self.far_hand, self.head = near_hand, far_hand, head


def person(x, *, facing=1, skin=SKIN[0], hair=HAIR[0], hair_style="short",
           shirt=SLATE, pants=INK, near_arm=(10, 0), far_arm=(-8, 0),
           sit=False, s=1.0, ground=GROUND, parts=None):
    """Draw a person standing (or sitting) on the ground at x.

    near_arm / far_arm = (shoulder angle, elbow bend) in degrees. Returns the
    SVG and a Pose with hand/head positions.
    """
    out = []
    fc = facing
    if sit:
        hip_y = ground - 52 * s
        knee = (x + 34 * s * fc, hip_y)
        foot = (x + 36 * s * fc, ground - 4 * s)
        for dx in (-3, 3):
            out.append(line(x + dx * s, hip_y, knee[0] + dx * s, knee[1], pants, 12 * s))
            out.append(line(knee[0] + dx * s, knee[1], foot[0] + dx * s, foot[1], pants, 11 * s))
        out.append(f'<ellipse cx="{f(foot[0] + 5 * s * fc)}" cy="{f(ground - 3 * s)}" rx="{f(9 * s)}" ry="{f(4 * s)}" fill="{INK}"/>')
    else:
        hip_y = ground - 80 * s
        for dx, spread in ((-6, -9), (6, 9)):
            out.append(line(x + dx * s, hip_y, x + spread * s, ground - 5 * s, pants, 12 * s))
            out.append(f'<ellipse cx="{f(x + spread * s + 4 * s * fc)}" cy="{f(ground - 3 * s)}" '
                       f'rx="{f(9 * s)}" ry="{f(4 * s)}" fill="{INK}"/>')

    shoulder_y = hip_y - 60 * s
    sx = x
    head = (x + 3 * s * fc, shoulder_y - 24 * s)

    def arm(angles, far):
        a1, bend = angles
        ox = sx - (5 * s * fc if far else -2 * s * fc)
        oy = shoulder_y + 9 * s
        elbow = polar(ox, oy, 29 * s, a1, fc)
        hand = polar(*elbow, 27 * s, a1 + bend, fc)
        sleeve = shade(shirt) if far else shirt
        tone = shade(skin) if far else skin
        return [
            line(ox, oy, *elbow, sleeve, 11 * s),
            line(*elbow, *hand, tone, 8.5 * s),
            circle(*hand, 5.5 * s, tone),
        ], hand

    far_svg, far_hand = arm(far_arm, True)
    out += far_svg
    # Torso and neck.
    out.append(rect(x - 3.5 * s, shoulder_y - 8 * s, 7 * s, 10 * s, skin))
    out.append(rect(x - 19 * s, shoulder_y, 38 * s, hip_y - shoulder_y + 8 * s, shirt, rx=14 * s))
    # Head, hair, face.
    hx, hy = head
    if hair_style == "long":
        back_x = hx - 19 * s if fc > 0 else hx - 1 * s
        out.append(rect(back_x, hy - 10 * s, 20 * s, 36 * s, hair, rx=9 * s))
    out.append(circle(hx, hy, 17 * s, skin))
    if hair_style == "bun":
        out.append(circle(hx - 12 * s * fc, hy - 15 * s, 7.5 * s, hair))
    if hair_style == "curly":
        for i, (dx, dy) in enumerate(((-13, -8), (-8, -15), (0, -18), (8, -15), (13, -8), (-15, 0))):
            out.append(circle(hx + dx * s * fc if i < 5 else hx - 15 * s * fc, hy + dy * s, 7 * s, hair))
    elif hair_style != "bald":
        out.append(path(
            f"M {f(hx - 17 * s)} {f(hy - 1 * s)} "
            f"A {f(17 * s)} {f(17 * s)} 0 0 1 {f(hx + 17 * s)} {f(hy - 1 * s)} "
            f"Q {f(hx + 4 * s * fc)} {f(hy - 11 * s)} {f(hx - 17 * s)} {f(hy - 1 * s)} Z",
            fill=hair))
    out.append(circle(hx + 7 * s * fc, hy + 1 * s, 1.9 * s, INK))
    out.append(path(f"M {f(hx + 4 * s * fc)} {f(hy + 8 * s)} q {f(4 * s * fc)} {f(3 * s)} {f(8 * s * fc)} 0",
                    stroke=INK, width=1.6 * s))
    near_svg, near_hand = arm(near_arm, False)
    out += near_svg
    if parts is not None:
        parts.append("".join(out))
    return "".join(out), Pose(near_hand, far_hand, head)


def shade(hex_color, k=0.82):
    r, g, b = (int(hex_color[i:i + 2], 16) for i in (1, 3, 5))
    return "#%02x%02x%02x" % (int(r * k), int(g * k), int(b * k))


def shadow(x, w=34, ground=GROUND):
    return f'<ellipse cx="{f(x)}" cy="{f(ground + 1)}" rx="{f(w)}" ry="5" fill="{INK}" opacity="0.08"/>'


# ── Props ─────────────────────────────────────────────────────────────────────

def phone(cx, cy, w=22, h=38, screen="#fdf8f0", tilt=0, content=""):
    g = [rect(cx - w / 2, cy - h / 2, w, h, INK, rx=4),
         rect(cx - w / 2 + 2.5, cy - h / 2 + 3.5, w - 5, h - 8, screen, rx=2), content]
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


def book_open(cx, cy, w=70, h=34, lines=True):
    half = w / 2
    out = [path(f"M {f(cx)} {f(cy - h / 2 + 3)} Q {f(cx - half / 2)} {f(cy - h / 2 - 3)} {f(cx - half)} {f(cy - h / 2 + 1)} "
                f"L {f(cx - half)} {f(cy + h / 2)} Q {f(cx - half / 2)} {f(cy + h / 2 - 5)} {f(cx)} {f(cy + h / 2)} Z",
                fill="#fffaf0", stroke=ACCENT, width=2),
           path(f"M {f(cx)} {f(cy - h / 2 + 3)} Q {f(cx + half / 2)} {f(cy - h / 2 - 3)} {f(cx + half)} {f(cy - h / 2 + 1)} "
                f"L {f(cx + half)} {f(cy + h / 2)} Q {f(cx + half / 2)} {f(cy + h / 2 - 5)} {f(cx)} {f(cy + h / 2)} Z",
                fill="#fffaf0", stroke=ACCENT, width=2)]
    if lines:
        for i in range(3):
            y = cy - h / 2 + 10 + i * 7
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
    return (line(x, ground, x, ground - 70 * s, "#7a5a3c", 10 * s)
            + circle(x, ground - 92 * s, 34 * s, SAGE)
            + circle(x - 22 * s, ground - 74 * s, 22 * s, shade(SAGE, 0.92))
            + circle(x + 22 * s, ground - 76 * s, 24 * s, shade(SAGE, 0.88)))


def table(x, w=110, top=GROUND - 62):
    return (rect(x - w / 2, top, w, 8, "#a8774d", rx=3)
            + line(x - w / 2 + 10, top + 8, x - w / 2 + 10, GROUND, "#8a5f3b", 6)
            + line(x + w / 2 - 10, top + 8, x + w / 2 - 10, GROUND, "#8a5f3b", 6))


def chair(x, facing=1, seat=GROUND - 50):
    back = x - 16 * facing
    return (line(back, seat - 44, back, GROUND, "#8a5f3b", 6)
            + rect(x - 20, seat, 40, 7, "#a8774d", rx=3)
            + line(x + 14 * facing, seat + 7, x + 14 * facing, GROUND, "#8a5f3b", 6))


def laptop(cx, base_y, facing=1):
    screen_x = cx + 4 * facing
    return (path(f"M {f(cx - 30)} {f(base_y)} L {f(cx + 30)} {f(base_y)} L {f(cx + 26)} {f(base_y + 5)} L {f(cx - 26)} {f(base_y + 5)} Z", fill="#9aa3ad")
            + f'<g transform="skewX({f(-8 * facing)})">'
            + rect(screen_x - 26 + (base_y * 0.14 * facing), base_y - 38, 50, 38, INK, rx=3)
            + rect(screen_x - 22 + (base_y * 0.14 * facing), base_y - 34, 42, 30, "#e8eef3", rx=1)
            + "</g>")


def bubble(cx, cy, w, h, tail_x, tail_y, fill="#ffffff"):
    return (rect(cx - w / 2, cy - h / 2, w, h, fill, rx=12, extra=f'stroke="{CARD_EDGE}" stroke-width="2"')
            + path(f"M {f(cx - 8)} {f(cy + h / 2 - 1)} L {f(tail_x)} {f(tail_y)} L {f(cx + 8)} {f(cy + h / 2 - 1)} Z",
                   fill=fill, stroke=CARD_EDGE, width=2)
            + rect(cx - 9, cy + h / 2 - 3, 18, 4, fill))


def svg(title, desc, body, sky=None):
    bg = [rect(1, 1, W - 2, H - 2, PAPER, rx=18, extra=f'stroke="{CARD_EDGE}" stroke-width="2"')]
    if sky:
        bg.append(rect(1, 1, W - 2, GROUND - 1, sky, rx=18))
        bg.append(rect(1, GROUND - 30, W - 2, 30, sky))
    bg.append(rect(1, GROUND, W - 2, H - GROUND - 1, "#f1e4cf", rx=0))
    bg.append(path(f"M 1 {H - 19} L 1 {H - 19} Q 1 {H - 1} 19 {H - 1} L {W - 19} {H - 1} Q {W - 1} {H - 1} {W - 1} {H - 19} L {W - 1} {GROUND} L 1 {GROUND} Z",
                   fill="#f1e4cf"))
    bg.append(line(1, GROUND, W - 1, GROUND, CARD_EDGE, 2, cap="butt"))
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
            f'role="img" aria-labelledby="t d">\n<title id="t">{title}</title>\n<desc id="d">{desc}</desc>\n'
            + "".join(bg) + "\n" + body + "\n</svg>\n")


# ── Scenes ────────────────────────────────────────────────────────────────────

def scene_write():
    body = [chair(150, facing=1), table(250, w=140)]
    body.append(shadow(160, 40))
    p, pose = person(150, facing=1, sit=True, skin=SKIN[1], hair=HAIR[1], hair_style="long",
                     shirt=ROSE, pants=SLATE, near_arm=(60, 28), far_arm=(52, 38))
    body.append(p)
    body.append(book_open(262, GROUND - 76, w=86, h=30))
    # Pen in hand, candle for warmth.
    hx, hy = pose.near_hand
    body.append(line(hx + 2, hy + 2, hx + 14, hy - 10, INK, 3))
    body.append(rect(318, GROUND - 92, 10, 22, "#fffaf0", rx=2, extra=f'stroke="{CARD_EDGE}" stroke-width="1.5"'))
    body.append(path(f"M 323 {GROUND - 106} q 6 8 0 13 q -6 -5 0 -13 Z", fill="#f2c14e"))
    body.append(text(240, 34, "Write it down: a prayer, or a thanksgiving", 15))
    return svg("Writing a prayer",
               "Someone sits at a table writing in an open prayer book by candlelight.", "".join(body))


def scene_book():
    body = [shadow(150, 36)]
    p, pose = person(150, facing=1, skin=SKIN[2], hair=HAIR[3], hair_style="short",
                     shirt=SAGE, near_arm=(38, 70), far_arm=(-6, 0))
    body.append(p)
    hx, hy = pose.near_hand
    body.append(phone(hx + 8, hy - 8, tilt=8))
    # Big page with a page-turn.
    body.append(rect(270, 70, 150, 150, "#fffaf0", rx=6, extra=f'stroke="{ACCENT}" stroke-width="2"'))
    for i in range(6):
        body.append(line(288, 100 + i * 17, 400 - (i % 3) * 18, 100 + i * 17, GOLD, 2))
    body.append(path("M 420 176 L 420 220 L 376 220 Q 404 206 420 176 Z", fill="#efe0c6", stroke=ACCENT, width=2))
    body.append(path("M 250 150 q -30 -24 -58 -6", stroke=ACCENT, width=2.5))
    body.append(path("M 198 138 l -8 8 l 11 3", stroke=ACCENT, width=2.5))
    body.append(text(345, 244, "one page at a time", 12, ACCENT, weight="500"))
    body.append(text(240, 34, "A book, not a feed: swipe to turn the page", 15))
    return svg("Paging through the book",
               "Someone swipes on their phone and a single page of the prayer book turns over.",
               "".join(body))


def scene_qr():
    body = [tree(430, s=0.8), shadow(140, 36), shadow(340, 36)]
    left, lp = person(140, facing=1, skin=SKIN[0], hair=HAIR[2], hair_style="bun",
                      shirt=PLUM, pants=SLATE, near_arm=(62, 58), far_arm=(-8, 0))
    right, rp = person(340, facing=-1, skin=SKIN[3], hair=HAIR[0], hair_style="curly",
                       shirt=GOLD, pants=INK, near_arm=(62, 52), far_arm=(-6, 0))
    body += [left, right]
    # Left holds up a phone showing the group's QR code, screen toward the right.
    lx, ly = lp.near_hand
    body.append(phone(lx + 9, ly - 14, w=30, h=46, content=qr(lx + 9, ly - 16, 20)))
    # Right points a phone at it; dashed scan lines between the two.
    rx, ry = rp.near_hand
    body.append(phone(rx - 8, ry - 10, w=22, h=36, screen="#e8eef3", tilt=-10))
    for dy in (-12, 0, 12):
        body.append(dashed(f"M {f(rx - 22)} {f(ry - 10 + dy / 2)} L {f(lx + 28)} {f(ly - 16 + dy)}", color=ACCENT, width=2))
    body.append(sparkle(240, 100, 8))
    body.append(text(240, 34, "Scan to join: groups start in real space", 15))
    return svg("Joining a group by QR code",
               "Two people stand together outside. One holds up a phone showing the group's QR code; "
               "the other scans it with their own phone to join.", "".join(body), sky=SKY)


def scene_invite():
    body = [shadow(110, 34), shadow(375, 34)]
    # Two homes, far apart.
    for hx in (110, 375):
        wall = shade(PAPER, 0.95)
        body.append(rect(hx - 55, GROUND - 120, 110, 120, wall))
        body.append(path(f"M {hx - 66} {GROUND - 118} L {hx} {GROUND - 164} L {hx + 66} {GROUND - 118} Z", fill=shade(PAPER, 0.9)))
        body.append(rect(hx + 26, GROUND - 100, 18, 18, SKY, rx=2))
    left, lp = person(110, facing=1, skin=SKIN[1], hair=HAIR[0], hair_style="short",
                      shirt=SLATE, near_arm=(118, 10), far_arm=(-6, 0))
    right, rp = person(375, facing=-1, skin=SKIN[0], hair=HAIR[4], hair_style="long",
                       shirt=ROSE, near_arm=(40, 60), far_arm=(-6, 0))
    body += [left, right]
    body.append(dashed("M 150 110 Q 240 40 330 110"))
    body.append(envelope(240, 72, tilt=-6))
    rx, ry = rp.near_hand
    body.append(phone(rx - 8, ry - 8, tilt=-6, content=envelope(rx - 8, ry - 9, w=12, h=8)))
    body.append(text(240, 34, "Invite by email or text", 15))
    return svg("Inviting someone to a group",
               "Someone waves, and an invitation envelope flies across to a friend in another home, "
               "who opens it on their phone.", "".join(body))


def scene_share():
    body = []
    # The group: three people around a rug.
    body.append(f'<ellipse cx="330" cy="{GROUND}" rx="120" ry="16" fill="{GOLD}" opacity="0.35"/>')
    for x, sk, hr, st, sh in ((262, SKIN[3], HAIR[3], "short", SAGE), (330, SKIN[1], HAIR[1], "bun", PLUM),
                              (398, SKIN[0], HAIR[2], "long", SLATE)):
        body.append(person(x, facing=-1 if x > 330 else 1, skin=sk, hair=hr, hair_style=st, shirt=sh,
                           near_arm=(30, 60), far_arm=(-4, 0), s=0.86)[0])
    # Someone offers a page to the group.
    body.append(shadow(105, 34))
    p, pose = person(105, facing=1, skin=SKIN[2], hair=HAIR[0], hair_style="curly",
                     shirt=ROSE, near_arm=(95, -10), far_arm=(-6, 0))
    body.append(p)
    hx, hy = pose.near_hand
    body.append(f'<g transform="rotate(-8 {f(hx + 26)} {f(hy - 6)})">'
                + rect(hx + 6, hy - 30, 40, 50, "#fffaf0", rx=3, extra=f'stroke="{ACCENT}" stroke-width="2"')
                + "".join(line(hx + 13, hy - 20 + i * 9, hx + 39, hy - 20 + i * 9, GOLD, 1.6) for i in range(4))
                + "</g>")
    body.append(dashed(f"M {f(hx + 52)} {f(hy - 20)} Q 210 60 280 78"))
    body.append(heart(300, 84, 7))
    body.append(text(240, 34, "Share a prayer with your group", 15))
    return svg("Sharing a prayer with a group",
               "Someone hands a written prayer toward three friends gathered together, sharing it with the group.",
               "".join(body))


def scene_praying_now():
    body = []
    # Two windows: different places, same prayer.
    for x0 in (22, 262):
        body.append(rect(x0, 56, 196, 180, SKY, rx=10, extra=f'stroke="{CARD_EDGE}" stroke-width="3"'))
    body.append(line(240, 60, 240, GROUND - 6, CARD_EDGE, 3, extra='stroke-dasharray="4 8"'))
    body.append(shadow(120, 30))
    p, pose = person(120, facing=1, skin=SKIN[3], hair=HAIR[3], hair_style="short",
                     shirt=GOLD, near_arm=(40, 72), far_arm=(-6, 0), s=0.95)
    body.append(p)
    hx, hy = pose.near_hand
    body.append(phone(hx + 8, hy - 8, content=text(hx + 8, hy - 3, "🙏", 12)))
    body.append(shadow(360, 30))
    q, qpose = person(360, facing=-1, skin=SKIN[0], hair=HAIR[2], hair_style="long",
                      shirt=PLUM, near_arm=(40, 72), far_arm=(-6, 0), s=0.95)
    body.append(q)
    qx, qy = qpose.near_hand
    body.append(phone(qx - 8, qy - 8, content=text(qx - 8, qy - 3, "3", 12, ACCENT)))
    # +1s drifting across.
    for i, (x, y) in enumerate(((176, 104), (228, 86), (282, 100))):
        body.append(text(x, y, "+1", 14 - i, ACCENT, weight="700"))
    body.append(heart(254, 130, 6))
    body.append(text(240, 34, "Praying now: tap 🙏 every time you pray", 15))
    return svg("Praying now",
               "Two people in different places. One taps the praying-now button on their phone, and the "
               "other sees the count of prayers for their request go up.", "".join(body))


def scene_answered():
    body = [sun(360, 96, 26), shadow(170, 36)]
    p, pose = person(170, facing=1, skin=SKIN[1], hair=HAIR[1], hair_style="curly",
                     shirt=SAGE, near_arm=(152, 12), far_arm=(-150, -12))
    body.append(p)
    for x, y in ((120, 70), (230, 64), (250, 120)):
        body.append(sparkle(x, y, 7))
    # The book: the original prayer, and the answer added after it.
    body.append(rect(290, 150, 150, 86, "#fffaf0", rx=6, extra=f'stroke="{ACCENT}" stroke-width="2"'))
    for i in range(3):
        body.append(line(304, 166 + i * 10, 420 - i * 20, 166 + i * 10, GOLD, 2))
    body.append(line(304, 200, 426, 200, CARD_EDGE, 1.5))
    body.append(text(306, 216, "ANSWERED", 9, ACCENT, anchor="start", weight="700"))
    body.append(line(306, 225, 400, 225, ACCENT, 2))
    body.append(text(240, 34, "Answered: turn a prayer into a thanksgiving", 15))
    return svg("A prayer answered",
               "Someone raises both hands in thanks in the sunshine. In the book, the original prayer "
               "stays as written, with a note about how it was answered added underneath.", "".join(body), sky=SKY)


def scene_release():
    body = [shadow(200, 36)]
    p, pose = person(200, facing=1, skin=SKIN[2], hair=HAIR[4], hair_style="bun",
                     shirt=SLATE, pants=INK, near_arm=(128, 20), far_arm=(-128, -20))
    body.append(p)
    for i, (x, y, s) in enumerate(((262, 92, 1.3), (318, 66, 1.0), (366, 48, 0.8), (404, 36, 0.6))):
        body.append(bird(x, y, s, color=INK if i == 0 else shade(INK, 1.6)))
    body.append(dashed("M 236 112 Q 300 90 400 40", color=GOLD, width=2))
    body.append(text(240, 272, "It leaves the book, with your reason kept.", 12, ACCENT, weight="500"))
    body.append(text(240, 34, "Release: let a prayer go", 15))
    return svg("Releasing a prayer",
               "Someone opens their hands and a bird flies up and away: a prayer being released.",
               "".join(body), sky=SKY)


def scene_ai():
    body = [chair(150, facing=1), table(270, w=150), shadow(160, 40)]
    p, pose = person(150, facing=1, sit=True, skin=SKIN[0], hair=HAIR[3], hair_style="short",
                     shirt=PLUM, pants=SLATE, near_arm=(62, 32), far_arm=(55, 40))
    body.append(p)
    body.append(laptop(272, GROUND - 63))
    body.append(bubble(372, 92, 150, 58, 318, 142))
    body.append(sparkle(318, 84, 9))
    body.append(text(385, 88, "Your AI, via", 11, INK, weight="500"))
    body.append(text(385, 104, "MCP", 13, ACCENT, weight="700"))
    body.append(book_open(410, GROUND - 76, w=54, h=22, lines=False))
    body.append(dashed("M 300 190 Q 356 176 386 178", color=ACCENT, width=2))
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
