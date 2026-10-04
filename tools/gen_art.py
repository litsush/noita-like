#!/usr/bin/env python3
"""Procedural pixel-art generator for "The Last Apprentice".

Pure Python 3 standard library (zlib + struct PNG/TTF writers), fully
deterministic (seeded random.Random).  Run from the repo root:

    python3 tools/gen_art.py                    # regenerate every asset
    python3 tools/gen_art.py --preview DIR      # also write upscaled previews
    python3 tools/gen_art.py --only wizard,icons,props,spells,creatures,
                                    stalker,core,ui,bg,fonts

All PNGs are RGBA8 with a transparent background unless stated.  Coordinates:
x right, y down; frames are packed left-to-right from x=0, rows top-to-bottom;
unused slots are fully transparent.  Scale: 1 game cell = 1 art pixel.  Light
comes from the top-left; characters/objects carry a 1px dark (#100b17)
outline.  Palette: Endesga-32 pushed darker (deep violets, cold teals, sickly
greens, bone, dim gold) with glowing accents for magic.

School colour ramps (dark, mid, light, glow) are in SCHOOL_RAMP, order:
0 Pyromancy, 1 Hydromancy, 2 Terramancy, 3 Cryomancy, 4 Aeromancy,
5 Fulmancy, 6 Alchemy, 7 Mycomancy.

assets/sprites/wizard_robe.png, wizard_trim.png, wizard_base.png
    Each 128x176, 16x16 frames, 8 cols x 11 rows, IDENTICAL layout and pixel
    alignment.  Composite in this order: robe (multiply-tinted by the first
    school's colour), trim (tinted by the second school's colour), base on
    top.  robe = hood + robe/cloak body in neutral greys 142-240; trim =
    sash, hood lining, cuffs and the staff gem in neutral greys 150-255;
    base = everything else: the 1px outline around the whole figure, face in
    shadow with glowing eyes, hands, boots, wooden staff with a crystal tip,
    and the cast glows/flares (semi-transparent, no outline).  Hurt/death
    flash frames live entirely in base.  The apprentice faces RIGHT, feet on
    y=15, body centred at x~8 (~8 px wide, 14 px tall without the staff;
    collision box 5x12), pointed hood, staff taller than the head.
    Rows (frame counts):
      0 idle (6; robe sway, hood flutter, staff glow pulse)
      1 run (6)            2 jump (4: crouch, launch, rise, apex)
      3 fall (4; robe billows)
      4 climb (6; clinging to a wall on the right, staff on the back)
      5 cast_side (6; staff thrust forward, tip flaring)
      6 cast_down (6; staff pointed down)   7 cast_up (6; staff pointed up)
      8 cast_spell (6; staff raised, burst of light at the tip)
      9 hurt (4; frame 0 white flash, frame 2 red flash)
     10 death (8; collapses, staff falls, glow dies out; 4-7 lying)

assets/sprites/icons.png  256x128, 16x16 icons, 16 cols x 8 rows,
    index = row*16 + col:
      0-7    school sigils (round rune badges): 0 Pyromancy 1 Hydromancy
             2 Terramancy 3 Cryomancy 4 Aeromancy 5 Fulmancy 6 Alchemy
             7 Mycomancy
      8-44   scrolls (vellum scroll, school-coloured caps/tint + emblem):
             8 Fire Bolt 9 Magma Bore 10 Salamander Ward 11 Ember Heart
             12 Pocket Sun 13 Tidecall 14 Flood Orb 15 Gills of the Deep
             16 Undertow 17 Crumbling Touch 18 Stone Shape 19 Earthen Grip
             20 Seismic Stomp 21 Frost Seed 22 Ice Lance 23 Frozen Path
             24 Rime Shell 25 Gust 26 Updraft 27 Levitate 28 Air Bubble
             29 Spark Bolt 30 Chain Lightning 31 Arc Drill 32 Grounded
             33 Acid Flask 34 Alchemist's Charge 35 Transmutation
             36 Volatile Core 37 Spore Sowing 38 Rot Touch 39 Mycelial Bridge
             40 Symbiosis 41 Beacon 42 Glass Focus 43 Iron Soles 44 Blink
             (41-44 neutral: bone/silver)
      45-72  fusions (gold-rimmed medallion split between the two schools'
             colours + emblem), pair order (a,b) for a<b:
             45 Steam Burst 46 Magma Surge 47 Thermal Shock 48 Firestorm
             49 Plasma Lance 50 Napalm 51 Spore Bomb 52 Mudslide 53 Glacier
             54 Typhoon 55 Storm Flood 56 Acid Rain 57 Swamp Bloom
             58 Frozen Earth 59 Sandstorm 60 Railshot 61 Petrify
             62 Root Lattice 63 Blizzard 64 Cryo Arc 65 Shatter 66 Rimebloom
             67 Thunderstorm 68 Miasma 69 Spore Gale 70 Electrolysis
             71 Storm Spores 72 Decay Bloom
      73 aether shard  74 light orb   75 chest closed  76 chest open
      77 altar         78 shrine      79 fallen apprentice remains
      80 journal page  81 heart full  82 heart half    83 heart empty
      84 lock          85 flame       86 bubble        87 thermometer
      88 skull         89 rite seal   90 staff (dig)   91 question mark
      92 compass/seed  93 hourglass   94 mana star     95 closed eye
      96 smart-dig reticle  97 cursor-dig beam  98 mimic  99 stalker eye
      100 rune circle  101 Initiate   102 Prodigy      103 Scavenger
      104 Archmage's Heir  105 Twin-Souled  106-127 transparent

assets/sprites/props.png  128x48, 16x24 frames, 8 cols x 2 rows, world
    scale, bottom-aligned on y=23 unless noted:
      row 0: 0-3 light orb loop (7x7 ball + halo, CENTRED on pixel (8,12),
             flickering)  4 chest closed  5 chest open  6 altar (scrolls
             float above)  7 ruined shrine (~14x22, glowing sigil)
      row 1: 8 fallen apprentice remains (~14x10)  9 remains searched
             10 journal page on the ground (~7x4)  11 mimic closed (teeth
             under the lid, red eye in the keyhole)  12 mimic open (maw,
             tongue)  13 mimic biting  14 aether shard pickup (5x7, CENTRED
             on pixel (8,12))  15 rune stone
    (frame index = row*8 + col)

assets/sprites/spells.png  64x80, 8x8 frames, 8 cols x 10 rows:
      rows 0-7, one per school (Pyro, Hydro, Terra, Cryo, Aero, Fulm, Alch,
      Myco): cols 0-3 projectile animation facing RIGHT (core ~ (5,3)),
      cols 4-7 impact burst animation (centred).
      row 8: cols 0-3 light mote, cols 4-7 dig-beam segment (tiles
      horizontally, core on rows 3-4, pale violet-white).
      row 9: cols 0-3 shard sparkle, cols 4-7 rune glyph flicker.

assets/sprites/creatures.png  128x160, 16x16 frames, 8 cols x 10 rows,
    facing RIGHT, feet on y=15 unless they float (centred):
      row 0 gnawling walk (4)
      row 1 hollowed apprentice walk (4) + cast (4)
      row 2 blind wyrm head (4, maw opening/closing) + body segment (2)
            + tail (2); segments ~10 px round, side-on
      row 3 spore puppet walk (4) + burst (4)
      row 4 spore drifter float (4)     row 5 cinder wraith float (4)
      row 6 lightseeker fly (4)         row 7 mimic hop (4) + bite (4)
      rows 8-9 unused (transparent)

assets/sprites/stalker.png  128x32, 16x32 frames: 0-5 walk, 6-7 idle.
    The Hollow Stalker, side view facing right, feet on y=31.

assets/sprites/core.png   384x48, 8 frames of 48x48 (seamless loop): the
    Trial, a beating heart of molten gold-white light (r~11-13) centred at
    (24,24) inside a rotating rune ring (r~18-22), transparent around.

assets/sprites/ui.png  64x32 grimoire UI ornaments:
      (0,0)  16x16 ornate panel corner, top-left orientation (mirror for the
             others): outer gold line on row/col 1, dark gap on 2, inner gold
             line on row/col 3, jewelled boss at (4..7, 4..7)
      (16,0) 16x16 round sigil badge frame (gold ring, transparent centre
             r~5)
      (32,0) 32x8 divider flourish        (32,8) 32x8 smaller divider
      (0,16) 16x16 tileable dark vellum texture (opaque)
      rest transparent

assets/backgrounds/layer{0..4}_{far,near}.png  256x256, seamlessly tileable
    in x and y.  "far" is opaque and dark (mean HSV value ~0.11-0.13),
    "near" has alpha (silhouettes over transparency).
      0 The Whispering Crust (roots, buried bones, faint carved faces)
      1 The Drowned Halls (flooded arcades, columns, drips, cold teal)
      2 The Fungal Abyss (towering mushrooms, spore haze, magenta/teal)
      3 The Molten Sanctum (ruined temple pillars, ember-lit cracks)
      4 The Heart of the World (vast rune circles, gold-white glow)

assets/fonts/pixel.ttf  TrueType pixel font, unitsPerEm 1024, 1 px = 128
    units, cap height 7 px, descender 2 px, proportional advances.
    ASCII 32-126 plus · • … × ← → ↑ ↓ ° and a .notdef box.  Crisp at
    em sizes that are multiples of 8 px (egui/skrifa font size 8, 16, 24);
    ab_glyph PxScale is ascent-descent (10 font px), so use multiples of 10.

assets/fonts/title.ttf  Ornate grimoire display pixel font ("Last Apprentice
    Title"): same TrueType rules (1024 upem, 1 px = 128 units, crisp at
    multiples of 8 px), cap height 9 px, x-height 6 px, ascent 10 px,
    descent 3 px (ab_glyph PxScale = 13 font px), 2 px stems with serifs,
    full ASCII 32-126 plus .notdef.  Both fonts are parsed back and
    self-checked after writing."""
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
    # darker, moodier extension for "The Last Apprentice"
    "ink": "100b17", "void": "1a1226", "vio_dk": "2b1a3d", "vio": "4a2d66",
    "vio_l": "7b52a3", "lilac": "b394e0", "magic": "e2d6ff",
    "teal_d": "0f2629", "teal": "1e5559", "teal_l": "3f9a96", "aether": "8ff5ec",
    "sick_d": "28331a", "sick": "56702a", "sick_l": "9db446",
    "bone_d": "7d725f", "bone": "bfb399", "bone_l": "e6dcc2",
    "gold_d": "6e4f1f", "gold": "b08636", "gold_l": "e3c069",
    "blood": "6a1424", "wood_d": "3d2719", "wood": "6b4428", "wood_l": "9a6a3e",
    "flesh_d": "3a2733", "flesh": "6a4a52", "pale": "c9b2a6", "pale_d": "8c7272",
    "stone_d": "232031", "stone": "3b3850", "stone_l": "5d5a78", "stone_h": "8481a0",
    "mag": "d04fa0", "mag_l": "ff9fe0",
}


def hexc(h, a=255):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


C = {k: hexc(v) for k, v in PAL_HEX.items()}
CLEAR = (0, 0, 0, 0)
OUTLINE = "ink"


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
# Schools (shared colour ramps: dark, mid, light, glow)
# --------------------------------------------------------------------------
SCHOOLS = ["Pyromancy", "Hydromancy", "Terramancy", "Cryomancy", "Aeromancy", "Fulmancy", "Alchemy", "Mycomancy"]
SCHOOL_RAMP = [
    ("#4a1010", "#a52a1c", "#ea6a26", "#ffc35a"),   # 0 Pyro: orange-red
    ("#0d1f4a", "#1c56a0", "#3a98dc", "#a6e2ff"),   # 1 Hydro: blue
    ("#33200f", "#6e4824", "#ad7e3c", "#e6c07a"),   # 2 Terra: brown-ochre
    ("#173e52", "#3f8fb0", "#93dcec", "#effeff"),   # 3 Cryo: pale cyan
    ("#203d30", "#5e8f6e", "#acd8b2", "#f0fff0"),   # 4 Aero: pale green-white
    ("#2e1856", "#6a40b0", "#e6c838", "#fff6a8"),   # 5 Fulm: violet + yellow
    ("#1a330c", "#3f7d18", "#8ccf2a", "#e2ff86"),   # 6 Alch: acid green
    ("#0f3436", "#1b7a78", "#c8489a", "#ff9ede"),   # 7 Myco: teal + magenta
]
NEUTRAL_RAMP = ("#3c3648", "#7e778a", "#c4bba6", "#f2ecdc")
SCHOOL_TINT = ["#e0582a", "#3a8ee0", "#b0844a", "#9ee0f0", "#b4dcb4", "#9a6ae0", "#8ccf2a", "#c8489a"]


# --------------------------------------------------------------------------
# Wizard (three aligned tint layers: robe, trim, base)
# --------------------------------------------------------------------------
ROBE, TRIM, BASE = 0, 1, 2
# neutral light greys so multiply tinting by a school colour works
RG = [(142, 142, 142, 255), (174, 174, 174, 255), (206, 206, 206, 255), (240, 240, 240, 255)]
TG = [(150, 150, 150, 255), (190, 190, 190, 255), (226, 226, 226, 255), (255, 255, 255, 255)]
W_FACE = col("#22172a")
W_FACE_L = col("#3d2a3c")
W_EYE = col("#c6fbff")
W_EYE_D = col("#5fb3c4")
W_SKIN = col("#c7a493")
W_SKIN_D = col("#8b6a68")
W_BOOT = col("#4a3236")
W_BOOT_L = col("#6e4e46")
W_WOOD = [col("wood"), col("#80522e"), col("wood_l")]
W_CRYS = col("#efe8ff")
W_CRYS_D = col("#a993e6")


class LSprite:
    """Layered sprite: each pixel is None or (layer, rgba)."""

    def __init__(self, w=16, h=16):
        self.w, self.h = w, h
        self.px = [None] * (w * h)

    def set(self, x, y, layer, c):
        x, y = int(math.floor(x)), int(math.floor(y))
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y * self.w + x] = (layer, col(c))

    def get(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[y * self.w + x]
        return None

    def clear(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y * self.w + x] = None

    def flash(self, kind):
        """Move everything into the base layer as a flat white/red flash."""
        for i, p in enumerate(self.px):
            if p is None:
                continue
            c = p[1]
            l_ = lum(c) if p[0] == BASE else lum(c) * 0.85
            if kind == "white":
                nc = (255, 255, 255, 255) if l_ > 0.12 else (230, 226, 240, 255)
            else:
                nc = col("salmon") if l_ > 0.6 else (col("red") if l_ > 0.25 else col("red_dk"))
            self.px[i] = (BASE, nc)

    def rotated(self, deg, cx, cy):
        t = math.radians(deg)
        ct, st = math.cos(t), math.sin(t)
        out = LSprite(self.w, self.h)
        for y in range(self.h):
            for x in range(self.w):
                X, Y = x + 0.5 - cx, y + 0.5 - cy
                sx = cx + X * ct - Y * st
                sy = cy + X * st + Y * ct
                p = self.get(int(math.floor(sx)), int(math.floor(sy)))
                if p:
                    out.px[y * self.w + x] = p
        return out

    def shifted(self, dx, dy):
        out = LSprite(self.w, self.h)
        for y in range(self.h):
            for x in range(self.w):
                p = self.px[y * self.w + x]
                if p:
                    out.set(x + dx, y + dy, p[0], p[1])
        return out

    def layers(self, outline=True):
        out = [Sprite(self.w, self.h) for _ in range(3)]
        for i, p in enumerate(self.px):
            if p:
                out[p[0]].px[i] = p[1]
        if outline:
            oc = col(OUTLINE)
            for y in range(self.h):
                for x in range(self.w):
                    if self.px[y * self.w + x]:
                        continue
                    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        if self.get(x + dx, y + dy):
                            out[BASE].px[y * self.w + x] = oc
                            break
        return out


def wiz_staff(s, hand, ang, up, down, glow=2, gem=True):
    """Staff through the hand at angle `ang` (deg, 90 = straight up)."""
    a = math.radians(ang)
    dx, dy = math.cos(a), -math.sin(a)
    hx, hy = hand
    bx_, by_ = hx - dx * down, hy - dy * down
    tx, ty = hx + dx * up, hy + dy * up
    pts = line_pts(bx_, by_, tx, ty)
    for i, (x, y) in enumerate(pts):
        c = W_WOOD[1] if (i % 4) else W_WOOD[0]
        if i == len(pts) - 1:
            c = W_WOOD[2]
        s.set(x, y, BASE, c)
    px_, py_ = -dy, dx
    g1 = (round(tx + dx), round(ty + dy))
    g2 = (round(tx + dx * 2), round(ty + dy * 2))
    # forked head: two claws beside the gem
    for k in (-1, 1):
        cxk, cyk = round(tx + dx + px_ * k), round(ty + dy + py_ * k)
        if (cxk, cyk) != g2:
            s.set(cxk, cyk, BASE, W_WOOD[2] if k < 0 else W_WOOD[0])
    cl = [W_CRYS_D, W_CRYS_D, W_CRYS, (255, 255, 255, 255)][max(0, min(3, glow))]
    s.set(g2[0], g2[1], BASE, cl)
    if gem:
        s.set(g1[0], g1[1], TRIM, TG[3] if glow >= 2 else TG[1])
    else:
        s.set(g1[0], g1[1], BASE, (90, 86, 104, 255))
    return g2, (dx, dy)


# Hand-drawn master body (idle), no front arm / staff / boots.
# robe a<b<c<d (dark->light), trim 1<2<3<4, f face, g face light, e eye.
WIZ_BODY = [
    (2, 3, "d"),
    (3, 3, "cd"),
    (4, 4, "cdd"),
    (5, 4, "cddc"),
    (6, 4, "bcddd3"),
    (7, 4, "bcc3gf"),
    (8, 4, "bbc3fe"),
    (9, 4, "abbc32"),
    (10, 3, "cddcbbda"),
    (11, 3, "dccbcbba"),
    (12, 3, "2333334a"),
    (13, 3, "c1bcbbba"),
    (14, 2, "cccbcbcbba"),
]
WIZ_KEY = {"a": (ROBE, RG[0]), "b": (ROBE, RG[1]), "c": (ROBE, RG[2]), "d": (ROBE, RG[3]),
           "1": (TRIM, TG[0]), "2": (TRIM, TG[1]), "3": (TRIM, TG[2]), "4": (TRIM, TG[3]),
           "f": (BASE, W_FACE), "g": (BASE, W_FACE_L), "e": (BASE, W_EYE), "E": (BASE, W_EYE_D),
           "h": (BASE, W_SKIN), "H": (BASE, W_SKIN_D), "k": (BASE, W_BOOT), "K": (BASE, W_BOOT_L)}


def wiz_body(lean=0, sway=0, flare=0, crouch=0, stretch=0, billow=0, hd=(0, 0), eyes=2, tip=0):
    """Return {(x,y): ch} for the body template with simple pose transforms.
    lean: shift rows <= 11 forward; sway: shift hem (row 14, half of row 13);
    flare: extra trailing cloth at the back of rows 13-14 (run / jump);
    crouch: drop torso rows (upper body moves down); stretch: raise upper body
    1px and narrow the hem; billow: hem lifts and flares outward (falling)."""
    P = {}
    for (y, x0, row) in WIZ_BODY:
        for i, ch in enumerate(row):
            if ch == ".":
                continue
            x = x0 + i
            yy = y
            if y <= 9:
                x += hd[0]
                yy += hd[1]
            if y <= 11:
                x += lean
            if y == 14:
                x += sway
            if y == 13 and sway > 0 and i >= 4:
                x += 1
            if y == 13 and sway < 0 and i < 4:
                x -= 1
            if crouch and y <= 11:
                if y == 11 and crouch >= 1:
                    continue
                yy += crouch
            if stretch and y <= 12:
                yy -= stretch
            P[(x, yy)] = ch
    if eyes < 2:
        for k, v in list(P.items()):
            if v == "e":
                P[k] = "E" if eyes == 1 else "f"
    if tip:
        # hood tip flutter
        for k, v in list(P.items()):
            if k[1] == 2 + hd[1] - stretch + (crouch or 0) and v == "d":
                del P[k]
                P[(k[0] + tip, k[1])] = "d"
    if stretch:
        # fill the gap row and taper the hem
        for x in range(3, 11):
            if (x, 12) not in P and (x, 11) in P:
                P[(x, 12)] = P.get((x, 13), "b")
        for x in (2, 11):
            P.pop((x, 14), None)
    if flare:
        ys = [13, 14]
        for y in ys:
            xs = [x for (x, yy) in P if yy == y]
            if not xs:
                continue
            l = min(xs)
            for k in range(1, flare + 1):
                if y == 14 or k < flare:
                    P[(l - k, y - (1 if (k == flare and y == 14 and flare > 1) else 0))] = "b" if k < flare else "a"
    if billow:
        # hem rises and flares: corners flap upward
        hem = {k: v for k, v in P.items() if k[1] >= 13}
        for k in hem:
            del P[k]
        for (x, y), v in hem.items():
            P[(x, y - 1)] = v
        xs = [x for (x, y) in P if y == 13]
        l, r = min(xs), max(xs)
        P[(l - 1, 13)] = "c"
        P[(l - 2, 12 - (billow > 1))] = "c"
        P[(l - 1, 12)] = "b"
        P[(r + 1, 13)] = "a"
        P[(r + 1, 12)] = "a"
        if billow > 1:
            P[(r + 2, 11)] = "a"
        for x in ((l + 1, r - 2) if billow == 1 else (l + 2, l + 5)):
            if (x, 13) in P:
                P[(x, 14)] = "b"
    return P


def wizard(bx=0, by=0, hd=(0, 0), sway=0, feet=((4, 15), (8, 15)),
           hand_f=None, hand_b=None, staff=None, staff_behind=False,
           glow=2, lean=0, flare=0, eyes=2, gem=True, sleeve_up=False,
           crouch=0, stretch=0, billow=0, tip=0, grip=True, show_back=None):
    """Compose one wizard pose into an LSprite.  feet are boot left-x/y."""
    s = LSprite()
    if hand_f is None:
        hand_f = (11 + lean, 10 + crouch - stretch)
    if staff is not None and staff_behind:
        wiz_staff(s, staff[0], staff[1], staff[2], staff[3], glow, gem)
    sy = crouch - stretch
    # back arm (in shadow) only when it sticks out of the silhouette
    if hand_b is not None:
        shb = (5 + lean + bx, 10 + sy + by)
        for (x, y) in line_pts(shb[0], shb[1], hand_b[0], hand_b[1])[:-1]:
            s.set(x, y, ROBE, RG[0])
        s.set(hand_b[0], hand_b[1], BASE, W_SKIN_D)
    # boots
    for i, (fx, fy) in enumerate(feet):
        s.set(fx, fy, BASE, W_BOOT)
        s.set(fx + 1, fy, BASE, W_BOOT_L if i == 1 else W_BOOT)
    P = wiz_body(lean, sway, flare, crouch, stretch, billow, hd, eyes, tip)
    for (x, y), ch in P.items():
        s.set(x + bx, y + by, *WIZ_KEY[ch])
    # front arm: sleeve + cuff + hand
    sh = (9 + lean + bx, 10 + sy + by)
    arm = line_pts(sh[0], sh[1], hand_f[0], hand_f[1])
    sleeve = arm[:-1]
    for i, (x, y) in enumerate(sleeve):
        last = (i == len(sleeve) - 1 and len(sleeve) > 1)
        if last:
            s.set(x, y, TRIM, TG[2])
        else:
            s.set(x, y, ROBE, RG[3] if i == 0 else RG[2])
        if sleeve_up:
            if s.get(x - 1, y) is None or s.get(x - 1, y)[0] != BASE:
                s.set(x - 1, y, ROBE, RG[1]) if not last else s.set(x - 1, y, TRIM, TG[0])
        else:
            if s.get(x, y + 1) is None or s.get(x, y + 1)[0] == ROBE:
                s.set(x, y + 1, ROBE, RG[0]) if not last else s.set(x, y + 1, TRIM, TG[0])
    if staff is not None and not staff_behind:
        wiz_staff(s, staff[0], staff[1], staff[2], staff[3], glow, gem)
    s.set(hand_f[0], hand_f[1], BASE, W_SKIN)
    if grip and staff is not None and not staff_behind:
        gx, gy = staff[0]
        if (gx, gy) != hand_f:
            s.set(gx, gy, BASE, W_SKIN_D)
    return s


def staff_tip(staff):
    (hx, hy), ang, up, _ = staff
    a = math.radians(ang)
    dx, dy = math.cos(a), -math.sin(a)
    return (int(round(hx + dx * (up + 2))), int(round(hy + dy * (up + 2)))), (dx, dy)


def glow_fx(base, x, y, level, colr="#d9ccff"):
    """Halo / star burst around a point, drawn after the outline (no outline).
    Levels: 1 soft cross, 2 + diagonals, 3 + long arms, 4 full burst."""
    if level <= 0:
        return
    c = col(colr)
    hot = mix(c, "white", 0.6)
    pts = []
    pts += [((x + dx, y + dy), c, 150) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))]
    if level >= 2:
        pts += [((x + dx, y + dy), c, 80) for dx, dy in ((1, 1), (-1, 1), (1, -1), (-1, -1))]
    if level >= 3:
        pts += [((x + dx, y + dy), hot, 190) for dx, dy in ((1, 0), (-1, 0), (0, -1))]
        pts += [((x + dx, y + dy), c, 130) for dx, dy in ((2, 0), (-2, 0), (0, -2), (0, 2))]
    if level >= 4:
        pts += [((x + dx, y + dy), c, 90) for dx, dy in ((3, 0), (-3, 0), (0, -3))]
        pts += [((x + dx, y + dy), c, 110) for dx, dy in ((2, 2), (-2, 2), (2, -2), (-2, -2))]
        pts += [((x + dx, y + dy), hot, 220) for dx, dy in ((1, 1), (-1, 1), (1, -1), (-1, -1))]
    for (px_, py_), cc, a in pts:
        if not base.inb(px_, py_):
            continue
        q = base.get(px_, py_)
        if q[3] == 255 and q != col(OUTLINE):
            continue
        if q == col(OUTLINE):
            # glow eats the outline a little: tint it instead
            base.set(px_, py_, mix(q, cc, a / 400.0))
            continue
        base.blend(px_, py_, with_alpha(cc, a))


def flare_fx(base, x, y, d, level):
    """Directional cast flare (a small cone of light) from the staff tip."""
    hot = col("#ffffff")
    c = col("#e6dcff")
    c2 = col("lilac")
    px_, py_ = -d[1], d[0]
    done = set()

    def put(X, Y, cc, a):
        X, Y = int(round(X)), int(round(Y))
        if (X, Y) in done or not base.inb(X, Y):
            return
        done.add((X, Y))
        q = base.get(X, Y)
        if q[3] == 255 and q != col(OUTLINE):
            return
        base.set(X, Y, with_alpha(cc, a)) if q[3] == 0 else base.set(X, Y, mix(q, cc, a / 255.0))
    put(x, y, hot, 255)
    for k in range(1, level + 1):
        w = min(k - 1, 2) if level >= 3 else (k - 1) // 2
        for j in range(-w, w + 1):
            edge = abs(j) == w and w > 0
            cc = hot if (j == 0 and k <= 2) else (c2 if edge else c)
            a = 255 if j == 0 and k == 1 else max(90, 230 - 40 * k - 30 * abs(j))
            put(x + d[0] * k + px_ * j, y + d[1] * k + py_ * j, cc, a)
    if level >= 3:
        for k in (-1, 1):
            put(x + px_ * k, y + py_ * k, c, 170)
            put(x - d[0] + px_ * k * 1, y - d[1] + py_ * k, c2, 90)


WIZ_STAFF_KEY = {"W": (BASE, W_WOOD[1]), "w": (BASE, W_WOOD[0]), "V": (BASE, W_WOOD[2]),
                 "X": (BASE, W_CRYS), "x": (BASE, W_CRYS_D), "h": (BASE, W_SKIN), "H": (BASE, W_SKIN_D)}


def wiz_ascii(rows, y0, glow=2, gem=True):
    s = LSprite()
    key = dict(WIZ_KEY)
    key.update(WIZ_STAFF_KEY)
    dead = (104, 98, 120, 255)
    for r, line in enumerate(rows):
        for x, ch in enumerate(line):
            if ch == "G":   # staff gem
                if gem:
                    s.set(x, y0 + r, TRIM, TG[3] if glow >= 2 else (TG[1] if glow == 1 else TG[0]))
                else:
                    s.set(x, y0 + r, BASE, (78, 72, 92, 255))
            elif ch == "X":
                s.set(x, y0 + r, BASE, [dead, W_CRYS_D, W_CRYS, (255, 255, 255, 255)][glow])
            elif ch in key:
                s.set(x, y0 + r, *key[ch])
    return s


WIZ_SLUMP = [
    "......d.........",
    ".....cdd........",
    ".....bcddc......",
    ".....bcdd3f.....",
    "....bbcc3ff.....",
    "...cbbcbb3H.....",
    "...dccbcba......",
    "..d2333334a..wWV",
    ".kKcbcbcbba.WWGX",
]
WIZ_LYING = [
    "............dc..",
    "...cddcc...cdd3.",
    "kKcbcbbbcbcbbb3f",
    ".VWWwWWWWwWWWWGX",
]


def wiz_lying(glow, gem):
    """Collapsed apprentice, face down; the staff lies on the ground in front."""
    return wiz_ascii(WIZ_LYING, 12, glow, gem)


def merge(dst, src):
    for i, p in enumerate(src.px):
        if p and dst.px[i] is None:
            dst.px[i] = p
    return dst


def wiz_frames():
    """All wizard frames in master coordinates (shifted +1 x at the end so the
    body centre sits at x~8)."""
    F = {}
    gl = {}

    def put(c, r, s, fx=()):
        F[(c, r)] = s.shifted(1, 0)
        gl[(c, r)] = [(lambda b, f=f: f(b, 1)) for f in fx]

    def halo(staff, lvl, colr="#e9e0ff"):
        t, d = staff_tip(staff)
        return lambda b, ox: glow_fx(b, t[0] + ox, t[1], lvl, colr)

    def flare(staff, lvl):
        t, d = staff_tip(staff)
        return lambda b, ox: flare_fx(b, t[0] + ox, t[1], d, lvl)

    # row 0 idle: hem sway, hood-tip flutter, staff glow pulse
    sways = [0, 0, 1, 1, 0, -1]
    tips = [0, 0, 0, 1, 1, 0]
    glows = [1, 2, 3, 3, 2, 1]
    for i in range(6):
        st = ((12, 10), 90, 8, 5)
        s = wizard(sway=sways[i], tip=tips[i], hand_f=(11, 10), staff=st, glow=2 if glows[i] < 3 else 3)
        put(i, 0, s, [halo(st, glows[i])])
    # row 1 run: lean forward, cloak trailing, staff angled forward
    for i in range(6):
        ph = 2 * math.pi * i / 6
        by = 0 if i in (0, 3) else -1

        def foot(p):
            fc = 6.5 + 2.6 * math.cos(p)
            lift = int(round(1.2 * max(0.0, -math.sin(p))))
            return (int(round(fc - 1)), 15 - lift)
        ff, bf = foot(ph), foot(ph + math.pi)
        st = ((12, 10 + by), 60, 7, 4)
        s = wizard(by=by, lean=1, flare=1 + (i % 2), sway=-1, feet=(bf, ff), hand_f=(11, 10 + by), staff=st)
        put(i, 1, s, [halo(st, 1)])
    # row 2 jump: crouch, launch, rise, apex
    st = ((12, 11), 80, 7, 4)
    s = wizard(crouch=1, feet=((3, 15), (8, 15)), hand_f=(11, 11), staff=st, flare=1)
    put(0, 2, s, [halo(st, 1)])
    st = ((12, 8), 85, 6, 6)
    s = wizard(by=-1, stretch=1, feet=((4, 14), (7, 15)), hand_f=(11, 8), staff=st, sleeve_up=True)
    put(1, 2, s, [halo(st, 2)])
    st = ((12, 9), 75, 6, 5)
    s = wizard(by=-1, feet=((4, 14), (8, 13)), hand_f=(11, 9), staff=st, flare=1, sway=-1)
    put(2, 2, s, [halo(st, 1)])
    st = ((12, 9), 70, 6, 5)
    s = wizard(by=-1, feet=((4, 14), (8, 14)), hand_f=(11, 9), staff=st, billow=1)
    put(3, 2, s, [halo(st, 1)])
    # row 3 fall: robe billows up, arms raised
    for i in range(4):
        st = ((12, 6 + i % 2), 72, 5, 6)
        s = wizard(by=-1, billow=1 + (i % 2), feet=((4, 14 + (i % 2)), (8, 15 - (i % 2))),
                   hand_f=(11, 6 + i % 2), hand_b=(3, 7 + (i + 1) % 2), staff=st, sleeve_up=True, tip=1 if i % 2 else 0)
        put(i, 3, s, [halo(st, 1 + i % 2)])
    # row 4 climb: clinging to a wall on the right, staff slung across the back
    for i in range(6):
        sn = math.sin(2 * math.pi * i / 6)
        hf = (12, int(round(7 - 2.0 * sn)))
        hb = (12, int(round(7 + 2.0 * sn)))
        ff = (10, 15 - int(round(2 * max(0.0, sn))))
        bf = (9, 15 - int(round(2 * max(0.0, -sn))))
        st = ((6, 9), 130, 6, 6)
        s = wizard(bx=1, lean=0, feet=(bf, ff), hand_f=hf, hand_b=None, staff=st, staff_behind=True, glow=1,
                   sleeve_up=True, sway=0, tip=1 if i in (2, 3) else 0)
        # back hand reaching past the hood
        s.set(hb[0], hb[1], BASE, W_SKIN_D)
        s.set(hb[0] - 1, hb[1], ROBE, RG[1])
        put(i, 4, s, [])
    # row 5 cast_side: ready, draw back, thrust, flare, sustain, recover
    side = [((12, 10), 70, 7, 4, 0, 0, 1), ((11, 10), 25, 4, 4, -1, 0, 1), ((11, 10), 0, 1, 6, 1, 2, 0),
            ((11, 10), 0, 1, 6, 1, 3, 0), ((11, 10), 0, 1, 6, 1, 2, 0), ((11, 10), 12, 2, 5, 0, 1, 0)]
    for i, (h, a, up, dn, ln, fl, hl) in enumerate(side):
        st = (h, a, up, dn)
        hf = (h[0] - 1, h[1])
        s = wizard(lean=max(0, ln), sway=-1 if ln > 0 else 0, flare=1 if ln > 0 else 0, hand_f=hf, staff=st,
                   feet=((3, 15), (8, 15)) if ln > 0 else ((4, 15), (8, 15)), glow=3 if fl else 2,
                   hand_b=(5, 11) if ln < 0 else None)
        put(i, 5, s, [flare(st, fl)] if fl else [halo(st, hl)])
    # row 6 cast_down: staff pointed down in front
    down = [((12, 10), 60, 7, 4, 0), ((12, 8), 115, 5, 3, 0), ((12, 10), -88, 2, 3, 2),
            ((12, 10), -90, 2, 3, 3), ((12, 10), -90, 2, 3, 2), ((12, 10), -70, 2, 3, 1)]
    for i, (h, a, up, dn, fl) in enumerate(down):
        st = (h, a, up, dn)
        cr = 1 if fl >= 3 else 0
        s = wizard(hand_f=(h[0] - 1, h[1]), staff=st, glow=3 if fl else 2, crouch=cr, feet=((3, 15), (8, 15)) if cr else ((4, 15), (8, 15)))
        put(i, 6, s, [flare(st, fl)] if fl else [halo(st, 1)])
    # row 7 cast_up: staff thrust straight up
    up_ = [((12, 10), 90, 8, 5, 0), ((12, 8), 90, 6, 6, 0), ((12, 6), 90, 4, 7, 3),
           ((12, 5), 90, 3, 7, 4), ((12, 5), 90, 3, 7, 3), ((12, 7), 90, 5, 6, 1)]
    for i, (h, a, up, dn, fl) in enumerate(up_):
        st = (h, a, up, dn)
        s = wizard(hand_f=(h[0] - 1, h[1]), staff=st, glow=3 if fl else 2, sleeve_up=h[1] < 9,
                   stretch=1 if fl >= 3 else 0, by=0)
        put(i, 7, s, [flare(st, fl)] if fl else [halo(st, 1)])
    # row 8 cast_spell: staff raised overhead in both hands, burst of light
    sp_ = [((12, 10), 85, 7, 4, 0, 1), ((11, 7), 80, 4, 5, 0, 2), ((11, 5), 80, 3, 6, 1, 3),
           ((11, 5), 80, 3, 6, 1, 4), ((11, 5), 80, 3, 6, 1, 3), ((12, 8), 85, 5, 5, 0, 2)]
    for i, (h, a, up, dn, rise, g) in enumerate(sp_):
        st = (h, a, up, dn)
        s = wizard(stretch=rise, hand_f=(h[0] - 1, h[1]), hand_b=(h[0] - 2, h[1] + 1) if h[1] < 9 else None, staff=st,
                   glow=3, sleeve_up=h[1] < 9, billow=0, tip=rise)
        put(i, 8, s, [halo(st, g, "#fff3d6")])
    # row 9 hurt: recoil (frame 0 white flash, frame 2 red flash)
    rec = dict(lean=-1, hd=(-1, 0), feet=((3, 15), (7, 14)), hand_f=(10, 9), hand_b=(3, 9), sway=1,
               staff=((11, 9), 105, 6, 5), sleeve_up=True)
    s = wizard(**rec)
    s.flash("white")
    put(0, 9, s)
    put(1, 9, wizard(**rec), [halo(rec["staff"], 1)])
    s = wizard(**rec)
    s.flash("red")
    put(2, 9, s)
    st = ((12, 10), 95, 8, 5)
    put(3, 9, wizard(hand_f=(11, 10), staff=st, sway=1), [halo(st, 1)])
    # row 10 death: hit, buckle, kneel (staff tipping), topple, lying, glow fades
    s = wizard(**rec)
    s.flash("red")
    put(0, 10, s)
    st = ((11, 11), 108, 6, 4)
    put(1, 10, wizard(lean=-1, crouch=1, hd=(-1, 0), feet=((3, 15), (7, 15)), hand_f=(10, 11), staff=st, eyes=2),
        [halo(st, 1)])
    kneel = dict(crouch=2, feet=((3, 15), (7, 15)), hand_f=(10, 13), eyes=1, glow=1)
    s = wizard(**kneel)
    ls = LSprite()
    wiz_staff(ls, (11, 13), 35, 5, 2, 1)
    merge(s, ls)
    put(2, 10, s)
    s = wiz_ascii(WIZ_SLUMP, 7, 1)
    put(3, 10, s)
    for k, (dy, g, gem_) in enumerate([(-1, 2, True), (0, 1, True), (0, 1, True), (0, 0, False)]):
        s = wiz_lying(g, gem_).shifted(0, dy)
        lvl = [2, 1, 0, 0][k]
        put(4 + k, 10, s, [lambda b, ox, d=dy, l=lvl: glow_fx(b, 15 + ox - 1, 15 + d, l)] if lvl else [])
    return F, gl


def gen_wizard():
    F, gl = wiz_frames()
    sheets = [Sprite(128, 176) for _ in range(3)]
    for (c, r), s in F.items():
        lay = s.layers()
        for fx in gl.get((c, r), []):
            fx(lay[BASE])
        for k in range(3):
            sheets[k].blit(lay[k], c * 16, r * 16)
    return sheets

# --------------------------------------------------------------------------
# Icons (16x16, 16 cols x 8 rows, index = row*16 + col)
# --------------------------------------------------------------------------
def ramp_of(school):
    return NEUTRAL_RAMP if school is None else SCHOOL_RAMP[school]


def draw_ascii(sp, rows, ox, oy, key):
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            if ch in key and key[ch] is not None:
                sp.set(ox + x, oy + y, key[ch])


def ascii_size(rows):
    return max(len(r) for r in rows), len(rows)


def emblem_key(ramp):
    return {"1": ramp[0], "2": ramp[1], "3": ramp[2], "4": ramp[3], "w": "white", "k": "ink",
            "b": "bone", "B": "bone_l", "d": "bone_d", "g": "stone_l", "G": "stone_h", "s": "stone",
            "r": "red", "R": "red_dk", "o": "orange", "y": "yellow", "Y": "yellow_o", "c": "aether",
            "C": "teal_l", "m": "mag", "M": "mag_l", "t": "teal", "v": "vio_l", "V": "lilac",
            "n": "wood", "N": "wood_l", "p": "pale", "e": "sick_l", "E": "sick", "u": "blue", "U": "cyan",
            "a": "gold", "A": "gold_l", "x": "#5a3a6a"}


# ---- school sigils --------------------------------------------------------
SIGIL_GLYPHS = [
    # 0 Pyromancy: flame
    ["....#....",
     "...##....",
     "...###.#.",
     "..####.#.",
     "..##+###.",
     ".##+++##.",
     ".#++++##.",
     ".##+++#..",
     "..####..."],
    # 1 Hydromancy: droplet over a wave
    ["....#....",
     "...###...",
     "..##+##..",
     "..#+++#..",
     "..##+##..",
     "...###...",
     ".........",
     ".##..##..",
     "#..##..##"],
    # 2 Terramancy: twin peaks
    [".........",
     "...#.....",
     "..###....",
     "..#+##.#.",
     ".##++####",
     ".#+++#+##",
     "##+++++##",
     "#########",
     "........."],
    # 3 Cryomancy: snowflake
    ["....#....",
     ".#..#..#.",
     "..#.#.#..",
     "...#+#...",
     "####+####",
     "...#+#...",
     "..#.#.#..",
     ".#..#..#.",
     "....#...."],
    # 4 Aeromancy: swirl
    ["..####...",
     ".#....#..",
     "#..##..#.",
     "#.#..#.#.",
     "#.#.#..#.",
     "#..#..#..",
     ".#...#...",
     "..###....",
     "........."],
    # 5 Fulmancy: lightning bolt
    [".....###.",
     "....###..",
     "...###...",
     "..######.",
     ".....##..",
     "....##...",
     "...##....",
     "..##.....",
     "..#......"],
    # 6 Alchemy: flask
    ["...###...",
     "....#....",
     "...#.#...",
     "...#.#...",
     "..#...#..",
     ".#+++++#.",
     ".#+#++#+.",
     "..#####..",
     "........."],
    # 7 Mycomancy: mushroom with spores
    ["#.......#",
     "..#####..",
     ".#+++++#.",
     "#++#++#+#",
     "#########",
     "...#+#...",
     "...#+#...",
     "..##+##..",
     "........."],
]


def badge_disc(sp, dark, mid, light, rim_hi, rim_lo, r=6.9, cx=8, cy=8):
    disc = m_ellipse(cx, cy, r, r)
    inner = m_ellipse(cx, cy, r - 1.0, r - 1.0)
    for (x, y) in disc:
        dx, dy = x + 0.5 - cx, y + 0.5 - cy
        if (x, y) not in inner:
            sp.set(x, y, rim_hi if dx + dy < -1.5 else (rim_lo if dx + dy > 1.5 else mix(rim_hi, rim_lo, 0.5)))
        else:
            d = math.hypot(dx + 2.0, dy + 2.0) / (r + 2.0)
            sp.set(x, y, ramp_soft([light, mid, dark], d * 1.3, x, y))
    return disc, inner


def ico_sigil(sp, school):
    ramp = SCHOOL_RAMP[school]
    dark, mid, light, glow = ramp
    badge_disc(sp, mix(dark, "ink", 0.35), dark, mix(dark, mid, 0.55), mix(light, "white", 0.15), mix(dark, "ink", 0.2))
    # rune ticks on the rim
    for a in range(8):
        t = a * math.pi / 4 + math.pi / 8
        x, y = 8 + math.cos(t) * 5.0 - 0.5, 8 + math.sin(t) * 5.0 - 0.5
        sp.set(round(x), round(y), mix(mid, light, 0.4))
    g = SIGIL_GLYPHS[school]
    w, h = ascii_size(g)
    ox, oy = 4, 4
    gm = {(ox + x, oy + y) for y, r in enumerate(g) for x, c in enumerate(r) if c in "#+"}
    # drop shadow, then glyph
    for (x, y) in gm:
        if (x + 1, y + 1) not in gm:
            sp.set(x + 1, y + 1, mix(dark, "ink", 0.5))
    for y, r in enumerate(g):
        for x, c in enumerate(r):
            if c == "#":
                sp.set(ox + x, oy + y, glow)
            elif c == "+":
                sp.set(ox + x, oy + y, light)
    sp.outline()


# ---- scrolls ---------------------------------------------------------------
SCROLL_EMBLEMS = {
    # Pyromancy
    8: ["......4.",            # Fire Bolt: comet
        "....4443",
        "...44w43",
        "..3443w3",
        ".2334432",
        "2.3332..",
        ".2.2....",
        "2.......", ],
    9: ['.344443.', '.3w4443.', '..3443..', '..3w43..', '...44...', 'g..43..g', 'sg.4..gs', 'ssg4.gss'],
    10: [".bbbbbb.",           # Salamander Ward: shield with flame
         ".b.33.b.",
         ".b3443b.",
         ".b34w3b.",
         ".b3443b.",
         "..b33b..",
         "...bb...",
         "........", ],
    11: [".33..33.",           # Ember Heart: heart with flame core
         "3443344.",
         "34w4w443",
         "3444w443",
         ".34w443.",
         "..3443..",
         "...33...",
         "........", ],
    12: ["3..4...3",           # Pocket Sun
         ".3.4.3..",
         "..444...",
         "4444w444",
         "..444...",
         ".3.4.3..",
         "3..4...3",
         "........", ],
    # Hydromancy
    13: ["...333..",           # Tidecall: curling wave
         "..3...3.",
         ".3..44.3",
         ".3.4..3.",
         "3..4.3..",
         "3.4.....",
         "34..3.33",
         ".3333...", ],
    14: ["..3333..",           # Flood Orb
         ".344443.",
         "34w44443",
         "3w444443",
         "34444423",
         "34444223",
         ".322223.",
         "..3333..", ],
    15: ["...333..",           # Gills of the Deep: fish head with gills
         "..34443.",
         ".34w4443",
         "3444444.",
         "3.4.4.4.",
         "3.4.4.4.",
         ".3444443",
         "..3333..", ],
    16: ["..3333..",           # Undertow: whirlpool
         ".3....3.",
         "3..44..3",
         "3.4..4.3",
         "3.4.w4.3",
         "3..4...3",
         ".3...33.",
         "..333...", ],
    # Terramancy
    17: ['.333333.', '.344w43.', '.34.4.3.', '.3.43.3.', '..3..3..', '.3.3..3.', '..3..3..', '.3..3..3'],
    18: ["...44...",           # Stone Shape: raised ledge with up arrow
         "..4444..",
         "...44...",
         "...44...",
         "34444443",
         "33333332",
         ".222222.",
         ".2....2.", ],
    19: ['.4.4.4..', '.4.4.4.4', '.4444444', '.444444.', '4444444.', '.44444..', '..3333..', '........'],
    20: ['...44...', '...44...', '.444444.', '..4444..', '...44...', '33333333', '3.3..3.3', '.3.33.3.'],
    # Cryomancy
    21: ["...4.4..",           # Frost Seed
         "....4...",
         "...4w4..",
         "....3...",
         "..3333..",
         ".344443.",
         ".344423.",
         "..3332..", ],
    22: [".......4",           # Ice Lance
         "......44",
         ".....4w.",
         "....4w..",
         "...343..",
         "..343...",
         ".242....",
         "22......", ],
    23: ["...4.4..",           # Frozen Path: footprints on ice tiles
         "..444...",
         "....4.4.",
         "...444..",
         "........",
         "33443344",
         "34433443",
         "22332233", ],
    24: ["..4444..",           # Rime Shell: crystal dome
         ".4w4334.",
         "4w43..34",
         "443....4",
         "43.....4",
         "34.....3",
         "33333333",
         "22222222", ],
    # Aeromancy
    25: ["....333.",           # Gust: wind streams
         "33333..3",
         "......3.",
         "44444...",
         "......4.",
         ".444444.",
         ".......4",
         "....444.", ],
    26: ['...4....', '..4w4...', '.4...4..', '...3....', '..3.3...', '.3...3..', '...2....', '..2.2...'],
    27: ["......4.",           # Levitate: feather
         ".....444",
         "....4w43",
         "...4w43.",
         "..4w43..",
         ".443....",
         ".3......",
         "3.......", ],
    28: ["..3333..",           # Air Bubble
         ".3w...3.",
         "3w.....3",
         "3......3",
         "3.....43",
         ".3...43.",
         "..3333.4",
         "......44", ],
    # Fulmancy
    29: ["...3....",           # Spark Bolt: four-point spark
         "...4....",
         "..343...",
         "3444w443",
         "..343...",
         "...4....",
         "...3..2.",
         "......2.", ],
    30: ["4......4",           # Chain Lightning: zigzag between nodes
         ".3....3.",
         "..3..3..",
         "...44...",
         "..3.3...",
         ".3...3..",
         "3....3..",
         "4.....4.", ],
    31: ['gg......', 'gGg..4..', 'gGGg.4..', 'gGGGg44.', 'gGGg.4..', 'gGg.4...', 'gg...4..', '........'],
    32: ["...44...",           # Grounded: earth symbol
         "...44...",
         "...44...",
         "44444444",
         "........",
         ".333333.",
         "........",
         "..2222..", ],
    # Alchemy
    33: ["..bbb...",           # Acid Flask
         "...b....",
         "..3.3...",
         ".3.4.3..",
         "3.4...3.",
         "34444w3.",
         "3444443.",
         ".33333..", ],
    34: ["......4w",           # Alchemist's Charge: bomb + fuse
         ".....b4.",
         "..3333..",
         ".344443.",
         "34w44443",
         "3444443.",
         "3444423.",
         ".33222..", ],
    35: ["...44...",           # Transmutation: triangle in circle
         "..4..4..",
         ".4.33.4.",
         "4.3..3.4",
         "4.3..3.4",
         ".43333..",
         "..4..4..",
         "...44...", ],
    36: ["..3..3..",           # Volatile Core: cracked unstable orb
         ".3.33.3.",
         "..3443..",
         "334ww433",
         "..34443.",
         ".3.33.3.",
         "..3..3..",
         "........", ],
    # Mycomancy
    37: ['4..4..4.', '..4..4..', '.4..4...', '..3333..', '.344443.', '3444w443', '.222222.', '...22...'],
    38: ['.4...4..', '444.444.', '.2...2..', '.bbbbbb.', 'bkkbkkbb', 'bbbbbbbb', '.bbkbbb.', '..b.b.b.'],
    39: ["........",           # Mycelial Bridge: arch of fungus
         "..4444..",
         ".43333..",
         "43....34",
         "3......3",
         "3......3",
         "2......2",
         "2......2", ],
    40: ['.33..33.', '3443344.', '34w44443', '3442w443', '.34424..', '..3443..', '...33...', '........'],
    # Neutral
    41: ['3..3..3.', '.3.3.3..', '..bbb...', '.b4w4b..', '.b444b..', '.b444b..', '..bbb...', '...b....'],
    42: ["..3333..",           # Glass Focus: lens
         ".3wc..3.",
         "3wc....3",
         "3c.....3",
         "3......3",
         ".3....3.",
         "..33333.",
         "......33", ],
    43: [".ggg....",           # Iron Soles: iron boot
         ".gGg....",
         ".gGg....",
         ".gGgg...",
         ".gGGggg.",
         ".gggggg.",
         ".333333.",
         "........", ],
    44: ["....44..",           # Blink: dotted trail into a star
         "...4ww4.",
         "....44..",
         "..3.....",
         ".3......",
         "2.......",
         "..2.....",
         "2.......", ],
}
SCROLL_SCHOOL = {}
_S_SCH = [0] * 5 + [1] * 4 + [2] * 4 + [3] * 4 + [4] * 4 + [5] * 4 + [6] * 4 + [7] * 4 + [None] * 4
for _i, _sch in enumerate(_S_SCH):
    SCROLL_SCHOOL[8 + _i] = _sch

VELLUM = ["#1d1524", "#271c30", "#30233b"]


def ico_scroll(sp, idx):
    sch = SCROLL_SCHOOL[idx]
    ramp = ramp_of(sch)
    key = emblem_key(ramp)
    # vellum sheet
    vel = [mix(v, mix(ramp[0], "ink", 0.35), 0.55) for v in VELLUM]
    for y in range(3, 13):
        for x in range(3, 13):
            v = 0.95 - 0.07 * (x - 3) - 0.05 * (y - 3)
            sp.set(x, y, ramp_soft(vel, v, x, y))
    # faint ruled lines / ragged edge
    for y in (3, 12):
        for x in range(3, 13):
            if (x * 7 + y) % 5 == 0:
                sp.set(x, y, vel[0])
    # rolls (bone) with school-coloured end caps
    for (y0, top) in ((1, True), (13, False)):
        for x in range(2, 14):
            sp.set(x, y0, "bone_l" if top else "bone")
            sp.set(x, y0 + 1, "bone_d" if top else "bone_d")
        sp.set(2, y0, "bone")
        sp.set(13, y0 + 1, mix("bone_d", "ink", 0.3))
        for x, c, c2 in ((1, ramp[3], ramp[2]), (2, ramp[2], ramp[1]), (13, ramp[2], ramp[1]), (14, ramp[1], ramp[0])):
            sp.set(x, y0, c)
            sp.set(x, y0 + 1, c2)
    # inner shadow under the top roll
    for x in range(3, 13):
        sp.set(x, 3, mix(sp.get(x, 3), "ink", 0.45))
    sp.outline()
    em = SCROLL_EMBLEMS[idx]
    w, h = ascii_size(em)
    draw_ascii(sp, em, 8 - w // 2, 8 - h // 2 + (0 if h % 2 == 0 else 0), key)


# ---- fusion medallions -------------------------------------------------------
FUSION_PAIRS = [(a, b) for a in range(8) for b in range(a + 1, 8)]
FUSION_EMBLEMS = [
    # 45 Steam Burst: billowing puffs
    ["...##....", "..#++#.#.", ".#++++#+#", "..#++++#.", ".##+++##.", "#++#+#++#", ".##.#.##.", ".........", "........."],
    # 46 Magma Surge: volcano spilling
    [".#.#.#...", "..###....", "...#.....", "..#+#....", ".#+++#...", ".#+#++#..", "#+++#++#.", "#########", "........."],
    # 47 Thermal Shock: cracked block
    ['....#....', '...#+#...', '..#+#+#..', '.#++#++#.', '#+++#+++#', '.#+#+++#.', '..#+#+#..', '...#+#...', '....#....'],
    # 48 Firestorm: flame vortex
    ["..####...", ".#....#..", "...###.#.", "..#...#..", "...##.#..", "....#.#..", "...#.#...", "....#....", "........."],
    # 49 Plasma Lance: beam with burst
    [".......#.", "......###", ".......#.", "......#..", ".....#...", "....#....", "...#.....", "..#......", "##......."],
    # 50 Napalm: burning flask
    ["....#....", "...#+#...", "...##....", "...#.#...", "..#...#..", ".#+++++#.", ".#+#+++#.", "..#####..", "........."],
    # 51 Spore Bomb: pod with fuse spark
    [".......#.", "......#..", "...##.#..", "..#++#...", ".#+#++#..", ".#++#+#..", ".#+++#...", "..###....", "........."],
    # 52 Mudslide: slumping wave
    ["#........", "##.......", "#+#......", "#++#.....", "#+#+#....", "#++++##..", "#+#++++##", "#########", "........."],
    # 53 Glacier: ice mass
    ["...#.....", "..#+#.#..", "..#++#+#.", ".#++++++#", ".#+#++#+#", "#++++++++", "#+#+++#+#", "#########", "........."],
    # 54 Typhoon: spiral
    ["..#####..", ".#.....#.", "#..###..#", "#.#...#.#", "#.#.#.#..", "#..#..#..", ".#....#..", "..####...", "........."],
    # 55 Storm Flood: wave + bolt
    ["....##...", "...##....", "..####...", "...##....", "..##.....", ".........", ".##..##..", "#..##..##", "........."],
    # 56 Acid Rain: cloud and drops
    ["..###....", ".#+++##..", "#+++++#..", "#######..", ".........", ".#..#..#.", "#..#..#..", ".........", "........."],
    # 57 Swamp Bloom: mushroom on water
    ["..####...", ".#++++#..", "#++#+++#.", "########.", "...#+#...", "...#+#...", ".#.###.#.", "#.#...#.#", "........."],
    # 58 Frozen Earth: rock with frost star
    ["......#..", ".....###.", "......#..", "..###....", ".#+++#...", "#++#++#..", "#+++++##.", "########.", "........."],
    # 59 Sandstorm: swirl and grains
    ['.....#.#.', '######...', '.......#.', '.#.######', '.........', '####...#.', '....###..', '.#.......', '.........'],
    # 60 Railshot: slug with trail
    [".........", ".........", "#.#.###..", "..#++++#.", "#.#++++##", "..#++++#.", "#.#.###..", ".........", "........."],
    # 61 Petrify: stone face
    ["..#####..", ".#+++++#.", "#+##+##+#", "#+++++++#", "#++#+#++#", "#+++#+++#", ".#+###+#.", "..#####..", "........."],
    # 62 Root Lattice: crossing roots
    ['.#....#..', '.#....#..', '########.', '..#....#.', '..#....#.', '########.', '.#....#..', '#.#..#.#.', '.........'],
    # 63 Blizzard: flakes blown sideways
    ["#.#......", ".#...#.#.", "#.#...#..", ".....#.#.", "####.....", ".....####", "..#.#....", "...#.....", "..#.#...."],
    # 64 Cryo Arc: bolt with flake
    [".....#.#.", "......#..", ".....#.#.", "....##...", "...##....", "..####...", "....##...", "...##....", "..#......"],
    # 65 Shatter: burst of shards
    ["#...#...#", ".#..#..#.", "..#...#..", "....#....", "##.###.##", "....#....", "..#...#..", ".#..#..#.", "#...#...#"],
    # 66 Rimebloom: frost flower
    ["....#....", "..#.#.#..", "...###...", ".#######.", "...###...", "..#.#.#..", "....#....", "....#....", "...###..."],
    # 67 Thunderstorm: cloud + bolt
    ["..###....", ".#+++##..", "#+++++##.", "########.", "....##...", "...##....", "..####...", "....#....", "...#....."],
    # 68 Miasma: skull in fumes
    ["#..#..#..", ".#..#..#.", "..#####..", ".#+++++#.", ".#k+#k+#.", ".#++#++#.", "..#+#+#..", "..#.#.#..", "........."],
    # 69 Spore Gale: wind with spores
    ['.......#.', '#####.#..', '.....#...', '.#.#.....', '######...', '.......#.', '.####.#..', '.....#...', '.........'],
    # 70 Electrolysis: droplet split by bolt
    ["....#....", "...#+#...", "..#+##...", "..#++#...", ".#++#+#..", ".#+#+++#.", "..#####..", "..#.#.#..", "........."],
    # 71 Storm Spores: mushroom struck
    [".....##..", "....##...", "..######.", ".#++#+++#", "#########", "...#+#...", "...#+#...", "..##+##..", "........."],
    # 72 Decay Bloom: wilting flower with drips
    ["...###...", "..#+#+#..", "...###.#.", "....#.#..", "....##...", "...#.....", "..#..#...", ".......#.", "..#......"],
]


def ico_fusion(sp, k):
    a, b = FUSION_PAIRS[k]
    ra, rb = SCHOOL_RAMP[a], SCHOOL_RAMP[b]
    disc = m_ellipse(8, 8, 6.9, 6.9)
    inner = m_ellipse(8, 8, 5.6, 5.6)
    for (x, y) in disc:
        dx, dy = x + 0.5 - 8, y + 0.5 - 8
        if (x, y) not in inner:
            # gold rim with bevel and 8 notches
            sp.set(x, y, "gold_l" if dx + dy < -2.5 else ("gold_d" if dx + dy > 2.5 else "gold"))
        else:
            # diagonal split: school A top-left, school B bottom-right, dithered seam
            t = (dx + dy) / 2.0
            r = ra if t < -0.4 or (abs(t) <= 0.4 and (x + y) % 2 == 0) else rb
            d = math.hypot(dx + 1.2, dy + 1.2) / 7.0
            sp.set(x, y, ramp_pick([r[2], r[1], mix(r[0], r[1], 0.4)], d, x, y))
    for (x, y) in ((7, 1), (8, 14), (1, 8), (14, 7)):
        sp.set(x, y, "#fff0b0" if x + y < 15 else "gold_l")
    sp.outline()
    em = FUSION_EMBLEMS[k]
    w, h = ascii_size(em)
    ox, oy = 4, 4
    pts = {(ox + x, oy + y): c for y, r in enumerate(em) for x, c in enumerate(r) if c != "."}
    for (x, y), c in pts.items():
        if (x + 1, y + 1) not in pts and (x + 1, y + 1) in inner:
            sp.set(x + 1, y + 1, mix(sp.get(x + 1, y + 1), "ink", 0.6))
    for (x, y), c in pts.items():
        sp.set(x, y, {"#": "#fffaf0", "+": mix(mix(ra[2], rb[2], 0.5), "ink", 0.15), "k": "ink"}[c])


# ---- misc icons -------------------------------------------------------------
def ico_shard(sp):
    m = m_poly([(8, 1), (12, 5), (11, 12), (8, 15), (5, 12), (4, 5)])
    for (x, y) in m:
        sp.set(x, y, "teal_l" if x < 8 else "teal")
    for (x, y) in m:
        if x < 8 and y < 9:
            sp.set(x, y, "aether")
    for y in range(3, 14):
        sp.set(8, y, "#d8fffb" if y < 9 else "teal_l")
    sp.set(6, 4, "white")
    sp.set(6, 5, "white")
    sp.set(10, 11, "teal_d")
    sp.set(9, 13, "teal_d")
    sp.outline("ink")
    for (x, y, a) in [(2, 3, 200), (13, 2, 170), (14, 9, 140), (2, 11, 120), (1, 3, 90), (3, 3, 90), (2, 2, 90), (2, 4, 90)]:
        if sp.get(x, y)[3] == 0:
            sp.set(x, y, with_alpha("aether", a))


def ico_light_orb(sp):
    for (x, y) in m_ellipse(8, 8, 6.8, 6.8):
        d = math.hypot(x + 0.5 - 8, y + 0.5 - 8)
        sp.set(x, y, with_alpha("lilac", int(max(0, 120 - 15 * d))))
    sphere(sp, 8, 8, 4.2, ["vio_l", "lilac", "magic", "white"])
    sp.set(6, 6, "white")
    for (x, y) in [(8, 1), (8, 14), (1, 8), (14, 8)]:
        sp.set(x, y, with_alpha("magic", 200))


def draw_chest(sp, x0, y0, open_=False, mimic=0):
    """12x10 iron-bound dark-wood chest at (x0,y0) (lid top-left).  mimic: 0 no,
    1 closed-but-wrong, 2 open maw, 3 biting."""
    wood = ["#2a1a1c", "#4a2c26", "#6e4232", "#8c5a3e"]
    iron = ["stone_d", "stone", "stone_l", "stone_h"]
    if not open_:
        lid = m_rect(x0, y0, x0 + 11, y0 + 3) - {(x0, y0), (x0 + 11, y0)}
        shade_fill(sp, lid, wood[1], wood[2], wood[3])
        for x in range(x0, x0 + 12):
            sp.set(x, y0 + 3, wood[0])
    else:
        lid = m_rect(x0, y0 - 3, x0 + 11, y0 - 1) - {(x0, y0 - 3), (x0 + 11, y0 - 3)}
        shade_fill(sp, lid, wood[1], wood[2], wood[3])
        sp.rect(x0 + 1, y0, x0 + 10, y0 + 3, "ink")
        for x in range(x0, x0 + 12, 11):
            for y in range(y0, y0 + 4):
                sp.set(x, y, wood[1])
        if not mimic:
            # spilled glow from inside: aether shards
            for x in range(x0 + 2, x0 + 10):
                sp.set(x, y0 + 3, "teal" if x % 2 else "teal_l")
            sp.set(x0 + 4, y0 + 2, "aether")
            sp.set(x0 + 7, y0 + 2, "teal_l")
            sp.set(x0 + 6, y0 + 3, "aether")
    body = m_rect(x0, y0 + 4, x0 + 11, y0 + 9)
    shade_fill(sp, body, wood[0], wood[1], wood[2])
    for x in range(x0, x0 + 12):
        sp.set(x, y0 + 9, wood[0])
        if (x - x0) % 4 == 2:
            for y in range(y0 + 5, y0 + 9):
                sp.set(x, y, wood[0]) if (x - x0) not in (1, 10) else None
    for bx in (x0 + 1, x0 + 10):
        for y in range(y0 - (3 if open_ else 0), y0 + 10):
            if open_ and y0 <= y <= y0 + 3:
                continue
            sp.set(bx, y, iron[2] if y % 3 else iron[3])
    if not open_:
        sp.rect(x0 + 5, y0 + 3, x0 + 6, y0 + 5, "gold")
        sp.set(x0 + 5, y0 + 3, "gold_l")
        sp.set(x0 + 6, y0 + 5, "gold_d")
        if mimic == 1:
            sp.set(x0 + 5, y0 + 4, "#ff3a3a")
            sp.set(x0 + 6, y0 + 4, "#a01020")
            # teeth peeking under the lid
            for x in (x0 + 2, x0 + 4, x0 + 8, x0 + 9):
                sp.set(x, y0 + 4, "bone_l")
        else:
            sp.set(x0 + 5, y0 + 4, "ink")
            sp.set(x0 + 6, y0 + 4, "ink")


def ico_chest(sp, open_):
    draw_chest(sp, 2, 5 if not open_ else 6, open_)
    sp.outline()
    if open_:
        for (x, y, a) in [(5, 1, 220), (9, 2, 180), (7, 0, 140)]:
            sp.set(x, y, with_alpha("aether", a))


def draw_altar(sp, x0, y0, w=12):
    stone = ["stone_d", "stone", "stone_l", "stone_h"]
    slab = m_rect(x0, y0, x0 + w - 1, y0 + 1)
    shade_fill(sp, slab, stone[1], stone[2], stone[3])
    colm = m_rect(x0 + 2, y0 + 2, x0 + w - 3, y0 + 6)
    shade_fill(sp, colm, stone[0], stone[1], stone[2])
    base = m_rect(x0 + 1, y0 + 7, x0 + w - 2, y0 + 8)
    shade_fill(sp, base, stone[0], stone[1], stone[2])
    cx = x0 + w // 2
    # faint rune
    for (dx, dy, c) in [(-1, 3, "vio_l"), (0, 3, "lilac"), (-1, 4, "lilac"), (0, 5, "vio_l"), (-1, 5, "vio"), (0, 4, "vio")]:
        sp.set(cx + dx, y0 + dy, c)


def ico_altar(sp):
    draw_altar(sp, 2, 6)
    sp.outline()
    for (x, y, c) in [(7, 1, "lilac"), (8, 2, "magic"), (8, 0, with_alpha("lilac", 160)), (9, 1, "lilac"), (8, 1, "white")]:
        sp.set(x, y, c)


def draw_shrine(sp, x0, y0, w, h, glyph=True):
    """Ruined pointed-arch shrine with a cracked crown and glowing sigil."""
    cx = x0 + w / 2.0
    poly = [(x0, y0 + h), (x0, y0 + h * 0.35), (cx, y0), (x0 + w, y0 + h * 0.35), (x0 + w, y0 + h)]
    m = m_poly(poly)
    # broken top-right chunk
    m -= {(x, y) for (x, y) in m if x > cx + 1 and y < y0 + h * 0.3 + (x - cx) * 0.2}
    shade_fill(sp, m, "stone_d", "stone", "stone_l", hi="stone_h")
    for (x, y) in m:
        if (y - y0) % 4 == 3 and (x, y - 1) in m and (x, y + 1) in m and (x - 1, y) in m and (x + 1, y) in m:
            sp.set(x, y, "stone_d")
        if (y - y0) % 4 in (0, 1, 2) and (x - x0 + ((y - y0) // 4) * 3) % 6 == 0 and (x - 1, y) in m and (x + 1, y) in m and (x, y - 1) in m:
            sp.set(x, y, "stone_d")
    # crack
    for (x, y) in line_pts(x0 + 2, y0 + h * 0.45, x0 + 4, y0 + h * 0.7):
        if (x, y) in m:
            sp.set(x, y, "ink")
    nw = max(4, int(w * 0.5))
    nx0 = int(round(cx - nw / 2.0))
    ny0 = int(y0 + h * 0.3)
    ny1 = int(y0 + h * 0.78)
    niche = m_poly([(nx0, ny1), (nx0, ny0 + 2), (cx, ny0 - 1), (nx0 + nw, ny0 + 2), (nx0 + nw, ny1)])
    sp.fill(niche, "void")
    for (x, y) in niche:
        if (x, y - 1) not in niche or (x - 1, y) not in niche:
            sp.set(x, y, "ink")
    for x in range(x0 - 1, x0 + w + 1):
        sp.set(x, y0 + h, "stone")
    return niche, int(cx), (ny0 + ny1) // 2


def shrine_sigil(sp, niche, cx, cy, big=False):
    for (x, y) in niche:
        if sp.get(x, y) == col("void") and abs(x - cx) + abs(y - (cy - 1)) <= (4 if big else 3):
            sp.set(x, y, "vio_dk")
    pts = [(0, -2, "vio_l"), (-1, -1, "vio_l"), (0, -1, "magic"), (1, -1, "vio_l"), (0, 0, "lilac"), (0, 1, "vio_l")]
    if big:
        pts += [(-2, -1, "vio"), (2, -1, "vio"), (-1, 1, "vio"), (1, 1, "vio"), (0, 2, "vio"), (0, -3, "vio")]
    for (dx, dy, c) in pts:
        sp.set(cx + dx, cy + dy, c)


def ico_shrine(sp):
    niche, cx, cy = draw_shrine(sp, 3, 1, 10, 13)
    shrine_sigil(sp, niche, cx, cy)
    sp.outline()


REMAINS_ICON = [
    "....pppp........",
    "...pppppp.......",
    "...pkpkpp.......",
    "...pppppp.......",
    "....pkkp..xx....",
    ".....pp..xxxx...",
    "...xxxxxxxxxxx..",
    "..xxXxxxXxxxxxx.",
    ".xxXxxxxXxxxxxxx",
    ".xxxxxxxxxxxxxx.",
]


def ico_remains(sp):
    key = {"p": "bone", "k": "ink", "x": "vio_dk", "X": "vio"}
    draw_ascii(sp, REMAINS_ICON, 0, 4, key)
    # skull shading
    sp.set(4, 4, "bone_l")
    sp.set(5, 4, "bone_l")
    sp.set(9, 6, "bone_d")
    sp.set(8, 7, "bone_d")
    # bones poking out
    for (x, y) in [(12, 9), (13, 9), (14, 8)]:
        sp.set(x, y, "bone")
    sp.outline()


def ico_journal(sp):
    page = m_poly([(3, 2), (12, 1), (13, 13), (4, 14)])
    for (x, y) in page:
        sp.set(x, y, "bone" if (x + y) % 7 else "bone_l")
    for (x, y) in page:
        if (x + 1, y) not in page or (x, y + 1) not in page:
            sp.set(x, y, "bone_d")
    # curled corner
    sp.set(12, 12, "bone_d")
    sp.set(11, 13, "bone_l")
    # writing
    for row, (a, b) in enumerate([(5, 11), (5, 10), (5, 11), (5, 9), (6, 11), (5, 8)]):
        y = 4 + row * 1.5
        for x in range(a, b):
            if (x * 3 + row) % 4 != 0:
                sp.set(x, int(y + (x - 3) * -0.08 + 0.5), "#4a3040")
    sp.set(10, 12, "blood")
    sp.set(9, 12, "blood")
    sp.set(10, 11, "#8a2030")
    sp.outline()


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


def heart_icon(sp, kind):
    m = m_ascii(HEART_BIG, 3, 3)
    if kind == "empty":
        shade_fill(sp, m, "vio_dk", "vio", "vio_l")
    else:
        shade_fill(sp, m, "blood", "red_dk", "red", hi="salmon")
        sp.set(5, 5, "salmon")
        sp.set(5, 6, "red")
        if kind == "half":
            empty = {p for p in m if p[0] >= 8}
            shade_fill(sp, empty, "vio_dk", "vio", "vio_l")
            sp.fill({p for p in m if p[0] == 8}, "vio_dk")
    sp.outline()


def ico_lock(sp):
    sh = m_ellipse(8, 6, 3.6, 4.0) - m_ellipse(8, 6, 2.0, 2.6)
    sh = {p for p in sh if p[1] <= 7}
    shade_fill(sp, sh, "stone", "stone_l", "stone_h")
    body = m_rect(3, 7, 12, 14) - {(3, 14), (12, 14)}
    shade_fill(sp, body, "gold_d", "gold", "gold_l")
    for (x, y) in [(7, 9), (8, 9), (7, 10), (8, 10), (7, 11)]:
        sp.set(x, y, "ink")
    sp.set(8, 11, "gold_d")
    sp.outline()


def ico_flame(sp):
    draw_flame(sp, 8, 1, 14, 5.2, lean=1.5, cols=("red_dk", "rust", "orange", "yellow_o", "yellow"))
    sp.outline()


def ico_bubble(sp):
    m = m_ellipse(7.5, 8.5, 5.6, 5.6)
    for (x, y) in m:
        sp.set(x, y, with_alpha("blue_dk", 120))
    ring = m - m_ellipse(7.5, 8.5, 4.5, 4.5)
    for (x, y) in ring:
        sp.set(x, y, "cyan" if x + y < 17 else "blue")
    for (x, y) in [(5, 5), (4, 6), (6, 5), (4, 7)]:
        sp.set(x, y, "white")
    sp.outline()
    for (x, y) in m_ellipse(13.5, 3.5, 1.6, 1.6):
        sp.set(x, y, "cyan")
    sp.set(13, 3, "white")


def ico_thermo(sp):
    sp.fill(m_rect(7, 2, 8, 11), "bone_l")
    sp.set(8, 2, "bone")
    for y in range(6, 12):
        sp.set(7, y, "red")
        sp.set(8, y, "red_dk")
    shade_fill(sp, m_ellipse(8, 12.5, 2.6, 2.6), "red_dk", "red", "salmon", hi="white")
    sp.outline()
    for y in (3, 5, 7, 9):
        sp.set(10, y, "bone")


SKULL = ["..#####..", ".#######.", "#########", "#########", "#########", "#########", ".#######.", "..#####..", "..#.#.#.."]


def ico_skull(sp, x0=3, y0=3, eyes="ink"):
    m = m_ascii(SKULL, x0, y0)
    shade_fill(sp, m, "bone_d", "bone", "bone_l")
    for (x, y) in [(2, 4), (3, 4), (2, 5), (3, 5), (6, 4), (7, 4), (6, 5), (7, 5)]:
        sp.set(x0 + x, y0 + y, eyes)
    sp.set(x0 + 4, y0 + 6, "stone")
    sp.set(x0 + 5, y0 + 6, "stone")
    for x in (3, 5, 7):
        sp.set(x0 + x, y0 + 8, "bone_d")
    sp.outline()


def ico_rite_seal(sp):
    # wax seal with a rune and ribbon tails
    for (x, y) in [(5, 12), (4, 13), (4, 14), (10, 12), (11, 13), (11, 14)]:
        sp.set(x, y, "vio")
    m = m_ellipse(8, 7.5, 5.8, 5.8)
    shade_fill(sp, m, "gold_d", "gold", "gold_l", hi="#fff0b0")
    inner = m_ellipse(8, 7.5, 3.6, 3.6)
    for (x, y) in inner:
        if (x + 1, y + 1) not in inner:
            sp.set(x, y, "gold_l")
    for (x, y) in [(8, 5), (7, 6), (9, 6), (8, 7), (8, 8), (7, 9), (9, 9), (8, 10)]:
        sp.set(x, y, "gold_d")
    sp.outline()


def ico_staff(sp):
    for (x, y) in line_pts(3, 14, 11, 6):
        sp.set(x, y, "wood_l")
        sp.set(x + 1, y, "wood")
    sp.set(10, 4, "wood_l")
    sp.set(13, 7, "wood")
    for (x, y) in m_poly([(12, 1), (14, 3), (12, 6), (10, 4)]):
        sp.set(x, y, "lilac")
    sp.set(11, 3, "white")
    sp.set(12, 2, "magic")
    sp.set(13, 4, "vio_l")
    sp.outline()
    for (x, y, a) in [(15, 1, 160), (14, 0, 120), (9, 1, 120)]:
        sp.set(x, y, with_alpha("magic", a))


def ico_question(sp):
    m = m_ascii(["..####..", ".######.", "##....##", "......##", "....###.", "...###..", "...##...", "........", "...##...", "...##..."], 4, 3)
    shade_fill(sp, m, "bone_d", "bone", "bone_l")
    sp.outline()


def ico_compass(sp):
    rim = m_ellipse(8, 8.5, 6.2, 6.2)
    shade_fill(sp, rim, "gold_d", "gold", "gold_l")
    face = m_ellipse(8, 8.5, 4.8, 4.8)
    sp.fill(face, "void")
    for (x, y) in face:
        if (x - 1, y) not in face or (x, y - 1) not in face:
            sp.set(x, y, "ink")
    sp.set(8, 2, "gold_d")
    sp.set(7, 2, "gold_d")
    for (x, y) in [(10, 6), (11, 5), (9, 7)]:
        sp.set(x, y, "aether")
    for (x, y) in [(7, 9), (6, 10), (5, 11)]:
        sp.set(x, y, "stone_l")
    sp.set(8, 8, "white")
    sp.outline()


def ico_hourglass(sp):
    for x in range(3, 13):
        sp.set(x, 1, "wood_l")
        sp.set(x, 2, "wood")
        sp.set(x, 13, "wood_l")
        sp.set(x, 14, "wood")
    glass = m_poly([(4, 3), (12, 3), (12, 4), (8.5, 8), (12, 12), (12, 13), (4, 13), (4, 12), (7.5, 8), (4, 4)])
    for (x, y) in glass:
        sp.set(x, y, with_alpha("stone_h", 120))
    for (x, y) in glass:
        if y >= 10:
            sp.set(x, y, "bone" if y > 10 else "bone_l")
        if 4 <= y <= 5:
            sp.set(x, y, "bone")
    for y in (7, 8, 9):
        sp.set(8, y, "bone_l")
    sp.set(5, 4, "white")
    sp.outline()


def ico_mana(sp):
    star = m_poly([(8, 1), (10, 6), (15, 8), (10, 10), (8, 15), (6, 10), (1, 8), (6, 6)])
    shade_fill(sp, star, "vio", "vio_l", "lilac", hi="magic")
    for (x, y) in m_ellipse(8, 8, 2.2, 2.2):
        sp.set(x, y, "magic")
    sp.set(7, 7, "white")
    sp.outline()


def ico_closed_eye(sp):
    rows = [
        "................",
        "................",
        "................",
        "................",
        ".LL..........LL.",
        "..LL........LL..",
        "...LLL....LLL...",
        "....DLLLLLLD....",
        ".....DDDDDD.....",
        "...v..v..v..v...",
        "..v...v..v...v..",
        "................",
    ]
    draw_ascii(sp, rows, 0, 1, {"L": "lilac", "D": "vio_l", "v": "vio_l"})
    sp.outline()


def ico_reticle(sp):
    ring = m_ellipse(8, 8, 6.0, 6.0) - m_ellipse(8, 8, 4.9, 4.9)
    for (x, y) in ring:
        if not (abs(x + 0.5 - 8) < 1.6 or abs(y + 0.5 - 8) < 1.6):
            sp.set(x, y, "lilac")
    for k in range(3):
        sp.set(7 + 0, 1 + k, "magic")
        sp.set(8, 1 + k, "magic")
        sp.set(7, 12 + k, "magic")
        sp.set(8, 12 + k, "magic")
        sp.set(1 + k, 7, "magic")
        sp.set(1 + k, 8, "magic")
        sp.set(12 + k, 7, "magic")
        sp.set(12 + k, 8, "magic")
    sp.set(7, 7, "white")
    sp.set(8, 8, "white")
    sp.set(7, 8, "lilac")
    sp.set(8, 7, "lilac")
    sp.outline()


def ico_beam(sp):
    # staff tip on the left firing a crackling beam to the right into rock
    for (x, y) in m_rect(12, 3, 15, 13):
        sp.set(x, y, "stone" if (x + y) % 3 else "stone_l")
    for x in range(3, 13):
        y = 8 + (1 if (x * 5) % 7 == 0 else (-1 if (x * 3) % 5 == 0 else 0))
        sp.set(x, y, "magic")
        sp.set(x, 8, "white")
        sp.set(x, 7, with_alpha("lilac", 200)) if sp.get(x, 7)[3] == 0 else None
        sp.set(x, 9, with_alpha("vio_l", 200)) if sp.get(x, 9)[3] == 0 else None
    for (x, y) in [(1, 7), (2, 8), (1, 9), (0, 8)]:
        sp.set(x, y, "lilac")
    sp.set(1, 8, "white")
    for (x, y) in [(12, 6), (12, 10), (11, 5), (11, 11), (13, 8)]:
        sp.set(x, y, "white")
    sp.outline()


def ico_mimic(sp):
    draw_chest(sp, 2, 6, True, mimic=2)
    # maw: teeth on lid edge and box edge, tongue
    for x in range(3, 13, 2):
        sp.set(x, 5, "bone_l")
        sp.set(x + 1, 6, "bone_l")
    for x in range(4, 12):
        sp.set(x, 8, "blood")
    sp.set(6, 8, "#c03050")
    sp.set(7, 8, "#e05070")
    sp.set(8, 7, "#c03050")
    sp.set(5, 7, "#ff3a3a")
    sp.set(10, 7, "#ff3a3a")
    sp.outline()


def ico_stalker_eye(sp):
    m = m_poly([(1, 8), (5, 4), (11, 4), (15, 8), (11, 12), (5, 12)])
    shade_fill(sp, m, "ink", "void", "vio_dk")
    iris = m_ellipse(8, 8, 3.2, 3.2)
    for (x, y) in iris:
        sp.set(x, y, "vio")
    for (x, y) in m_ellipse(8, 8, 1.6, 2.6):
        sp.set(x, y, "ink")
    sp.set(8, 7, "#ff5a6a")
    sp.set(7, 6, "magic")
    sp.outline("blood")
    for (x, y) in [(3, 2), (13, 2), (2, 13), (13, 14), (8, 1)]:
        sp.set(x, y, "blood")


def ico_rune_circle(sp):
    ring = m_ellipse(8, 8, 7.0, 7.0) - m_ellipse(8, 8, 5.9, 5.9)
    for (x, y) in ring:
        sp.set(x, y, "vio_l")
    inner = m_ellipse(8, 8, 3.6, 3.6) - m_ellipse(8, 8, 2.6, 2.6)
    for (x, y) in inner:
        sp.set(x, y, "lilac")
    for a in range(6):
        t = a * math.pi / 3 - math.pi / 2
        x, y = 8 + math.cos(t) * 4.9 - 0.5, 8 + math.sin(t) * 4.9 - 0.5
        sp.set(round(x), round(y), "magic")
    tri = [(8 + math.cos(a * 2 * math.pi / 3 - math.pi / 2) * 5.6, 8 + math.sin(a * 2 * math.pi / 3 - math.pi / 2) * 5.6) for a in range(3)]
    for i in range(3):
        for (x, y) in line_pts(tri[i][0] - 0.5, tri[i][1] - 0.5, tri[(i + 1) % 3][0] - 0.5, tri[(i + 1) % 3][1] - 0.5):
            if sp.get(x, y)[3] == 0:
                sp.set(x, y, "vio")
    sp.set(7, 7, "white")
    sp.set(8, 8, "magic")
    sp.outline()


# starting-choice emblems: framed heraldic plaques
def plaque(sp, accent):
    m = m_poly([(2, 1), (14, 1), (14, 9), (8, 15), (2, 9)])
    shade_fill(sp, m, "vio_dk", "void", "vio")
    edge = {p for p in m if any((p[0] + dx, p[1] + dy) not in m for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))}
    for (x, y) in edge:
        sp.set(x, y, "gold_l" if (x + y) < 12 else ("gold" if x < 9 else "gold_d"))
    sp.outline()
    return m


def ico_initiate(sp):
    plaque(sp, "lilac")
    # three light orbs
    for (cx, cy) in [(6, 6), (10, 6), (8, 10)]:
        sp.set(cx, cy, "white")
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            sp.set(cx + dx, cy + dy, "lilac")


def ico_prodigy(sp):
    plaque(sp, "gold")
    star = m_poly([(8, 2.5), (9.3, 6), (12.5, 6.2), (10, 8.3), (11, 11.5), (8, 9.6), (5, 11.5), (6, 8.3), (3.5, 6.2), (6.7, 6)])
    for (x, y) in star:
        sp.set(x, y, "gold_l" if x < 8 else "gold")
    sp.set(7, 6, "white")


def ico_scavenger(sp):
    plaque(sp, "aether")
    for (x, y) in m_poly([(6, 3), (8, 5), (7, 10), (5, 10), (4, 6)]):
        sp.set(x, y, "teal_l")
    for (x, y) in m_poly([(10, 5), (12, 7), (11, 11), (9, 11), (8.5, 8)]):
        sp.set(x, y, "teal")
    sp.set(5, 5, "aether")
    sp.set(6, 4, "white")
    sp.set(10, 6, "aether")


def ico_heir(sp):
    plaque(sp, "gold")
    # crown above a staff
    for (x, y) in [(5, 3), (8, 2), (11, 3)]:
        sp.set(x, y, "gold_l")
    for x in range(5, 12):
        sp.set(x, 4, "gold")
        sp.set(x, 5, "gold_d" if x % 2 else "gold")
    for y in range(6, 13):
        sp.set(8, y, "wood_l")
    sp.set(8, 6, "lilac")
    sp.set(7, 6, "magic")
    sp.set(9, 6, "vio_l")


def ico_twin(sp):
    plaque(sp, "lilac")
    # two interlocking half-moons in two hues
    for (x, y) in m_ellipse(6.5, 7, 3.2, 3.2) - m_ellipse(7.8, 7, 2.4, 2.4):
        sp.set(x, y, "#ea6a26")
    for (x, y) in m_ellipse(9.5, 8, 3.2, 3.2) - m_ellipse(8.2, 8, 2.4, 2.4):
        sp.set(x, y, "#3a98dc")
    sp.set(8, 7, "white")


def gen_icons():
    fns = {}
    for i in range(8):
        fns[i] = lambda s, i=i: ico_sigil(s, i)
    for i in range(8, 45):
        fns[i] = lambda s, i=i: ico_scroll(s, i)
    for k in range(28):
        fns[45 + k] = lambda s, k=k: ico_fusion(s, k)
    misc = [ico_shard, ico_light_orb, lambda s: ico_chest(s, False), lambda s: ico_chest(s, True), ico_altar,
            ico_shrine, ico_remains, ico_journal, lambda s: heart_icon(s, "full"), lambda s: heart_icon(s, "half"),
            lambda s: heart_icon(s, "empty"), ico_lock, ico_flame, ico_bubble, ico_thermo, ico_skull, ico_rite_seal,
            ico_staff, ico_question, ico_compass, ico_hourglass, ico_mana, ico_closed_eye, ico_reticle, ico_beam,
            ico_mimic, ico_stalker_eye, ico_rune_circle, ico_initiate, ico_prodigy, ico_scavenger, ico_heir, ico_twin]
    for j, fn in enumerate(misc):
        fns[73 + j] = fn
    assert max(fns) == 105, max(fns)
    out = Sprite(256, 128)
    for i, fn in fns.items():
        s = Sprite(16, 16)
        fn(s)
        out.blit(s, (i % 16) * 16, (i // 16) * 16)
    return out

# --------------------------------------------------------------------------
# Props (world scale, 16x24 frames, 8 cols x 2 rows, bottom aligned on y=23)
# --------------------------------------------------------------------------
def light_orb(sp, cx, cy, r, halo, phase, core="white", ramp=("lilac", "magic", "#f3efff", "white")):
    """Floating ball of pale light: soft halo (alpha) + shaded core, no outline.
    (cx, cy) is the centre pixel."""
    fx, fy = cx + 0.5, cy + 0.5
    for y in range(int(fy - halo) - 1, int(fy + halo) + 2):
        for x in range(int(fx - halo) - 1, int(fx + halo) + 2):
            d = math.hypot(x + 0.5 - fx, y + 0.5 - fy)
            if r < d <= halo:
                t = 1 - (d - r) / (halo - r)
                a = int(150 * t * t + 18)
                if bayer(x + phase, y) < t * 1.2:
                    sp.blend(x, y, with_alpha(ramp[1], a))
    for (x, y) in m_ellipse(fx, fy, r, r):
        d = math.hypot(x + 0.5 - fx + 0.8, y + 0.5 - fy + 0.8) / (r + 0.8)
        sp.set(x, y, ramp_soft(list(reversed(ramp)), d * 1.15, x, y))
    sp.set(cx - 1, cy - 1, core)
    sp.set(cx, cy - 1, core)
    sp.set(cx - 1, cy, core)


def shard_crystal(sp, cx, cy, bright=0):
    """~5x7 glowing aether crystal centred on pixel (cx, cy)."""
    m = m_ascii(["..#..", ".###.", "#####", "#####", "#####", ".###.", "..#.."], cx - 2, cy - 3)
    for (x, y) in m:
        sp.set(x, y, "teal_l" if x <= cx else "teal")
    for (x, y) in m:
        if x < cx and y <= cy:
            sp.set(x, y, "aether")
    for y in range(cy - 2, cy + 3):
        sp.set(cx, y, "#d8fffb" if y <= cy else "teal_l")
    sp.set(cx - 1, cy - 1, "white" if bright else "aether")
    sp.set(cx + 1, cy + 2, "teal_d")
    sp.outline("ink")


REMAINS = [
    "....xxx.........",
    "...xXXXx........",
    "..xXhbbBx.......",
    "..xXbkbkb.......",
    ".xXhxbdb........",
    ".xXhXxXbb.......",
    "xXhXXXXxbdb.....",
    "xXXXxXXXXx.bb...",
    "xxXXXxXXXXx.wwwc",
    ".xxx.xxxx.xx....",
]
REMAINS_SEARCHED = [
    "................",
    "................",
    "................",
    "................",
    "...........bBb..",
    "..........bkbkb.",
    "...........bdb..",
    ".b.....xx.......",
    "bd..xXXhXx...bdb",
    "xxxxXXxXXXxx.n..",
]
REMAINS_KEY = {"x": "vio_dk", "X": "vio", "h": "vio_l", "n": "wood", "b": "bone", "B": "bone_l", "d": "bone_d",
               "k": "ink", "w": "wood", "c": "stone_l"}


def prop_remains(searched=False):
    s = Sprite(16, 24)
    rows = REMAINS_SEARCHED if searched else REMAINS
    draw_ascii(s, rows, 0, 14, REMAINS_KEY)
    if searched:
        # torn robe scrap and a cracked, dark crystal
        s.set(4, 21, "vio_l")
        s.set(15, 23, "stone")
    s.outline()
    return s


def prop_journal():
    s = Sprite(16, 24)
    rows = [".BBBBB",
            "BkkBkkB",
            "BBkkkBb",
            ".bbBBbd"]
    draw_ascii(s, rows, 5, 20, {"b": "bone", "B": "bone_l", "k": "#7a6050", "d": "bone_d"})
    s.outline()
    return s


def prop_mimic(state):
    """state 0 closed (subtly wrong), 1 gaping maw, 2 biting."""
    s = Sprite(16, 24)
    if state == 0:
        draw_chest(s, 2, 14, False, mimic=1)
        s.outline()
        return s
    wood = ["#2a1a1c", "#4a2c26", "#6e4232", "#8c5a3e"]
    if state == 1:
        # lid flung up, fangs, dark maw with eyes, tongue lolling over the front
        lid = m_rect(2, 8, 13, 10) - {(2, 8), (13, 8)}
        shade_fill(s, lid, wood[1], wood[2], wood[3])
        for x in (3, 12):
            for y in range(8, 11):
                s.set(x, y, "stone_l")
        maw = m_rect(2, 11, 13, 17)
        for (x, y) in maw:
            edge = x in (2, 13) or y in (11, 17)
            s.set(x, y, "blood" if edge else "#2a0610")
        for x in (3, 6, 9, 12):
            s.set(x, 11, "bone_l")
            s.set(x, 12, "bone")
        for x in (4, 8, 11):
            s.set(x, 11, "bone")
        for x in (4, 7, 10, 13):
            s.set(x, 17, "bone_l")
            s.set(x, 16, "bone")
        for x in (5, 9):
            s.set(x, 17, "bone")
        body = m_rect(2, 18, 13, 23)
        shade_fill(s, body, wood[0], wood[1], wood[2])
        for bx in (3, 12):
            for y in range(18, 24):
                s.set(bx, y, "stone_l" if y % 3 else "stone_h")
        for (x, y, c) in [(7, 15, "#8a1830"), (8, 15, "#c03050"), (8, 16, "#e05070"), (8, 17, "#e05070"),
                          (9, 18, "#c03050"), (9, 19, "#e05070"), (10, 20, "#c03050"), (9, 20, "#8a1830")]:
            s.set(x, y, c)
        s.set(5, 13, "#ff3a3a")
        s.set(10, 13, "#ff3a3a")
        s.set(5, 14, "#6a0a14")
        s.set(10, 14, "#6a0a14")
        s.outline()
        return s
    # biting: lunging forward/up, lid slammed, teeth interlocked
    lid = m_rect(3, 12, 14, 15) - {(3, 12), (14, 12)}
    shade_fill(s, lid, wood[1], wood[2], wood[3])
    for x in range(3, 15):
        s.set(x, 16, "bone_l" if x % 2 else "ink")
        s.set(x, 17, "ink" if x % 2 else "bone_l")
    body = m_rect(2, 18, 13, 22)
    shade_fill(s, body, wood[0], wood[1], wood[2])
    for bx in (4, 13):
        for y in range(12, 16):
            s.set(bx, y, "stone_l")
    for bx in (3, 12):
        for y in range(18, 23):
            s.set(bx, y, "stone_l" if y % 3 else "stone_h")
    s.set(9, 14, "#ff3a3a")
    s.set(10, 14, "#a01020")
    # little stubby legs mid-hop
    for (x, y) in [(4, 23), (11, 23)]:
        s.set(x, y, wood[0])
    s.outline()
    return s


def prop_rune_stone():
    s = Sprite(16, 24)
    m = m_poly([(4, 23), (4.5, 14), (6, 11), (9, 10.5), (11.5, 12.5), (12, 23)])
    shade_fill(s, m, "stone_d", "stone", "stone_l", hi="stone_h")
    for (x, y) in m:
        if (x * 5 + y * 3) % 11 == 0 and (x + 1, y) in m and (x - 1, y) in m:
            s.set(x, y, "stone_d")
    # glowing glyph
    for (x, y, c) in [(8, 13, "aether"), (7, 14, "teal_l"), (8, 14, "aether"), (9, 14, "teal_l"), (8, 15, "aether"),
                      (8, 16, "teal_l"), (7, 17, "teal_l"), (9, 17, "teal_l"), (8, 18, "aether"), (8, 19, "teal_l")]:
        s.set(x, y, c)
    # moss at the base
    for x in range(4, 12):
        if x % 3:
            s.set(x, 23, "sick")
    s.set(5, 22, "sick_d")
    s.outline()
    for (x, y, a) in [(8, 11, 90), (6, 16, 70), (10, 16, 70)]:
        if s.get(x, y)[3] == 0:
            s.set(x, y, with_alpha("aether", a))
    return s


def gen_props():
    frames = {}
    for i in range(4):
        s = Sprite(16, 24)
        r = [3.5, 3.3, 3.6, 3.4][i]
        halo = [7.0, 6.2, 7.6, 6.6][i]
        light_orb(s, 8, 12, r, halo, i)
        # flickering sparks around it
        for (x, y) in [[(3, 8), (13, 15)], [(12, 7)], [(4, 16), (13, 9)], [(5, 6)]][i]:
            s.set(x, y, with_alpha("magic", 200))
        frames[(i, 0)] = s
    s = Sprite(16, 24)
    draw_chest(s, 2, 14)
    s.outline()
    frames[(4, 0)] = s
    s = Sprite(16, 24)
    draw_chest(s, 2, 14, open_=True)
    s.outline()
    for (x, y, a) in [(5, 9, 200), (10, 8, 160), (8, 6, 120)]:
        s.set(x, y, with_alpha("aether", a))
    frames[(5, 0)] = s
    s = Sprite(16, 24)
    draw_altar(s, 1, 15, 14)
    s.outline()
    for (x, y, a) in [(7, 14, 90), (8, 14, 120), (9, 14, 90), (8, 13, 60)]:
        s.blend(x, y, with_alpha("lilac", a))
    frames[(6, 0)] = s
    s = Sprite(16, 24)
    niche, cx, cy = draw_shrine(s, 1, 2, 14, 21)
    shrine_sigil(s, niche, cx, cy, big=True)
    s.outline()
    # rubble at the foot
    for (x, y, c) in [(0, 23, "stone"), (15, 23, "stone_l"), (14, 22, "stone")]:
        s.set(x, y, c)
    frames[(7, 0)] = s
    frames[(0, 1)] = prop_remains(False)
    frames[(1, 1)] = prop_remains(True)
    frames[(2, 1)] = prop_journal()
    frames[(3, 1)] = prop_mimic(0)
    frames[(4, 1)] = prop_mimic(1)
    frames[(5, 1)] = prop_mimic(2)
    s = Sprite(16, 24)
    for (x, y, a) in [(8, 7, 120), (8, 17, 90), (4, 12, 90), (12, 12, 90), (5, 9, 60), (11, 15, 60)]:
        s.set(x, y, with_alpha("aether", a))
    shard_crystal(s, 8, 12, 1)
    frames[(6, 1)] = s
    frames[(7, 1)] = prop_rune_stone()
    return sheet(8, 2, 16, 24, frames)

# --------------------------------------------------------------------------
# Spells (8x8 frames, 8 cols x 10 rows)
# --------------------------------------------------------------------------
def spark_px(sp, x, y, c, a=255):
    if sp.inb(int(x), int(y)):
        if a >= 255:
            sp.set(x, y, c)
        else:
            sp.blend(int(x), int(y), with_alpha(c, a))


def proj_energy(sp, ramp, f, tail_len=4, wob=1.0, rng=None):
    """Glowing bolt: bright core at the right, flickering tail to the left."""
    dark, mid, light, glow = ramp
    cx, cy = 5, 3
    for k in range(tail_len, 0, -1):
        x = cx - k
        off = int(round(math.sin(f * 1.7 + k * 1.3) * wob * (k / tail_len)))
        a = int(255 * (1 - k / (tail_len + 1.5)))
        spark_px(sp, x, cy + off, mid if k > 2 else light, max(80, a))
        if k <= 2:
            spark_px(sp, x, cy + 1 + off, mid, max(80, a - 40))
    for (x, y) in m_ellipse(cx + 0.5, cy + 1.0, 2.3, 2.1):
        sp.set(x, y, mid)
    for (x, y) in m_ellipse(cx + 0.8, cy + 0.8, 1.5, 1.3):
        sp.set(x, y, light)
    sp.set(cx, cy, glow)
    sp.set(cx + 1, cy, glow)
    sp.set(cx + 1, cy + 1, light if f % 2 else glow)


def impact_burst(sp, ramp, f, rays=8, solid=False, seed=0):
    """Expanding burst: ring + flying particles that fade out over 4 frames."""
    dark, mid, light, glow = ramp
    rng = random.Random(seed)
    c = 3.5
    R = [1.2, 2.2, 3.0, 3.6][f]
    alpha = [255, 235, 180, 110][f]
    if f == 0:
        for (x, y) in m_ellipse(c + 0.5, c + 0.5, 2.0, 2.0):
            sp.set(x, y, light)
        for (x, y) in m_ellipse(c + 0.5, c + 0.5, 1.0, 1.0):
            sp.set(x, y, glow)
        return
    ring = m_ellipse(c + 0.5, c + 0.5, R, R) - m_ellipse(c + 0.5, c + 0.5, R - 1.0, R - 1.0)
    for (x, y) in ring:
        spark_px(sp, x, y, light if f == 1 else mid, alpha)
    if f == 1:
        for (x, y) in m_ellipse(c + 0.5, c + 0.5, 1.0, 1.0):
            sp.set(x, y, glow)
    for k in range(rays):
        t = 2 * math.pi * k / rays + rng.uniform(-0.3, 0.3)
        r = R + 0.8 + rng.uniform(0, 0.6)
        x, y = c + 0.5 + math.cos(t) * r, c + 0.5 + math.sin(t) * r
        spark_px(sp, int(x), int(y), glow if f < 3 else light, alpha)


def gen_spells():
    frames = {}
    for school in range(8):
        ramp = [col(c) for c in SCHOOL_RAMP[school]]
        dark, mid, light, glow = ramp
        for f in range(4):
            s = Sprite(8, 8)
            if school == 0:      # Pyro: fire bolt with licking tail
                proj_energy(s, ramp, f, 4, 1.2)
                for (x, y) in [(1, 2 + f % 2), (0, 4 - f % 2)]:
                    spark_px(s, x, y, "yellow_o", 140)
            elif school == 1:    # Hydro: water orb with droplets
                for (x, y) in m_ellipse(5, 4, 2.4, 2.4):
                    s.set(x, y, mid)
                for (x, y) in m_ellipse(5.3, 3.7, 1.6, 1.4):
                    s.set(x, y, light)
                s.set(4, 3, glow)
                s.outline(dark)
                for k, (x, y) in enumerate([(1, 3 + (f % 2)), (0, 5 - (f % 2)), (2, 5 + (f // 2) % 2)]):
                    spark_px(s, x, y, light if k == 0 else mid, 220 - 50 * k)
            elif school == 2:    # Terra: tumbling rock chunk
                shapes = [[".##.", "####", "###.", ".##."], [".##.", "####", ".###", ".##."],
                          ["..#.", ".###", "####", ".##."], [".#..", "###.", "####", ".##."]]
                m = m_ascii(shapes[f], 3, 2)
                shade_fill(s, m, dark, mid, light, hi=glow)
                s.outline("ink")
                spark_px(s, 0, 3 + f % 2, mid, 150)
                spark_px(s, 1, 5 - f % 2, light, 120)
            elif school == 3:    # Cryo: ice shard pointing right with sparkle trail
                m = m_ascii(["..#...", ".####.", "######", ".####.", "..#..."], 1, 1) - {(1, 3)}
                m = {(x, y) for (x, y) in m}
                shard = m_ascii(["...#....", ".#####..", "#######.", ".#####..", "...#...."], 0, 1)
                shard = {(x, y) for (x, y) in shard if x >= 1}
                for (x, y) in shard:
                    s.set(x, y, light if y <= 3 else mid)
                s.set(7, 3, glow)
                s.set(6, 3, glow)
                s.set(4, 2, glow)
                s.outline(dark)
                spark_px(s, [0, 1, 0, 1][f], [1, 5, 4, 1][f], glow, 200)
            elif school == 4:    # Aero: wind crescent with streaks
                arc = m_ellipse(3.5, 4, 3.4, 3.4) - m_ellipse(2.4, 4, 3.0, 3.0)
                for (x, y) in arc:
                    s.set(x, y, light if y < 4 else mid)
                for (x, y) in [(5, 2), (6, 4), (5, 5)]:
                    s.set(x, y, glow)
                for k in range(3):
                    x0 = (f + k * 2) % 4
                    spark_px(s, x0, 2 + k * 2, mid, 150)
                    spark_px(s, x0 + 1, 2 + k * 2, mid, 110)
            elif school == 5:    # Fulm: crackling spark ball with forks
                proj_energy(s, [dark, mid, light, glow], f, 3, 0.0)
                zig = [[(1, 2), (2, 3), (1, 4)], [(2, 2), (1, 3), (2, 4), (1, 5)], [(1, 1), (2, 2), (1, 3)], [(2, 4), (1, 5), (0, 4)]][f]
                for (x, y) in zig:
                    s.set(x, y, light)
                fork = [(7, 1), (7, 6), (6, 0), (7, 5)][f]
                s.set(fork[0], fork[1], glow)
            elif school == 6:    # Alchemy: tumbling flask
                flask = m_ascii(["..#.", ".##.", "####", "####", ".##."], 2, 1)
                ang = [0, 90, 180, 270][f]
                fs = Sprite(8, 8)
                for (x, y) in flask:
                    fs.set(x, y, mid if y >= 3 else (180, 200, 190, 200))
                fs.set(3, 4, glow)
                fs.set(3, 1, "bone")
                fs = rotate_sprite(fs, ang, 4, 4)
                fs.outline("ink")
                s.blit(fs, 0, 0)
                spark_px(s, 0, 2 + f % 3, light, 160)
            else:                # Myco: spore pod with drifting spores
                for (x, y) in m_ellipse(5, 4, 2.2, 2.0):
                    s.set(x, y, mid)
                for (x, y) in m_ellipse(5.4, 3.6, 1.2, 1.0):
                    s.set(x, y, light)
                s.set(5, 3, glow)
                s.outline(dark)
                for k, (x, y) in enumerate([(1, 2 + f % 2), (2, 6 - f % 2), (0, 4), (3, 1 + (f + 1) % 2)]):
                    spark_px(s, x, y, light if k % 2 else mid, 210 - 40 * k)
            frames[(f, school)] = s
        # impact bursts
        for f in range(4):
            s = Sprite(8, 8)
            if school == 2:
                # debris flying out
                rng = random.Random(42)
                for k in range(6):
                    t = 2 * math.pi * k / 6 + 0.4
                    r = [0.5, 1.6, 2.6, 3.3][f]
                    x, y = 3.5 + math.cos(t) * r, 3.5 + math.sin(t) * r + [0, 0, 0.6, 1.4][f]
                    s.set(int(x), int(y), [light, mid, dark][k % 3] if f < 3 else mid)
                    if f < 2:
                        s.set(int(x) + 1, int(y), mid)
                if f == 0:
                    for (x, y) in m_rect(2, 2, 5, 5):
                        s.set(x, y, mid)
                    s.set(3, 3, light)
                s.outline("ink") if f < 3 else None
            elif school == 3:
                impact_burst(s, ramp, f, rays=6, seed=3)
                for k in range(4):
                    t = math.pi / 4 + k * math.pi / 2
                    for r in range(1, [1, 3, 4, 4][f] + 1):
                        spark_px(s, int(3.5 + math.cos(t) * r + 0.5), int(3.5 + math.sin(t) * r + 0.5), glow if r < 3 else light, [255, 255, 200, 120][f])
            elif school == 5:
                impact_burst(s, ramp, f, rays=5, seed=5)
                if f in (1, 2):
                    for (x, y) in [(3, 0), (4, 1), (3, 2), (5, 5), (6, 6), (1, 5), (0, 6)]:
                        spark_px(s, x, y, glow, 230)
            elif school == 6:
                impact_burst(s, ramp, f, rays=6, seed=6)
                for k, (x, y) in enumerate([(2, 6), (5, 6), (4, 5)]):
                    if f >= 1:
                        spark_px(s, x, y - (f - 1), light, 220 - 50 * f)
            elif school == 7:
                # spore puff cloud
                for k in range(7):
                    t = 2 * math.pi * k / 7
                    r = [0.6, 1.6, 2.4, 3.0][f]
                    x, y = 3.5 + math.cos(t) * r, 3.5 + math.sin(t) * r - f * 0.3
                    spark_px(s, int(x), int(y), mid if k % 2 else light, [255, 230, 170, 100][f])
                    if f < 2:
                        spark_px(s, int(x) + 1, int(y), mid, 160)
                if f < 2:
                    s.set(3, 3, glow)
                    s.set(4, 4, light)
            else:
                impact_burst(s, ramp, f, rays=8 if school != 4 else 4, seed=school)
            frames[(4 + f, school)] = s
    # row 8: light mote + horizontally tileable dig beam
    for f in range(4):
        s = Sprite(8, 8)
        r = [1, 2, 1, 0][f]
        s.set(3, 3, "white")
        s.set(4, 4, "magic") if f != 3 else None
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            if r >= 1:
                spark_px(s, 3 + dx, 3 + dy, "lilac", 200 if r == 2 else 130)
        if r == 2:
            for dx, dy in ((2, 0), (-2, 0), (0, 2), (0, -2)):
                spark_px(s, 3 + dx, 3 + dy, "lilac", 90)
        frames[(f, 8)] = s
        b = Sprite(8, 8)
        for x in range(8):
            w1 = math.sin(2 * math.pi * (x / 8.0) + f * 1.6)
            w2 = math.sin(2 * math.pi * (2 * x / 8.0) - f * 2.1)
            for y in range(8):
                d = abs(y + 0.5 - 4.0)
                if 1.0 < d < 3.6:
                    b.set(x, y, with_alpha("vio_l", int(85 * (1 - (d - 1.0) / 2.6) * (0.75 + 0.25 * w1))))
            b.set(x, 3, "white")
            b.set(x, 4, "magic")
            b.set(x, 2, with_alpha("lilac", 150))
            b.set(x, 5, with_alpha("lilac", 120))
            # crackling filament wandering around the core
            y2 = int(round(3.5 + 2.4 * w1 * (0.6 + 0.4 * w2)))
            if y2 not in (3, 4):
                b.set(x, y2, with_alpha("magic", 235))
        frames[(4 + f, 8)] = b
    # row 9: shard sparkle + rune glyph flicker
    for f in range(4):
        s = Sprite(8, 8)
        L = [1, 2, 3, 1][f]
        s.set(3, 3, "white" if f in (1, 2) else "aether")
        for k in range(1, L + 1):
            a = 255 if k == 1 else (170 if k == 2 else 90)
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                spark_px(s, 3 + dx * k, 3 + dy * k, "aether" if k == 1 else "teal_l", a)
        if f == 2:
            for dx, dy in ((1, 1), (-1, 1), (1, -1), (-1, -1)):
                spark_px(s, 3 + dx, 3 + dy, "teal_l", 120)
        frames[(f, 9)] = s
        g = Sprite(8, 8)
        glyph = ["..#..", ".#.#.", "#.#.#", "..#..", ".###.", "..#..", ".#.#."]
        lvl = [0.55, 1.0, 0.8, 0.35][f]
        cmain = mix("vio", "magic", lvl)
        for y, row in enumerate(glyph):
            for x, ch in enumerate(row):
                if ch == "#":
                    g.set(1 + x, y, cmain)
        if f == 1:
            for (x, y) in g.points():
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if g.get(x + dx, y + dy)[3] == 0:
                        g.set(x + dx, y + dy, with_alpha("vio_l", 90))
        frames[(4 + f, 9)] = g
    return sheet(8, 10, 8, 8, frames)

# --------------------------------------------------------------------------
# Creatures (16x16 frames, 8 cols x 10 rows), facing RIGHT
# --------------------------------------------------------------------------
def ascii_frame(rows, key, oy=0, ox=0, outline=True, w=16, h=16):
    s = Sprite(w, h)
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            c = key.get(ch)
            if c is not None:
                s.set(ox + x, oy + y, c)
    if outline:
        s.outline()
    return s


def shift_rows(rows, dy):
    """Shift an ASCII frame vertically (positive = down) inside 16 rows."""
    blank = "." * len(rows[0])
    if dy > 0:
        return [blank] * dy + rows[:-dy]
    if dy < 0:
        return rows[-dy:] + [blank] * (-dy)
    return rows


GNAW_KEY = {"a": "#7a6466", "P": "#b9a7a0", "L": "#dccfc6", "p": "#8f7a78", "d": "#5c4652",
            "k": "#2a0f18", "w": "bone_l", "l": "#6e5a5e"}
GNAW_BODY = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "......aaa.......",
    "....aaLLLaa.....",
    "...aLLPPPPPaLL..",
    "..aLPPPPPPPPPPLL",
    ".aPPPpPPPpPPkwkw",
    ".dpPdpPdpPdPkkkk",
    "................",
    "................",
]
GNAW_LEGS = [
    ["..l...l...l..l..", ".l...l...l...l.."],
    ["...l...l...l.l..", "..l...l...l..l.."],
    ["..l...l...l..l..", "...l...l...l..l."],
    [".l...l...l...l..", "..l...l...l...l."],
]


def creature_gnawling(f):
    body = list(GNAW_BODY)
    bob = 1 if f % 2 else 0
    body = shift_rows(body, -bob) if bob else body
    rows = body[:14] + GNAW_LEGS[f]
    if bob:
        rows = body[:13] + ["................"] + GNAW_LEGS[f]
    # mouth opens/closes with the gait
    s = ascii_frame(rows, GNAW_KEY)
    if f in (1, 2):
        y = 12 - bob
        s.set(15, y - 1, "bone_l")
    return s


HOLLOW_KEY = {"a": "#2c2636", "b": "#3e3650", "c": "#564c6a", "d": "#776c8c", "k": "#0c0810",
              "e": "#b8ff9a", "E": "#5a8a4a", "h": "#9a9088", "n": "wood", "N": "wood_l", "f": "#4a4240"}
HOLLOW_BASE = [
    "................",
    "................",
    "....d...........",
    "....cd..........",
    "....bcd.........",
    "....bccd........",
    "...bbcckk.......",
    "...bcckeke......",
    "...bbcckkk......",
    "..bbccccchh.....",
    "..bccbcbc.n.....",
    "..bcbcbcb.N.....",
    "..bbcbcbbbn.....",
    "...cb.bcb.n.....",
    "...b..b.b.......",
    "................",
]


def creature_hollow(f, cast=False):
    rows = [list(r) for r in HOLLOW_BASE]
    if not cast:
        bob = [0, 1, 0, 1][f]
        feet = [("ff", 4, "ff", 8), ("ff", 5, "ff", 7), ("ff", 4, "ff", 8), ("ff", 3, "ff", 8)][f]
        rows = [list(r) for r in shift_rows(["".join(r) for r in rows], bob)]
        rows[15] = list("................")
        rows[15][feet[1]] = "f"
        rows[15][feet[1] + 1] = "f"
        rows[15][feet[3]] = "f"
        rows[15][feet[3] + 1] = "f"
        if f % 2:
            rows[14] = list("..b.b..b.b......")
        s = ascii_frame(["".join(r) for r in rows], HOLLOW_KEY)
        return s
    # cast: raise the broken staff, sickly glow gathers
    rows = [list(r) for r in HOLLOW_BASE]
    rows[15] = list("....ff..ff......")
    for y in range(9, 14):
        for x in range(9, 12):
            if rows[y][x] in "hnN":
                rows[y][x] = "."
    hand = [(10, 8), (11, 6), (11, 5), (11, 7)][f]
    rows[hand[1]][hand[0]] = "h"
    for k in range(1, 4):
        y = hand[1] - k
        if 0 <= y < 16:
            rows[y][hand[0] + (1 if k == 3 else 0)] = "n" if k != 2 else "N"
    for y in range(hand[1] + 1, min(16, hand[1] + 3)):
        if rows[y][hand[0]] == ".":
            rows[y][hand[0]] = "n"
    rows[9][9] = "c"
    s = ascii_frame(["".join(r) for r in rows], HOLLOW_KEY)
    tip = (hand[0] + 1, hand[1] - 4)
    lvl = [1, 2, 3, 2][f]
    s.set(tip[0], tip[1], "#e8ffd8" if lvl >= 2 else "#b8ff9a")
    for dx, dy in ((1, 0), (-1, 0), (0, -1), (0, 1)):
        x, y = tip[0] + dx, tip[1] + dy
        if s.inb(x, y) and (s.get(x, y)[3] == 0 or s.get(x, y) == col(OUTLINE)):
            s.set(x, y, with_alpha("#8ad070", 110 + 45 * lvl))
    if lvl == 3:
        for dx, dy in ((1, 1), (-1, 1), (1, -1), (-1, -1)):
            x, y = tip[0] + dx, tip[1] + dy
            if s.inb(x, y) and s.get(x, y)[3] == 0:
                s.set(x, y, with_alpha("#5a8a4a", 120))
    if lvl == 3:
        s.set(8, 7, "white")
    return s


WYRM_KEY = {"a": "#3a2c40", "P": "#9a8890", "L": "#c4b4b4", "p": "#6c5a6a", "d": "#4e3e52",
            "k": "#1e0a14", "r": "#5a1424", "w": "bone_l", "W": "bone"}


def creature_wyrm_head(f):
    o = [0, 1, 2, 1][f]   # maw opening
    rows = [
        "................",
        "................",
        "................",
        ".....aaaa.......",
        "...aaLLLLaa.....",
        "..aLLPPPPPLa....",
        ".aLPPpPPPPPPa...",
        ".aPPPPPPPPPPPa..",
        "aLPpPPPPPPPPPPa.",
        "aPPPPPPPPPPPPPa.",
        "aPPpPPPPPPPPPa..",
        ".aPPPPPPPPPPPa..",
        ".adpPPpPPPPda...",
        "..addPdddda.....",
        "....aaaaa.......",
        "................",
    ]
    s = ascii_frame(rows, WYRM_KEY, outline=False)
    # ridged plates across the head
    for x in (4, 7, 10):
        for y in range(4, 13):
            if s.get(x, y) in (col(WYRM_KEY["P"]), col(WYRM_KEY["L"])):
                s.set(x, y, WYRM_KEY["p"] if y > 6 else WYRM_KEY["P"])
    # maw: a ring of teeth around a dark throat on the right
    cy = 9
    h = 1 + o
    for y in range(cy - h, cy + h + 1):
        for x in range(11, 15):
            if abs(y - cy) <= h - (1 if x == 14 else 0):
                s.set(x, y, "#1e0a14" if x > 11 else "#5a1424")
    for y in range(cy - h, cy + h + 1, 1):
        s.set(15 if abs(y - cy) < h else 14, y, "bone_l" if (y + f) % 2 else "bone")
    s.set(12, cy - h, "bone_l")
    s.set(12, cy + h, "bone")
    s.set(13, cy - h - 1, "bone") if o else None
    s.outline()
    return s


def creature_wyrm_segment(f):
    s = Sprite(16, 16)
    m = m_ellipse(8, 8.5, 5.0, 5.0)
    for (x, y) in m:
        d = math.hypot(x + 0.5 - 6.8, y + 0.5 - 6.8) / 6.5
        s.set(x, y, ramp_soft(["#c4b4b4", "#9a8890", "#6c5a6a", "#4e3e52"], d, x, y))
    for (x, y) in m:
        if (x + f * 2) % 4 == 0:
            s.set(x, y, mix(s.get(x, y), "#3a2c40", 0.55))
    # bristles on the back
    for x in range(5, 12, 2):
        s.set(x + f % 2, 3, "#6c5a6a")
    s.outline()
    return s


def creature_wyrm_tail(f):
    s = Sprite(16, 16)
    sw = [0, 1][f]
    pts = set()
    for x in range(3, 13):
        t = (x - 3) / 9.0
        r = 1.0 + 3.6 * t
        cy = 8.5 + sw * (1 - t) * 1.5
        for y in range(16):
            if abs(y + 0.5 - cy) <= r:
                pts.add((x, y))
    for (x, y) in pts:
        top = (x, y - 1) not in pts
        bot = (x, y + 1) not in pts
        s.set(x, y, "#c4b4b4" if top else ("#4e3e52" if bot else "#9a8890"))
        if (x + f) % 3 == 0 and not top and not bot:
            s.set(x, y, "#6c5a6a")
    s.outline()
    return s


SPUP_KEY = {"a": "#26303a", "b": "#3c4a4a", "c": "#56685e", "s": "#7e8a76", "k": "#0c0a10",
            "m": "mag", "M": "mag_l", "q": "#8a2a6a", "t": "teal_l", "T": "aether", "f": "#2e2a30"}
SPUP_BASE = [
    "................",
    "....mMm.........",
    "...mMMMm..mm....",
    "....qsq..mMMm...",
    "....cssc..q.....",
    "....cTks........",
    "....cssc........",
    "...bbccbb.......",
    "..bccTcccs......",
    "..bc.bccb.s.....",
    "..c..bcTb.......",
    ".....bccb.......",
    ".....b..b.......",
    ".....b..b.......",
    "................",
    "................",
]


def creature_spore_puppet(f, burst=False):
    if not burst:
        bob = [0, 1, 0, 1][f]
        rows = shift_rows(list(SPUP_BASE), 1 + bob)
        rows = list(rows)
        legs = [("....b...b.......", "....f...ff......"), (".....b.b........", ".....ff.f......."),
                ("......b..b......", "......f..ff....."), (".....b.b........", ".....f.ff.......")][f]
        rows[14] = legs[0]
        rows[15] = legs[1]
        s = ascii_frame(rows, SPUP_KEY)
        # drifting spores
        for (x, y) in [[(12, 2)], [(13, 1)], [(12, 0), (2, 3)], [(14, 2)]][f]:
            s.set(x, y, with_alpha("mag_l", 200))
        return s
    # burst: swell, glow, pop, cloud
    if f == 0:
        rows = shift_rows(list(SPUP_BASE), 1)
        rows = list(rows)
        rows[14] = "....b...b......."
        rows[15] = "....f...ff......"
        s = ascii_frame(rows, SPUP_KEY)
        for (x, y) in s.points():
            p = s.get(x, y)
            if p == col(SPUP_KEY["c"]) and (x + y) % 3 == 0:
                s.set(x, y, "teal_l")
        return s
    if f == 1:
        s = Sprite(16, 16)
        for (x, y) in m_ellipse(7.5, 9, 5.0, 5.4):
            s.set(x, y, "#56685e")
        for (x, y) in m_ellipse(7, 8.5, 3.6, 3.8):
            s.set(x, y, "teal_l" if (x + y) % 2 else "#7e8a76")
        for (x, y) in [(5, 6), (9, 7), (7, 11), (10, 10), (4, 9)]:
            s.set(x, y, "aether")
        for (x, y) in [(6, 3), (7, 3), (8, 3), (7, 2)]:
            s.set(x, y, "mag")
        s.outline()
        return s
    s = Sprite(16, 16)
    rng = random.Random(70 + f)
    R = [0, 0, 4.5, 6.5][f]
    for k in range(26):
        t = rng.uniform(0, 2 * math.pi)
        r = rng.uniform(0.3, 1.0) * R
        x, y = 7.5 + math.cos(t) * r, 9 + math.sin(t) * r * 0.85
        c = rng.choice(["mag", "mag_l", "teal_l", "aether"])
        s.set(int(x), int(y), with_alpha(c, 255 if f == 2 else 150))
    if f == 2:
        for (x, y) in m_ellipse(7.5, 9, 1.6, 1.6):
            s.set(x, y, "aether")
        s.set(7, 8, "white")
    return s


def creature_drifter(f):
    s = Sprite(16, 16)
    bob = [0, -1, 0, 1][f]
    cy = 7.0 + bob
    puff = m_ellipse(8, cy, 4.6, 4.0)
    for (x, y) in puff:
        d = math.hypot(x + 0.5 - 6.5, y + 0.5 - (cy - 1.5)) / 6.0
        s.set(x, y, ramp_soft(["#3fb0a4", "teal_l", "teal", "teal_d"], d, x, y))
    for (x, y) in [(6, int(cy) - 2), (10, int(cy) - 1), (8, int(cy) + 1), (5, int(cy) + 1)]:
        s.set(x, y, "mag")
    s.set(6, int(cy) - 3, "mag_l")
    s.set(9, int(cy) - 2, "mag_l")
    s.outline()
    for k, x in enumerate((5, 7, 9, 11)):
        y0 = int(cy + 4.5)
        sw = 1 if (k + f) % 2 else 0
        s.set(x, y0, "teal")
        s.set(x + sw - (1 - sw) * 0, y0 + 1, "teal_d")
        if k % 2 == 0:
            s.set(x + sw, y0 + 2, with_alpha("teal_d", 160))
    for (x, y) in [[(2, 3), (13, 4)], [(2, 4), (13, 2)], [(3, 2), (14, 3)], [(2, 3), (12, 1)]][f]:
        s.set(x, y, with_alpha("mag_l", 210))
    return s


def creature_wraith(f):
    s = Sprite(16, 16)
    bob = [0, -1, -1, 0][f]
    body = m_poly([(8, 2 + bob), (12, 6 + bob), (12, 10 + bob), (10, 14), (8, 12), (6, 15), (4, 11 + bob), (4, 6 + bob)])
    # wavy tattered bottom varies per frame
    for (x, y) in body:
        if y >= 12 + bob and (x + f) % 3 == 0:
            continue
        d = math.hypot(x + 0.5 - 6.5, y + 0.5 - (5 + bob)) / 9.0
        s.set(x, y, ramp_soft(["#4a3a3e", "#33262c", "#22181e", "#150e14"], d, x, y))
    # ember cracks
    for (x, y) in [(6, 8), (7, 9), (7, 10), (10, 7), (9, 11), (5, 10)]:
        if s.get(x, y + bob)[3]:
            s.set(x, y + bob, "orange" if (x + f) % 2 else "rust")
    # eyes
    s.set(8, 5 + bob, "yellow")
    s.set(10, 5 + bob, "yellow_o")
    s.outline("ink")
    # flame wisps on the crown
    wisps = [[(7, 1), (9, 0), (10, 2)], [(8, 0), (10, 1), (6, 1)], [(7, 0), (9, 1), (11, 3)], [(8, 1), (10, 0), (6, 2)]][f]
    for (x, y) in wisps:
        if s.get(x, y + bob)[3] == 0:
            s.set(x, y + bob, "orange")
        if s.get(x, y + bob + 1)[3] == 0 or s.get(x, y + bob + 1) == col("ink"):
            s.set(x, y + bob + 1, "red_dk")
    for (x, y) in [[(3, 13)], [(12, 14)], [(5, 15), (13, 12)], [(2, 12)]][f]:
        s.set(x, y, with_alpha("orange", 180))
    return s


def creature_lightseeker(f):
    """Pale moth-eel: eel body, moth wings with eye-spots, a cluster of eyes."""
    s = Sprite(16, 16)
    wings = [
        [(5, 8), (4, 1), (9, 0), (11, 3), (10, 8)],
        [(5, 8), (2, 4), (8, 3), (12, 5), (10, 8)],
        [(5, 8), (3, 14), (9, 15), (11, 12), (10, 8)],
        [(5, 8), (1, 7), (7, 6), (12, 7), (10, 9)],
    ][f]
    wm = m_poly(wings)
    for (x, y) in wm:
        edge = any((x + dx, y + dy) not in wm for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        s.set(x, y, "#8e857c" if edge else ("#d9d2c6" if (x + 2 * y) % 5 else "#b3aa9e"))
    # eye-spot on the wing
    pts = sorted(wm)
    if pts:
        cx = sum(p[0] for p in pts) // len(pts)
        cy = sum(p[1] for p in pts) // len(pts)
        if f == 2:
            cy += 1
        if f != 3:
            s.set(cx, cy, "#7a1a2a")
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                if (cx + dx, cy + dy) in wm:
                    s.set(cx + dx, cy + dy, "ink")
    body = set()
    for x in range(1, 13):
        t = x / 12.0
        r = 0.6 + 1.7 * t
        cy = 8.5 + math.sin(t * 5 + f * 1.6) * 0.8 * (1 - t)
        for y in range(16):
            if abs(y + 0.5 - cy) <= r:
                body.add((x, y))
    body |= m_ellipse(12.5, 8.5, 2.6, 2.5)
    for (x, y) in body:
        top = (x, y - 1) not in body
        bot = (x, y + 1) not in body
        s.set(x, y, "#eee8dc" if top else ("#8a8078" if bot else "#cbc3b6"))
        if not top and not bot and x % 3 == 0 and x < 11:
            s.set(x, y, "#a89e92")
    for (x, y) in [(11, 7), (13, 7), (12, 8), (14, 8), (11, 9), (13, 9), (12, 10)]:
        s.set(x, y, "ink")
    for (x, y) in [(13, 7), (12, 10)]:
        s.set(x, y, "#ff6a5a" if (f + x) % 2 else "#a01a2a")
    s.outline()
    return s


def creature_mimic(f, bite=False):
    s = Sprite(16, 16)
    wood = ["#2a1a1c", "#4a2c26", "#6e4232", "#8c5a3e"]
    if not bite:
        # hop: crouch, launch, airborne, land
        dy = [0, -2, -4, 0][f]
        sq = [0, -1, 0, 1][f]           # squash: wider & shorter
        x0, x1 = 2 - max(0, sq), 13 + max(0, sq)
        ytop = 6 + dy + max(0, sq)
        gap = 1 if f in (1, 2) else 0
        lid = m_rect(x0, ytop, x1, ytop + 2)
        shade_fill(s, lid, wood[1], wood[2], wood[3])
        if gap:
            for x in range(x0 + 1, x1):
                s.set(x, ytop + 3, "bone_l" if x % 2 else "#2a0610")
        body = m_rect(x0, ytop + 3 + gap, x1, ytop + 7 + gap)
        shade_fill(s, body, wood[0], wood[1], wood[2])
        for bx in (x0 + 1, x1 - 1):
            for y in range(ytop, ytop + 8 + gap):
                if s.get(bx, y)[3] and s.get(bx, y) != col("bone_l"):
                    s.set(bx, y, "stone_l" if y % 3 else "stone_h")
        s.set(8, ytop + 2, "gold")
        s.set(8, ytop + 3 + gap, "#ff3a3a")
        ly = ytop + 8 + gap
        legs = [[(4, 15), (11, 15)], [(3, ly), (3, ly + 1), (12, ly), (12, ly + 1)],
                [(4, ly), (11, ly)], [(3, 15), (12, 15)]][f]
        for (x, y) in legs:
            if y < 16:
                s.set(x, y, wood[0])
        s.outline()
        return s
    # bite: open wide, snap
    op = [1, 3, 4, 0][f]
    ytop = 7 - op
    lid = m_rect(2, ytop, 13, ytop + 2)
    shade_fill(s, lid, wood[1], wood[2], wood[3])
    for bx in (3, 12):
        for y in range(ytop, ytop + 3):
            s.set(bx, y, "stone_l")
    if op:
        maw = m_rect(2, ytop + 3, 13, 9)
        for (x, y) in maw:
            s.set(x, y, "#2a0610")
        for x in range(3, 13, 2):
            s.set(x, ytop + 3, "bone_l")
            s.set(x + 1, 9, "bone_l")
        if op >= 3:
            s.set(5, ytop + 4, "#ff3a3a")
            s.set(10, ytop + 4, "#ff3a3a")
            for (x, y) in [(8, 8), (9, 9), (10, 10)]:
                s.set(x, y, "#e05070")
    else:
        for x in range(2, 14):
            s.set(x, 9, "bone_l" if x % 2 else "ink")
    body = m_rect(2, 10, 13, 14)
    shade_fill(s, body, wood[0], wood[1], wood[2])
    for bx in (3, 12):
        for y in range(10, 15):
            s.set(bx, y, "stone_l" if y % 3 else "stone_h")
    if op >= 3:
        s.set(10, 10, "#e05070")
        s.set(10, 11, "#c03050")
    s.set(4, 15, wood[0])
    s.set(11, 15, wood[0])
    s.outline()
    return s


def gen_creatures():
    F = {}
    for f in range(4):
        F[(f, 0)] = creature_gnawling(f)
        F[(f, 1)] = creature_hollow(f)
        F[(4 + f, 1)] = creature_hollow(f, cast=True)
        F[(f, 2)] = creature_wyrm_head(f)
        F[(f, 3)] = creature_spore_puppet(f)
        F[(4 + f, 3)] = creature_spore_puppet(f, burst=True)
        F[(f, 4)] = creature_drifter(f)
        F[(f, 5)] = creature_wraith(f)
        F[(f, 6)] = creature_lightseeker(f)
        F[(f, 7)] = creature_mimic(f)
        F[(4 + f, 7)] = creature_mimic(f, bite=True)
    for f in range(2):
        F[(4 + f, 2)] = creature_wyrm_segment(f)
        F[(6 + f, 2)] = creature_wyrm_tail(f)
    return sheet(8, 10, 16, 16, F)


# --------------------------------------------------------------------------
# Hollow Stalker (16x32 frames: 0-5 walk, 6-7 idle)
# --------------------------------------------------------------------------
def thick_line(pts_set, a, b, r):
    (x0, y0), (x1, y1) = a, b
    n = int(max(abs(x1 - x0), abs(y1 - y0)) * 2) + 1
    for i in range(n + 1):
        t = i / float(n)
        x, y = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
        for yy in range(int(y - r) - 1, int(y + r) + 2):
            for xx in range(int(x - r) - 1, int(x + r) + 2):
                if (xx + 0.5 - x) ** 2 + (yy + 0.5 - y) ** 2 <= r * r:
                    pts_set.add((xx, yy))


def stalker_frame(f):
    """Side view, facing right: thin hunched figure with arms past the knees."""
    walk = f < 6
    ph = 2 * math.pi * f / 6 if walk else 0.0
    bob = (0 if f % 3 == 0 else -1) if walk else 0
    tilt = [0, 0, 0, 0, 0, 0, 1, 2][f]
    near, far = set(), set()
    hx, hy = 10.0 + tilt * 0.4, 5.5 + bob + tilt * 0.3
    head = m_ellipse(hx, hy, 2.3, 3.0)
    near |= head
    thick_line(near, (9.4, hy + 2.5), (8.8, 10.0 + bob), 0.7)
    torso = m_poly([(5.8, 10.5 + bob), (10.2, 10 + bob), (10.0, 13 + bob), (9.2, 19 + bob), (6.4, 19 + bob), (5.8, 14 + bob)])
    near |= torso
    hip = (7.9, 19.0 + bob)
    if walk:
        s1 = math.sin(ph)
        legs = []
        for k, sg in ((0, 1), (1, -1)):
            fx = 8 + 3.6 * s1 * sg
            lift = max(0.0, math.cos(ph) * sg) * 1.5
            kx = 8 + 2.2 * s1 * sg + 1.0
            ky = 25.5 + bob - lift * 0.6
            legs.append(((kx, ky), (fx, 31 - round(lift))))
    else:
        legs = [((8.6, 25.5), (8.5, 31)), ((7.4, 25.5), (6.8, 31))]
    for i, (k, ft) in enumerate(legs):
        tgt = near if i == 0 else far
        thick_line(tgt, hip, k, 0.8)
        thick_line(tgt, k, ft, 0.65)
        tgt.add((int(round(ft[0])) + 1, int(ft[1])))
    sw = math.sin(ph + math.pi) if walk else 0.0
    for i, sg in enumerate((1, -1)):
        tgt = near if i == 0 else far
        sh = (9.6 if i == 0 else 7.0, 11.0 + bob)
        el = (sh[0] + 1.6 + sw * sg * 1.2, 17.5 + bob)
        hd = (sh[0] + 2.0 + sw * sg * 2.6, 24.5 + bob + (1 if (f == 7 and i == 0) else 0))
        thick_line(tgt, sh, el, 0.5)
        thick_line(tgt, el, hd, 0.45)
        fx_, fy_ = int(round(hd[0])), int(round(hd[1]))
        for (dx, dy) in ((0, 1), (1, 1), (0, 2), (1, 3), (-1, 2), (0, 3)):
            tgt.add((fx_ + dx, fy_ + dy))
    s = Sprite(16, 32)
    for (x, y) in far:
        if s.inb(x, y):
            s.set(x, y, "#05040a")
    for (x, y) in near:
        if s.inb(x, y):
            s.set(x, y, "#0a0710")
    allp = near | far
    for (x, y) in near:
        if not s.inb(x, y):
            continue
        if (x - 1, y) not in near or (x, y - 1) not in near:
            s.set(x, y, "#2e1c44" if y > 12 + bob else "#46306a")
    for (x, y) in far:
        if s.inb(x, y) and (x, y) not in near and ((x - 1, y) not in allp):
            s.set(x, y, "#1a1028")
    # two pinprick eyes
    ex, ey = int(round(hx + 1.0)), int(round(hy - 0.3))
    s.set(ex, ey, "#f2ecff")
    s.set(ex - 2, ey + (1 if tilt == 2 else 0), "#b8a8dc")
    s.outline("ink")
    return s


def gen_stalker():
    return sheet(8, 1, 16, 32, {(f, 0): stalker_frame(f) for f in range(8)})

# --------------------------------------------------------------------------
# The Trial (core.png): beating heart of light in a rotating rune ring
# --------------------------------------------------------------------------
CORE_RUNES = [
    ["#.#", ".#.", "#.#"], ["###", "#..", "###"], [".#.", "###", ".#."], ["#..", "###", "..#"],
    ["##.", "#.#", ".##"], ["#.#", "###", "#.#"], [".#.", "#.#", ".#."], ["###", ".#.", "#.."],
    ["#.#", "#.#", "###"], ["..#", ".#.", "###"], ["##.", ".##", "##."], [".##", "#..", "##."],
]


def gen_core():
    rng = random.Random(77)
    n1 = FBM(rng, 4, 3, 0.55, size=16)
    ramp = ["#3a1e0a", "gold_d", "gold", "gold_l", "#fff0b4", "white"]
    beat = [0.0, 1.6, 0.6, 1.3, 0.2, -0.3, -0.5, -0.3]
    frames = {}
    cx = cy = 24.0
    for f in range(8):
        s = Sprite(48, 48)
        ph = 2 * math.pi * f / 8
        R = 11.5 + beat[f]
        glow = 0.5 + 0.5 * max(0.0, beat[f]) / 1.6
        # outer halo
        for y in range(48):
            for x in range(48):
                d = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
                if R < d < R + 7:
                    t = 1 - (d - R) / 7.0
                    a = int((40 + 70 * glow) * t * t)
                    if a > 6 and bayer(x, y) < 0.35 + 0.65 * t:
                        s.set(x, y, with_alpha("gold_l", a))
        # rune ring (rotates one rune-spacing per 8 frames -> seamless loop)
        rot = (2 * math.pi / len(CORE_RUNES)) * (f / 8.0)
        for r_ in (17.6, 22.4):
            for k in range(720):
                t = 2 * math.pi * k / 720
                x, y = cx + math.cos(t) * r_, cy + math.sin(t) * r_
                lit = math.cos(t + 2.3) * 0.5 + 0.5
                s.set(int(x), int(y), mix("gold_d", "gold", lit) if r_ < 20 else with_alpha(mix("#4a3014", "gold_d", lit), 230))
        for i, g in enumerate(CORE_RUNES):
            t = rot + 2 * math.pi * i / len(CORE_RUNES)
            gx, gy = cx + math.cos(t) * 20.0, cy + math.sin(t) * 20.0
            bright = 0.55 + 0.45 * math.sin(ph + i * 1.7)
            c = mix("gold", "#fff2c0", bright)
            for yy, row in enumerate(g):
                for xx, ch in enumerate(row):
                    if ch == "#":
                        s.set(int(gx) - 1 + xx, int(gy) - 1 + yy, c)
        # the heart: molten swirling sphere with dark veins
        for y in range(48):
            for x in range(48):
                dx, dy = x + 0.5 - cx, y + 0.5 - cy
                d = math.hypot(dx, dy) / R
                if d > 1.0:
                    continue
                depth = math.asin(min(1.0, d)) / (math.pi / 2)
                au = (math.atan2(dy, dx) / (2 * math.pi)) % 1.0
                u = au * 16 + depth * 4.0 + 1.2 * math.cos(ph)
                v = depth * 8.0 + 1.2 * math.sin(ph)
                tex = n1(u % 16, v % 16)
                vein = 1 - abs(2 * n1((u * 1.3 + 5) % 16, (v * 1.1 + 9) % 16) - 1)
                heat = 1.0 - depth ** 1.4
                lightdir = max(0.0, -(dx + dy) / (R * 1.6))
                t = 0.08 + 0.74 * heat + 0.35 * (tex - 0.5) + 0.12 * lightdir + 0.1 * glow
                if vein > 0.93 and 0.2 < depth < 0.95:
                    t -= 0.28
                if d > 0.92:
                    t = min(t, 0.35)
                s.set(x, y, ramp_soft(ramp, t, x, y))
        # rim outline of the heart
        heart = {(x, y) for y in range(48) for x in range(48) if math.hypot(x + 0.5 - cx, y + 0.5 - cy) <= R}
        for (x, y) in heart:
            if any((x + dx, y + dy) not in heart for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                s.set(x, y, "#5a3a10" if x + y > 48 else "gold")
        frames[(f, 0)] = s
    return sheet(8, 1, 48, 48, frames)

# --------------------------------------------------------------------------
# Backgrounds (256x256, seamless in x and y; far opaque & dark, near alpha)
# --------------------------------------------------------------------------
S = 256


def wrapped(sp):
    sp.wrap = True
    return sp


def finish_near(sp, rim, shade):
    """Rim-light top-left edges and shade bottom-right edges of silhouettes."""
    src = sp.copy()
    src.wrap = True
    for y in range(S):
        for x in range(S):
            p = src.get(x, y)
            if not p[3]:
                continue
            if not src.opaque(x - 1, y) or not src.opaque(x, y - 1):
                sp.set(x, y, with_alpha(rim, p[3]))
            elif not src.opaque(x + 1, y) or not src.opaque(x, y + 1):
                sp.set(x, y, with_alpha(shade, p[3]))


def stamp(sp, rows, ox, oy, key, blend_a=None):
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            c = key.get(ch)
            if c is None:
                continue
            if blend_a is not None:
                q = sp.get(ox + x, oy + y)
                sp.set(ox + x, oy + y, mix(q, c, blend_a))
            else:
                sp.set(ox + x, oy + y, c)


CARVED_FACE = [
    "...#####...",
    "..#.....#..",
    ".#.......#.",
    ".#.##.##.#.",
    ".#.##.##.#.",
    ".#.......#.",
    ".#...#...#.",
    ".#.......#.",
    "..#.###.#..",
    "...#...#...",
    "....###....",
]
BONE_SHAPES = [
    ["##.....##", ".#######.", "##.....##"],
    ["#.", "##", ".#", ".#", ".#", ".#", "##", "#."],
    ["..###..", ".#####.", "##.#.##", ".#####.", "..#.#.."],
    ["#.#.#.#", "#######", ".#.#.#."],
]


def bg_layer0():
    """The Whispering Crust: strata, roots, buried bones, faint carved faces."""
    rng = random.Random(2000)
    far = wrapped(Sprite(S, S))
    warp = FBM(rng, 4, 3)
    tex = FBM(rng, 8, 4)
    big = FBM(rng, 2, 2)
    ramp = ["#120d10", "#181115", "#1f161a", "#271c1f", "#2f2224"]
    for y in range(S):
        for x in range(S):
            w = warp(x, y)
            st = 0.5 + 0.5 * math.sin(2 * math.pi * 6 * (y + 36 * w) / S)
            v = 0.5 * tex(x, y) + 0.22 * st + 0.3 * big(x, y) - 0.12
            far.set(x, y, ramp_soft(ramp, v * 1.1, x, y))
    # buried bones
    for _ in range(26):
        b = rng.choice(BONE_SHAPES)
        stamp(far, b, rng.randrange(S), rng.randrange(S), {"#": "#5a4c46"}, 0.4)
    # faint carved faces watching from the strata
    for i in range(3):
        ox, oy = rng.randrange(S), rng.randrange(S)
        stamp(far, CARVED_FACE, ox, oy, {"#": "#3e302e"}, 0.6)
        stamp(far, CARVED_FACE, ox + 1, oy + 1, {"#": "#0a0708"}, 0.5)
    # thin far roots
    for _ in range(18):
        x, y = rng.random() * S, rng.random() * S
        for _i in range(rng.randint(20, 60)):
            far.set(x, y, mix(far.get(int(x), int(y)), "#0a0607", 0.55))
            x += rng.uniform(-0.8, 0.8)
            y += 1
    near = wrapped(Sprite(S, S))
    shape = FBM(rng, 3, 4, 0.55)
    tex2 = FBM(rng, 16, 2)
    for y in range(S):
        for x in range(S):
            colm = 0.5 + 0.5 * math.cos(2 * math.pi * 2 * (x + 25 * shape(y * 0.5, x)) / S)
            v = shape(x, y) * 0.8 + 0.24 * colm
            if v > 0.64:
                near.set(x, y, with_alpha(ramp_pick(["#21171a", "#281c1e", "#2f2123"], tex2(x, y), x, y), 235))
    # hanging roots (gnarled, branching)
    for _ in range(30):
        x, y = rng.random() * S, rng.random() * S
        thick = rng.choice([1, 1, 2, 2, 3])
        for i in range(rng.randint(40, 110)):
            for k in range(max(1, thick - i // 40)):
                near.set(x + k, y, ("#2c1e1c", 230))
            if rng.random() < 0.05:
                bx, by_ = x, y
                for _j in range(rng.randint(5, 16)):
                    bx += rng.choice([-1, 1]) * 0.8
                    by_ += 1
                    near.set(bx, by_, ("#261a19", 210))
            x += rng.uniform(-0.6, 0.6)
            y += 1
    finish_near(near, "#4a3632", "#120c0c")
    # a few pale bones caught in the near rock
    for _ in range(8):
        b = rng.choice(BONE_SHAPES)
        ox, oy = rng.randrange(S), rng.randrange(S)
        for yy, r in enumerate(b):
            for xx, ch in enumerate(r):
                if ch == "#" and near.opaque(ox + xx, oy + yy):
                    near.set(ox + xx, oy + yy, ("#6e625a", 235))
    return far, near


def bg_layer1():
    """The Drowned Halls: flooded arcades, columns, drips, cold teal."""
    rng = random.Random(2001)
    far = wrapped(Sprite(S, S))
    tex = FBM(rng, 8, 3)
    big = FBM(rng, 2, 2)
    ramp = ["#081316", "#0b1a1e", "#0f2226", "#132a2f", "#173237"]
    for y in range(S):
        for x in range(S):
            # masonry courses
            row = y // 8
            bx = (x + (row % 2) * 8) % 16
            mortar = (y % 8 == 7) or bx == 15
            v = 0.45 * tex(x, y) + 0.35 * big(x, y) + 0.1
            c = ramp_soft(ramp, v, x, y)
            if mortar:
                c = mix(c, "#04090b", 0.6)
            far.set(x, y, c)
    # arcade of arches (period 64 in x, 128 in y)
    for ay in (0, 128):
        for ax in range(0, S, 64):
            cx = ax + 32
            for y in range(ay + 30, ay + 128):
                for x in range(ax + 8, ax + 57):
                    dx = x + 0.5 - cx
                    top = ay + 30 + 24 - math.sqrt(max(0.0, 24 * 24 - dx * dx))
                    if y >= top and abs(dx) < 24:
                        q = far.get(x, y)
                        depth = min(1.0, (y - top) / 60.0)
                        far.set(x, y, mix(q, "#04080a", 0.55 + 0.2 * depth))
                        if abs(abs(dx) - 23.5) < 1 or abs(y - top) < 1:
                            far.set(x, y, mix(q, "#1d4246", 0.5))
    # waterline and ripples
    for wy in (100, 228):
        for x in range(S):
            for y in range(wy, wy + 28):
                q = far.get(x, y)
                k = (y - wy) / 28.0
                c = mix(q, "#0d3a40", 0.35 * (1 - k))
                if (y - wy) % 5 == 0 and (x + y * 3) % 11 < 6:
                    c = mix(c, "#2a6a6c", 0.35 * (1 - k))
                far.set(x, y, c)
    # drips
    for _ in range(40):
        x, y = rng.randrange(S), rng.randrange(S)
        for k in range(rng.randint(3, 10)):
            far.set(x, y + k, mix(far.get(x, y + k), "#3f8c8c", 0.35 * (1 - k / 10.0)))
    near = wrapped(Sprite(S, S))
    # columns (period 128) with capitals, broken at varying heights
    for cxi, cx in enumerate((20, 148)):
        w = 13
        for y in range(S):
            for x in range(cx - w // 2 - 3, cx + w // 2 + 4):
                dx = x - cx
                ycap = (y % 128)
                wid = w // 2 + (3 if ycap < 6 or 122 <= ycap else (1 if ycap < 9 or ycap >= 119 else 0))
                if abs(dx) <= wid:
                    flute = (dx + 6) % 4 == 0
                    c = ramp_pick(["#163034", "#11262a", "#0c1c20"], (dx + wid) / (2.0 * wid), x, y)
                    if flute and abs(dx) < wid - 1:
                        c = mix(c, "#081416", 0.6)
                    near.set(x, y, with_alpha(c, 225))
    # an arch springing between the columns
    for (ay, ax) in ((0, 20), (128, 148)):
        for y in range(ay, ay + 40):
            for x in range(ax, ax + 129):
                dx = x - (ax + 64)
                inner = math.hypot(dx, (y - (ay + 40)) * 1.6)
                if 58 < inner < 66 and y < ay + 40:
                    near.set(x, y, with_alpha("#122a2e", 225))
    # hanging weed / drips
    for _ in range(26):
        x, y = rng.random() * S, rng.random() * S
        for i in range(rng.randint(6, 26)):
            near.set(x, y, ("#16343a", 200))
            x += rng.uniform(-0.3, 0.3)
            y += 1
    finish_near(near, "#2e6466", "#06100f")
    for _ in range(30):
        x, y = rng.randrange(S), rng.randrange(S)
        if near.opaque(x, y) and not near.opaque(x, y + 1):
            near.set(x, y + 1, ("#5fc0c0", 200))
            near.set(x, y + 3, ("#5fc0c0", 120))
    return far, near


def mushroom(sp, cx, base_y, h, cap_w, cap_h, stem_w, colfn, rng=None):
    """Giant mushroom: flared stem, domed cap with a glowing gill rim and spots."""
    top = base_y - h
    for y in range(top + cap_h // 2, base_y + 1):
        t = (y - top) / float(h)
        wob = math.sin(y * 0.07 + cx) * 2.5 * t
        w = stem_w * (0.8 + 0.25 * t) + (3.0 * (t - 0.85) / 0.15 if t > 0.85 else 0)
        for x in range(int(cx + wob - w), int(cx + wob + w) + 1):
            sp.set(x, y, colfn("stem", (x - (cx + wob - w)) / (2 * w + 0.01)))
    cap = set()
    for y in range(top, top + cap_h + 1):
        k = (y - top) / float(cap_h)
        w = cap_w * (math.sqrt(max(0.0, 1 - (1 - k) ** 2)) * 0.85 + 0.15 * k)
        for x in range(int(cx - w), int(cx + w) + 1):
            cap.add((x, y))
            sp.set(x, y, colfn("cap", (x - (cx - w)) / (2 * w + 0.01) * 0.6 + k * 0.4))
    for x in range(int(cx - cap_w), int(cx + cap_w) + 1):
        sp.set(x, top + cap_h + 1, colfn("gill", 0.5))
        if (x + top) % 3 == 0:
            sp.set(x, top + cap_h + 2, colfn("gill2", 0.5))
    if rng is not None:
        pts = sorted(cap)
        for _ in range(max(2, cap_w // 4)):
            x, y = pts[rng.randrange(len(pts))]
            if (x, y - 1) in cap and (x - 1, y) in cap and (x + 1, y) in cap:
                sp.set(x, y, colfn("spot", 0))
                sp.set(x + 1, y, colfn("spot", 0))


def bg_layer2():
    """The Fungal Abyss: towering mushrooms, spore haze, magenta/teal glow."""
    rng = random.Random(2002)
    far = wrapped(Sprite(S, S))
    tex = FBM(rng, 8, 4)
    hue = FBM(rng, 2, 3)
    purple = ["#0e0a16", "#140e20", "#1a122a", "#211735"]
    teal = ["#081315", "#0b1b1e", "#0f2427", "#132d30"]
    for y in range(S):
        for x in range(S):
            v = tex(x, y) * 1.3 - 0.2
            h = hue(x, y)
            r = teal if (h - 0.5) * 4 + 0.5 > bayer(x + 1, y + 2) else purple
            far.set(x, y, ramp_soft(r, v, x, y))
    # distant mushroom silhouettes
    for i in range(6):
        cx, by_ = rng.randrange(S), rng.randrange(S)
        h = rng.randint(50, 90)
        cw = rng.randint(12, 22)
        mushroom(far, cx, by_, h, cw, int(cw * 0.6), max(2, cw // 6),
                 lambda part, t, fp=far: "#1c1328" if part not in ("gill", "spot") else "#3a1a40")
    # spore haze specks
    for _ in range(160):
        x, y = rng.randrange(S), rng.randrange(S)
        far.set(x, y, mix(far.get(x, y), rng.choice(["#d04fa0", "#3f9a96"]), 0.35))
    near = wrapped(Sprite(S, S))
    specs = [(40, 200, 120, 24), (150, 120, 90, 17), (210, 250, 150, 28), (95, 60, 60, 12), (5, 90, 70, 14), (180, 10, 50, 10)]
    for i, (cx, by_, h, cw) in enumerate(specs):
        hueM = i % 2 == 0

        def cf(part, t, hm=hueM):
            if part == "gill":
                return ("#b0408a" if hm else "#3fa39e", 230)
            if part == "gill2":
                return ("#6a2858" if hm else "#226260", 200)
            if part == "spot":
                return ("#d04fa0" if hm else "#57c4bc", 220)
            if part == "cap":
                return (ramp_pick(["#2d1a3c", "#231530", "#1a1024"] if hm else ["#173c3e", "#123032", "#0d2426"], t, 0, 0), 232)
            return ("#211830" if t > 0.55 else "#2a2038", 228)
        mushroom(near, cx, by_, h, cw, int(cw * 0.62), max(3, cw // 5), cf, rng)
    finish_near(near, "#4a3060", "#0a0710")
    # re-light the glowing bits that finish_near may have flattened
    return far, near


def bg_layer3():
    """The Molten Sanctum: ruined temple pillars, ember-lit cracks."""
    rng = random.Random(2003)
    far = wrapped(Sprite(S, S))
    tex = FBM(rng, 6, 4)
    vein = FBM(rng, 4, 4, 0.55)
    ramp = ["#120c0e", "#181012", "#1f1416", "#27191a", "#2f1e1e"]
    for y in range(S):
        for x in range(S):
            # temple wall: large carved blocks
            row = y // 32
            bx = (x + (row % 2) * 32) % 64
            v = 0.6 * tex(x, y) + 0.2
            c = ramp_soft(ramp, v, x, y)
            if y % 32 in (0, 31) or bx in (0, 63):
                c = mix(c, "#070405", 0.6)
            r = 1 - abs(2 * vein(x, y) - 1)
            if r > 0.992:
                c = col("#6e2a12")
            elif r > 0.98:
                c = mix(c, "#4a1c0e", 0.6)
            elif r > 0.962:
                c = mix(c, "#26110a", 0.45)
            far.set(x, y, c)
    # faint carved sun-relief on some blocks
    for i in range(4):
        cx, cy = rng.randrange(S), rng.randrange(S)
        for k in range(16):
            t = 2 * math.pi * k / 16
            for r in range(5, 9):
                far.set(cx + math.cos(t) * r, cy + math.sin(t) * r, mix(far.get(int(cx + math.cos(t) * r), int(cy + math.sin(t) * r)), "#3a2622", 0.6))
        for (x, y) in m_ellipse(cx, cy, 3.5, 3.5):
            far.set(x, y, mix(far.get(x, y), "#3a2622", 0.6))
    near = wrapped(Sprite(S, S))
    pillars = [(30, 26, 40), (158, 22, 170)]
    for (cx, w, brk) in pillars:
        for y in range(S):
            yy = (y - brk) % S
            broken_gap = yy < 26
            if broken_gap:
                continue
            for x in range(cx - w // 2 - 3, cx + w // 2 + 4):
                dx = x - cx
                cap = yy < 32 or yy > S - 8
                wid = w // 2 + (3 if cap and yy > 26 and yy < 31 else 0)
                # jagged break edges
                if yy < 30 and (x * 7 + yy * 3) % 5 < (30 - yy):
                    continue
                if abs(dx) <= wid:
                    c = ramp_pick(["#2a1c1c", "#211616", "#181010"], (dx + wid) / (2.0 * wid + 0.01), x, y)
                    if (dx + w) % 5 == 0 and abs(dx) < wid - 1:
                        c = mix(c, "#0e0808", 0.5)
                    near.set(x, y, with_alpha(c, 228))
    shape = FBM(rng, 4, 4)
    for y in range(S):
        for x in range(S):
            if not near.opaque(x, y) and shape(x, y) > 0.69:
                near.set(x, y, with_alpha(ramp_pick(["#1a1112", "#221616"], shape(x, y) * 2 - 1.3, x, y), 228))
    finish_near(near, "#5a2a1c", "#0a0505")
    for y in range(S):
        for x in range(S):
            if near.opaque(x, y):
                r = 1 - abs(2 * vein(x + 77, y + 31) - 1)
                if r > 0.992:
                    near.set(x, y, with_alpha("#9a4016", 228))
                elif r > 0.982:
                    near.set(x, y, with_alpha("#5a2010", 228))
    return far, near


def bg_layer4():
    """The Heart of the World: vast rune circles, gold-white glow."""
    rng = random.Random(2004)
    far = wrapped(Sprite(S, S))
    tex = FBM(rng, 6, 4)
    glow = FBM(rng, 2, 3)
    ramp = ["#0e0a08", "#15100b", "#1c150e", "#251b11", "#2e2214"]
    for y in range(S):
        for x in range(S):
            g = glow(x, y)
            v = 0.5 * tex(x, y) + 0.5 * g
            c = ramp_soft(ramp, (v - 0.2) * 1.6, x, y)
            far.set(x, y, c)
    # vast rune circles (centred in the tile and at the corners -> seamless)
    for (cx, cy, R) in ((128, 128, 92), (0, 0, 60), (128, 128, 70)):
        for k in range(int(2 * math.pi * R * 2)):
            t = k / (2.0 * R)
            for dr in (0, 4) if R > 80 else (0,):
                x, y = cx + math.cos(t) * (R - dr), cy + math.sin(t) * (R - dr)
                far.set(x, y, mix(far.get(int(x), int(y)), "#6e5426", 0.55))
        n = int(R / 4)
        for i in range(n):
            t = 2 * math.pi * i / n
            x, y = cx + math.cos(t) * (R - 2), cy + math.sin(t) * (R - 2)
            g = CORE_RUNES[i % len(CORE_RUNES)]
            for yy, row in enumerate(g):
                for xx, ch in enumerate(row):
                    if ch == "#":
                        far.set(int(x) - 1 + xx, int(y) - 1 + yy, mix(far.get(int(x) - 1 + xx, int(y) - 1 + yy), "#a07e3a", 0.5))
    # spokes
    for i in range(8):
        t = 2 * math.pi * i / 8
        for r in range(20, 70):
            x, y = 128 + math.cos(t) * r, 128 + math.sin(t) * r
            far.set(x, y, mix(far.get(int(x), int(y)), "#4a3a1c", 0.5))
    near = wrapped(Sprite(S, S))
    # floating rock shards lit from below by gold light
    for (cx, cy, w, h) in ((60, 50, 22, 16), (190, 110, 28, 20), (110, 200, 18, 14), (230, 230, 12, 10), (20, 160, 14, 12)):
        poly = [(cx - w, cy - h * 0.2), (cx - w * 0.6, cy - h * 0.8), (cx + w * 0.2, cy - h), (cx + w, cy - h * 0.4),
                (cx + w * 0.8, cy + h * 0.4), (cx + w * 0.2, cy + h * 0.9), (cx - w * 0.1, cy + h * 1.6),
                (cx - w * 0.5, cy + h * 0.7), (cx - w * 0.9, cy + h * 0.4)]
        for (x, y) in m_poly(poly):
            near.set(x, y, with_alpha(ramp_pick(["#1a140e", "#140f0a", "#0e0a07"], (y - cy + h) / (2.5 * h), x, y), 236))
    finish_near(near, "#3a2c18", "#070504")
    src = near.copy()
    src.wrap = True
    for y in range(S):
        for x in range(S):
            if src.opaque(x, y) and not src.opaque(x, y + 1):
                near.set(x, y, with_alpha("#b08636", 236))
            elif src.opaque(x, y) and not src.opaque(x, y + 2):
                near.set(x, y, with_alpha("#5a4220", 236))
    # drifting motes of light
    for _ in range(50):
        x, y = rng.randrange(S), rng.randrange(S)
        if not near.opaque(x, y):
            near.set(x, y, ("#e3c069", rng.choice([90, 140, 200])))
    return far, near

# --------------------------------------------------------------------------
# UI ornaments (ui.png, 64x32)
# --------------------------------------------------------------------------
UI_CORNER = [
    # g gold light, G gold, d gold dark, v violet gem, V gem light, k ink
    "kkkkkkkkkkkkkkkk",
    "kggggggggggggggg",
    "kgkkkkkkkkkkkkkk",
    "kgkGGGGGGGGGGGGG",
    "kgkGgggGd.......",
    "kgkGgVvGd.......",
    "kgkGgvvd........",
    "kgkGGdd.........",
    "kgkGd...........",
    "kgkG............",
    "kgkG............",
    "kgkG............",
    "kgkG............",
    "kgkG............",
    "kgkG............",
    "kgkG............",
]


def gen_ui():
    out = Sprite(64, 32)
    # (0,0) ornate panel corner: double gold border (outer line on row/col 1,
    # inner on row/col 2, dark gap on 3) with a jewelled corner boss
    key = {"g": "gold_l", "G": "gold", "d": "gold_d", "v": "vio_l", "V": "magic", "k": "ink"}
    corner = Sprite(16, 16)
    draw_ascii(corner, UI_CORNER, 0, 0, key)
    # curl flourish running along the edges
    for (x, y, c) in [(9, 4, "gold_d"), (10, 5, "gold"), (11, 5, "gold_d"), (12, 4, "gold"), (4, 9, "gold_d"),
                      (5, 10, "gold"), (5, 11, "gold_d"), (4, 12, "gold"), (13, 4, "gold_d"), (4, 13, "gold_d")]:
        corner.set(x, y, c)
    out.blit(corner, 0, 0)
    # (16,0) round sigil badge frame: bevelled gold ring, transparent centre
    badge = Sprite(16, 16)
    ring = m_ellipse(8, 8, 7.9, 7.9) - m_ellipse(8, 8, 5.6, 5.6)
    for (x, y) in ring:
        dx, dy = x + 0.5 - 8, y + 0.5 - 8
        d = math.hypot(dx, dy)
        if d > 7.0:
            c = "ink"
        elif d < 6.3:
            c = "gold_d" if dx + dy < 0 else "gold_l"
        else:
            c = "gold_l" if dx + dy < -2 else ("gold_d" if dx + dy > 2 else "gold")
        badge.set(x, y, c)
    inner_edge = m_ellipse(8, 8, 5.6, 5.6) - m_ellipse(8, 8, 4.9, 4.9)
    for (x, y) in inner_edge:
        badge.set(x, y, "ink")
    for (x, y) in ((7, 0), (8, 0), (7, 15), (8, 15), (0, 7), (0, 8), (15, 7), (15, 8)):
        badge.set(x, y, "gold")
    for (x, y) in ((7, 1), (8, 1), (1, 7), (1, 8)):
        badge.set(x, y, "#fff0b0")
    out.blit(badge, 16, 0)
    # (32,0) 32x8 divider flourish
    div = Sprite(32, 8)
    for x in range(2, 30):
        d = abs(x + 0.5 - 16)
        if d > 4:
            div.set(x, 3, "gold" if d < 11 else "gold_d")
            if d < 12:
                div.set(x, 4, "gold_d")
    for (x, y) in m_poly([(16, 0.5), (19.5, 4), (16, 7.5), (12.5, 4)]):
        div.set(x, y, "gold")
    for (x, y) in m_poly([(16, 2), (17.6, 4), (16, 6), (14.4, 4)]):
        div.set(x, y, "vio_l")
    div.set(15, 3, "magic")
    for (x, y, c) in [(12, 2, "gold_d"), (11, 1, "gold"), (10, 2, "gold_d"), (19, 2, "gold_d"), (20, 1, "gold"), (21, 2, "gold_d"),
                      (12, 5, "gold_d"), (11, 6, "gold"), (19, 5, "gold_d"), (20, 6, "gold"), (1, 3, "gold_d"), (30, 3, "gold_d"),
                      (0, 4, with_alpha("gold_d", 140)), (31, 4, with_alpha("gold_d", 140))]:
        div.set(x, y, c)
    out.blit(div, 32, 0)
    # (32,8) 32x8 smaller divider
    sd = Sprite(32, 8)
    for x in range(3, 29):
        d = abs(x + 0.5 - 16)
        if d > 2:
            sd.set(x, 4, "gold" if d < 9 else "gold_d")
    for (x, y, c) in [(15, 3, "gold_l"), (16, 3, "gold"), (15, 4, "gold"), (16, 4, "gold_d"), (14, 4, "gold_d"), (17, 3, "gold_d"),
                      (16, 5, "gold_d"), (15, 2, "gold_d")]:
        sd.set(x, y, c)
    out.blit(sd, 32, 8)
    # (0,16) 16x16 tileable dark vellum texture (opaque)
    rng = random.Random(901)
    n = TNoise(rng, 4)
    n2 = TNoise(rng, 8)
    tex = Sprite(16, 16)
    ramp = ["#120d18", "#16101d", "#1a1322", "#1e1626"]
    for y in range(16):
        for x in range(16):
            v = 0.6 * n.at(x / 4.0, y / 4.0) + 0.4 * n2.at(x / 2.0, y / 2.0)
            tex.set(x, y, ramp_soft(ramp, (v - 0.25) * 1.7, x, y))
    for (x, y) in [(3, 5), (4, 5), (11, 12), (12, 12), (13, 12), (7, 2), (9, 9)]:
        tex.set(x, y, "#231a2c")
    out.blit(tex, 0, 16)
    return out


GENERATORS = [("icons", gen_icons, "icons.png"), ("props", gen_props, "props.png"), ("spells", gen_spells, "spells.png"),
              ("creatures", gen_creatures, "creatures.png"), ("stalker", gen_stalker, "stalker.png"),
              ("core", gen_core, "core.png"), ("ui", gen_ui, "ui.png")]
BG_FUNCS = [bg_layer0, bg_layer1, bg_layer2, bg_layer3, bg_layer4]

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


PIXEL_FONT = dict(glyphs=GLYPHS, notdef=NOTDEF, cap=7, asc=8, desc=2, check="Ag0\u2192\u00b0",
                  names={1: "Descent Pixel", 2: "Regular", 3: "DescentPixel-Regular-1.000",
                         4: "Descent Pixel Regular", 5: "Version 1.000", 6: "DescentPixel-Regular", "xh": 5})

# --------------------------------------------------------------------------
# Title font (title.ttf): ornate grimoire display face, 9 px cap height,
# 2 px stems with hairline serifs.  Values are (row_offset, rows): row 0 is
# the cap-top row, row 8 sits on the baseline, rows 9-11 are descenders.
# --------------------------------------------------------------------------
def _mirror(rows):
    w = max(len(r) for r in rows)
    return [r.ljust(w, ".")[::-1] for r in rows]


TGLYPHS = {
    " ": (0, ["...."]),
    "A": (0, ["...##...", "...##...", "..#.##..", "..#.##..", ".#...##.", ".######.", ".#...##.", "#....##.", "###.####"]),
    "B": (0, ["######..", ".##..##.", ".##..##.", ".##.##..", ".#####..", ".##..##.", ".##..##.", ".##..##.", "######.."]),
    "C": (0, ["..####.#", ".##...##", "##.....#", "##......", "##......", "##......", "##.....#", ".##...##", "..####.."]),
    "D": (0, ["#####...", ".##.##..", ".##..##.", ".##..##.", ".##..##.", ".##..##.", ".##..##.", ".##.##..", "#####..."]),
    "E": (0, ["#######", ".##...#", ".##....", ".##..#.", ".#####.", ".##..#.", ".##....", ".##...#", "#######"]),
    "F": (0, ["#######", ".##...#", ".##....", ".##..#.", ".#####.", ".##..#.", ".##....", ".##....", "####..."]),
    "G": (0, ["..####.#", ".##...##", "##.....#", "##......", "##..####", "##....##", "##....##", ".##..###", "..###..#"]),
    "H": (0, ["####.####", ".##...##.", ".##...##.", ".##...##.", ".#######.", ".##...##.", ".##...##.", ".##...##.", "####.####"]),
    "I": (0, ["####", ".##.", ".##.", ".##.", ".##.", ".##.", ".##.", ".##.", "####"]),
    "J": (0, ["..####", "...##.", "...##.", "...##.", "...##.", "...##.", "#..##.", "##.##.", ".###.."]),
    "K": (0, ["####.###", ".##...#.", ".##..#..", ".##.#...", ".###....", ".##.##..", ".##..##.", ".##...##", "####.###"]),
    "L": (0, ["####...", ".##....", ".##....", ".##....", ".##....", ".##....", ".##...#", ".##..##", "#######"]),
    "M": (0, ["##.....##", ".##...##.", ".###.###.", ".##.#.##.", ".##.#.##.", ".##...##.", ".##...##.", ".##...##.", "###...###"]),
    "N": (0, ["###..###", ".##...#.", ".###..#.", ".#.##.#.", ".#.##.#.", ".#..###.", ".#...##.", ".#...##.", "###...#."]),
    "O": (0, ["..####..", ".##..##.", "##....##", "##....##", "##....##", "##....##", "##....##", ".##..##.", "..####.."]),
    "P": (0, ["######..", ".##..##.", ".##..##.", ".##..##.", ".#####..", ".##.....", ".##.....", ".##.....", "####...."]),
    "Q": (0, ["..####..", ".##..##.", "##....##", "##....##", "##....##", "##....##", "##.##.##", ".##.###.", "..####.#", ".......#"]),
    "R": (0, ["######..", ".##..##.", ".##..##.", ".##..##.", ".#####..", ".##.##..", ".##..##.", ".##..##.", "####..##"]),
    "S": (0, [".####.#", "##...##", "##....#", ".###...", "...###.", ".....##", "#....##", "##...##", "#.####."]),
    "T": (0, ["########", "#..##..#", "...##...", "...##...", "...##...", "...##...", "...##...", "...##...", "..####.."]),
    "U": (0, ["####.###", ".##...#.", ".##...#.", ".##...#.", ".##...#.", ".##...#.", ".##...#.", ".##..#..", "..###..."]),
    "V": (0, ["####.###", ".##...#.", ".##...#.", "..##.#..", "..##.#..", "..##.#..", "...###..", "...##...", "...#...."]),
    "W": (0, ["###...###", ".##...##.", ".##...##.", ".##.#.##.", ".##.#.##.", ".##.#.##.", ".#######.", ".###.###.", ".##...##."]),
    "X": (0, ["####.###", ".##...#.", "..##.#..", "...##...", "...##...", "..#.##..", ".#...##.", ".#...##.", "###.####"]),
    "Y": (0, ["####.###", ".##...#.", "..##.#..", "...##...", "...##...", "...##...", "...##...", "...##...", "..####.."]),
    "Z": (0, ["#######", "#...##.", "...##..", "...##..", "..##...", "..##...", ".##....", ".##...#", "#######"]),
    "a": (3, [".####.", "....##", ".#####", "##..##", "##..##", ".###.#"]),
    "b": (0, ["###....", ".##....", ".##....", ".#####.", ".##..##", ".##..##", ".##..##", ".##..##", "#.####."]),
    "c": (3, [".####", "##..#", "##...", "##...", "##..#", ".###."]),
    "d": (0, ["...###.", "....##.", "....##.", ".#####.", "##..##.", "##..##.", "##..##.", "##..##.", ".###.##"]),
    "e": (3, [".####.", "##..##", "######", "##....", "##...#", ".####."]),
    "f": (0, ["..###", ".##.#", ".##..", "####.", ".##..", ".##..", ".##..", ".##..", "####."]),
    "g": (3, [".#####", "##..##", "##..##", "##..##", "##..##", ".#####", "....##", "#...##", ".####."]),
    "h": (0, ["###....", ".##....", ".##....", ".#####.", ".##..##", ".##..##", ".##..##", ".##..##", "###.###"]),
    "i": (0, [".##", ".##", "...", "###", ".##", ".##", ".##", ".##", "###"]),
    "j": (0, ["..##", "..##", "....", ".###", "..##", "..##", "..##", "..##", "..##", "#.##", ".##."]),
    "k": (0, ["###....", ".##....", ".##....", ".##.###", ".##.#..", ".###...", ".##.#..", ".##..#.", "###.###"]),
    "l": (0, ["###", ".##", ".##", ".##", ".##", ".##", ".##", ".##", "###"]),
    "m": (3, ["###.##.#.", ".###.###.", ".##..#..#", ".##..#..#", ".##..#..#", "###.###.#"]),
    "n": (3, ["####...", ".##.##.", ".##..##", ".##..##", ".##..##", "###.###"]),
    "o": (3, [".####.", "##..##", "##..##", "##..##", "##..##", ".####."]),
    "p": (3, ["#####.", ".##.##", ".##.##", ".##.##", ".##.##", ".####.", ".##...", ".##...", "####.."]),
    "q": (3, [".#####", "##.##.", "##.##.", "##.##.", "##.##.", ".####.", "...##.", "...##.", "..####"]),
    "r": (3, ["###.##", ".####.", ".##...", ".##...", ".##...", "####.."]),
    "s": (3, [".####", "##..#", ".##..", "..##.", "#..##", "####."]),
    "t": (1, [".#...", ".##..", "#####", ".##..", ".##..", ".##..", ".##.#", "..##."]),
    "u": (3, ["##..##.", "##..##.", "##..##.", "##..##.", "##..##.", ".###.##"]),
    "v": (3, ["###.##", ".##..#", ".##..#", "..##.#", "..###.", "...#.."]),
    "w": (3, ["##...##", "##...#.", "##.#.#.", "##.#.#.", ".#####.", ".##.##."]),
    "x": (3, ["###.##", ".##.#.", "..##..", "..##..", ".#.##.", "##.###"]),
    "y": (3, ["###.##", ".##..#", ".##..#", "..##.#", "..###.", "...##.", "...#..", "#.##..", ".##..."]),
    "z": (3, ["######", "#..##.", "..##..", ".##...", "##...#", "######"]),
    "0": (0, ["..###..", ".##.##.", "##...##", "##..###", "##.#.##", "###..##", "##...##", ".##.##.", "..###.."]),
    "1": (0, ["..##.", ".###.", "#.##.", "..##.", "..##.", "..##.", "..##.", "..##.", ".####"]),
    "2": (0, [".####.", "##..##", "#...##", "....##", "...##.", "..##..", ".##...", "##...#", "######"]),
    "3": (0, [".####.", "##..##", "....##", "....##", "..###.", "....##", "....##", "##..##", ".####."]),
    "4": (0, ["....##.", "...###.", "..#.##.", ".#..##.", "#...##.", "#######", "....##.", "....##.", "...####"]),
    "5": (0, ["######", "##....", "##....", "#####.", "....##", "....##", "....##", "##..##", ".####."]),
    "6": (0, ["..###.", ".##...", "##....", "#####.", "##..##", "##..##", "##..##", "##..##", ".####."]),
    "7": (0, ["######", "#...##", "....##", "...##.", "...##.", "..##..", "..##..", "..##..", "..##.."]),
    "8": (0, [".####.", "##..##", "##..##", ".#..#.", ".####.", "##..##", "##..##", "##..##", ".####."]),
    "9": (0, [".####.", "##..##", "##..##", "##..##", ".#####", "....##", "....##", "...##.", ".###.."]),
    "!": (0, ["##", "##", "##", "##", "##", "##", "..", "##", "##"]),
    '"': (0, ["##.##", "##.##", ".#..#"]),
    "#": (1, [".#..#.", ".#..#.", "######", ".#..#.", ".#..#.", "######", ".#..#.", ".#..#."]),
    "$": (0, ["..#...", ".#####", "##.#..", "##.#..", ".####.", "..#.##", "..#.##", "#####.", "..#..."]),
    "%": (1, ["##...#", "##..#.", "...#..", "..#...", ".#....", "#...##", "....##"]),
    "&": (1, [".##...", "#..#..", "#..#..", ".##...", "##.#.#", "#..##.", "#...#.", ".###.#"]),
    "'": (0, ["##", "##", ".#"]),
    "(": (0, ["..#", ".##", "##.", "##.", "##.", "##.", "##.", ".##", "..#"]),
    ")": (0, ["#..", "##.", ".##", ".##", ".##", ".##", ".##", "##.", "#.."]),
    "*": (1, ["..#..", "#.#.#", ".###.", "#.#.#", "..#.."]),
    "+": (2, ["..#..", "..#..", "#####", "..#..", "..#.."]),
    ",": (7, ["##", "##", ".#", "#."]),
    "-": (5, ["#####"]),
    ".": (7, ["##", "##"]),
    "/": (0, ["....##", "....#.", "...##.", "...#..", "..##..", "..#...", ".##...", ".#....", "##...."]),
    ":": (3, ["##", "##", "..", "..", "##", "##"]),
    ";": (3, ["##", "##", "..", "..", "##", "##", ".#", "#."]),
    "<": (1, ["...##", "..##.", ".##..", "##...", ".##..", "..##.", "...##"]),
    "=": (3, ["#####", ".....", "#####"]),
    ">": (1, ["##...", ".##..", "..##.", "...##", "..##.", ".##..", "##..."]),
    "?": (0, [".####.", "##..##", "....##", "...##.", "..##..", "..##..", "......", "..##..", "..##.."]),
    "@": (1, [".#####.", "##...##", "##.####", "##.#.##", "##.####", "##.....", ".######"]),
    "[": (0, ["###", "##.", "##.", "##.", "##.", "##.", "##.", "##.", "###"]),
    "]": (0, ["###", ".##", ".##", ".##", ".##", ".##", ".##", ".##", "###"]),
    "^": (0, ["..#..", ".###.", "##.##"]),
    "_": (10, ["#######"]),
    "`": (0, ["##.", ".##"]),
    "{": (0, ["..##", ".##.", ".##.", ".##.", "##..", ".##.", ".##.", ".##.", "..##"]),
    "|": (0, ["##"] * 11),
    "~": (4, [".##..#", "#..##."]),
}
TGLYPHS["\\"] = (0, _mirror(TGLYPHS["/"][1]))
TGLYPHS["}"] = (0, _mirror(TGLYPHS["{"][1]))
TNOTDEF = ["######", "#....#", "#....#", "#....#", "#....#", "#....#", "#....#", "#....#", "######"]
assert all(chr(c) in TGLYPHS for c in range(32, 127)), [chr(c) for c in range(32, 127) if chr(c) not in TGLYPHS]
TITLE_FONT = dict(glyphs=TGLYPHS, notdef=TNOTDEF, cap=9, asc=10, desc=3, check="Ag0Q",
                  names={1: "Last Apprentice Title", 2: "Regular", 3: "LastApprenticeTitle-Regular-1.000",
                         4: "Last Apprentice Title Regular", 5: "Version 1.000", 6: "LastApprenticeTitle-Regular", "xh": 6})

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


def build_ttf(glyphs, notdef, cap, asc, desc, names, space=1, check=None):
    """Build a TrueType pixel font.  glyphs: char -> rows or (row_offset, rows)
    where row 0 is the cap-top row and row cap-1 sits on the baseline; asc/desc
    are in font pixels (desc positive).  1 font px = 128 units, 1024 upem."""
    UPM, PX = 1024, 128
    ASC, DESC = asc * PX, -desc * PX
    order = [(".notdef", None, (0, notdef))]
    chars = sorted(glyphs.keys(), key=ord)
    for ch in chars:
        g = glyphs[ch]
        order.append(("uni%04X" % ord(ch), ord(ch), g if isinstance(g, tuple) else (0, g)))
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
    for name, cp, (roff, rows) in order:
        width = max(len(r) for r in rows)
        adv = (width + space) * PX
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
            yb = (cap - r1 - roff) * PX
            yt = (cap - r0 - roff) * PX
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
                       ASC, -DESC, 1, 0, names.get("xh", 5) * PX, cap * PX, 0, 32, 1)
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
    names = {k: v for k, v in names.items() if isinstance(k, int)}
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


def check_ttf(data, probe="Ag0"):
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
    for ch in probe:
        gid = lookup(ord(ch))
        a, b = struct.unpack(">II", data[lo + 4 * gid:lo + 4 * gid + 8])
        ncont = struct.unpack(">h", data[go + a:go + a + 2])[0] if b > a else 0
        assert gid != 0 and b > a and ncont > 0, ch
        results[ch] = (gid, b - a, ncont)
    return results


# --------------------------------------------------------------------------
# Preview
# --------------------------------------------------------------------------
def upscale(sp, k, bg=True, bgc=None):
    out = Sprite(sp.w * k, sp.h * k)
    for y in range(out.h):
        for x in range(out.w):
            p = sp.px[(y // k) * sp.w + (x // k)]
            if bg:
                if bgc:
                    b = bgc
                else:
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


def tint(sp, c):
    c = col(c)
    out = sp.copy()
    out.px = [(p[0] * c[0] // 255, p[1] * c[1] // 255, p[2] * c[2] // 255, p[3]) if p[3] else p for p in sp.px]
    return out


def composite_wizard(layers, robe_c=None, trim_c=None):
    r, t, b = layers
    out = Sprite(r.w, r.h)
    out.blit(tint(r, robe_c) if robe_c else r, 0, 0)
    out.blit(tint(t, trim_c) if trim_c else t, 0, 0)
    out.blit(b, 0, 0)
    return out


def save_preview(d, name, sp, k, fw=None, fh=None, bgc=None):
    p = upscale(sp, k, bgc=bgc)
    if fw:
        grid_lines(p, fw, fh, k)
    write_png(os.path.join(d, name), p)


def font_preview(path, glyphs, notdef, lines, adv_extra=1, line_h=12, top=2):
    W = 4 + max(sum(max(len(r) for r in (glyphs.get(ch, notdef)[1] if isinstance(glyphs.get(ch, notdef), tuple) else glyphs.get(ch, notdef))) + adv_extra for ch in ln) for ln in lines)
    H = line_h * len(lines) + 4
    sp = Sprite(W, H, (24, 20, 37, 255))
    for li, line in enumerate(lines):
        x = 2
        for ch in line:
            g = glyphs.get(ch, notdef)
            off, rows = g if isinstance(g, tuple) else (0, g)
            for r, row in enumerate(rows):
                for cx, c in enumerate(row):
                    if c == "#":
                        sp.set(x + cx, top + li * line_h + r + off, (230, 214, 170, 255))
            x += max(len(r) for r in rows) + adv_extra
    write_png(path, upscale(sp, 4, bg=False))


def write_previews(d, outs, only):
    os.makedirs(d, exist_ok=True)
    dark = (20, 16, 28, 255)
    if "wizard" in outs:
        lay = outs["wizard"]
        save_preview(d, "wizard_white.png", composite_wizard(lay), 5, 16, 16, bgc=dark)
        save_preview(d, "wizard_tinted.png", composite_wizard(lay, SCHOOL_TINT[0], SCHOOL_TINT[1]), 5, 16, 16, bgc=dark)
        save_preview(d, "wizard_tinted2.png", composite_wizard(lay, SCHOOL_TINT[7], SCHOOL_TINT[3]), 5, 16, 16, bgc=dark)
        save_preview(d, "wizard_layers.png", lay[0], 3, 16, 16)
    for key, k, fw, fh in [("icons", 4, 16, 16), ("props", 5, 16, 24), ("spells", 8, 8, 8), ("creatures", 5, 16, 16),
                           ("stalker", 5, 16, 32), ("core", 3, 48, 48), ("ui", 8, None, None)]:
        if key in outs:
            save_preview(d, key + ".png", outs[key], k, fw, fh, bgc=dark)
    for i in range(5):
        if "bg%d" % i not in outs:
            continue
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
        if "wizard" in outs:
            t.blit(composite_wizard(outs["wizard"], SCHOOL_TINT[0], SCHOOL_TINT[1]), 20, 20)
        write_png(os.path.join(d, "bg%d.png" % i), t)
    if "fonts" in outs:
        font_preview(os.path.join(d, "font_pixel.png"), GLYPHS, NOTDEF,
                     ["The quick brown fox jumps", "over the lazy dog! 0123456789",
                      "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"])
        font_preview(os.path.join(d, "font_title.png"), TGLYPHS, TNOTDEF,
                     ["THE LAST APPRENTICE", "The Whispering Crust", "Drowned Halls, Fungal Abyss",
                      "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz", "0123456789 !?.,:;'\"-+=/()",
                      "#$%&*<>@[\\]^_`{|}~"], line_h=15, top=2)


# --------------------------------------------------------------------------
def main():
    preview = None
    if "--preview" in sys.argv:
        preview = sys.argv[sys.argv.index("--preview") + 1]
    only = None
    if "--only" in sys.argv:
        only = set(sys.argv[sys.argv.index("--only") + 1].split(","))

    def want(k):
        return only is None or k in only
    SPR = os.path.join(ASSETS, "sprites")
    outs = {}
    if want("wizard"):
        lay = gen_wizard()
        outs["wizard"] = lay
        for name, sp in zip(("robe", "trim", "base"), lay):
            write_png(os.path.join(SPR, "wizard_%s.png" % name), sp)
    for key, fn, fname in GENERATORS:
        if want(key):
            outs[key] = fn()
            write_png(os.path.join(SPR, fname), outs[key])
    if want("bg"):
        for i, fn in enumerate(BG_FUNCS):
            far, near = fn()
            far.wrap = near.wrap = False
            for p in far.px:
                assert p[3] == 255
            outs["bg%d" % i] = (far, near)
            write_png(os.path.join(ASSETS, "backgrounds", "layer%d_far.png" % i), far)
            write_png(os.path.join(ASSETS, "backgrounds", "layer%d_near.png" % i), near)
            vals = [max(p[:3]) / 255.0 for p in far.px]
            print("layer%d far: mean HSV value %.3f, max %.3f" % (i, sum(vals) / len(vals), max(vals)))
    if want("fonts"):
        os.makedirs(os.path.join(ASSETS, "fonts"), exist_ok=True)
        for fname, kw in (("pixel.ttf", PIXEL_FONT), ("title.ttf", TITLE_FONT)):
            ttf = build_ttf(**kw)
            with open(os.path.join(ASSETS, "fonts", fname), "wb") as f:
                f.write(ttf)
            res = check_ttf(ttf, kw["check"])
            print("%s self-check OK (%d bytes): " % (fname, len(ttf)) +
                  ", ".join("%r->gid %d (%d B, %d contours)" % (k, *v) for k, v in res.items()))
        outs["fonts"] = True
    for stale in ("player.png", "items.png"):
        p = os.path.join(SPR, stale)
        if os.path.exists(p):
            os.remove(p)
            print("removed obsolete", p)
    if preview:
        write_previews(preview, outs, only)
        print("previews written to", preview)


if __name__ == "__main__":
    main()
