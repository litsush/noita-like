#!/usr/bin/env python3
"""Procedural pixel-art generator for "Descent to the Core".

Pure Python 3 standard library (zlib + struct PNG/TTF writers), fully
deterministic (seeded random.Random).  Run from the repo root:

    python3 tools/gen_art.py                 # regenerate every asset
    python3 tools/gen_art.py --preview DIR   # also write 4x preview sheets

All PNGs are RGBA8.  Coordinates: x right, y down; frames are packed
left-to-right from x=0, rows top-to-bottom; unused slots are transparent.
Scale everywhere: 1 game cell = 1 art pixel.  Light comes from the top-left,
characters/objects carry a 1px dark (#181425) outline.  Palette: Endesga-32.

assets/sprites/player.png  128x160, 16x16 frames, 8 cols x 10 rows.
    Miner faces RIGHT (flip for left), feet on frame row y=15, body centred
    at x~8 (collision box 6x12).  Rows (frame counts):
      0 idle (6)      1 run (6)        2 jump (4)       3 fall (4)
      4 climb (6, facing a wall on the right; walls and ropes)
      5 dig_side (6)  6 dig_down (6)   7 dig_up (6)
      8 hurt (4; frame 0 white flash, frame 2 red flash)
      9 death (8; collapses, frames 5-7 lying down, lamp dies out)

assets/sprites/items.png   128x80, 16x16 icons, 8 cols x 5 rows,
    index = row*8 + col:
      0 crumbling_pick  1 magma_pick     2 glass_cannon_pick 3 water_canister
      4 acid_flask      5 blast_charges  6 salamander_skin   7 gill_mask
      8 heavy_boots     9 thermal_suit  10 frost_seed       11 fungal_spores
     12 pocket_sun     13 beacon_heart  14 volatile_core    15 spark_rod
     16 seismic_stomp  17 chest_closed  18 chest_open       19 altar
     20 shrine         21 rope          22 torch            23 ore
     24 gem            25 heart_full    26 heart_half       27 heart_empty
     28 lock           29 flame         30 bubble           31 thermometer
     32 skull          33 trophy        34 pickaxe          35 question_mark
     36 compass        37 hourglass     38-39 transparent

assets/sprites/props.png   128x24, 16x24 frames (1 row of 8), world scale,
    objects bottom-aligned (resting on y=23):
      0-3 torch flicker loop (~4x9 px)   4 chest_closed (12x10)
      5 chest_open                       6 altar (14x10, items float above)
      7 shrine (full 16x24 frame, glowing sigil; buy items with ore)

assets/sprites/core.png    384x48, 8 frames of 48x48 (looping pulse);
    molten sphere ~40 px across centred at (24,24), transparent around.

assets/sprites/creatures.png 64x48, 16x16 frames, 4 cols x 3 rows, facing
    RIGHT, feet on y=15:
      row 0 cave_crawler walk (4)  ~10x6
      row 1 magma_slug crawl (4)   ~12x6
      row 2 spore_drifter float (4) ~9x9, bobbing, vertically centred

assets/backgrounds/layer{0..4}_{far,near}.png  256x256, seamlessly tileable
    in x and y.  "far" is opaque, "near" has alpha (silhouettes over
    transparency).  0 Crust, 1 Upper Mantle, 2 Deep Mantle, 3 Outer Core,
    4 Core chamber.  Kept dark (value ~10-35%) to sit behind the playfield.

assets/fonts/pixel.ttf  TrueType pixel font, unitsPerEm 1024, 1 px = 128
    units, cap height 7 px, descender 2 px, proportional advances.
    ASCII 32-126 plus · • … × ← → ↑ ↓ ° and a .notdef box.  Crisp at
    em sizes that are multiples of 8 px (egui/skrifa font size 8, 16, 24);
    ab_glyph PxScale is ascent-descent (10 font px), so use multiples of 10.
"""
import math
import os
import random
import struct
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "assets")

# --------------------------------------------------------------------------
# Palette (Endesga-32)
# --------------------------------------------------------------------------
PAL_HEX = {
    "rust": "be4a2f", "orange_dk": "d77643", "cream": "ead4aa", "tan": "e4a672",
    "brown_l": "b86f50", "brown": "733e39", "brown_dk": "3e2731", "red_dk": "a22633",
    "red": "e43b44", "orange": "f77622", "yellow_o": "feae34", "yellow": "fee761",
    "green_l": "63c74d", "green": "3e8948", "green_dk": "265c42", "teal_dk": "193c3e",
    "blue_dk": "124e89", "blue": "0099db", "cyan": "2ce8f5", "white": "ffffff",
    "grey_l": "c0cbdc", "grey": "8b9bb4", "grey_d": "5a6988", "slate": "3a4466",
    "navy": "262b44", "black": "181425", "hot": "ff0044", "purple": "68386c",
    "pink": "b55088", "salmon": "f6757a", "skin": "e8b796", "skin_d": "c28569",
}


def hexc(h, a=255):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


C = {k: hexc(v) for k, v in PAL_HEX.items()}
CLEAR = (0, 0, 0, 0)
OUTLINE = "black"


def col(c):
    if c is None:
        return None
    if isinstance(c, str):
        return C[c] if c in C else hexc(c)
    if len(c) == 2:
        return with_alpha(c[0], c[1])
    if len(c) == 3:
        return (c[0], c[1], c[2], 255)
    return tuple(c)


def mix(a, b, t):
    a, b = col(a), col(b)
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(4))


def with_alpha(c, a):
    c = col(c)
    return (c[0], c[1], c[2], a)


BAYER4 = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]


def bayer(x, y):
    return (BAYER4[y & 3][x & 3] + 0.5) / 16.0


def ramp_pick(ramp, v, x, y):
    """Ordered-dithered pick from a colour ramp, v in [0,1]."""
    v = min(1.0, max(0.0, v)) * (len(ramp) - 1)
    i = int(v)
    if i >= len(ramp) - 1:
        return col(ramp[-1])
    return col(ramp[i + 1] if (v - i) > bayer(x, y) else ramp[i])


# --------------------------------------------------------------------------
# Sprite canvas
# --------------------------------------------------------------------------
class Sprite:
    def __init__(self, w, h, fill=CLEAR):
        self.w, self.h = w, h
        self.px = [fill] * (w * h)
        self.wrap = False

    def inb(self, x, y):
        return 0 <= x < self.w and 0 <= y < self.h

    def get(self, x, y):
        if self.wrap:
            return self.px[(y % self.h) * self.w + (x % self.w)]
        if self.inb(x, y):
            return self.px[y * self.w + x]
        return CLEAR

    def set(self, x, y, c):
        x, y = int(math.floor(x)), int(math.floor(y))
        if self.wrap:
            x %= self.w
            y %= self.h
        elif not self.inb(x, y):
            return
        self.px[y * self.w + x] = col(c)

    def opaque(self, x, y):
        return self.get(x, y)[3] > 0

    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)

    def line(self, x0, y0, x1, y1, c):
        for (x, y) in line_pts(x0, y0, x1, y1):
            self.set(x, y, c)

    def fill(self, pts, c):
        for (x, y) in pts:
            self.set(x, y, c)

    def outline(self, c=OUTLINE, diag=False):
        c = col(c)
        add = []
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y * self.w + x][3]:
                    continue
                nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                if diag:
                    nb += [(1, 1), (-1, 1), (1, -1), (-1, -1)]
                for dx, dy in nb:
                    if self.opaque(x + dx, y + dy):
                        add.append((x, y))
                        break
        for x, y in add:
            self.px[y * self.w + x] = c

    def blit(self, other, dx, dy):
        for y in range(other.h):
            for x in range(other.w):
                p = other.px[y * other.w + x]
                if p[3]:
                    self.set(dx + x, dy + y, p) if p[3] == 255 else self.blend(dx + x, dy + y, p)

    def blend(self, x, y, p):
        if not self.wrap and not self.inb(x, y):
            return
        q = self.get(x, y)
        a = p[3] / 255.0
        if q[3] == 0:
            self.set(x, y, p)
            return
        oa = a + q[3] / 255.0 * (1 - a)
        rgb = [int(round((p[i] * a + q[i] * (q[3] / 255.0) * (1 - a)) / oa)) for i in range(3)]
        self.set(x, y, (rgb[0], rgb[1], rgb[2], int(round(oa * 255))))

    def copy(self):
        s = Sprite(self.w, self.h)
        s.px = list(self.px)
        return s

    def map(self, fn):
        self.px = [fn(p) if p[3] else p for p in self.px]

    def points(self):
        return {(i % self.w, i // self.w) for i, p in enumerate(self.px) if p[3]}


def line_pts(x0, y0, x1, y1):
    x0, y0, x1, y1 = int(round(x0)), int(round(y0)), int(round(x1)), int(round(y1))
    pts = []
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        pts.append((x0, y0))
        if x0 == x1 and y0 == y1:
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy
    return pts


# ---- masks (sets of pixel coords) ----------------------------------------
def m_rect(x0, y0, x1, y1):
    return {(x, y) for y in range(y0, y1 + 1) for x in range(x0, x1 + 1)}


def m_ellipse(cx, cy, rx, ry):
    pts = set()
    for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
        for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
            if ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1.0:
                pts.add((x, y))
    return pts


def m_poly(poly):
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    pts = set()
    for y in range(int(min(ys)) - 1, int(max(ys)) + 2):
        for x in range(int(min(xs)) - 1, int(max(xs)) + 2):
            px, py = x + 0.5, y + 0.5
            inside = False
            j = len(poly) - 1
            for i in range(len(poly)):
                xi, yi = poly[i]
                xj, yj = poly[j]
                if (yi > py) != (yj > py):
                    if px < (xj - xi) * (py - yi) / (yj - yi) + xi:
                        inside = not inside
                j = i
            if inside:
                pts.add((x, y))
    return pts


def m_ascii(rows, ox=0, oy=0, ch="#"):
    return {(ox + x, oy + y) for y, r in enumerate(rows) for x, c in enumerate(r) if c in ch}


def shade_fill(sp, pts, dark, base, light, hi=None):
    """Fill a mask with top-left light / bottom-right shadow edges."""
    S = set(pts)
    for (x, y) in S:
        up = (x, y - 1) not in S
        lf = (x - 1, y) not in S
        dn = (x, y + 1) not in S
        rt = (x + 1, y) not in S
        if hi and up and lf:
            c = hi
        elif (up or lf) and not (dn or rt):
            c = light
        elif (dn or rt) and not (up or lf):
            c = dark
        elif up:
            c = light
        elif dn or rt:
            c = dark
        else:
            c = base
        sp.set(x, y, c)


LIGHT = (-0.55, -0.65, 0.52)


def sphere(sp, cx, cy, r, ramp, dither=True, mask=None):
    ln = math.sqrt(sum(v * v for v in LIGHT))
    L = [v / ln for v in LIGHT]
    for y in range(int(cy - r) - 1, int(cy + r) + 2):
        for x in range(int(cx - r) - 1, int(cx + r) + 2):
            dx, dy = (x + 0.5 - cx) / r, (y + 0.5 - cy) / r
            d2 = dx * dx + dy * dy
            if d2 > 1.0:
                continue
            if mask is not None and (x, y) not in mask:
                continue
            nz = math.sqrt(1 - d2)
            i = dx * L[0] + dy * L[1] + nz * L[2]
            v = (i + 0.35) / 1.35
            sp.set(x, y, ramp_pick(ramp, v, x, y) if dither else col(ramp[min(len(ramp) - 1, max(0, int(v * len(ramp))))]))


def flame_mask(cx, top, bot, hw, lean=0.0):
    pts = set()
    for y in range(int(top), int(bot) + 1):
        t = (y + 0.5 - top) / max(1.0, (bot + 1 - top))
        if t <= 0 or t >= 1:
            continue
        w = hw * (t ** 0.75) * math.sqrt(max(0.0, 1 - max(0.0, (t - 0.68) / 0.32) ** 2))
        sx = cx + lean * (1 - t) ** 2
        for x in range(int(cx - hw - 3), int(cx + hw + 4)):
            if abs(x + 0.5 - sx) <= w:
                pts.add((x, y))
    return pts


def draw_flame(sp, cx, top, bot, hw, lean=0.0, cols=("red", "orange", "yellow_o", "yellow", "white")):
    layers = len(cols)
    h = bot - top
    for i, c in enumerate(cols):
        f = i / layers
        m = flame_mask(cx, top + h * f * 0.55, bot - h * f * 0.12, hw * (1 - f * 0.78), lean * (1 - f * 0.5))
        sp.fill(m, c)


# --------------------------------------------------------------------------
# PNG writer / reader-free preview helpers
# --------------------------------------------------------------------------
def write_png(path, sp):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    raw = bytearray()
    for y in range(sp.h):
        raw.append(0)
        for x in range(sp.w):
            raw.extend(sp.px[y * sp.w + x])

    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    data = b"\x89PNG\r\n\x1a\n"
    data += chunk(b"IHDR", struct.pack(">IIBBBBB", sp.w, sp.h, 8, 6, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    data += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(data)


def sheet(cols, rows, fw, fh, frames):
    """frames: dict (col,row)->Sprite"""
    s = Sprite(cols * fw, rows * fh)
    for (c, r), f in frames.items():
        s.blit(f, c * fw, r * fh)
    return s


# --------------------------------------------------------------------------
# Player (miner)
# --------------------------------------------------------------------------
def rotate_sprite(sp, deg, px, py):
    """Rotate counter-clockwise on screen by deg around (px,py)."""
    t = math.radians(deg)
    ct, st = math.cos(t), math.sin(t)
    out = Sprite(sp.w, sp.h)
    for y in range(sp.h):
        for x in range(sp.w):
            X, Y = x + 0.5 - px, y + 0.5 - py
            sx = px + X * ct - Y * st
            sy = py + X * st + Y * ct
            p = sp.get(int(math.floor(sx)), int(math.floor(sy)))
            if p[3]:
                out.set(x, y, p)
    return out


def normalize_bottom(sp, bottom=14, cx=8):
    pts = sp.points()
    if not pts:
        return sp
    maxy = max(p[1] for p in pts)
    minx = min(p[0] for p in pts)
    maxx = max(p[0] for p in pts)
    dx = int(round(cx - (minx + maxx + 1) / 2.0))
    dy = bottom - maxy
    out = Sprite(sp.w, sp.h)
    for (x, y) in pts:
        out.set(x + dx, y + dy, sp.get(x, y))
    return out


def draw_leg(sp, hip, foot, pants, pants_d):
    hx, hy = hip
    fx, fy = foot
    n = fy - hy
    for i in range(n):
        t = (i + 1) / (n + 0.0001) if n > 1 else 1.0
        x = int(round(hx + (fx - hx) * (i / max(1, n - 1)) if n > 1 else fx))
        y = hy + i
        sp.set(x, y, pants)
        sp.set(x + 1, y, pants_d)
    # boot: 3 px, toe forward (right)
    sp.set(fx, fy, "brown")
    sp.set(fx + 1, fy, "brown_l")
    sp.set(fx + 2, fy, "brown")


def draw_pick(sp, hx, hy, ang, length=6, head_cols=("grey_d", "grey", "grey_l", "white")):
    a = math.radians(ang)
    dx, dy = math.cos(a), -math.sin(a)
    # handle (starts 1 px behind the hand)
    for i in range(-1, length):
        x, y = hx + dx * i, hy + dy * i
        sp.set(round(x), round(y), "brown_l" if i % 3 else "brown")
    tx, ty = hx + dx * length, hy + dy * length
    px_, py_ = -dy, dx  # perpendicular
    for k in range(-3, 4):
        back = 0.9 if abs(k) == 3 else (0.35 if abs(k) == 2 else 0.0)
        x = tx + px_ * k - dx * back
        y = ty + py_ * k - dy * back
        c = head_cols[3] if abs(k) == 3 else (head_cols[1] if k > 0 else head_cols[2])
        sp.set(round(x), round(y), c)
    sp.set(round(tx), round(ty), head_cols[0])


def miner(bx=0, by=0, hand_f=None, hand_b=None, feet=None, pick=None, behind=False,
          lamp="white", flash=None, head_d=(0, 0), rot=0.0, hatless=False):
    sp = Sprite(16, 16)
    if hand_f is None:
        hand_f = (7 + bx, 10 + by)
    if hand_b is None:
        hand_b = (8 + bx, 10 + by)
    if feet is None:
        feet = ((6, 14), (8, 14))  # back, front (boot left x, y)
    if pick:
        hand_f = (pick[0], pick[1])
    if pick and behind:
        draw_pick(sp, pick[0], pick[1], pick[2])
    # back arm
    sx, sy = 8 + bx, 8 + by
    for (x, y) in line_pts(sx, sy, hand_b[0], hand_b[1])[:-1]:
        sp.set(x, y, "slate")
    sp.set(hand_b[0], hand_b[1], "skin_d")
    # legs
    hip_y = 11 + by
    draw_leg(sp, (6 + bx, hip_y), feet[0], "slate", "slate")
    draw_leg(sp, (8 + bx, hip_y), feet[1], "grey_d", "slate")
    # torso
    ox, oy = bx, by
    for y in range(8, 11):
        for x in range(6, 10):
            c = "blue_dk"
            if x == 6 or y == 8:
                c = "blue"
            if x == 9 and y > 8:
                c = "navy"
            sp.set(x + ox, y + oy, c)
    for x in range(6, 10):
        sp.set(x + ox, 10 + oy, "brown")
    sp.set(8 + ox, 10 + oy, "yellow_o")
    # head
    hx, hy = bx + head_d[0], by + head_d[1]
    sp.set(5 + hx, 6 + hy, "brown_dk")
    sp.set(5 + hx, 7 + hy, "brown_dk")
    for x in (6, 7, 9):
        sp.set(x + hx, 6 + hy, "skin")
    sp.set(8 + hx, 6 + hy, "black")
    sp.set(10 + hx, 6 + hy, "skin")
    sp.set(6 + hx, 7 + hy, "skin_d")
    for x in (7, 8, 9):
        sp.set(x + hx, 7 + hy, "skin")
    sp.set(9 + hx, 7 + hy, "skin_d")
    if not hatless:
        for x, c in zip(range(6, 10), ("yellow", "yellow", "yellow_o", "yellow_o")):
            sp.set(x + hx, 3 + hy, c)
        for x, c in zip(range(5, 10), ("yellow_o", "yellow", "yellow_o", "yellow_o", "orange")):
            sp.set(x + hx, 4 + hy, c)
        sp.set(10 + hx, 4 + hy, "grey_d")
        sp.set(11 + hx, 4 + hy, lamp)
        for x in range(5, 12):
            sp.set(x + hx, 5 + hy, "orange" if x < 10 else "rust")
    else:
        for x in range(6, 10):
            sp.set(x + hx, 5 + hy, "brown_dk")
        sp.set(5 + hx, 5 + hy, "brown_dk")
    # pick (in front)
    if pick and not behind:
        draw_pick(sp, pick[0], pick[1], pick[2])
    # front arm
    sx, sy = 7 + bx, 8 + by
    pts = line_pts(sx, sy, hand_f[0], hand_f[1])
    for (x, y) in pts[:-1]:
        sp.set(x, y, "cyan" if (x, y) == (sx, sy) else "blue")
    sp.set(hand_f[0], hand_f[1], "skin")
    if flash == "white":
        sp.map(lambda p: C["white"])
    elif flash == "red":
        def tint(p):
            lum = 0.3 * p[0] + 0.59 * p[1] + 0.11 * p[2]
            return C["salmon"] if lum > 150 else (C["red"] if lum > 70 else C["red_dk"])
        sp.map(tint)
    if rot:
        sp = rotate_sprite(sp, rot, 6.5, 14.5)
        sp = normalize_bottom(sp, 14, 8)
    sp.outline()
    return sp


LYING = [
    "................",
    "..LoS...........",
    "..YoES.......W..",
    ".YYoSSCBSK...w..",
    ".OOoSsBDDWGGGW..",
    "..orHHDNNWTTTW..",
]
LYING_KEY = {"L": None, "o": "orange", "S": "skin", "Y": "yellow", "E": "black", "W": "brown",
             "C": "cyan", "B": "blue", "K": "yellow_o", "w": "brown_l", "O": "yellow_o",
             "s": "skin_d", "D": "blue_dk", "T": "slate", "G": "grey_d", "r": "rust", "H": "brown_dk", "N": "navy"}


def miner_lying(lamp, dy=0):
    sp = Sprite(16, 16)
    for r, row in enumerate(LYING):
        for x, ch in enumerate(row):
            if ch != ".":
                sp.set(x, 9 + r + dy, lamp if ch == "L" else LYING_KEY[ch])
    sp.outline()
    return sp


def gen_player():
    frames = {}
    # row 0 idle: breathing (head dips on frames 3-4) and headlamp flicker
    lamps = ["white", "white", "yellow", "white", "yellow", "white"]
    bobs = [0, 0, 0, 1, 1, 0]
    for i in range(6):
        b = bobs[i]
        frames[(i, 0)] = miner(head_d=(0, b), hand_f=(7, 10), hand_b=(8, 10), lamp=lamps[i])
    # row 1 run
    for i in range(6):
        ph = 2 * math.pi * i / 6

        def foot(p):
            fc = 7.5 + 2.6 * math.cos(p)
            lift = int(round(1.4 * max(0.0, -math.sin(p))))
            return (int(round(fc - 1)), 14 - lift)
        by = 0 if i in (0, 3) else -1
        ff, bf = foot(ph), foot(ph + math.pi)
        hf = (8 + int(round(-2.2 * math.cos(ph))), 9 + by)
        hb = (7 + int(round(2.2 * math.cos(ph))), 9 + by)
        frames[(i, 1)] = miner(by=by, feet=(bf, ff), hand_f=hf, hand_b=hb)
    # row 2 jump: crouch, launch, tuck, apex
    frames[(0, 2)] = miner(by=1, feet=((5, 14), (8, 14)), hand_f=(5, 10), hand_b=(6, 10))
    frames[(1, 2)] = miner(by=-1, feet=((6, 14), (8, 13)), hand_f=(9, 4), hand_b=(5, 5))
    frames[(2, 2)] = miner(by=-1, feet=((5, 13), (9, 12)), hand_f=(10, 5), hand_b=(4, 6))
    frames[(3, 2)] = miner(by=-1, feet=((6, 13), (9, 12)), hand_f=(11, 7), hand_b=(4, 7))
    # row 3 fall: arms flailing up, legs dangling
    falls = [((10, 5), (4, 6), ((5, 14), (8, 14))),
             ((10, 4), (4, 5), ((5, 14), (9, 13))),
             ((11, 5), (3, 5), ((6, 14), (9, 14))),
             ((10, 4), (4, 6), ((5, 13), (8, 14)))]
    for i, (hf, hb, ft) in enumerate(falls):
        frames[(i, 3)] = miner(by=-1, feet=ft, hand_f=hf, hand_b=hb, lamp="white" if i % 2 == 0 else "yellow")
    # row 4 climb (facing the wall on the right; alternating reach)
    for i in range(6):
        s = math.sin(2 * math.pi * i / 6)
        hf = (12, int(round(5 - 2.0 * s)))
        hb = (12, int(round(5 + 2.0 * s)))
        ff = (10, 14 - int(round(2 * max(0.0, s))))
        bf = (9, 14 - int(round(2 * max(0.0, -s))))
        frames[(i, 4)] = miner(bx=1, feet=(bf, ff), hand_f=hf, hand_b=hb)
    # row 5 dig_side: ready, raise, wind-up, swing, impact, recoil
    side = [((8, 9), 70, False, 0), ((7, 8), 115, False, 0), ((6, 7), 150, True, -1),
            ((9, 8), 55, False, 0), ((10, 9), 5, False, 1), ((9, 9), 25, False, 1)]
    for i, (h, a, beh, bx) in enumerate(side):
        frames[(i, 5)] = miner(bx=bx, pick=(h[0], h[1], a), behind=beh, hand_b=h,
                               feet=((5, 14), (9, 14)) if bx > 0 else None)
    # row 6 dig_down
    down = [((8, 8), 100, False, 0), ((7, 7), 130, True, 0), ((8, 6), 95, False, 0),
            ((9, 9), 20, False, 0), ((9, 11), -60, False, 1), ((9, 11), -35, False, 1)]
    for i, (h, a, beh, by) in enumerate(down):
        frames[(i, 6)] = miner(by=by, pick=(h[0], h[1], a), behind=beh, hand_b=h,
                               feet=((5, 14), (8, 14)) if by else None)
    # row 7 dig_up
    up = [((9, 9), 30, False), ((9, 10), -10, False), ((8, 10), -30, False),
          ((9, 7), 60, False), ((9, 6), 88, False), ((9, 7), 75, False)]
    for i, (h, a, beh) in enumerate(up):
        frames[(i, 7)] = miner(pick=(h[0], h[1], a), behind=beh, hand_b=h)
    # row 8 hurt
    recoil = dict(bx=-1, feet=((4, 14), (7, 13)), hand_f=(9, 6), hand_b=(3, 7))
    frames[(0, 8)] = miner(flash="white", **recoil)
    frames[(1, 8)] = miner(head_d=(-1, 0), **recoil)
    frames[(2, 8)] = miner(flash="red", head_d=(-1, 0), bx=-1, feet=((4, 14), (7, 14)), hand_f=(9, 7), hand_b=(3, 8))
    frames[(3, 8)] = miner(feet=((5, 14), (8, 14)), hand_f=(8, 9), hand_b=(6, 9))
    # row 9 death: hit, knees buckle, kneel, topple, thud (bounce), lamp dies
    frames[(0, 9)] = miner(flash="red", head_d=(-1, 0), **recoil)
    frames[(1, 9)] = miner(bx=-1, by=1, feet=((4, 14), (8, 14)), hand_f=(8, 9), hand_b=(4, 10), head_d=(-1, 0))
    kneel = dict(bx=-1, by=2, feet=((4, 14), (8, 14)), hand_f=(7, 12), hand_b=(6, 12))
    frames[(2, 9)] = miner(**kneel)
    frames[(3, 9)] = miner(rot=35, **kneel)
    frames[(4, 9)] = miner_lying("white", dy=-1)
    frames[(5, 9)] = miner_lying("white")
    frames[(6, 9)] = miner_lying("yellow_o")
    frames[(7, 9)] = miner_lying("grey_d")
    return sheet(8, 10, 16, 16, frames)


# --------------------------------------------------------------------------
# Items
# --------------------------------------------------------------------------
def bez(p0, c, p2, t):
    return ((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * c[0] + t * t * p2[0],
            (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * c[1] + t * t * p2[1])


def pick_icon(sp, ramp=("grey_d", "grey", "grey_l", "white"), handle=("brown", "brown_l"),
              chips=(), skip_outline=False):
    # handle: 2px diagonal from bottom-left to top-right
    for (x, y) in line_pts(2, 14, 9, 7):
        sp.set(x, y, handle[1])
        sp.set(x + 1, y, handle[0])
    sp.set(2, 14, "brown_dk")
    sp.set(3, 14, "brown_dk")
    sp.set(3, 13, "brown_dk")
    # head: arched bezier
    p0, cc, p2 = (3.5, 3.5), (12.6, 3.0), (12.5, 12.5)
    outer, inner = set(), set()
    for i in range(80):
        t = i / 79.0
        x, y = bez(p0, cc, p2, t)
        outer.add((int(x), int(y)))
        if 0.18 < t < 0.82:
            inner.add((int(x - 0.8), int(y + 0.8)))
    inner -= outer
    for p in outer:
        sp.set(p[0], p[1], ramp[2])
    for p in inner:
        sp.set(p[0], p[1], ramp[1])
    sp.set(3, 3, ramp[3])
    sp.set(12, 12, ramp[3])
    sp.set(4, 3, ramp[3])
    sp.set(12, 11, ramp[2])
    # ferrule
    sp.set(9, 6, ramp[0])
    sp.set(10, 7, ramp[0])
    for p in chips:
        sp.set(p[0], p[1], CLEAR)
    if not skip_outline:
        sp.outline()


def ico_crumbling(sp):
    pick_icon(sp, ramp=("grey_d", "tan", "cream", "cream"), handle=("brown_dk", "brown"), chips=[(12, 12), (12, 11), (4, 3)])
    sp.set(7, 4, "brown_dk")
    sp.set(8, 4, "brown")
    sp.set(11, 8, "brown_dk")
    for (x, y, c) in [(13, 13, "tan"), (12, 14, "brown_l"), (14, 15, "tan"), (11, 15, "brown_l"), (14, 11, "brown_l")]:
        sp.set(x, y, c)


def ico_magma(sp):
    pick_icon(sp, ramp=("red_dk", "orange", "yellow_o", "yellow"), handle=("brown_dk", "brown"))
    sp.set(8, 3, "yellow")
    sp.set(11, 6, "white")
    for (x, y, c) in [(13, 13, "orange"), (13, 14, "yellow_o"), (12, 15, "orange"), (5, 5, "orange"), (5, 6, "red"), (14, 9, "orange")]:
        sp.set(x, y, c)


def ico_glass(sp):
    pick_icon(sp, ramp=("blue", "cyan", "cyan", "white"), handle=("grey_d", "grey_l"))
    sp.set(8, 3, "white")
    sp.set(9, 4, "blue")
    sp.set(10, 5, "blue")
    sp.set(11, 8, "blue")
    for (x, y) in [(14, 2), (13, 1), (15, 1), (14, 0)]:
        sp.set(x, y, "white")
    sp.set(14, 1, "cyan")
    sp.set(1, 7, "cyan")


def ico_pickaxe(sp):
    pick_icon(sp)


def ico_water_canister(sp):
    body = m_rect(4, 4, 11, 14) - {(4, 4), (11, 4), (4, 14), (11, 14)}
    for (x, y) in body:
        t = (x - 4) / 7.0
        c = ["grey_l", "grey_l", "grey", "grey", "grey", "grey_d", "grey_d", "slate"][x - 4]
        sp.set(x, y, c)
    for x in range(4, 12):
        sp.set(x, 6, "slate" if x > 9 else "grey_d")
        sp.set(x, 13, "slate" if x > 9 else "grey_d")
    sp.rect(6, 2, 9, 3, "grey_d")
    sp.set(6, 2, "grey")
    sp.set(7, 2, "grey_l")
    # drop
    drop = m_ascii(["..#..", ".###.", "#####", "#####", ".###."], 5, 7)
    shade_fill(sp, drop, "blue_dk", "blue", "cyan", hi="white")
    sp.set(6, 9, "white")
    sp.outline()


def ico_acid_flask(sp):
    body = m_ellipse(8, 10.5, 5.2, 4.8)
    for (x, y) in body:
        if y >= 9:
            sp.set(x, y, "green")
        else:
            sp.set(x, y, with_alpha("grey_l", 170))
    for (x, y) in body:
        if y >= 9:
            if (x + 1, y) not in body or (x, y + 1) not in body:
                sp.set(x, y, "green_dk")
            elif y == 9:
                sp.set(x, y, "green_l")
    sp.rect(7, 3, 8, 5, with_alpha("grey_l", 190))
    sp.rect(6, 1, 9, 2, "brown_l")
    sp.set(9, 2, "brown")
    sp.set(9, 1, "brown")
    sp.set(4, 8, "white")
    sp.set(4, 9, "white")
    sp.set(5, 7, "white")
    sp.outline()
    for (x, y, c) in [(7, 11, "green_l"), (10, 12, "yellow"), (9, 10, "green_l"), (6, 13, "green_l")]:
        sp.set(x, y, c)
    for (x, y, c) in [(11, 2, "green_l"), (12, 0, "green_l"), (13, 3, "yellow")]:
        sp.set(x, y, c)


def ico_blast_charges(sp):
    for i, x0 in enumerate((3, 6, 9)):
        top = 7 if i != 1 else 6
        for y in range(top, 15):
            sp.set(x0, y, "salmon")
            sp.set(x0 + 1, y, "red")
            sp.set(x0 + 2, y, "red_dk")
        sp.set(x0 + 1, top, "brown_dk")
    for x in range(3, 12):
        sp.set(x, 10, "brown_dk" if x % 3 else "brown")
        sp.set(x, 11, "brown")
    sp.outline()
    for (x, y) in [(7, 5), (8, 4), (9, 3), (10, 3)]:
        sp.set(x, y, "cream")
    sp.set(11, 2, "yellow")
    sp.set(12, 1, "white")
    sp.set(12, 3, "orange")
    sp.set(13, 2, "yellow_o")
    sp.set(11, 1, "orange")


def ico_salamander(sp):
    m = m_ascii([
        "......##......",
        ".....####.....",
        ".#...####...#.",
        ".##..####..##.",
        "..##########..",
        "....######....",
        ".....####.....",
        ".....####.....",
        "....######....",
        "..##########..",
        ".##..####..##.",
        ".#....##....#.",
        "......##......",
        ".......##.....",
        "........#.....",
    ], 1, 0)
    shade_fill(sp, m, "red_dk", "orange", "yellow_o")
    for (x, y) in m:
        inner = all(((x + dx, y + dy) in m) for dx, dy in [(1, 0), (-1, 0), (0, 1), (0, -1)])
        if inner and (x + 2 * y) % 3 == 0:
            sp.set(x, y, "rust")
    for y in range(2, 12):
        sp.set(7 if y % 2 else 8, y, "yellow_o" if y % 3 else "yellow")
    sp.set(6, 1, "black")
    sp.set(9, 1, "black")
    sp.outline()


def ico_gill_mask(sp):
    frame = m_rect(2, 4, 13, 10) - {(2, 4), (13, 4), (2, 10), (13, 10)}
    sp.fill(frame, "slate")
    for (x, y) in frame:
        if y == 4:
            sp.set(x, y, "grey_d")
    for lx in (3, 8):
        lens = m_rect(lx, 5, lx + 4, 8) - {(lx, 5), (lx + 4, 8)}
        shade_fill(sp, lens, "blue_dk", "blue", "cyan")
        sp.set(lx + 1, 6, "white")
    sp.set(7, 6, "navy")
    sp.set(7, 7, "navy")
    mouth = m_rect(5, 11, 10, 13) - {(5, 13), (10, 13)}
    shade_fill(sp, mouth, "green_dk", "green", "green_l")
    for x in (6, 8, 10):
        sp.set(x - 0, 12, "teal_dk")
    sp.set(1, 6, "brown")
    sp.set(1, 7, "brown")
    sp.set(14, 6, "brown")
    sp.set(14, 7, "brown")
    sp.outline()


def ico_heavy_boots(sp):
    back = m_ascii(["#####...", "#####...", "#####...", "#####...", "######..", "#########", "#########"], 6, 3)
    shade_fill(sp, back, "navy", "slate", "grey_d")
    front = m_ascii(["######...", "######...", "######...", "######...", "#######..", "#########", "#########", "#########"], 2, 6)
    shade_fill(sp, front, "slate", "grey_d", "grey", hi="grey_l")
    for x in range(2, 11):
        sp.set(x, 13, "navy")
    for x in range(2, 8):
        sp.set(x, 7, "slate")
    sp.set(3, 9, "grey_l")
    sp.set(6, 9, "grey_l")
    sp.set(9, 11, "grey_l")
    sp.outline()


def ico_thermal_suit(sp):
    vest = m_poly([(3, 2), (6, 2), (8, 5), (10, 2), (13, 2), (14, 6), (13, 7), (13, 15), (3, 15), (3, 7), (2, 6)])
    shade_fill(sp, vest, "rust", "orange", "yellow_o")
    for y in (7, 10, 13):
        for x in range(3, 14):
            if (x, y) in vest:
                sp.set(x, y, "orange_dk")
    for y in range(6, 15):
        if (8, y) in vest:
            sp.set(8, y, "grey_l" if y % 2 else "grey")
    for x in range(3, 14):
        if (x, 11) in vest and x != 8:
            sp.set(x, 11, "grey_l")
    sp.outline()
    for (x, y) in [(5, 9), (6, 8), (5, 7)]:
        sp.set(x, y, "yellow")
    for (x, y) in [(11, 9), (12, 8), (11, 7)]:
        sp.set(x, y, "yellow")


def ico_frost_seed(sp):
    m = m_ellipse(8, 9.5, 3.6, 4.5) | {(8, 4), (7, 4), (8, 3)}
    shade_fill(sp, m, "blue_dk", "blue", "cyan", hi="white")
    sp.set(6, 8, "white")
    sp.set(7, 7, "white")
    for y in range(9, 13):
        sp.set(9, y, "blue_dk")
    sp.outline()
    for (cx, cy) in [(2, 3), (13, 5), (12, 14), (3, 12)]:
        sp.set(cx, cy, "white")
        for dx, dy in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
            if sp.get(cx + dx, cy + dy)[3] == 0:
                sp.set(cx + dx, cy + dy, "cyan")


def ico_fungal(sp):
    cap = m_ellipse(8, 7.5, 6.0, 4.2)
    cap = {p for p in cap if p[1] <= 8}
    shade_fill(sp, cap, "green", "green_l", "green_l")
    for (x, y) in cap:
        if y == 8:
            sp.set(x, y, "green_dk")
    for (x, y) in [(5, 5), (9, 4), (11, 6), (7, 7)]:
        sp.set(x, y, "yellow")
    stem = m_rect(6, 9, 9, 14)
    shade_fill(sp, stem, "tan", "cream", "cream")
    sp.set(6, 14, "tan")
    sp.outline()
    for (x, y, c) in [(1, 2, "green_l"), (3, 1, "yellow"), (13, 1, "green_l"), (14, 3, "yellow"), (12, 12, "green_l"), (2, 11, "green_l")]:
        sp.set(x, y, c)


def ico_pocket_sun(sp):
    sphere(sp, 8, 8, 4.6, ["orange", "yellow_o", "yellow", "white"])
    sp.outline("rust")
    for a in range(8):
        t = a * math.pi / 4
        dx, dy = math.cos(t), math.sin(t)
        for r in (6.5, 7.5):
            x, y = 8 + dx * r - 0.5, 8 + dy * r - 0.5
            if a % 2 == 0 or r == 6.5:
                sp.set(round(x), round(y), "yellow" if r == 6.5 else "yellow_o")


HEART = [".##...##.",
         "####.####",
         "#########",
         "#########",
         ".#######.",
         "..#####..",
         "...###...",
         "....#...."]
HEART_BIG = ["..##...##..",
             ".####.####.",
             "###########",
             "###########",
             "###########",
             ".#########.",
             "..#######..",
             "...#####...",
             "....###....",
             ".....#....."]


def ico_beacon_heart(sp):
    m = m_ascii(HEART, 4, 5)
    shade_fill(sp, m, "red_dk", "red", "salmon")
    sp.set(5, 6, "white")
    sp.set(6, 6, "salmon")
    sp.set(8, 8, "salmon")
    sp.outline()
    for (x, y) in [(8, 1), (8, 2), (1, 9), (2, 9), (14, 9), (15, 9), (2, 3), (3, 4), (13, 4), (14, 3), (3, 14), (13, 14)]:
        sp.set(x, y, "yellow")
    sp.set(8, 0, "yellow_o")
    sp.set(0, 9, "yellow_o")


def ico_volatile_core(sp):
    sphere(sp, 8, 8.5, 5.3, ["purple", "pink", "pink", "salmon"])
    for (x, y) in [(6, 5), (7, 6), (7, 7), (8, 8), (9, 9), (10, 9), (11, 10), (7, 8), (6, 9), (6, 10), (9, 7), (10, 6)]:
        sp.set(x, y, "white" if (x + y) % 3 else "yellow")
    sp.set(8, 8, "white")
    sp.outline()
    for (x, y, c) in [(2, 2, "hot"), (14, 3, "salmon"), (13, 14, "hot"), (1, 12, "salmon"), (15, 9, "white")]:
        sp.set(x, y, c)


def ico_spark_rod(sp):
    for (x, y) in line_pts(3, 14, 10, 7):
        sp.set(x, y, "grey_l")
        sp.set(x + 1, y, "grey_d")
    for (x, y) in line_pts(3, 14, 5, 12):
        sp.set(x, y, "brown_l")
        sp.set(x + 1, y, "brown")
    tip = m_ellipse(11.5, 5.5, 2.3, 2.3)
    shade_fill(sp, tip, "blue", "cyan", "white")
    sp.outline()
    for (x, y, c) in [(14, 1, "white"), (13, 2, "cyan"), (15, 3, "cyan"), (8, 2, "yellow"), (9, 1, "white"), (7, 1, "yellow"),
                      (14, 8, "cyan"), (15, 9, "white"), (13, 9, "yellow")]:
        if sp.get(x, y)[3] == 0:
            sp.set(x, y, c)


def ico_seismic(sp):
    boot = m_ascii(["####..", "####..", "####..", "#####.", "######", "######"], 5, 5)
    shade_fill(sp, boot, "slate", "grey_d", "grey", hi="grey_l")
    for x in range(5, 11):
        sp.set(x, 10, "navy")
    sp.set(6, 7, "grey_l")
    sp.outline()
    for x in range(0, 16):
        sp.set(x, 13, "brown" if x % 4 else "brown_l")
    for (x, y) in [(2, 10), (1, 9), (3, 11), (13, 10), (14, 9), (12, 11), (0, 7), (15, 7)]:
        sp.set(x, y, "cream")
    for (x, y) in [(7, 14), (8, 15), (6, 15), (10, 14)]:
        sp.set(x, y, "brown_dk")


def draw_chest(sp, x0, y0, open_=False):
    # interior box 10 wide x 8 tall at (x0,y0)
    if not open_:
        lid = m_rect(x0, y0, x0 + 9, y0 + 2) - {(x0, y0), (x0 + 9, y0)}
        shade_fill(sp, lid, "brown", "brown_l", "tan")
        for x in range(x0, x0 + 10):
            sp.set(x, y0 + 3, "brown_dk")
    else:
        lid = m_rect(x0, y0 - 3, x0 + 9, y0 - 1) - {(x0, y0 - 3), (x0 + 9, y0 - 3)}
        shade_fill(sp, lid, "brown", "brown_l", "tan")
        sp.rect(x0 + 1, y0, x0 + 8, y0 + 3, "brown_dk")
        for x in range(x0 + 1, x0 + 9):
            sp.set(x, y0 + 2, "yellow_o" if x % 2 else "yellow")
            sp.set(x, y0 + 3, "yellow_o")
        sp.set(x0 + 3, y0 + 1, "yellow")
        sp.set(x0 + 6, y0 + 1, "yellow_o")
        sp.set(x0, y0, "brown")
        sp.set(x0 + 9, y0, "brown")
        sp.set(x0, y0 + 1, "brown")
        sp.set(x0 + 9, y0 + 1, "brown")
        sp.set(x0, y0 + 2, "brown")
        sp.set(x0 + 9, y0 + 2, "brown")
        sp.set(x0, y0 + 3, "brown_dk")
        sp.set(x0 + 9, y0 + 3, "brown_dk")
    body = m_rect(x0, y0 + 4, x0 + 9, y0 + 7)
    shade_fill(sp, body, "brown", "brown_l", "brown_l")
    for x in range(x0, x0 + 10):
        sp.set(x, y0 + 7, "brown")
    for bx in (x0 + 1, x0 + 8):
        for y in range(y0 - (3 if open_ else 0), y0 + 8):
            if y != y0 + 3 or not open_:
                if not (open_ and y0 <= y <= y0 + 3):
                    sp.set(bx, y, "grey" if y % 3 else "grey_l")
    if not open_:
        sp.rect(x0 + 4, y0 + 3, x0 + 5, y0 + 5, "yellow_o")
        sp.set(x0 + 4, y0 + 3, "yellow")
        sp.set(x0 + 5, y0 + 5, "orange")
        sp.set(x0 + 4, y0 + 4, "black")


def ico_chest_closed(sp):
    draw_chest(sp, 3, 5)
    sp.outline()


def ico_chest_open(sp):
    draw_chest(sp, 3, 6, open_=True)
    sp.outline()
    sp.set(5, 3, "white") if sp.get(5, 3)[3] == 0 else None
    sp.set(12, 4, "yellow")


def draw_altar(sp, x0, y0, w=12):
    # slab
    slab = m_rect(x0, y0, x0 + w - 1, y0 + 1)
    shade_fill(sp, slab, "grey_d", "grey", "grey_l")
    col_ = m_rect(x0 + 2, y0 + 2, x0 + w - 3, y0 + 5)
    shade_fill(sp, col_, "slate", "grey_d", "grey")
    base = m_rect(x0 + 1, y0 + 6, x0 + w - 2, y0 + 7)
    shade_fill(sp, base, "slate", "grey_d", "grey")
    cx = x0 + w // 2
    sp.set(cx - 1, y0 + 3, "cyan")
    sp.set(cx, y0 + 3, "blue")
    sp.set(cx - 1, y0 + 4, "blue")
    sp.set(cx, y0 + 4, "cyan")


def ico_altar(sp):
    draw_altar(sp, 2, 6)
    sp.outline()
    sp.set(7, 2, "cyan")
    sp.set(8, 3, "white")
    sp.set(8, 1, "cyan")
    sp.set(9, 2, "cyan")


def draw_shrine(sp, x0, y0, w, h):
    # pointed-arch stone shrine with glowing sigil in a niche
    cx = x0 + w / 2.0
    poly = [(x0, y0 + h), (x0, y0 + h * 0.35), (cx, y0), (x0 + w, y0 + h * 0.35), (x0 + w, y0 + h)]
    m = m_poly(poly)
    shade_fill(sp, m, "slate", "grey_d", "grey", hi="grey_l")
    # brick lines
    for (x, y) in m:
        if (y - y0) % 4 == 3 and (x, y - 1) in m and (x, y + 1) in m and (x - 1, y) in m and (x + 1, y) in m:
            sp.set(x, y, "slate")
        if (y - y0) % 4 in (0, 1, 2) and (x - x0 + ((y - y0) // 4) * 3) % 6 == 0 and (x - 1, y) in m and (x + 1, y) in m and (x, y - 1) in m:
            sp.set(x, y, "slate")
    # niche
    nw = max(4, int(w * 0.5))
    nx0 = int(round(cx - nw / 2.0))
    ny0 = int(y0 + h * 0.3)
    ny1 = int(y0 + h * 0.78)
    niche = m_poly([(nx0, ny1), (nx0, ny0 + 2), (cx, ny0 - 1), (nx0 + nw, ny0 + 2), (nx0 + nw, ny1)])
    sp.fill(niche, "navy")
    for (x, y) in niche:
        if (x, y - 1) not in niche or (x - 1, y) not in niche:
            sp.set(x, y, "black")
    # base step
    for x in range(x0 - 1, x0 + w + 1):
        sp.set(x, y0 + h, "grey_d")
    return niche, int(cx), (ny0 + ny1) // 2


def ico_shrine(sp):
    niche, cx, cy = draw_shrine(sp, 3, 1, 10, 13)
    for (x, y, c) in [(cx, cy - 2, "pink"), (cx - 1, cy - 1, "pink"), (cx, cy - 1, "white"), (cx - 1, cy, "salmon"),
                      (cx, cy, "pink"), (cx - 1, cy - 2, "purple"), (cx, cy + 1, "purple")]:
        sp.set(x, y, c)
    sp.outline()


def ico_rope(sp):
    for r, c in [(6.0, "brown_l"), (4.6, "tan"), (3.2, "brown_l")]:
        ring = m_ellipse(8, 9.5, r, r * 0.7) - m_ellipse(8, 9.5, r - 1.2, (r - 1.2) * 0.7)
        for (x, y) in ring:
            sp.set(x, y, c if (x + y) % 3 else "brown")
    for (x, y) in [(13, 9), (14, 10), (14, 11), (13, 12), (13, 13), (14, 14)]:
        sp.set(x, y, "tan")
    sp.outline()


def ico_torch(sp):
    for y in range(8, 15):
        sp.set(7, y, "brown_l")
        sp.set(8, y, "brown")
    sp.rect(6, 7, 9, 8, "grey_d")
    sp.set(6, 7, "grey")
    draw_flame(sp, 8, 1, 6, 2.8, lean=0.6)
    sp.outline()


def ico_ore(sp):
    m = m_poly([(3, 7), (6, 3), (11, 4), (14, 8), (12, 13), (6, 14), (2, 11)])
    shade_fill(sp, m, "slate", "grey_d", "grey")
    rng = random.Random(23)
    for (x, y) in sorted(m):
        if rng.random() < 0.12 and (x + 1, y) in m and (x, y + 1) in m:
            sp.set(x, y, "slate")
    for (x, y) in [(6, 6), (9, 8), (10, 7), (7, 10), (11, 11), (5, 11)]:
        sp.set(x, y, "yellow_o")
    for (x, y) in [(6, 5), (10, 6), (7, 9)]:
        sp.set(x, y, "yellow")
    sp.outline()


def ico_gem(sp):
    crown = m_poly([(5, 4), (11, 4), (14, 7), (2, 7)])
    pav = m_poly([(2, 7), (14, 7), (8, 14)])
    sp.fill(crown, "salmon")
    sp.fill(pav, "pink")
    for (x, y) in pav:
        if x >= 8 + (y - 7) * 0.0 and x > 8:
            sp.set(x, y, "purple")
    for (x, y) in crown:
        if x > 9:
            sp.set(x, y, "pink")
    for x in range(2, 15):
        if (x, 7) in crown | pav:
            sp.set(x, 7, "white" if x < 6 else "salmon")
    sp.set(6, 5, "white")
    sp.set(7, 5, "white")
    sp.outline()
    sp.set(13, 2, "white")
    sp.set(13, 1, "salmon")
    sp.set(12, 2, "salmon")
    sp.set(14, 2, "salmon")


def heart_icon(sp, kind):
    m = m_ascii(HEART_BIG, 3, 3)
    if kind == "empty":
        shade_fill(sp, m, "navy", "slate", "grey_d")
    else:
        shade_fill(sp, m, "red_dk", "red", "salmon", hi="white")
        sp.set(5, 5, "white")
        if kind == "half":
            empty = {p for p in m if p[0] >= 8}
            shade_fill(sp, empty, "navy", "slate", "grey_d")
            sp.fill({p for p in m if p[0] == 8}, "slate")
    sp.outline()


def ico_lock(sp):
    sh = m_ellipse(8, 6, 3.6, 4.0) - m_ellipse(8, 6, 2.0, 2.6)
    sh = {p for p in sh if p[1] <= 7}
    shade_fill(sp, sh, "grey_d", "grey", "grey_l")
    body = m_rect(3, 7, 12, 14) - {(3, 14), (12, 14)}
    shade_fill(sp, body, "orange", "yellow_o", "yellow", hi="white")
    sp.set(7, 9, "black")
    sp.set(8, 9, "black")
    sp.set(7, 10, "black")
    sp.set(8, 10, "black")
    sp.set(7, 11, "black")
    sp.set(8, 11, "brown_dk")
    sp.outline()


def ico_flame(sp):
    draw_flame(sp, 8, 1, 14, 5.2, lean=1.5)
    sp.outline()


def ico_bubble(sp):
    m = m_ellipse(7.5, 8.5, 5.6, 5.6)
    for (x, y) in m:
        sp.set(x, y, with_alpha("blue", 90))
    ring = m - m_ellipse(7.5, 8.5, 4.5, 4.5)
    for (x, y) in ring:
        sp.set(x, y, "cyan" if x + y < 17 else "blue")
    sp.set(5, 5, "white")
    sp.set(4, 6, "white")
    sp.set(6, 5, "white")
    sp.set(4, 7, "white")
    sp.outline()
    small = m_ellipse(13.5, 3.5, 1.6, 1.6)
    for (x, y) in small:
        sp.set(x, y, "cyan")
    sp.set(13, 3, "white")


def ico_thermo(sp):
    tube = m_rect(7, 2, 8, 11)
    sp.fill(tube, "grey_l")
    sp.set(8, 2, "grey")
    for y in range(6, 12):
        sp.set(7, y, "red")
        sp.set(8, y, "red_dk")
    bulb = m_ellipse(8, 12.5, 2.6, 2.6)
    shade_fill(sp, bulb, "red_dk", "red", "salmon", hi="white")
    sp.outline()
    for y in (3, 5, 7, 9):
        sp.set(10, y, "grey_l")
        sp.set(11, y, "grey" if y % 4 == 1 else CLEAR)


def ico_skull(sp):
    m = m_ascii([".######.", "########", "########", "########", "########", ".######.", "..####..", "..#.##.."], 4, 3)
    m = m_ascii(["..#####..", ".#######.", "#########", "#########", "#########", "#########", ".#######.", "..#####..", "..#.#.#.."], 3, 3)
    shade_fill(sp, m, "grey", "cream", "white")
    for (x, y) in [(5, 7), (6, 7), (5, 8), (6, 8), (9, 7), (10, 7), (9, 8), (10, 8)]:
        sp.set(x, y, "black")
    sp.set(6, 7, "brown_dk")
    sp.set(10, 7, "brown_dk")
    sp.set(7, 9, "grey_d")
    sp.set(8, 9, "grey_d")
    sp.set(6, 11, "grey")
    sp.set(8, 11, "grey")
    sp.set(10, 11, "grey")
    sp.outline()


def ico_trophy(sp):
    cup = m_poly([(3, 2), (13, 2), (12, 6), (10, 9), (6, 9), (4, 6)])
    shade_fill(sp, cup, "orange", "yellow_o", "yellow", hi="white")
    for (x, y) in [(2, 3), (1, 4), (2, 5), (3, 6), (14, 3), (15, 4), (14, 5), (13, 6)]:
        sp.set(x, y, "yellow_o")
    sp.rect(7, 10, 8, 11, "yellow_o")
    sp.set(8, 11, "orange")
    sp.rect(5, 12, 10, 14, "brown")
    sp.rect(5, 12, 10, 12, "brown_l")
    sp.set(7, 13, "yellow_o")
    sp.set(8, 13, "yellow_o")
    sp.set(5, 3, "white")
    sp.outline()


def ico_question(sp):
    m = m_ascii([".####.", "##..##", "....##", "...##.", "..##..", "..##..", "......", "..##..", "..##.."], 5, 3)
    m = m_ascii(["..####..", ".######.", "##....##", "......##", "....###.", "...###..", "...##...", "........", "...##...", "...##..."], 4, 3)
    shade_fill(sp, m, "grey", "grey_l", "white")
    sp.outline()


def ico_compass(sp):
    rim = m_ellipse(8, 8.5, 6.2, 6.2)
    shade_fill(sp, rim, "orange", "yellow_o", "yellow")
    face = m_ellipse(8, 8.5, 4.8, 4.8)
    sp.fill(face, "cream")
    for (x, y) in face:
        if (x + 1, y) not in face or (x, y + 1) not in face:
            sp.set(x, y, "tan")
    sp.set(8, 2, "brown")
    sp.set(7, 2, "brown")
    for (x, y) in [(10, 6), (11, 5), (9, 7)]:
        sp.set(x, y, "red")
    for (x, y) in [(7, 9), (6, 10), (5, 11)]:
        sp.set(x, y, "grey_d")
    sp.set(8, 8, "black")
    sp.outline()


def ico_hourglass(sp):
    for x in range(3, 13):
        sp.set(x, 1, "brown_l")
        sp.set(x, 2, "brown")
        sp.set(x, 13, "brown_l")
        sp.set(x, 14, "brown")
    glass = m_poly([(4, 3), (12, 3), (12, 4), (8.5, 8), (12, 12), (12, 13), (4, 13), (4, 12), (7.5, 8), (4, 4)])
    for (x, y) in glass:
        sp.set(x, y, with_alpha("grey_l", 140))
    for (x, y) in glass:
        if y >= 10:
            sp.set(x, y, "tan" if y > 10 else "yellow_o")
        if 4 <= y <= 5:
            sp.set(x, y, "tan")
    sp.set(8, 8, "yellow_o")
    sp.set(8, 9, "yellow_o")
    sp.set(8, 7, "tan")
    sp.set(5, 4, "white")
    sp.outline()


def gen_items():
    fns = [ico_crumbling, ico_magma, ico_glass, ico_water_canister, ico_acid_flask, ico_blast_charges,
           ico_salamander, ico_gill_mask, ico_heavy_boots, ico_thermal_suit, ico_frost_seed, ico_fungal,
           ico_pocket_sun, ico_beacon_heart, ico_volatile_core, ico_spark_rod, ico_seismic,
           ico_chest_closed, ico_chest_open, ico_altar, ico_shrine, ico_rope, ico_torch, ico_ore, ico_gem,
           lambda s: heart_icon(s, "full"), lambda s: heart_icon(s, "half"), lambda s: heart_icon(s, "empty"),
           ico_lock, ico_flame, ico_bubble, ico_thermo, ico_skull, ico_trophy, ico_pickaxe, ico_question,
           ico_compass, ico_hourglass]
    frames = {}
    for i, fn in enumerate(fns):
        s = Sprite(16, 16)
        fn(s)
        frames[(i % 8, i // 8)] = s
    return sheet(8, 5, 16, 16, frames)


# --------------------------------------------------------------------------
# Props (world scale, 16x24 frames, bottom aligned)
# --------------------------------------------------------------------------
def gen_props():
    frames = {}
    flames = [
        ["..r..", ".ro..", ".oyo.", "royyo", "oyWYo", ".oyo."],
        ["...r.", "..ro.", ".oyo.", ".oYyr", "oyWYo", ".oyo."],
        [".....", ".r...", ".or..", ".oyo.", "rYWyo", ".oyo."],
        ["..r..", "..o..", ".ryo.", "royYo", "oYWyo", ".oyo."],
    ]
    fkey = {"r": "red", "o": "orange", "y": "yellow_o", "Y": "yellow", "W": "white"}
    embers = [[(10, 10)], [(6, 9)], [(9, 9), (5, 11)], [(10, 8)]]
    for i, fl in enumerate(flames):
        s = Sprite(16, 24)
        for y in range(19, 23):
            s.set(7, y, "brown_l")
            s.set(8, y, "brown")
        s.rect(6, 18, 9, 18, "grey_d")
        s.set(6, 18, "grey")
        s.outline()
        for r, row in enumerate(fl):
            for x, ch in enumerate(row):
                if ch != ".":
                    s.set(6 + x, 12 + r, fkey[ch])
        for (x, y) in embers[i]:
            s.set(x, y, "yellow_o")
        frames[(i, 0)] = s
    s = Sprite(16, 24)
    draw_chest(s, 3, 15)
    s.outline()
    frames[(4, 0)] = s
    s = Sprite(16, 24)
    draw_chest(s, 3, 15, open_=True)
    s.outline()
    s.set(5, 10, "yellow")
    s.set(10, 11, "white")
    frames[(5, 0)] = s
    s = Sprite(16, 24)
    draw_altar(s, 2, 15)
    s.outline()
    frames[(6, 0)] = s
    s = Sprite(16, 24)
    niche, cx, cy = draw_shrine(s, 1, 1, 14, 21)
    sig = [(cx, cy - 3, "pink"), (cx - 1, cy - 2, "pink"), (cx, cy - 2, "salmon"), (cx + 1, cy - 2, "pink"),
           (cx - 2, cy - 1, "purple"), (cx - 1, cy - 1, "salmon"), (cx, cy - 1, "white"), (cx + 1, cy - 1, "salmon"), (cx + 2, cy - 1, "purple"),
           (cx - 1, cy, "pink"), (cx, cy, "salmon"), (cx + 1, cy, "pink"), (cx, cy + 1, "pink"), (cx, cy + 2, "purple")]
    for (x, y, c) in sig:
        s.set(x, y, c)
    # glow around sigil (inside niche)
    for (x, y) in niche:
        if s.get(x, y) == C["navy"] and abs(x - cx) + abs(y - (cy - 1)) <= 4:
            s.set(x, y, "purple")
    s.outline()
    frames[(7, 0)] = s
    return sheet(8, 1, 16, 24, frames)


# --------------------------------------------------------------------------
# Tileable noise
# --------------------------------------------------------------------------
class TNoise:
    def __init__(self, rng, period):
        self.p = period
        self.g = [rng.random() for _ in range(period * period)]

    def at(self, x, y):
        p = self.p
        xi, yi = int(math.floor(x)), int(math.floor(y))
        fx, fy = x - xi, y - yi
        fx = fx * fx * (3 - 2 * fx)
        fy = fy * fy * (3 - 2 * fy)
        x0, x1 = xi % p, (xi + 1) % p
        y0, y1 = yi % p, (yi + 1) % p
        g = self.g
        a = g[y0 * p + x0] + (g[y0 * p + x1] - g[y0 * p + x0]) * fx
        b = g[y1 * p + x0] + (g[y1 * p + x1] - g[y1 * p + x0]) * fx
        return a + (b - a) * fy


class FBM:
    def __init__(self, rng, base, octaves, pers=0.5, size=256):
        self.n = [TNoise(rng, base * (2 ** o)) for o in range(octaves)]
        self.a = [pers ** o for o in range(octaves)]
        self.tot = sum(self.a)
        self.size = size

    def __call__(self, x, y):
        v = 0.0
        for n, a in zip(self.n, self.a):
            v += a * n.at(x * n.p / self.size, y * n.p / self.size)
        return v / self.tot


class Worley:
    def __init__(self, rng, cells, size=256):
        self.c = cells
        self.cs = size / cells
        self.size = size
        self.pts = [(rng.random(), rng.random()) for _ in range(cells * cells)]

    def __call__(self, x, y):
        cs, c = self.cs, self.c
        cx, cy = int(x // cs), int(y // cs)
        d1 = d2 = 1e9
        for oy in (-1, 0, 1):
            for ox in (-1, 0, 1):
                gx, gy = cx + ox, cy + oy
                px, py = self.pts[(gy % c) * c + (gx % c)]
                X, Y = (gx + px) * cs, (gy + py) * cs
                d = math.hypot(X - x, Y - y)
                if d < d1:
                    d1, d2 = d, d1
                elif d < d2:
                    d2 = d
        return d1, d2


def lum(c):
    return (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]) / 255.0


# --------------------------------------------------------------------------
# Backgrounds
# --------------------------------------------------------------------------
S = 256


def finish_near(sp, rim, shade, alpha=235):
    """Rim-light top-left edges and shadow bottom-right edges of opaque
    silhouettes in a wrapping sprite."""
    sp.wrap = True
    src = sp.copy()
    src.wrap = True
    for y in range(S):
        for x in range(S):
            p = src.get(x, y)
            if not p[3]:
                continue
            if not src.opaque(x - 1, y) or not src.opaque(x, y - 1):
                sp.set(x, y, with_alpha(rim, p[3]))
            elif not src.opaque(x + 1, y) or not src.opaque(x, y + 1) or not src.opaque(x + 1, y + 1):
                sp.set(x, y, with_alpha(shade, p[3]))


def draw_wrapped_poly(sp, poly, colorfn):
    for (x, y) in m_poly(poly):
        c = colorfn(x, y)
        if c:
            sp.set(x, y, c)


def bg_layer0():
    rng = random.Random(1000)
    far = Sprite(S, S)
    far.wrap = True
    warp = FBM(rng, 4, 3)
    tex = FBM(rng, 8, 4)
    big = FBM(rng, 2, 2)
    ramp = ["#140e0d", "#1c1412", "#251a16", "#30211b", "#3c2a21", "#47322a"]
    for y in range(S):
        for x in range(S):
            w = warp(x, y)
            st = 0.5 + 0.5 * math.sin(2 * math.pi * 7 * (y + 40 * w) / S)
            st2 = 0.5 + 0.5 * math.sin(2 * math.pi * 19 * (y + 30 * w) / S)
            v = 0.45 * tex(x, y) + 0.26 * st + 0.06 * st2 + 0.3 * big(x, y) - 0.1
            far.set(x, y, ramp_pick(ramp, v, x, y))
    # pebbles
    for _ in range(140):
        cx, cy = rng.random() * S, rng.random() * S
        r = rng.uniform(1.0, 2.6)
        for (x, y) in m_ellipse(cx, cy, r * 1.4, r):
            base = far.get(x, y)
            top = (y + 0.5 - cy) < -r * 0.2
            far.set(x, y, mix(base, "#5a4032" if top else "#3a2a22", 0.55))
    # thin roots in the far layer
    for _ in range(16):
        x, y = rng.random() * S, rng.random() * S
        for _i in range(rng.randint(20, 50)):
            far.set(x, y, mix(far.get(int(x), int(y)), "#120c0b", 0.6))
            x += rng.uniform(-0.8, 0.8)
            y += 1

    near = Sprite(S, S)
    near.wrap = True
    shape = FBM(rng, 3, 4, 0.55)
    tex2 = FBM(rng, 16, 2)
    for y in range(S):
        for x in range(S):
            colm = 0.5 + 0.5 * math.cos(2 * math.pi * 3 * (x + 25 * shape(y * 0.5, x)) / S)
            v = shape(x, y) * 0.8 + 0.26 * colm
            if v > 0.62:
                t = tex2(x, y)
                near.set(x, y, with_alpha(ramp_pick(["#36261e", "#3f2d23", "#493428"], t, x, y), 235))
    # hanging roots with alpha
    for _ in range(26):
        x, y = rng.random() * S, rng.random() * S
        thick = rng.choice([1, 1, 2])
        for i in range(rng.randint(30, 90)):
            near.set(x, y, ("#4a3426", 220))
            if thick == 2 and i < 40:
                near.set(x + 1, y, ("#2e2019", 220))
            if rng.random() < 0.04:
                bx, by_ = x, y
                for _j in range(rng.randint(4, 12)):
                    bx += rng.choice([-1, 1]) * 0.7
                    by_ += 1
                    near.set(bx, by_, ("#3e2c21", 200))
            x += rng.uniform(-0.6, 0.6)
            y += 1
    finish_near(near, "#5e4432", "#22170f")
    return far, near


def bg_layer1():
    rng = random.Random(1001)
    far = Sprite(S, S)
    far.wrap = True
    tex = FBM(rng, 6, 4)
    big = FBM(rng, 2, 2)
    wor = Worley(rng, 8)
    wor2 = Worley(rng, 16)
    crk = FBM(rng, 4, 3)
    ramp = ["#160b0b", "#1f0f0d", "#29130f", "#341812", "#3f1d14"]
    for y in range(S):
        for x in range(S):
            v = 0.6 * tex(x, y) + 0.4 * big(x, y)
            c = ramp_pick(ramp, (v - 0.2) * 1.6, x, y)
            d1, d2 = wor(x, y)
            e = d2 - d1
            m = crk(x, y)
            if e < 1.5 and m > 0.5:
                c = mix(c, "#6e2a12", 0.8 if (e < 0.7 and m > 0.56) else 0.4)
            else:
                d1b, d2b = wor2(x, y)
                if d2b - d1b < 0.8 and m > 0.58:
                    c = mix(c, "#4a1c10", 0.4)
            far.set(x, y, c)
    near = Sprite(S, S)
    near.wrap = True
    shape = FBM(rng, 3, 4, 0.55)
    tex2 = FBM(rng, 12, 3)
    for y in range(S):
        for x in range(S):
            # stalactite-ish vertical streaking
            st = 0.5 + 0.5 * math.cos(2 * math.pi * 5 * (x + 14 * shape(x, y)) / S)
            v = 0.75 * shape(x, y) + 0.22 * st * (0.5 + 0.5 * math.sin(2 * math.pi * 2 * y / S))
            if v > 0.6:
                near.set(x, y, with_alpha(ramp_pick(["#34180f", "#3d1d13", "#472216"], tex2(x, y), x, y), 235))
    finish_near(near, "#5e2818", "#1a0b0a")
    # faint embers in cracks of near rock
    w3 = Worley(rng, 10)
    for y in range(S):
        for x in range(S):
            p = near.get(x, y)
            if p[3] and p != with_alpha("#5e2818", 235):
                d1, d2 = w3(x, y)
                if d2 - d1 < 0.8:
                    near.set(x, y, with_alpha("#7a2e12", 235))
    return far, near


def bg_layer2():
    rng = random.Random(1002)
    far = Sprite(S, S)
    far.wrap = True
    tex = FBM(rng, 8, 4)
    hue = FBM(rng, 2, 3)
    purple = ["#110c1a", "#181125", "#211731", "#2a1e3d"]
    teal = ["#0a1517", "#0e1e22", "#13282c", "#183337"]
    for y in range(S):
        for x in range(S):
            v = tex(x, y) * 1.4 - 0.2
            h = hue(x, y)
            r = teal if (h - 0.5) * 4 + 0.5 > bayer(x + 1, y + 2) else purple
            far.set(x, y, ramp_pick(r, v, x, y))
    # faint far crystals
    for _ in range(14):
        cx, cy = rng.random() * S, rng.random() * S
        for _k in range(rng.randint(2, 4)):
            ang = math.radians(rng.uniform(-35, 35) + (180 if rng.random() < 0.3 else 0))
            L, W = rng.uniform(8, 20), rng.uniform(2, 4)
            dx, dy = math.sin(ang), -math.cos(ang)
            px, py = -dy, dx
            poly = [(cx + px * W, cy + py * W), (cx + dx * L + px * W, cy + dy * L + py * W),
                    (cx + dx * (L + W * 1.5), cy + dy * (L + W * 1.5)),
                    (cx + dx * L - px * W, cy + dy * L - py * W), (cx - px * W, cy - py * W)]
            draw_wrapped_poly(far, poly, lambda x, y: mix(far.get(x, y), "#3a2852", 0.5))
    # fungus glow specks
    for _ in range(40):
        cx, cy = rng.random() * S, rng.random() * S
        for _k in range(rng.randint(3, 7)):
            x, y = cx + rng.gauss(0, 3), cy + rng.gauss(0, 2)
            far.set(x, y, mix(far.get(int(x), int(y)), "#3e8948", 0.45))
    near = Sprite(S, S)
    near.wrap = True
    shape = FBM(rng, 3, 4, 0.55)
    for y in range(S):
        for x in range(S):
            v = shape(x, y)
            if v > 0.63:
                near.set(x, y, with_alpha(ramp_pick(["#1a1424", "#211a2e", "#281f38"], (v - 0.63) * 4, x, y), 235))
    # crystal clusters
    for _ in range(18):
        cx, cy = rng.random() * S, rng.random() * S
        down = rng.random() < 0.35
        for _k in range(rng.randint(3, 6)):
            ang = math.radians(rng.uniform(-40, 40) + (180 if down else 0))
            L, W = rng.uniform(10, 30), rng.uniform(2.5, 5)
            dx, dy = math.sin(ang), -math.cos(ang)
            px, py = -dy, dx
            tipx, tipy = cx + dx * (L + W * 1.6), cy + dy * (L + W * 1.6)
            left = [(cx + px * W, cy + py * W), (cx + dx * L + px * W, cy + dy * L + py * W), (tipx, tipy), (cx + dx * L, cy + dy * L), (cx, cy)]
            right = [(cx, cy), (cx + dx * L, cy + dy * L), (tipx, tipy), (cx + dx * L - px * W, cy + dy * L - py * W), (cx - px * W, cy - py * W)]
            lit_left = px < 0 or (px == 0 and py < 0)
            draw_wrapped_poly(near, left, lambda x, y, c=("#45305e" if lit_left else "#2f2043"): with_alpha(c, 225))
            draw_wrapped_poly(near, right, lambda x, y, c=("#2f2043" if lit_left else "#45305e"): with_alpha(c, 225))
    finish_near(near, "#5a3f75", "#140f1d")
    # glowing fungus on near silhouettes
    for _ in range(60):
        x, y = int(rng.random() * S), int(rng.random() * S)
        for _t in range(40):
            if near.opaque(x, y) and not near.opaque(x, y - 1):
                break
            y += 1
        if near.opaque(x, y):
            near.set(x, y - 1, ("#63c74d", 160))
            near.set(x + 1, y - 1, ("#3e8948", 160))
            near.set(x, y - 2, ("#3e8948", 120))
    return far, near


def bg_layer3():
    rng = random.Random(1003)
    far = Sprite(S, S)
    far.wrap = True
    tex = FBM(rng, 6, 4)
    vein = FBM(rng, 4, 4, 0.55)
    streak = FBM(rng, 8, 2)
    ramp = ["#111215", "#17181d", "#1e2026", "#262930", "#2e323a"]
    for y in range(S):
        for x in range(S):
            v = 0.7 * tex(x, y) + 0.3 * streak(x * 0.25, y * 2.0 % S)
            c = ramp_pick(ramp, (v - 0.25) * 1.7, x, y)
            r = 1 - abs(2 * vein(x, y) - 1)
            if r > 0.988:
                c = col("#8a3c16")
            elif r > 0.975:
                c = mix(c, "#5a2810", 0.8)
            elif r > 0.955:
                c = mix(c, "#2e1810", 0.5)
            far.set(x, y, c)
    near = Sprite(S, S)
    near.wrap = True
    wob = FBM(rng, 4, 3)
    cols_ = [(rng.random() * S, rng.uniform(6, 13)) for _ in range(5)]
    for y in range(S):
        for x in range(S):
            for (cx, hw) in cols_:
                w = hw * (0.75 + 0.5 * wob(cx, y))
                off = 10 * (wob(y, cx) - 0.5)
                d = ((x - cx - off + S / 2) % S) - S / 2
                if abs(d) <= w:
                    t = (d / w + 1) / 2
                    c = ramp_pick(["#3a3e48", "#2c2f37", "#22242a", "#191a1f"], t, x, y)
                    if int(y + cx) % 23 == 0 and abs(d) < w - 2:
                        c = col("#4a4e58")
                    near.set(x, y, with_alpha(c, 230))
                    break
    shape = FBM(rng, 4, 4)
    for y in range(S):
        for x in range(S):
            if not near.opaque(x, y) and shape(x, y) > 0.68:
                near.set(x, y, with_alpha(ramp_pick(["#202227", "#2a2c33"], shape(x, y) * 2 - 1.3, x, y), 230))
    finish_near(near, "#4a4f5a", "#121317", alpha=230)
    # orange glow seams on near metal
    for y in range(S):
        for x in range(S):
            if near.opaque(x, y):
                r = 1 - abs(2 * vein(x + 77, y + 31) - 1)
                if r > 0.985:
                    near.set(x, y, with_alpha("#9a4418", 230))
    return far, near


def bg_layer4():
    rng = random.Random(1004)
    far = Sprite(S, S)
    far.wrap = True
    tex = FBM(rng, 6, 4)
    glow = FBM(rng, 2, 3)
    vein = FBM(rng, 4, 4, 0.55)
    ramp = ["#0b0507", "#12070a", "#1a0a0b", "#240e0d", "#30120e"]
    for y in range(S):
        for x in range(S):
            g = glow(x, y)
            v = 0.55 * tex(x, y) + 0.45 * g
            c = ramp_pick(ramp, (v - 0.2) * 1.7, x, y)
            if g > 0.6:
                c = mix(c, "#5a1a0c", min(0.7, (g - 0.6) * 4))
            r = 1 - abs(2 * vein(x, y) - 1)
            if r > 0.99 and g > 0.5:
                c = col("#f77622")
            elif r > 0.98:
                c = col("#be4a2f") if g > 0.45 else mix(c, "#7a2210", 0.8)
            elif r > 0.96:
                c = mix(c, "#5a1a0c", 0.6)
            far.set(x, y, c)
    near = Sprite(S, S)
    near.wrap = True
    shape = FBM(rng, 3, 5, 0.6)
    jag = FBM(rng, 32, 1)
    for y in range(S):
        for x in range(S):
            v = shape(x, y) + 0.12 * (jag(x, y) - 0.5)
            if v > 0.6:
                near.set(x, y, with_alpha(ramp_pick(["#0c0607", "#140809", "#1c0b0b"], (v - 0.6) * 5, x, y), 240))
    finish_near(near, "#7a2810", "#060304", alpha=240)
    # molten drips/underglow on the bottom edges (light from the core below)
    src = near.copy()
    src.wrap = True
    for y in range(S):
        for x in range(S):
            if src.opaque(x, y) and not src.opaque(x, y + 1) and src.opaque(x, y - 1):
                near.set(x, y, with_alpha("#8a2c12", 240))
    return far, near


# --------------------------------------------------------------------------
# Core
# --------------------------------------------------------------------------
def ramp_soft(ramp, v, x, y):
    """Posterised ramp pick, dithering only near band boundaries."""
    v = min(1.0, max(0.0, v)) * (len(ramp) - 1)
    i = int(v)
    if i >= len(ramp) - 1:
        return col(ramp[-1])
    f = v - i
    if f < 0.38:
        return col(ramp[i])
    if f > 0.62:
        return col(ramp[i + 1])
    return col(ramp[i + 1] if (x + y) % 2 else ramp[i])


def gen_core():
    rng = random.Random(77)
    n1 = FBM(rng, 4, 3, 0.55, size=16)
    n2 = TNoise(rng, 16)
    ramp = ["#4a0f1c", "red_dk", "rust", "red", "orange", "yellow_o", "yellow", "white"]
    frames = {}
    for f in range(8):
        s = Sprite(48, 48)
        ph = 2 * math.pi * f / 8
        R = 19.5 + 0.5 * math.sin(ph)
        cx = cy = 24.0
        for y in range(48):
            for x in range(48):
                dx, dy = x + 0.5 - cx, y + 0.5 - cy
                d = math.hypot(dx, dy) / R
                au = (math.atan2(dy, dx) / (2 * math.pi)) % 1.0
                if d <= 1.0:
                    depth = math.asin(min(1.0, d)) / (math.pi / 2)
                    # swirling convection cells; circular path in noise space loops in 8 frames
                    u = au * 16 + depth * 5.0 + 1.2 * math.cos(ph)
                    v = depth * 9.0 + 1.2 * math.sin(ph)
                    tex = n1(u % 16, v % 16)
                    heat = 1.0 - depth ** 1.25
                    amp = 0.75 * depth ** 0.6
                    rid = 1 - abs(2 * n1((u * 1.0 + 7) % 16, (v * 1.0 + 3) % 16) - 1)
                    t = 0.2 + 0.78 * heat + amp * (tex - 0.5) + 0.05 * math.sin(ph)
                    if rid > 0.9 and depth > 0.25:
                        t += 0.22
                    if d > 0.95:
                        t = min(t, 0.3)
                    s.set(x, y, ramp_soft(ramp, t, x, y))
                elif d <= 1.3:
                    fl = n2.at((au * 16 + 1.5 * math.cos(ph)) % 16, 2.0 + 1.5 * math.sin(ph))
                    reach = 1.06 + 0.24 * max(0.0, fl - 0.35) / 0.65
                    if d <= reach:
                        k = 1 - (d - 1.0) / (reach - 1.0 + 1e-6)
                        c = "red" if k > 0.66 else ("rust" if k > 0.33 else "red_dk")
                        s.set(x, y, with_alpha(c, int(70 + 150 * k)))
        frames[(f, 0)] = s
    return sheet(8, 1, 48, 48, frames)


# --------------------------------------------------------------------------
# Creatures
# --------------------------------------------------------------------------
def gen_creatures():
    frames = {}
    # cave crawler: segmented isopod
    body = ["..ggg...", ".gGGGGd.", "gGsGsGdR", "dsdsdsdd"]
    key = {"g": "grey_l", "G": "grey", "s": "slate", "d": "grey_d", "R": "red"}
    for i in range(4):
        s = Sprite(16, 16)
        bob = 1 if i in (1, 3) else 0
        for r, row in enumerate(body):
            for x, ch in enumerate(row):
                if ch != ".":
                    s.set(4 + x, 10 + r - (1 if (bob and r == 0) else 0) + (0 if r == 0 else 0), key[ch])
        if bob:
            s.set(6, 9, "grey_l")
        s.outline()
        legs = [5, 7, 9, 11] if i % 2 == 0 else [4, 6, 8, 10]
        for x in legs:
            s.set(x, 15, "black")
        ant = [(12, 9), (13, 8), (14, 8)] if i % 2 == 0 else [(12, 9), (13, 9), (14, 8)]
        for (x, y) in ant:
            s.set(x, y, "grey_d")
        frames[(i, 0)] = s
    # magma slug
    for i in range(4):
        s = Sprite(16, 16)
        stretch = [0, 1, 2, 1][i]
        x0 = 2 + (1 if i == 2 else 0) - stretch // 2
        length = 10 + stretch
        h = 4 - (1 if stretch == 2 else 0)
        body = set()
        for x in range(x0, x0 + length):
            t = (x - x0) / (length - 1)
            hh = h if t > 0.25 else max(1, int(round(h * (0.4 + 2.4 * t))))
            if t > 0.88:
                hh = h - 1
            for y in range(14 - hh + 1, 15):
                body.add((x, y))
        for (x, y) in body:
            top = (x, y - 1) not in body
            if y == 14:
                s.set(x, y, "yellow_o")
            elif top:
                s.set(x, y, "brown")
            else:
                s.set(x, y, "rust")
        for (x, y) in body:
            if y < 14 and (x + i) % 3 == 0 and (x, y - 1) in body:
                s.set(x, y, "orange")
            if y == 13 and (x * 7 + i) % 5 == 0:
                s.set(x, y, "yellow")
        hx = x0 + length - 1
        s.set(hx - 1, 14 - h + 1, "yellow")
        s.outline()
        s.set(hx, 14 - h - 1, "yellow_o")
        s.set(hx + 1, 14 - h - 2 + (i % 2), "yellow")
        s.set(hx - 2, 14 - h - 1, "orange")
        frames[(i, 1)] = s
    # spore drifter
    for i in range(4):
        s = Sprite(16, 16)
        bob = [0, -1, 0, 1][i]
        cy = 7.5 + bob
        puff = m_ellipse(8, cy, 4.2, 3.9)
        shade_fill(s, puff, "green", "green_l", "green_l", hi="yellow")
        core = m_ellipse(8.5, cy + 0.5, 1.6, 1.6)
        s.fill(core, "yellow")
        s.set(8, int(cy), "white")
        for (x, y) in puff:
            if (x * 3 + y * 5) % 7 == 0 and (x, y) not in core:
                s.set(x, y, "green")
        s.outline()
        for k, x in enumerate((6, 8, 10)):
            y0 = int(cy + 4.5)
            sw = (k + i) % 2
            s.set(x + sw - 0, y0, "green")
            s.set(x, y0 + 1, "green_dk")
        sp_pos = [[(3, 3), (13, 4)], [(2, 4), (13, 2)], [(3, 2), (14, 3)], [(2, 3), (12, 2)]][i]
        for (x, y) in sp_pos:
            s.set(x, y, "green_l")
        frames[(i, 2)] = s
    return sheet(4, 3, 16, 16, frames)


# --------------------------------------------------------------------------
# Pixel font (TrueType)
# --------------------------------------------------------------------------
# Rows from cap top (row 0) down to the baseline row (row 6); rows 7-8 are
# descenders.  Width = row string length (proportional).
GLYPHS = {
    " ": ["..."],
    "!": ["#", "#", "#", "#", "#", ".", "#"],
    '"': ["#.#", "#.#"],
    "#": [".#.#.", ".#.#.", "#####", ".#.#.", "#####", ".#.#.", ".#.#."],
    "$": ["..#..", ".####", "#.#..", ".###.", "..#.#", "####.", "..#.."],
    "%": ["##..#", "##..#", "...#.", "..#..", ".#...", "#..##", "#..##"],
    "&": [".##..", "#..#.", "#.#..", ".#...", "#.#.#", "#..#.", ".##.#"],
    "'": ["#", "#"],
    "(": ["..#", ".#.", "#..", "#..", "#..", ".#.", "..#"],
    ")": ["#..", ".#.", "..#", "..#", "..#", ".#.", "#.."],
    "*": [".....", "..#..", "#.#.#", ".###.", "#.#.#", "..#..", "....."],
    "+": [".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."],
    ",": ["..", "..", "..", "..", "..", ".#", ".#", "#."],
    "-": ["....", "....", "....", "####"],
    ".": [".", ".", ".", ".", ".", ".", "#"],
    "/": ["....#", "....#", "...#.", "..#..", ".#...", "#....", "#...."],
    "0": [".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."],
    "1": [".#.", "##.", ".#.", ".#.", ".#.", ".#.", "###"],
    "2": [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
    "3": ["#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."],
    "4": ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
    "5": ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
    "6": ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."],
    "7": ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
    "8": [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
    "9": [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."],
    ":": [".", ".", "#", ".", ".", ".", "#"],
    ";": ["..", "..", ".#", "..", "..", "..", ".#", "#."],
    "<": ["...#", "..#.", ".#..", "#...", ".#..", "..#.", "...#"],
    "=": ["....", "....", "####", "....", "####"],
    ">": ["#...", ".#..", "..#.", "...#", "..#.", ".#..", "#..."],
    "?": [".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."],
    "@": [".###.", "#...#", "#.###", "#.#.#", "#.##.", "#....", ".####"],
    "A": [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    "B": ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
    "C": [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
    "D": ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
    "E": ["####", "#...", "#...", "###.", "#...", "#...", "####"],
    "F": ["####", "#...", "#...", "###.", "#...", "#...", "#..."],
    "G": [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"],
    "H": ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    "I": ["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"],
    "J": ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
    "K": ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
    "L": ["#...", "#...", "#...", "#...", "#...", "#...", "####"],
    "M": ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
    "N": ["#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"],
    "O": [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    "P": ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    "Q": [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"],
    "R": ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
    "S": [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
    "T": ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
    "U": ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    "V": ["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."],
    "W": ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#."],
    "X": ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
    "Y": ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
    "Z": ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
    "[": ["###", "#..", "#..", "#..", "#..", "#..", "###"],
    "\\": ["#....", "#....", ".#...", "..#..", "...#.", "....#", "....#"],
    "]": ["###", "..#", "..#", "..#", "..#", "..#", "###"],
    "^": ["..#..", ".#.#.", "#...#"],
    "_": [".....", ".....", ".....", ".....", ".....", ".....", ".....", "#####"],
    "`": ["#.", ".#"],
    "a": [".....", ".....", ".###.", "....#", ".####", "#...#", ".####"],
    "b": ["#....", "#....", "#.##.", "##..#", "#...#", "#...#", "####."],
    "c": ["....", "....", ".###", "#...", "#...", "#...", ".###"],
    "d": ["....#", "....#", ".##.#", "#..##", "#...#", "#...#", ".####"],
    "e": [".....", ".....", ".###.", "#...#", "#####", "#....", ".###."],
    "f": ["..##", ".#..", ".#..", "###.", ".#..", ".#..", ".#.."],
    "g": [".....", ".....", ".####", "#...#", "#...#", "#...#", ".####", "....#", "####."],
    "h": ["#....", "#....", "#.##.", "##..#", "#...#", "#...#", "#...#"],
    "i": ["#", ".", "#", "#", "#", "#", "#"],
    "j": ["..#", "...", "..#", "..#", "..#", "..#", "..#", "#.#", ".#."],
    "k": ["#...", "#...", "#..#", "#.#.", "##..", "#.#.", "#..#"],
    "l": ["#.", "#.", "#.", "#.", "#.", "#.", ".#"],
    "m": [".....", ".....", "##.#.", "#.#.#", "#.#.#", "#.#.#", "#.#.#"],
    "n": [".....", ".....", "#.##.", "##..#", "#...#", "#...#", "#...#"],
    "o": [".....", ".....", ".###.", "#...#", "#...#", "#...#", ".###."],
    "p": [".....", ".....", "####.", "#...#", "#...#", "#...#", "####.", "#....", "#...."],
    "q": [".....", ".....", ".####", "#...#", "#...#", "#...#", ".####", "....#", "....#"],
    "r": ["....", "....", "#.##", "##..", "#...", "#...", "#..."],
    "s": [".....", ".....", ".####", "#....", ".###.", "....#", "####."],
    "t": ["....", ".#..", "###.", ".#..", ".#..", ".#..", "..##"],
    "u": [".....", ".....", "#...#", "#...#", "#...#", "#..##", ".##.#"],
    "v": [".....", ".....", "#...#", "#...#", "#...#", ".#.#.", "..#.."],
    "w": [".....", ".....", "#...#", "#...#", "#.#.#", "#.#.#", ".#.#."],
    "x": [".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#"],
    "y": [".....", ".....", "#...#", "#...#", "#...#", "#...#", ".####", "....#", ".###."],
    "z": [".....", ".....", "#####", "...#.", "..#..", ".#...", "#####"],
    "{": ["..##", ".#..", ".#..", "#...", ".#..", ".#..", "..##"],
    "|": ["#", "#", "#", "#", "#", "#", "#", "#"],
    "}": ["##..", "..#.", "..#.", "...#", "..#.", "..#.", "##.."],
    "~": [".....", ".....", ".#...", "#.#.#", "...#."],
    "·": ["..", "..", "..", "##", "##"],
    "•": ["...", "...", ".#.", "###", ".#."],
    "…": [".....", ".....", ".....", ".....", ".....", ".....", "#.#.#"],
    "×": [".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#"],
    "←": [".......", "..#....", ".#.....", "#######", ".#.....", "..#...."],
    "→": [".......", "....#..", ".....#.", "#######", ".....#.", "....#.."],
    "↑": ["..#..", ".###.", "#.#.#", "..#..", "..#..", "..#..", "..#.."],
    "↓": ["..#..", "..#..", "..#..", "..#..", "#.#.#", ".###.", "..#.."],
    "°": [".#.", "#.#", ".#."],
}
NOTDEF = ["#####", "#...#", "#...#", "#...#", "#...#", "#...#", "#####"]


def glyph_rects(rows):
    """Merge lit pixels into rectangles (x0, x1, r0, r1) half-open."""
    runs_by_row = []
    for r, line in enumerate(rows):
        runs = []
        x = 0
        while x < len(line):
            if line[x] == "#":
                s = x
                while x < len(line) and line[x] == "#":
                    x += 1
                runs.append((s, x))
            else:
                x += 1
        runs_by_row.append(runs)
    rects = []
    open_ = {}
    for r, runs in enumerate(runs_by_row):
        new_open = {}
        for run in runs:
            if run in open_:
                new_open[run] = open_.pop(run)
            else:
                new_open[run] = r
        for run, r0 in open_.items():
            rects.append((run[0], run[1], r0, r))
        open_ = new_open
    for run, r0 in open_.items():
        rects.append((run[0], run[1], r0, len(runs_by_row)))
    return sorted(rects, key=lambda t: (t[2], t[0]))


def build_ttf():
    UPM, PX = 1024, 128
    ASC, DESC = 8 * PX, -2 * PX
    order = [(".notdef", None, NOTDEF)]
    chars = sorted(GLYPHS.keys(), key=ord)
    for ch in chars:
        order.append(("uni%04X" % ord(ch), ord(ch), GLYPHS[ch]))
    glyf = bytearray()
    loca = []
    hmtx = []
    maxp_pts = maxp_cont = 0
    gx0 = gy0 = 10 ** 6
    gx1 = gy1 = -10 ** 6
    adv_max = 0
    min_lsb = min_rsb = 10 ** 6
    xmax_ext = 0
    widths = []
    for name, cp, rows in order:
        width = max(len(r) for r in rows)
        adv = (width + 1) * PX
        adv_max = max(adv_max, adv)
        widths.append(adv)
        rects = glyph_rects(rows)
        loca.append(len(glyf))
        if not rects:
            hmtx.append((adv, 0))
            continue
        pts = []
        ends = []
        for (x0, x1, r0, r1) in rects:
            yb = (7 - r1) * PX
            yt = (7 - r0) * PX
            X0, X1 = x0 * PX, x1 * PX
            pts += [(X0, yb), (X0, yt), (X1, yt), (X1, yb)]  # clockwise (y up)
            ends.append(len(pts) - 1)
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        xmin, xmax, ymin, ymax = min(xs), max(xs), min(ys), max(ys)
        gx0, gy0, gx1, gy1 = min(gx0, xmin), min(gy0, ymin), max(gx1, xmax), max(gy1, ymax)
        min_lsb = min(min_lsb, xmin)
        min_rsb = min(min_rsb, adv - xmax)
        xmax_ext = max(xmax_ext, xmax)
        maxp_pts = max(maxp_pts, len(pts))
        maxp_cont = max(maxp_cont, len(ends))
        g = struct.pack(">hhhhh", len(ends), xmin, ymin, xmax, ymax)
        g += b"".join(struct.pack(">H", e) for e in ends)
        g += struct.pack(">H", 0)
        g += bytes([0x01]) * len(pts)
        px_, py_ = 0, 0
        xb, yb_ = b"", b""
        for (x, y) in pts:
            xb += struct.pack(">h", x - px_)
            yb_ += struct.pack(">h", y - py_)
            px_, py_ = x, y
        g += xb + yb_
        while len(g) % 4:
            g += b"\0"
        glyf += g
        hmtx.append((adv, xmin))
    loca.append(len(glyf))
    n = len(order)
    if min_lsb == 10 ** 6:
        min_lsb = min_rsb = 0

    head = struct.pack(">IIIIHHqqhhhhHHhhh", 0x00010000, 0x00010000, 0, 0x5F0F3CF5, 0x000B, UPM,
                       3786825600, 3786825600, gx0, gy0, gx1, gy1, 0, 8, 2, 1, 0)
    hhea = struct.pack(">IhhhH" + "h" * 11 + "H", 0x00010000, ASC, DESC, 0, adv_max, min_lsb, min_rsb,
                       xmax_ext, 1, 0, 0, 0, 0, 0, 0, 0, n)
    maxp = struct.pack(">I" + "H" * 14, 0x00010000, n, maxp_pts, maxp_cont, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0)
    avg = int(sum(widths) / len(widths))
    os2 = struct.pack(">HhHHH" + "h" * 11, 4, avg, 400, 5, 0, 512, 512, 0, 128, 512, 512, 0, 384, PX, 3 * PX, 0)
    os2 += bytes([2, 11, 6, 9, 0, 0, 0, 0, 0, 0])  # panose: monospace-ish pixel
    os2 += struct.pack(">IIII", (1 << 0) | (1 << 1) | (1 << 31), (1 << 5), 0, 0)
    os2 += b"DTTC"
    cps = [o[1] for o in order if o[1] is not None]
    os2 += struct.pack(">HHHhhhHHIIhhHHH", 0x40 | 0x80, min(cps), min(max(cps), 0xFFFF), ASC, DESC, 0,
                       ASC, -DESC, 1, 0, 5 * PX, 7 * PX, 0, 32, 1)
    hmtx_b = b"".join(struct.pack(">Hh", a, l) for a, l in hmtx)
    loca_b = b"".join(struct.pack(">I", o) for o in loca)
    # cmap format 4
    pairs = [(o[1], gid) for gid, o in enumerate(order) if o[1] is not None]
    pairs.sort()
    segs = []
    for cp, gid in pairs:
        if segs and cp == segs[-1][1] + 1 and gid == segs[-1][2] + (cp - segs[-1][0]):
            segs[-1][1] = cp
        else:
            segs.append([cp, cp, gid])
    segs.append([0xFFFF, 0xFFFF, 0])
    sc = len(segs)
    es = int(math.floor(math.log2(sc)))
    sr = 2 * (2 ** es)
    sub = struct.pack(">HHHHHHH", 4, 16 + 8 * sc, 0, sc * 2, sr, es, sc * 2 - sr)
    sub += b"".join(struct.pack(">H", s[1]) for s in segs)
    sub += struct.pack(">H", 0)
    sub += b"".join(struct.pack(">H", s[0]) for s in segs)
    sub += b"".join(struct.pack(">H", ((s[2] - s[0]) if s[0] != 0xFFFF else 1) & 0xFFFF) for s in segs)
    sub += b"".join(struct.pack(">H", 0) for s in segs)
    cmap = struct.pack(">HH", 0, 2) + struct.pack(">HHI", 0, 3, 20) + struct.pack(">HHI", 3, 1, 20) + sub
    # name
    names = {1: "Descent Pixel", 2: "Regular", 3: "DescentPixel-Regular-1.000",
             4: "Descent Pixel Regular", 5: "Version 1.000", 6: "DescentPixel-Regular"}
    recs, strings = [], b""
    for nid, s in names.items():
        b = s.encode("ascii")
        recs.append((1, 0, 0, nid, len(b), len(strings)))
        strings += b
    for nid, s in names.items():
        b = s.encode("utf-16-be")
        recs.append((3, 1, 0x409, nid, len(b), len(strings)))
        strings += b
    recs.sort()
    name = struct.pack(">HHH", 0, len(recs), 6 + 12 * len(recs))
    name += b"".join(struct.pack(">HHHHHH", *r) for r in recs) + strings
    post = struct.pack(">IihhIIIII", 0x00030000, 0, -PX, PX, 0, 0, 0, 0, 0)

    tables = {b"head": head, b"hhea": hhea, b"maxp": maxp, b"OS/2": os2, b"hmtx": hmtx_b,
              b"cmap": cmap, b"loca": loca_b, b"glyf": bytes(glyf), b"name": name, b"post": post}

    def csum(b):
        b = b + b"\0" * ((4 - len(b) % 4) % 4)
        return sum(struct.unpack(">%dI" % (len(b) // 4), b)) & 0xFFFFFFFF

    tags = sorted(tables.keys())
    nt = len(tags)
    es = int(math.floor(math.log2(nt)))
    sr = (2 ** es) * 16
    out = struct.pack(">IHHHH", 0x00010000, nt, sr, es, nt * 16 - sr)
    offset = 12 + 16 * nt
    dir_, body = b"", b""
    head_off = 0
    for t in tags:
        d = tables[t]
        if t == b"head":
            head_off = offset
        dir_ += struct.pack(">4sIII", t, csum(d), offset, len(d))
        pad = d + b"\0" * ((4 - len(d) % 4) % 4)
        body += pad
        offset += len(pad)
    font = bytearray(out + dir_ + body)
    adj = (0xB1B0AFBA - csum(bytes(font))) & 0xFFFFFFFF
    font[head_off + 8:head_off + 12] = struct.pack(">I", adj)
    return bytes(font)


def check_ttf(data):
    """Parse the font back and verify cmap lookups hit non-empty glyphs."""
    nt = struct.unpack(">H", data[4:6])[0]
    tabs = {}
    tags = []
    for i in range(nt):
        tag, cs, off, ln = struct.unpack(">4sIII", data[12 + 16 * i:28 + 16 * i])
        tabs[tag] = (off, ln)
        tags.append(tag)
        d = data[off:off + ln]
        if tag != b"head":
            d2 = d + b"\0" * ((4 - len(d) % 4) % 4)
            assert sum(struct.unpack(">%dI" % (len(d2) // 4), d2)) & 0xFFFFFFFF == cs, tag
    assert tags == sorted(tags), "table records not sorted"
    total = data + b"\0" * ((4 - len(data) % 4) % 4)
    assert sum(struct.unpack(">%dI" % (len(total) // 4), total)) & 0xFFFFFFFF == 0xB1B0AFBA, "checkSumAdjustment"
    co, _ = tabs[b"cmap"]
    nsub = struct.unpack(">H", data[co + 2:co + 4])[0]
    sub = None
    for i in range(nsub):
        p, e, o = struct.unpack(">HHI", data[co + 4 + 8 * i:co + 12 + 8 * i])
        if (p, e) == (3, 1):
            sub = co + o
    assert sub is not None
    segx2 = struct.unpack(">H", data[sub + 6:sub + 8])[0]
    sc = segx2 // 2
    ends = struct.unpack(">%dH" % sc, data[sub + 14:sub + 14 + segx2])
    starts = struct.unpack(">%dH" % sc, data[sub + 16 + segx2:sub + 16 + 2 * segx2])
    deltas = struct.unpack(">%dH" % sc, data[sub + 16 + 2 * segx2:sub + 16 + 3 * segx2])

    def lookup(cp):
        for s, e, d in zip(starts, ends, deltas):
            if s <= cp <= e:
                return (cp + d) & 0xFFFF
        return 0

    lo, _ = tabs[b"loca"]
    go, _ = tabs[b"glyf"]
    results = {}
    for ch in "Ag0→°":
        gid = lookup(ord(ch))
        a, b = struct.unpack(">II", data[lo + 4 * gid:lo + 4 * gid + 8])
        ncont = struct.unpack(">h", data[go + a:go + a + 2])[0] if b > a else 0
        assert gid != 0 and b > a and ncont > 0, ch
        results[ch] = (gid, b - a, ncont)
    return results


# --------------------------------------------------------------------------
# Preview
# --------------------------------------------------------------------------
def upscale(sp, k, bg=True):
    out = Sprite(sp.w * k, sp.h * k)
    for y in range(out.h):
        for x in range(out.w):
            p = sp.px[(y // k) * sp.w + (x // k)]
            if bg:
                b = (58, 58, 70, 255) if ((x // 8 + y // 8) % 2) else (46, 46, 56, 255)
                if p[3] == 255:
                    out.px[y * out.w + x] = p
                else:
                    a = p[3] / 255.0
                    out.px[y * out.w + x] = tuple(int(p[i] * a + b[i] * (1 - a)) for i in range(3)) + (255,)
            else:
                out.px[y * out.w + x] = p
    return out


def grid_lines(sp, fw, fh, k):
    for y in range(sp.h):
        for x in range(sp.w):
            if (x % (fw * k) == 0) or (y % (fh * k) == 0):
                sp.px[y * sp.w + x] = (90, 30, 90, 255)


def font_preview(path):
    text = ["The quick brown fox jumps", "over the lazy dog! 0123456789",
            "DESCENT TO THE CORE  ore×3 ←→↑↓ 40° … · •",
            "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~", "gjpqy  Depth: 1,250m  HP 3/5"]
    W, H = 220, 12 * len(text) + 4
    sp = Sprite(W, H, (24, 20, 37, 255))
    for li, line in enumerate(text):
        x = 2
        for ch in line:
            rows = GLYPHS.get(ch, NOTDEF)
            for r, row in enumerate(rows):
                for cx, c in enumerate(row):
                    if c == "#":
                        sp.set(x + cx, 2 + li * 12 + r, (255, 255, 255, 255))
            x += max(len(r) for r in rows) + 1
    write_png(path, upscale(sp, 4, bg=False))


def write_previews(d, outs):
    os.makedirs(d, exist_ok=True)
    k = 4
    p = upscale(outs["player"], k)
    grid_lines(p, 16, 16, k)
    write_png(os.path.join(d, "player.png"), p)
    p = upscale(outs["items"], k)
    grid_lines(p, 16, 16, k)
    write_png(os.path.join(d, "items.png"), p)
    p = upscale(outs["props"], 5)
    grid_lines(p, 16, 24, 5)
    write_png(os.path.join(d, "props.png"), p)
    p = upscale(outs["core"], 3)
    write_png(os.path.join(d, "core.png"), p)
    p = upscale(outs["creatures"], 6)
    grid_lines(p, 16, 16, 6)
    write_png(os.path.join(d, "creatures.png"), p)
    # 1x-size player sheet over a mock cave background to judge readability
    for i in range(5):
        far, near = outs["bg%d" % i]
        t = Sprite(512, 512)
        for y in range(512):
            for x in range(512):
                t.px[y * 512 + x] = far.px[(y % 256) * 256 + (x % 256)]
        nr = Sprite(512, 512)
        for y in range(512):
            for x in range(512):
                nr.px[y * 512 + x] = near.px[(y % 256) * 256 + (x % 256)]
        t.blit(nr, 0, 0)
        # drop a few sprites on top at 1x to judge contrast
        t.blit(outs["player"], 20, 20)
        write_png(os.path.join(d, "bg%d.png" % i), t)
    font_preview(os.path.join(d, "font.png"))


# --------------------------------------------------------------------------
def main():
    preview = None
    if "--preview" in sys.argv:
        preview = sys.argv[sys.argv.index("--preview") + 1]
    outs = {}
    outs["player"] = gen_player()
    write_png(os.path.join(ASSETS, "sprites", "player.png"), outs["player"])
    outs["items"] = gen_items()
    write_png(os.path.join(ASSETS, "sprites", "items.png"), outs["items"])
    outs["props"] = gen_props()
    write_png(os.path.join(ASSETS, "sprites", "props.png"), outs["props"])
    outs["core"] = gen_core()
    write_png(os.path.join(ASSETS, "sprites", "core.png"), outs["core"])
    outs["creatures"] = gen_creatures()
    write_png(os.path.join(ASSETS, "sprites", "creatures.png"), outs["creatures"])
    for i, fn in enumerate([bg_layer0, bg_layer1, bg_layer2, bg_layer3, bg_layer4]):
        far, near = fn()
        far.wrap = near.wrap = False
        for p in far.px:
            assert p[3] == 255
        outs["bg%d" % i] = (far, near)
        write_png(os.path.join(ASSETS, "backgrounds", "layer%d_far.png" % i), far)
        write_png(os.path.join(ASSETS, "backgrounds", "layer%d_near.png" % i), near)
        lf = sum(lum(p) for p in far.px) / len(far.px)
        vals = [max(p[:3]) / 255.0 for p in far.px]
        print("layer%d far: mean luminance %.3f, mean HSV value %.3f" % (i, lf, sum(vals) / len(vals)))
    ttf = build_ttf()
    os.makedirs(os.path.join(ASSETS, "fonts"), exist_ok=True)
    with open(os.path.join(ASSETS, "fonts", "pixel.ttf"), "wb") as f:
        f.write(ttf)
    res = check_ttf(ttf)
    print("font self-check OK (%d bytes):" % len(ttf), ", ".join("%r->gid %d (%d B, %d contours)" % (k, *v) for k, v in res.items()))
    if preview:
        write_previews(preview, outs)
        print("previews written to", preview)


if __name__ == "__main__":
    main()
