#!/usr/bin/env python3
"""Procedural pixel-art generator for Planet Terraforming (plant punk).

Deterministic and standard-library only (zlib + struct PNG writer, hand-built
TrueType writer).  Run from the repository root:

    python3 tools/gen_art.py                    # (re)write every asset below
    python3 tools/gen_art.py --preview DIR      # also write 4x previews to DIR
    python3 tools/gen_art.py --only icons,domes # only some groups
    python3 tools/gen_art.py --serial           # no multiprocessing

--only names: icons, domes, props, fx, foliage, surface, caves, mist, menu,
ship, ui, fonts.

Conventions: 1 art pixel = 1 game cell (shown at 3x, nearest filtering).
Light comes from the top-left.  Objects carry a 1 px outline of #12101a
tinted by the local colour.  All PNGs are RGBA8 with a transparent
background unless stated.  Frames are packed left-to-right from x=0, rows
top-to-bottom.  Palette: muted teal, rust, moss, bone, indigo shadows, with
luminous cyan / lime / amber / magenta accents.

===========================================================================
1. assets/sprites/icons.png   256x128, 16x16 cells, 16 columns x 8 rows
===========================================================================
index = row*16 + col; cell origin = (16*(index%16), 16*(index//16)).
Art sits inside the cell with its outline (at most 14x14 art + 1 px rim).

  0 Stone          1 Dirt           2 Sand            3 Wood (log)
  4 Plant Fiber    5 Iron           6 Copper          7 Silicon
  8 Gold           9 Titanium      10 Xenite         11 Aurorium
 12 Chitin        13 Silk          14 Bio-gel        15 Glass
 16 Fertilizer    17 Meal          18 Egg            19 Milk
 20 Fish          21 Water Canister 22 Watering Can  23 Lamp
 24 Chest         25 Charging Pylon 26 Pump          27 Water Tank
 28 Copper Pipe   29 Oxygen Generator 30 Robot (kit) 31 Synthesizer
 32 Gene Splicer  33 Green Dome kit 34 Storage Dome kit 35 Bedroom Dome kit
 36 Kitchen Dome kit 37 Dark Dome kit 38 Dome Dome kit 39 Aqua Dome kit
 40 Apartment Dome kit 41 Barn Dome kit 42 Water Generator Dome kit
 43 Cluckbug      44 Milk Grub     45 Fish Fry (bag)
 46 Seed pouch (NEUTRAL GREY: multiply-tint per species)
 47 Crop        (NEUTRAL GREY: multiply-tint per species)
 48 Multitool     49 Laser bolt glyph 50 Credits     51 Oxygen (O2)
 52 Battery       53 Heart         54 Water drop     55 Food
 56 Housing       57 Power         58 Lock           59 Trash can
 60 Star (favourite) 61 Map pin    62 Worker         63 Blueprint
 64 Sprinkler     65 Compost       66 slot Head      67 slot Eyes
 68 slot Torso    69 slot Back     70 slot Arms      71 slot Hands
 72 slot Legs     73 slot Feet     74 Sort           75 Quick stack
 76 Deposit all   77 Search        78 Warning        79 Check
 80 Weather clear 81 Weather mist  82 Weather rain   83 Sun
 84 Moon          85 Robot status  86 Map            87 Codex
 88 Gear          89 Thermometer (cold) 90 Toxin     91 Skull
 92 Battery Flower 93 Dome (generic) 94 Pipe overlay 95 Power overlay
 96 Suit (helmet) 97 Bed           98 Earth          99 Ship
100 Colonist group 101 Tier I pip (copper) 102 Tier II pip (steel, x2)
103 Tier III pip (gold, x3) 104 Legendary pip (glowing star)
105 Arrow up (green) 106 Arrow down (red) 107 Dash (steady)
108 Hand (interact) 109 Pause      110 Play          111 Area (dashed box)
112-127 unused (fully transparent).

Dome kit icons 33-42 share one layout (glass dome on a steel crate) and
differ by glass colour + crate glyph: green/leaf, storage/crate,
bedroom/bed, kitchen/pot, dark/moon, domedome/dome, aqua/wave,
apartment/building, barn/barn, watergen/droplet.

===========================================================================
2. assets/sprites/domes/<name>_back.png and <name>_front.png
===========================================================================
  name       W x H        name       W x H        name       W x H
  starter    216x80       green      104x52       storage    72x44
  bedroom    72x44        kitchen    72x44        dark       104x52
  domedome   104x52       aqua       104x52       apartment  136x64
  barn       104x52       watergen   72x44

Geometry (exact; the game builds matching cells).  For a W x H sprite the
dome is the upper half of the ellipse centred at (W/2, H), radii a = W/2,
b = H.  Pixel (x, y) is inside the dome when
    ((x+0.5-W/2)/a)^2 + ((y+0.5-H)/b)^2 <= 1.
The INTERIOR is the same test with radii (a-2, b-2); the SHELL is inside
the dome but not in the interior.  The floor is the row y = H just below
the sprite (drawn by the game), so the bottom sprite row is the lowest
interior row.

<name>_back.png: opaque (alpha 255) on exactly the interior pixels and
  transparent everywhere else (including the shell).  A dark back wall with
  ribs, seams, lamps, vines and type-specific scenery; the bottom 26 rows
  are faded toward the dark wall tone so fixtures drawn on top read
  clearly.  Draw order: back, fixtures/plants/players, front.
    starter   teal-steel habitat, central support pillar (x = W/2-4..W/2+3),
              worn mission roundel + stripe (left), porthole (right),
              screens, shelves of pots, grow-lights, heavy vines
    green     deep green wall, trellis, pink grow-lamps, copper irrigation
              line with drips, leaf clumps
    storage   shelving racks loaded with crates, hazard plate, amber lamp
    bedroom   warm plum wall, dusk window with the ringed moon, lockers,
              string lights
    kitchen   tiled wall, extractor hood and duct, hanging pots, jar shelves
    dark      near-black violet wall, three UV strips, glowing mushrooms,
              mycelium threads
    domedome  gantry truss, two robot arms, weld sparks, blueprint screen
    aqua      deep-blue tank backdrop, light shafts, kelp shadows, bubbles
    apartment two storeys of lit windows with balconies and planters, lift
              shaft in the centre, string lights
    barn      wooden slats, loft beam with hay bales, X bracing, lantern
    watergen  condenser coil bank, dripping copper down-pipes, gauge, frost
<name>_front.png: drawn OVER players.  The 2 px shell (outer pixel metal
  frame, inner pixel translucent glass, opaque ribs about every 14 px of
  arc, rust and moss patches, vines draped over the outside) plus faint
  glass sheen streaks over the interior (alpha <= 30/255, most of the
  interior is fully transparent; a few opaque vine tendrils and moss tufts
  hang just inside the shell).  The bottom 11 rows of the shell at both
  ends are an airlock marking (cyan lamp over amber/black hazard stripes)
  with a small glow spilling inside (alpha <= 60/255).  `dark` instead has
  an opaque dark shell and a violet tint (#2a1648, alpha about 52-76 of
  255, roughly 25%) over the whole interior.

===========================================================================
3. assets/sprites/props.png   128x1024, 32x32 cells, 4 frames x 32 rows
===========================================================================
Frame f of row r is at (32*f, 32*r).  Every prop is bottom-centre anchored:
it stands on the cell's bottom row (y = 31, the outline; art ends at y = 30)
and is centred on x = 16.  Soft glows / steam / spray are translucent.

  row  prop              frames
   0   bed (empty)       status light: on, on, on, dim
   1   bed occupied      sleeper under blanket, 4-frame breathing (+ "z")
   2   chest             closed, opening, open, open (glint variant)
   3   suit rack         3 suits, 2 suits, 1 suit, empty
   4   synthesizer       4-frame working loop (scan beam, item growing)
   5   comm terminal     4-frame loop (screen wave, beacon, signal arcs)
   6   mod bay           4-frame loop (ring lights chase, arm moves)
   7   planter           dry, damp, wet, wet + fertilized sparkle
                         (trough x 3..28, soil surface at y = 22..23)
   8   dark planter      dry, damp, wet, wet + fertilized (log bed,
                         substrate surface at y = 22..23)
   9   aqua planter      4-frame water shimmer (tank x 4..27, y 11..30,
                         water surface at y = 14)
  10   cooker            4-frame loop (fire, steam)
  11   compost vat       4-frame bubbling loop
  12   assembly ring     4-frame loop (arms weld a tiny dome; panes fill in)
  13   fish pool         4-frame loop (ripples, fish) (tank x 1..30, y 19..30)
  14   animal stall      empty wooden pen (4 identical frames)
  15   feed trough       empty, 1/3, 2/3, full
  16   condenser         4-frame dripping loop
  17   colonist berth    two stacked pods: empty dark, empty lit, occupied,
                         occupied (alternate)
  18   sprinkler head    4-frame spray loop
  19   water tap         4-frame drip loop (drop falls, splashes on frame 3)
  20   lamp              pole lamp, 4-frame subtle flicker (soft halo)
  21   charging pylon    empty/dim, 2 flowers, 4 flowers, 6 flowers + glow
  22   pump              4-frame piston loop
  23   water tank        sight glass: empty, 1/3, 2/3, full
  24   oxygen generator  4-frame loop (fan, algae bubbles)
  25   gene splicer      4-frame loop (helix turns, seeds pulse)
  26   drop pod          closed, opening, open, open (glint variant)
  27   lost pack         beacon blink: on (tight halo), on (wide halo), off, off
  28   map pin flag      4-frame wave (the pole is the x = 15..16 anchor,
                         the pennant flies to the right)
  29   crate             empty, some, full, full
  30   bunk (worker bed) status light: on, on, on, dim
  31   dome kit crate    4-frame idle shimmer

===========================================================================
4. assets/sprites/fx.png   128x128, 16x16 cells, 8 columns x 8 rows
===========================================================================
Cell (col, row) is at (16*col, 16*row).
  row 0  col 0-3 laser bolt travelling right (8x3 core, centred on (8,8))
         col 4-7 big charged bolt (about 12x6, centred)
  row 1  col 0-5 bolt impact burst (centred); col 6-7 unused
  row 2  col 0-3 muzzle flash bursting right from (2,8)
         col 4-7 rocket flame pointing down, top-centre anchored at (8,0)
  row 3  col 0-5 dust puff (ground at the bottom of the cell); 6-7 unused
  row 4  col 0-5 water splash, bottom-centre anchored (surface = row y 15);
         col 6-7 unused
  row 5  col 0-3 pickup sparkle (centred); col 4-7 heal plus-signs rising
  row 6  col 0-5 leaf burst (centred; for harvesting); col 6-7 unused
  row 7  col 0-3 electric spark (centred)
         col 4-7 shield bubble arc (bulging right, fading)

===========================================================================
5. assets/sprites/foliage.png   128x96, 16x16 cells, 8 columns x 6 rows
===========================================================================
Single still frames of decorative plants.  Rows 0, 1, 3, 4, 5 are ground
plants, bottom-centre anchored (root on y = 15, around x = 8).  Row 2 is
HANGING, top-centre anchored (attached on y = 0, around x = 8).
  row 0  grass / sedge tufts (teal-green): short, tall, wind-bent,
         seed heads, low wide, curled tips, tall with cyan dots, thick
  row 1  surface plants: fern, fiddlehead + leaf, twin fiddleheads,
         glow bulb, broad-leaf rosette, cyan star flowers, magenta bells,
         amber pod plant
  row 2  hanging: vine, twin vines, root tangle, moss curtain, three vines
         with lime buds, curled tendril, moss curtain with drips, bone roots
  row 3  cave plants: pale fungi, pale spire fungi, cyan glow bulb, lime
         glow bulbs, cyan crystal-moss, magenta crystal-moss, lime shelf
         fungi, pale fern with magenta tips
  row 4  water edge: reeds, reeds with seed heads, lily pads + flower,
         amber cattails, horsetails, lily pads + bud, arching rushes,
         float bulbs
  row 5  deep / abyss: bone coral, coral fan, ember pod, ember pod cluster,
         thorny tendrils, crossed thorn tendrils, ember-tipped coral,
         thorn bush with ember buds

===========================================================================
6. assets/backgrounds/
===========================================================================
  surface_far.png  512x256  tiles in X only.  Transparent sky.  Hazy,
                   desaturated megastructures (stepped arcology with a
                   habitat ring, broken colossal ring, needle spire,
                   viaduct) behind nearer ruined towers wrapped in creepers.
                   The bottom 60 rows are fully opaque.
  surface_mid.png  512x256  tiles in X only.  Transparent sky.  Nearer alien
                   forest: giant fern-trees, bulbous canopy, hanging-garden
                   root bridges with baskets and vines, luminous specks.
                   Darker and more saturated than far.  The bottom 48 rows
                   are fully opaque.
  cave_far.png     256x256  opaque, tiles in X and Y.  Dark teal rock strata,
                   cracks, faint roots and moss flecks.
  cave_near.png    256x256  alpha, tiles in X and Y.  Sparse dark rock
                   shelves with stalactites and dangling roots.
  deep_far.png     256x256  opaque, tiles in X and Y.  Indigo strata with
                   blue crystal veins and small crystals.
  deep_near.png    256x256  alpha, tiles in X and Y.  Clusters of dark
                   crystal columns.
  abyss_far.png    256x256  opaque, tiles in X and Y.  Columnar basalt with
                   glowing ember cracks.
  abyss_near.png   256x256  alpha, tiles in X and Y.  Basalt slabs with
                   teeth and a few ember glints.
                   (all three far layers: mean luminance 10-13%)
  mist.png         256x256  tiles in X and Y.  Pure white, alpha 0..110:
                   soft cloud noise for drifting mist / fog.
  menu.png         640x400  opaque title backdrop: a glass biodome glowing
                   warmly in a rainy alien jungle at dusk, giant fern-trees,
                   ruined machinery, megastructures, a ringed moon (upper
                   right).  The upper-centre is kept calm for the title.

===========================================================================
7. assets/sprites/ship.png (192x96) and ship_flame.png (256x48)
===========================================================================
  ship.png        the Earth cargo lander, bottom-centre anchored (feet on
                  y = 95, centred on x = 96).  Cargo door and ramp on the
                  right (ramp foot near x = 190).  Three engine bells centred
                  on x = 62, 96 and 128 whose mouths end at y = 79 (put the
                  flame's top row at y = 80).
  ship_flame.png  4 frames of 64x48 (frame f at x = 64*f): exhaust flame
                  pointing down, top-centre anchored at (32, 0) of each
                  frame; attach under an engine bell.

===========================================================================
8. assets/sprites/ui.png   96x48
===========================================================================
  (0,0)   16x16  panel corner for the TOP-LEFT (mirror for the others):
                 riveted bracket, 1 px lines, with a small vine
  (16,0)  16x16  round badge frame (ring with a dark inset centre)
  (32,0)  32x8   divider with a central leaf motif; columns 0-7 and 24-31
                 are a plain 2 px line and can be stretched
  (32,8)  32x8   simple divider (line + centre stud), ends stretchable
  (64,0)  32x16  tab / label plate
  (0,16)  16x16  inventory slot frame
  (16,16) 16x16  slot frame, selected (cyan border)
  (32,16) 16x16  slot frame, favourite (gold star in the top-right corner)
  (48,16) 16x16  slot frame, hotbar (amber corners + bottom tab)
  (0,32)  96x16  six 16x16 dark brushed-metal panel tiles (each tiles with
                 itself; variants: seams, rivets, moss)

===========================================================================
9. assets/fonts/pixel.ttf and title.ttf
===========================================================================
Both: 1024 units per em, 1 font pixel = 128 units, so they are crisp at
font sizes that are multiples of 8 px.  Advance = glyph width + 1 px.
  pixel.ttf  cap height 7 px, x-height 5, ascent 8, descent 2.  ASCII
             32-126 plus  · • … × ← → ↑ ↓ ° ₂  and a .notdef box.
  title.ttf  display face: tall stencil capitals, cap height 9 px, ascent
             10, descent 3.  ASCII 32-126 plus .notdef; lower-case letters
             are 7 px small caps.
"""
import math
import os
import random
import struct
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "assets")
PREVIEW_DIR = None  # set by --preview


# --------------------------------------------------------------------------
# Palette: muted teal, rust, moss, bone, indigo shadows + luminous accents
# --------------------------------------------------------------------------
def hexc(h, a=255):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


INK = hexc("12101a")
CLEAR = (0, 0, 0, 0)
RAMPS = {
    "steel": ["232838", "3a4256", "58647a", "8490a4", "b6c0cc"],
    "bone": ["5c5648", "8f8770", "c4b99a", "e8dfc2", "f8f3e0"],
    "rust": ["3d1c1c", "6e2f22", "a4502c", "cf7a3c", "eaa860"],
    "moss": ["1f3322", "2f5230", "4a7a3a", "77a648", "b4d267"],
    "teal": ["173a40", "1f5257", "2f7a78", "4fa89c", "8fd6c0"],
    "water": ["12284a", "153a5c", "1f6a9a", "3aa0d0", "8ad8f0"],
    "wood": ["2a1a14", "3a2418", "5e3c24", "8a5c34", "b8864e"],
    "gold": ["3e2a10", "6a4a18", "b08628", "e8c040", "fff0a0"],
    "red": ["3a0c18", "5a1420", "a82a34", "e04848", "ff9080"],
    "violet": ["1c0f2e", "2a1640", "4a2a70", "7a4ab0", "b48af0"],
    "indigo": ["14122a", "1a1830", "24223e", "2f2d52", "45437a"],
    "stone": ["24222e", "34323f", "55525f", "7b7884", "a7a4aa"],
    "mag": ["4a1446", "8a2a7a", "d04fb0", "ff8fe0", "ffd0f4"],
    "cyan": ["0e4a5c", "1a8aa0", "3cc8e0", "5ff0ff", "c8fcff"],
    "lime": ["2a4a14", "4a8a20", "7ac832", "b6ff5a", "e6ffb0"],
    "amber": ["5a3410", "b8741c", "f09a28", "ffc04a", "ffe9a0"],
    "grey": ["4c4c4c", "7a7a7a", "a8a8a8", "d4d4d4", "f6f6f6"],
    "copper": ["4a2216", "8a4222", "c46a30", "e89850", "ffd0a0"],
    "patina": ["1c4a44", "2c6e62", "3f8f7a", "7fcfae", "c0f0d8"],
}
RAMPS = {k: [hexc(c) for c in v] for k, v in RAMPS.items()}
WHITE = (255, 255, 255, 255)


def R(name, i=None):
    return RAMPS[name] if i is None else RAMPS[name][i]


def col(c):
    if c is None:
        return None
    if isinstance(c, str):
        return hexc(c)
    if len(c) == 3:
        return (c[0], c[1], c[2], 255)
    return c


def mix(a, b, t):
    a, b = col(a), col(b)
    return (int(round(a[0] + (b[0] - a[0]) * t)), int(round(a[1] + (b[1] - a[1]) * t)),
            int(round(a[2] + (b[2] - a[2]) * t)), int(round(a[3] + (b[3] - a[3]) * t)))


def wa(c, a):
    c = col(c)
    return (c[0], c[1], c[2], int(a))


def scale(c, k):
    c = col(c)
    return (min(255, int(c[0] * k)), min(255, int(c[1] * k)), min(255, int(c[2] * k)), c[3])


BAYER4 = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]


def bayer(x, y):
    return (BAYER4[y & 3][x & 3] + 0.5) / 16.0


def ramp_pick(ramp, v, x, y):
    """Ordered-dithered pick from a colour ramp, v in [0,1]."""
    v = min(1.0, max(0.0, v)) * (len(ramp) - 1)
    i = int(v)
    if i >= len(ramp) - 1:
        return ramp[-1]
    return ramp[i + 1] if (v - i) > bayer(x, y) else ramp[i]


def ramp_soft(ramp, v, x, y, band=0.1):
    """Posterised pick, checker-dithering only near band boundaries."""
    v = min(1.0, max(0.0, v)) * (len(ramp) - 1)
    i = int(v)
    if i >= len(ramp) - 1:
        return ramp[-1]
    f = v - i
    if f < 0.5 - band:
        return ramp[i]
    if f > 0.5 + band:
        return ramp[i + 1]
    return ramp[i + 1] if (x + y) % 2 else ramp[i]


def ramp_lerp(ramp, v):
    v = min(1.0, max(0.0, v)) * (len(ramp) - 1)
    i = min(len(ramp) - 2, int(v))
    return mix(ramp[i], ramp[i + 1], v - i)


def clamp(v, lo=0.0, hi=1.0):
    return lo if v < lo else hi if v > hi else v


def hash2(x, y, s=0):
    h = (x * 374761393 + y * 668265263 + s * 2147483647 + 0x9E3779B9) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((h ^ (h >> 16)) & 0xFFFF) / 65536.0


# --------------------------------------------------------------------------
# Canvas
# --------------------------------------------------------------------------
class Sprite:
    def __init__(self, w, h, fill=CLEAR):
        self.w, self.h = w, h
        self.px = [fill] * (w * h)
        self.wrap = False
        self.wrapx = False
        self.clip = None  # optional set of allowed (x, y)

    def inb(self, x, y):
        return 0 <= x < self.w and 0 <= y < self.h

    def get(self, x, y):
        if self.wrap:
            return self.px[(y % self.h) * self.w + (x % self.w)]
        if self.wrapx:
            x %= self.w
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[y * self.w + x]
        return CLEAR

    def set(self, x, y, c):
        x, y = int(math.floor(x)), int(math.floor(y))
        if self.wrap:
            x %= self.w
            y %= self.h
        else:
            if self.wrapx:
                x %= self.w
            if not (0 <= x < self.w and 0 <= y < self.h):
                return
        if self.clip is not None and (x, y) not in self.clip:
            return
        self.px[y * self.w + x] = c

    def opaque(self, x, y):
        return self.get(x, y)[3] > 0

    def blend(self, x, y, p):
        x, y = int(math.floor(x)), int(math.floor(y))
        if self.wrap:
            x %= self.w
            y %= self.h
        else:
            if self.wrapx:
                x %= self.w
            if not (0 <= x < self.w and 0 <= y < self.h):
                return
        if self.clip is not None and (x, y) not in self.clip:
            return
        if p[3] <= 0:
            return
        q = self.px[y * self.w + x]
        if q[3] == 0 or p[3] >= 255:
            self.px[y * self.w + x] = (p[0], p[1], p[2], min(255, p[3]))
            return
        a = p[3] / 255.0
        qa = q[3] / 255.0
        oa = a + qa * (1 - a)
        self.px[y * self.w + x] = (int(round((p[0] * a + q[0] * qa * (1 - a)) / oa)),
                                   int(round((p[1] * a + q[1] * qa * (1 - a)) / oa)),
                                   int(round((p[2] * a + q[2] * qa * (1 - a)) / oa)), int(round(oa * 255)))

    def add(self, x, y, c, k=1.0):
        """Additive light on an existing opaque pixel (or alpha blend on clear)."""
        x, y = int(math.floor(x)), int(math.floor(y))
        q = self.get(x, y)
        if q[3] == 0:
            self.blend(x, y, wa(c, 255 * min(1.0, k)))
            return
        self.set(x, y, (min(255, int(q[0] + c[0] * k)), min(255, int(q[1] + c[1] * k)),
                        min(255, int(q[2] + c[2] * k)), q[3]))

    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)

    def brect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.blend(x, y, c)

    def hline(self, x0, x1, y, c):
        for x in range(min(x0, x1), max(x0, x1) + 1):
            self.set(x, y, c)

    def vline(self, x, y0, y1, c):
        for y in range(min(y0, y1), max(y0, y1) + 1):
            self.set(x, y, c)

    def line(self, x0, y0, x1, y1, c):
        for (x, y) in line_pts(x0, y0, x1, y1):
            self.set(x, y, c)

    def bline(self, x0, y0, x1, y1, c):
        for (x, y) in line_pts(x0, y0, x1, y1):
            self.blend(x, y, c)

    def fill(self, pts, c):
        for (x, y) in pts:
            self.set(x, y, c)

    def bfill(self, pts, c):
        for (x, y) in pts:
            self.blend(x, y, c)

    def outline(self, c=INK, tint=0.16, diag=False, thresh=200):
        """1 px outline around opaque pixels: ink tinted by the local colour."""
        add = []
        nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        if diag:
            nb += [(1, 1), (-1, 1), (1, -1), (-1, -1)]
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y * self.w + x][3] >= thresh:
                    continue
                acc = [0, 0, 0]
                n = 0
                for dx, dy in nb:
                    q = self.get(x + dx, y + dy)
                    if q[3] >= thresh:
                        acc[0] += q[0]
                        acc[1] += q[1]
                        acc[2] += q[2]
                        n += 1
                if n:
                    add.append((x, y, mix(c, (acc[0] // n, acc[1] // n, acc[2] // n, 255), tint) if tint else c))
        for x, y, cc in add:
            self.px[y * self.w + x] = cc

    def blit(self, other, dx, dy):
        for y in range(other.h):
            for x in range(other.w):
                p = other.px[y * other.w + x]
                if p[3] == 255:
                    self.set(dx + x, dy + y, p)
                elif p[3]:
                    self.blend(dx + x, dy + y, p)

    def copy(self):
        s = Sprite(self.w, self.h)
        s.px = list(self.px)
        s.wrap, s.wrapx = self.wrap, self.wrapx
        return s

    def points(self):
        return {(i % self.w, i // self.w) for i, p in enumerate(self.px) if p[3]}

    def bbox(self):
        pts = self.points()
        if not pts:
            return None
        return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))

    def crop(self, x0, y0, w, h):
        s = Sprite(w, h)
        for y in range(h):
            for x in range(w):
                s.px[y * w + x] = self.get(x0 + x, y0 + y)
        return s

    def flipx(self):
        s = Sprite(self.w, self.h)
        for y in range(self.h):
            for x in range(self.w):
                s.px[y * self.w + x] = self.px[y * self.w + (self.w - 1 - x)]
        return s


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


def curve_pts(p0, p1, p2, n=24):
    """Quadratic bezier as a deduplicated pixel list."""
    out = []
    last = None
    for i in range(n + 1):
        t = i / n
        x = (1 - t) ** 2 * p0[0] + 2 * t * (1 - t) * p1[0] + t * t * p2[0]
        y = (1 - t) ** 2 * p0[1] + 2 * t * (1 - t) * p1[1] + t * t * p2[1]
        q = (int(round(x)), int(round(y)))
        if last is not None and q != last:
            for pt in line_pts(last[0], last[1], q[0], q[1])[1:]:
                out.append(pt)
        elif last is None:
            out.append(q)
        last = q
    return out


def m_rect(x0, y0, x1, y1):
    return {(x, y) for y in range(y0, y1 + 1) for x in range(x0, x1 + 1)}


def m_ellipse(cx, cy, rx, ry):
    pts = set()
    for y in range(int(math.floor(cy - ry)) - 1, int(cy + ry) + 2):
        for x in range(int(math.floor(cx - rx)) - 1, int(cx + rx) + 2):
            if ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1.0:
                pts.add((x, y))
    return pts


def m_poly(poly):
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    pts = set()
    for y in range(int(math.floor(min(ys))) - 1, int(max(ys)) + 2):
        for x in range(int(math.floor(min(xs))) - 1, int(max(xs)) + 2):
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


def shade_fill(sp, pts, ramp, hi=True):
    """Fill a mask from a ramp (dark, base, light[, hi]) with top-left lit and
    bottom-right shaded edges."""
    S = pts if isinstance(pts, set) else set(pts)
    dark, base, light = ramp[0], ramp[1], ramp[2]
    hic = ramp[3] if len(ramp) > 3 and hi else None
    for (x, y) in S:
        up = (x, y - 1) not in S
        lf = (x - 1, y) not in S
        dn = (x, y + 1) not in S
        rt = (x + 1, y) not in S
        if hic and up and lf:
            c = hic
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


def box(sp, x0, y0, x1, y1, ramp, hi=False):
    """Bevelled box: ramp = (dark, base, light[, hi])."""
    sp.rect(x0, y0, x1, y1, ramp[1])
    sp.hline(x0, x1, y1, ramp[0])
    sp.vline(x1, y0, y1, ramp[0])
    sp.hline(x0, x1, y0, ramp[2])
    sp.vline(x0, y0, y1 - 1, ramp[2])
    if hi and len(ramp) > 3:
        sp.set(x0, y0, ramp[3])


def cyl(sp, x0, y0, x1, y1, ramp):
    """Upright cylinder shaded across its width (lit from the left).
    ramp: 4-5 tones dark->light."""
    w = x1 - x0
    n = len(ramp)
    for x in range(x0, x1 + 1):
        t = (x - x0) / max(1, w)
        if t < 0.12:
            c = ramp[n - 3]
        elif t < 0.34:
            c = ramp[n - 2] if w < 5 else ramp[n - 1] if 0.16 < t < 0.3 else ramp[n - 2]
        elif t < 0.62:
            c = ramp[n - 3]
        elif t < 0.86:
            c = ramp[max(0, n - 4)]
        else:
            c = ramp[0]
        sp.vline(x, y0, y1, c)


def hcyl(sp, x0, y0, x1, y1, ramp):
    """Horizontal pipe shaded across its height (lit from the top)."""
    h = y1 - y0
    n = len(ramp)
    for y in range(y0, y1 + 1):
        t = (y - y0) / max(1, h)
        if t < 0.2:
            c = ramp[n - 2]
        elif t < 0.4:
            c = ramp[n - 1] if h >= 3 else ramp[n - 2]
        elif t < 0.7:
            c = ramp[n - 3]
        else:
            c = ramp[max(0, n - 4)]
        sp.hline(x0, x1, y, c)


def ball(sp, cx, cy, r, ramp, dither=False, mask=None):
    """Sphere lit from the top-left."""
    L = (-0.56, -0.66, 0.5)
    n = len(ramp)
    for y in range(int(math.floor(cy - r)) - 1, int(cy + r) + 2):
        for x in range(int(math.floor(cx - r)) - 1, int(cx + r) + 2):
            dx, dy = (x + 0.5 - cx) / r, (y + 0.5 - cy) / r
            d2 = dx * dx + dy * dy
            if d2 > 1.0 or (mask is not None and (x, y) not in mask):
                continue
            nz = math.sqrt(1 - d2)
            v = (dx * L[0] + dy * L[1] + nz * L[2] + 0.42) / 1.42
            sp.set(x, y, ramp_pick(ramp, v, x, y) if dither else ramp_soft(ramp, v, x, y))


def glow(sp, cx, cy, r, c, amax=120, power=1.6):
    """Soft radial halo blended over whatever is there."""
    for y in range(int(cy - r) - 1, int(cy + r) + 2):
        for x in range(int(cx - r) - 1, int(cx + r) + 2):
            d = math.hypot(x + 0.5 - cx, y + 0.5 - cy) / r
            if d < 1:
                sp.blend(x, y, wa(c, amax * (1 - d) ** power))


def glow_under(sp, cx, cy, r, c, amax=110, power=1.5):
    """Halo only on pixels that are currently transparent (keeps art crisp)."""
    for y in range(int(cy - r) - 1, int(cy + r) + 2):
        for x in range(int(cx - r) - 1, int(cx + r) + 2):
            if not sp.inb(x, y) or sp.px[y * sp.w + x][3]:
                continue
            d = math.hypot(x + 0.5 - cx, y + 0.5 - cy) / r
            if d < 1:
                a = int(amax * (1 - d) ** power)
                if a > 6:
                    sp.px[y * sp.w + x] = wa(c, a)


def stamp(sp, rows, ox, oy, key):
    for y, r in enumerate(rows):
        for x, ch in enumerate(r):
            c = key.get(ch)
            if c is not None:
                sp.set(ox + x, oy + y, c)


def write_png(path, sp):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    raw = bytearray()
    w = sp.w
    for y in range(sp.h):
        raw.append(0)
        for p in sp.px[y * w:(y + 1) * w]:
            raw.extend(p)

    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    data = b"\x89PNG\r\n\x1a\n"
    data += chunk(b"IHDR", struct.pack(">IIBBBBB", sp.w, sp.h, 8, 6, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    data += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(data)


# --------------------------------------------------------------------------
# Tileable noise
# --------------------------------------------------------------------------
class TNoise:
    def __init__(self, rng, px, py=None):
        self.px, self.py = px, py or px
        self.g = [rng.random() for _ in range(self.px * self.py)]

    def at(self, x, y):
        px, py = self.px, self.py
        xi, yi = int(math.floor(x)), int(math.floor(y))
        fx, fy = x - xi, y - yi
        fx = fx * fx * (3 - 2 * fx)
        fy = fy * fy * (3 - 2 * fy)
        x0, x1 = xi % px, (xi + 1) % px
        y0, y1 = yi % py, (yi + 1) % py
        g = self.g
        a = g[y0 * px + x0] + (g[y0 * px + x1] - g[y0 * px + x0]) * fx
        b = g[y1 * px + x0] + (g[y1 * px + x1] - g[y1 * px + x0]) * fx
        return a + (b - a) * fy


class FBM:
    """Fractal noise that tiles with period (w, h) pixels."""

    def __init__(self, rng, base, octaves, pers=0.5, w=256, h=None, basey=None):
        h = h or w
        basey = basey or max(1, int(round(base * h / w)))
        self.n = [TNoise(rng, base * (2 ** o), basey * (2 ** o)) for o in range(octaves)]
        self.a = [pers ** o for o in range(octaves)]
        self.tot = sum(self.a)
        self.w, self.h = w, h

    def __call__(self, x, y):
        v = 0.0
        for n, a in zip(self.n, self.a):
            v += a * n.at(x * n.px / self.w, y * n.py / self.h)
        return v / self.tot


class Worley:
    def __init__(self, rng, cells, size=256):
        self.c = cells
        self.cs = size / cells
        self.pts = [(rng.random(), rng.random()) for _ in range(cells * cells)]

    def __call__(self, x, y):
        cs, c = self.cs, self.c
        cx, cy = int(x // cs), int(y // cs)
        d1 = d2 = 1e9
        for oy in (-1, 0, 1):
            for ox in (-1, 0, 1):
                gx, gy = cx + ox, cy + oy
                px, py = self.pts[(gy % c) * c + (gx % c)]
                d = math.hypot((gx + px) * cs - x, (gy + py) * cs - y)
                if d < d1:
                    d1, d2 = d, d1
                elif d < d2:
                    d2 = d
        return d1, d2


def lum(c):
    return (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]) / 255.0


# --------------------------------------------------------------------------
# Icons (icons.png): ASCII pixel art, auto-centred in 16x16 and auto-outlined
# --------------------------------------------------------------------------
def _key():
    k = {"#": WHITE, "K": INK}
    for chars, ramp, idx in (("12345", "steel", (0, 1, 2, 3, 4)), ("67890", "bone", (0, 1, 2, 3, 4)),
                             ("rRoO", "rust", (1, 2, 3, 4)), ("gGmMl", "moss", (0, 1, 2, 3, 4)),
                             ("tTeEa", "teal", (0, 1, 2, 3, 4)), ("bBwW", "water", (1, 2, 3, 4)),
                             ("cC", "cyan", (3, 4)), ("L", "lime", (3,)), ("yYh", "amber", (1, 3, 4)),
                             ("pPqQ", "mag", (1, 2, 3, 4)), ("dDnN", "wood", (1, 2, 3, 4)),
                             ("xXvV", "red", (1, 2, 3, 4)), ("uUiI", "violet", (1, 2, 3, 4)),
                             ("jJ", "indigo", (1, 3)), ("fFAH", "gold", (1, 2, 3, 4)),
                             ("zZsS", "stone", (1, 2, 3, 4))):
        for ch, i in zip(chars, idx):
            k[ch] = RAMPS[ramp][i]
    return k


KEY = _key()
GREYKEY = dict(KEY)
for _ch, _c in zip("67890", ("707070", "9a9a9a", "c4c4c4", "e6e6e6", "ffffff")):
    GREYKEY[_ch] = hexc(_c)

ICONS = {}


def icon(i, art, key=None, outline=True, dy=0, dx=0):
    ICONS[i] = (art, key, outline, dx, dy)


def ascii_sprite(art, key=None, w=16, h=16, outline=True, dx=0, dy=0, bottom=None):
    rows = [r for r in art.strip("\n").split("\n")]
    aw = max(len(r) for r in rows)
    ah = len(rows)
    if outline and (aw > w - 2 or ah > h - 2):
        raise SystemExit("ascii art too big (%dx%d):\n%s" % (aw, ah, art))
    sp = Sprite(w, h)
    ox = (w - aw) // 2 + dx
    oy = (h - ah) // 2 + dy if bottom is None else bottom - ah + 1
    stamp(sp, rows, ox, oy, key or KEY)
    if outline:
        sp.outline()
    return sp


icon(0, """
....SSSs....
..sSS#SssZ..
.sSSSsssZZZ.
sSSssZssZZZz
sSssZZZZsZzz
sssZZZzZZzzz
ZsZZZzzZzzzz
.ZZZzzzzzzz.
..ZzzzzzzZ..
""")
icon(1, """
......M.....
....m.Mm.M..
...nNNMnnm..
.nNNNnnnnDD.
nNNnnnDnnDDD
nNnnDnnnDDDd
nnnsnnDDDDdd
DnnDDDDDsDdd
.DDDDDdddDd.
..DDddddd...
""")
icon(2, """
.....H0.....
....H09A....
...H099AA...
..H0999A9A..
.H0999A99AF.
.09A999A9AAF
H99999AA9AFF
9A99A9AAAFFf
.AAAAAFFFFf.
""")
icon(3, """
..NNNNNNNNNn..
.nNnNNnNNNNnD.
n0NnDnnnnnnnDD
0NN0DnnnDnnnDd
0N0NDnDnnnnDDd
n0NnDnnnnDnDDd
.nnnDDDDDDDDd.
..DDDDDDDDDd..
""")
icon(4, """
.M..l..M.m..
.Ml.lM.Mm.m.
..MlMMmMm.m.
..MlMlmMmm..
...MlMmMm...
...N0NnNn...
...nNnnDD...
...MlMmMm...
..MMlMmmMm..
.MM.lM.mMmm.
.M..l..m..m.
""")
icon(5, """
....4554....
..445#5543..
.44554433Z2.
3455443ZZ322
345433Z43322
3443Z3443221
.33ZZ3432211
.Zz3332221z.
..zZz2211z..
""")
icon(6, """
....oOOo....
..oOO#OoRR..
.oOOoooERRr.
RoOooEEaERRr
RooRoEERRRrr
RoRRRooRrErr
.RRERRRrrEEr
.ZRREErrrrr.
..ZzZrrrzz..
""")
icon(7, """
......#5....
.....#554...
....#5544...
....#5543...
...#55443.5.
...#5543.#54.
..#55443.#43.
..#5544335432
.#5544332432.
.Z554433Z33Z.
ZZZ4433ZZZZz.
.zzZZZzzzzz..
""")
icon(8, """
....AHHA....
..AAH#HAAF..
.AHHAAAAFFF.
AH#AAAFAAFFf
AHAAAFFAFFff
AAAFAAFFFFff
.FAAFFFFfff.
..FFFfffff..
""")
icon(9, """
.....##5....
...##55544..
.##55555443.
#555555444332
.4555544433221
..45554433221.
...454433221..
....4433221...
.....33221....
......221.....
""")
icon(10, """
.....Q......
....QqP.....
....QqP..Q..
...Q#qPp.qP.
...QqPPpQqPp
.Q.QqPPpQ#Pp
.qPQqPPpqPPp
QqPpqPPpqPpp
Q#PpqPppqPpu
.qPppPppPpu.
..uupppuuu..
""")
icon(11, """
.....#......
....CCcA....
..CC#CcAH...
.CCCccAAHA..
.CccEcAH#AF.
CccEEAAAAFF.
.cEEeAAFAFF.
.EeecAFFFf..
..eeEcFff...
....e.f.....
""")
icon(12, """
...iIIIIi...
.iII#IIiiUU.
iIIiiiiUUUUu
IiiUUUUUuuuu
.UiIIIIiUUu.
.iIIiiiUUUu.
.IiiUUUUuuu.
..UiIIiUUu..
..iIiiUUuu..
...UUuuuu...
""")
icon(13, """
.NNNNNNNNNn.
.nnnnnnnnDD.
..DDDDDDDD..
..#0000998..
..00099988..
..#0909998..
..00099888..
..#0998988..
..09998887..
..nNNNNNnn..
.NNNNNNNNNn.
.nnnnDDDDDD.
""")
icon(14, """
...344432...
...233221...
..5aaaaaa3..
.5a#aaaaaa3.
.5lLLLLLLM3.
.5L#LLLLMM3.
.5LLLLLMMm3.
.5LLlLMMMm3.
.5LLLMMMmm3.
.5MLMMMmmG3.
..3MMmmmG3..
...333332...
""")
icon(15, """
444444444443
4aaaaaaWWaa2
4aaaaaWWaaE2
4aa#aWWaaEE2
4a#aWWaaEEE2
4aaWWaaEEEE2
4aWWaaEEEaE2
4WWaaEEEaaE2
4WaaEEEaaEE2
4aaEEEaaEEE2
4aEEEEEEEEE2
322222222222
""")
icon(16, """
....7007....
...79dd97...
....7997....
..79000987..
.7900000987.
.790Ml09987.
7900MMm99887
790mMlM99887
7900mMG98887
79990G998887
.7999998887.
..77888877..
""")
icon(17, """
...#....#...
....#..#....
...#..#..#..
....#...#...
............
3555555555542
.4OoMoOoMoR3.
.35OOoMoORR2.
..345554432..
...3344322...
....22221....
""")
icon(18, """
....7997....
...790097...
..79#00997..
..7900u998..
.790u009998.
.7900009u98.
.79u0099988.
.7900u99887.
.7990999887.
..79998u87..
..7899887...
....7777....
""")
icon(19, """
....3443....
....2332....
....5aa3....
...5a##a3...
..5a#000a3..
..5#000093..
..5000009E..
..5000099E..
..5BwwwwBE..
..5000999E..
..5009999E..
...E9998E...
""")
icon(20, """
............a.
...EaaaE...aE.
.EaaaaaaaE.aEe
Ea#aaaEEEEaEe.
aaKaaEEEeEEe..
EaaaEEEeeeEEe.
.EEEEeeeTe.eEe
...eeeTT...eT.
.....e......e.
""")
icon(21, """
...3333.44..
..34443.4...
.3555554443.
.35wWWWW#42.
.35WW#Www42.
.35WwwwwB42.
.35wwwwBB42.
.35wwwBBB42.
.35wwBBBb42.
.34444444321
..2222222211
""")
icon(22, """
.........4..
...4444..43.
..4....4.43.
.4.4555543..
.4.45#54332.c
.4.455443324.
..4455443321.c
...45443332.c.
...44433322..
...44333222...
....2222221...
""")
icon(23, """
....3443....
...4YhhY4...
..4Yh##hY3..
..4Yh#hhY3..
..4YhhhYy3..
..4YYhYYy3..
...4YYYy3...
....3443....
....2332....
...334432...
..33444322..
""")
icon(24, """
.NNNNNNNNNNn.
NNnnnnnnnnnDD
N44nnn44nnn4D
Nnnnn4AA4nnDD
4444444AH4444
3333334AA3333
NnnnnnD44nnDD
NnnnnnnnnnDDD
N44nnnnnnn4Dd
nDDDDDDDDDDDd
.dddddddddd..
""")
icon(25, """
.....cC.....
....3c#4....
....34c3....
..L.3443.L..
..lL3553Ll..
....3443....
..L.3553.L..
..lL3443Ll..
....3553....
...344443...
..34455432..
..22222221..
""")
icon(26, """
....3443....
....3553....
..33455433..
.3455555543.
.45#wwwW542.
.45wWWwww42.
.45wwwwBw42oO
.35wwwBBB3oOo
.345555543oo.
..3344332.R..
.3444443322.
.222222221..
""")
icon(27, """
..33444433..
.3455555543.
.45#5555442.
.4WWWWWWWW2.
.4W#wwwwww2.
.4WwwwwwwB2.
.4wwwwwwBB2.
.4wwwwwBBB2.
.4wwwBBBBb2.
.3444444432.
.3.3....3.2.
.2.2....2.1.
""")
icon(28, """
..............
.OOOOOOOO4....
oOO#OOOOo45...
ooooEoooo4oO..
RRRRRRRRo4OOo.
rrrrrrrRo4oEoR
.......4444445
.......oOOoRr.
.......oOooRr.
.......oEooRr.
.......oOooRr.
""")
icon(29, """
...344443...
..3#C##C42..
.35C4CC4C42.
.34#C22C#42.
.35C4CC4C42.
..3CC##C42..
...344432...
..MlM33MmM..
.3MMlMMmMm2.
.34MMmMmm32.
.3344443322.
.222222221..
""")
icon(30, """
.....c......
.....4......
..34555543..
.3555555542.
.45KKKKKK42.
.45KcCKcK42.
.45KCcKCK42.
.45KKKKKK42.
.3455554432.
..3OO44RR2..
...344432...
....2..2....
""")
icon(31, """
.34444444432.
3555555555442
35KKKKKKK5Y42
35KcCCcCK5442
35KC#CCcK5v42
35KcCcccK5442
35KKKKKKK5L42
3444444444432
.33OO3333332.
.3.oO.....2..
.2........1..
""")
icon(32, """
..cC....Lc..
..c4....4c..
...cC..Lc...
....c4Lc....
.....cL.....
....Lc4c....
...Lc..cC...
..L4....4c..
..Lc....cC..
.3444444443.
.35Y55v5542.
.2222222221.
""")
# 33..42 are dome kits (procedural, see icons_sheet)
icon(43, """
.....EaE....
....Ea#aE.Y.
.Y..aaKaaYy.
.yY.EaaaE...
..EaaaaaaE..
.Ea#aaaEEEe.
.aaaaaEEEEe.
.EaaEEEEeee.
..EEEEeeeT..
...eY..Yt...
...yY..Yy...
""")
icon(44, """
..............
....0000......
..0090#009....
.00#00900998..
.09000090098#.
099909900988K8
099999999988v.
.8999898888...
..88888877....
..y.y.y.y.....
""")
icon(45, """
....5aa5....
....3NN3....
...5aaaa5...
..5aa#aaa5..
.5aaoaaaaa5.
.5a#aaaOoa5.
.5WwwWwwWw5.
.5wOowwwww5.
.5wwwwwoOw5.
.5wwOowwww5.
..5wwwwww5..
...555555...
""")
icon(46, """
...8....8...
...78..87...
....7997....
...768867...
..79000997..
.7900000987.
.790#009987.
790000979887
790009777887
799009978887
.7999998887.
..78888877..
""", key=GREYKEY)
icon(47, """
.......8....
....898.....
...990.8....
.....77.....
...7900987..
..790#00987.
.790#0009987
.79000099887
.79000999887
.79909998877
..799998877.
...7788877..
""", key=GREYKEY)
icon(48, """
..........cC..
.........3c...
..345554435...
.35#55554422..
3555OO544421..
.345OO4433....
..3443322.....
..NNn.........
.NNnD.........
.NnDD.........
.nDD..........
""")
icon(49, """
..............
..........c...
...ccc.cCCCc..
.cccCCCC###CCc
...ccc.cCCCc..
..........c...
..............
""")
icon(50, """
...AHHHHA...
..AHAAAAAF..
.AHAwwMwAAF.
AHAwMMMwwAFF
AHAMMMwwwAFF
AHAwMMwMMAFF
AHAwwMwMMAFf
AAAwwwwMwAFf
.FAAwwwwAFf.
..FFAAAAFf..
...FFFFff...
""")
icon(51, """
...aCCCa....
..C#aaaaE...
.C#a...aaE..
.Ca.CC..aE..
.Ca.C.C.aE.a
.Ca.CC..EE.Ea
..Ea...EE...
...EEEEE..CC.
............C
...........C.
..........CCC
""")
icon(52, """
....3443....
...344443...
...4L##L3...
...4LLLL3...
...4L#LL3...
...4LLLM3...
...4LLMM3...
...4MMMM3...
...4MMMm3...
...4MMmm3...
...322221...
""")
icon(53, """
..vVv...vvv..
.vV#Vv.vvvXv.
vV#Vvvvvvvvxv
vVVvvvvvvvXXx
vvvvvvvvvXXXx
.vvvvvvvXXXx.
..vvvvvXXXx..
...vvXXXXx...
....XXXXx....
.....XXx.....
......x......
""")
icon(54, """
.....W......
.....W......
....WWw.....
....W#w.....
...WW#ww....
...W#wwwB...
..WW#wwwB...
..W#wwwwBB..
..Wwwwwwwb..
..WwwwwwBb..
...wwwBBb...
....BBbb....
""")
icon(55, """
.9.9.9...lM.
.9.9.9..lMMm
.9.9.9.lMlMm
.99999.MMlMm
..999..MlMMm
...9...mMMm.
...9....Mm..
...9....m...
...9....G...
...9....G...
...8....G...
""")
icon(56, """
.....66.....
...6699966..
.66999999966
.3555555542.
.35Yh5Yh542.
.35YY5YY542.
.3555555542.
.35Yh5KK542.
.35YY5K3542.
.3555553542.
.2222222221.
""")
icon(57, """
......hhhY..
.....hhhY...
....hhhY....
...hhhY.....
..hhhhhhY...
..YYYhhhY...
....hhhY....
...hhhY.....
...hhY......
..hhY.......
..hY........
..Y.........
""")
icon(58, """
...344443...
..34....42..
..34....42..
..34....42..
.AHHHHHHHAF.
.AHAAAAAAFF.
.AAAAKKAAFF.
.AAAAKKAFFF.
.AAAAAKAFFF.
.AAAAAAFFFf.
.FFFFFFFFff.
""")
icon(59, """
....3443....
.3455555432.
.2222222221.
..45454543..
..45454543..
..45454543..
..45454543..
..45454543..
..35454542..
..34343432..
...333322...
""")
icon(60, """
......H......
.....AHA.....
.....AHA.....
.HHHHH#AAAAFF
..AH#AAAAAFF.
...AHAAAAFF..
...AAAAAAFF..
..AHAAFAAAFF.
..AAFF.FFAFF.
.AFF.....FFF.
""")
icon(61, """
...vVVVv....
..vV#VVvX...
.vV#000vvX..
.vVV000vXX..
.vvV000vXX..
.vvvvvvXXx..
..vvvvXXx...
..XvvXXXx...
...XvXXx....
....XXx.....
....Xx......
.....x......
""")
icon(62, """
...YhhhhY...
..Yh#hYYYy..
.YhhYYYYYyy.
.YYYYYYYYyy.
yyyyyyyyyyyy
..90000998..
..90K09K98..
..90000998..
..79009987..
...799887...
..33455433..
.3345554432.
""")
icon(63, """
..........BW
.......wWWBw
....wWWWwwwB
..wWW#WwwBBB
.wW#WwwwBBwb
.BwwwwBBwWBb
.BBwBBwWWwb.
.B#BBwWwBb..
.BWwBbBBb...
.BwwBbb.....
..BBb.......
""")
icon(64, """
.c...c..c...c
..c.c....c.c.
c..c..55..c..
.c...3443...c
....345543...
...34555432..
.....343.....
.....353.....
.....343.....
....33432....
...3444322...
""")
icon(65, """
....M...l...
...m.Mm.M...
..NNnMNnNn..
.NnndnMnnnD.
.nNNnnnnDDD.
.nDnnMnDnDD.
.nnMmnnnDDD.
.nDnnnDmDDd.
.nnnDnnDDDd.
.DnDDDDDDdd.
..DDDDdddd..
""")
# body-slot pictograms: bone on a faint body silhouette, highlighted part bright
icon(66, """
....0000....
...000009...
..00#00099..
..000000998.
..000000998.
..099009988.
...9999988..
....99988...
....7887....
..66777766..
.6666666666.
""")
icon(67, """
............
............
..0000..0000.
.0#00990#0099
.00KK9900KK99
.00KK9900KK98
.90009890009.
..9998..9998.
............
""")
icon(68, """
..00.7887.99.
.000978879998
0000000099998
0900#00099898
09.00000998.8
08.00009998.8
...0009998...
...0099988...
...9999888...
...7777777...
""")
icon(69, """
...4555543..
..35#555542.
..35555554200
.335444454209
.345555554208
.343555534298
.345555554298
.345544554288
.345555554277
.334444443277
..2222222217.
""")
icon(70, """
.000........
00#09.......
000999......
.00999......
.009998.....
.0099980000.
.00999000#09
..9990009999
...999999998
....9988888.
""")
icon(71, """
....0.0.....
..0.0.0.0...
..0.0.0.9...
..0.0.0.9...
..0.0.0.9.0.
..00#0099.09
..000009909.
..00000999..
..0000999...
...009998...
...79988....
...7888.....
""")
icon(72, """
..00000999..
..0#0009998.
..000.09998.
..009..9998.
..009..9988.
..099..9988.
..099..9988.
..099..9888.
..998..9888.
..998..9887.
..887..8877.
""")
icon(73, """
...0009.....
...0#09.....
...0009.....
...0099.....
...0099.....
...00999....
...000999990.
...0#0009999.
...000999988.
...777777777.
...333333332.
""")
icon(74, """
0000000..#..
9999999.#C#.
........c.c.
00000.....c.
99999.....c.
........c.c.
000.....cCc.
999......c..
""")
icon(75, """
.....c......
....cCc.....
...cC#Cc....
.....C......
.....c......
.NNNNNNNNNn.
.nnn4AA4nDD.
.4444AH4444.
.nnnn44nDDD.
.nnnnnnnDDd.
.DDDDDDDDdd.
""")
icon(76, """
..c..c..c...
.cCccCccCc..
..c..c..c...
............
.NNNNNNNNNn.
.nKKKKKKKKD.
.nnn4AA4nDD.
.4444AH4444.
.nnnn44nDDD.
.nnnnnnnDDd.
.DDDDDDDDdd.
""")
icon(77, """
...34443....
..4aaaaE3...
.4a#aaaaE2..
.4#aaaaaE2..
.4aaaaaEE2..
.4aaaaEEe2..
..3EEEEe2...
...32222N...
........NNn.
.........NNn
..........nD
""")
icon(78, """
.....Yh.....
....YhhY....
....hhhY....
...Yh44hY...
...hhKKhY...
..YhhKKhhY..
..hhhKKhhY..
.YhhhKKhhhY.
.hhhhhhhhhY.
YhhhhKKhhhhY
YYYYYYYYYYyy
""")
icon(79, """
..........L.
.........LLM
........LLM.
.......LLM..
.L....LLM...
LLM..LLM....
.LLMLLM.....
..LLLM......
...LM.......
""")
icon(80, """
.....h......
..h.....h...
....YhhY....
...Yh#hhY...
.h.hhhh000..
...Yhh0#0009.
....Y00000099
....000000999
....999999988
""")
icon(81, """
............
..5555554...
.4......3...
....555555554
...4........3
.555555554...
4........3...
...55555555..
..4.......3..
""")
icon(82, """
....5555....
..55#55554..
.5#55555444.
555555544443
.44444443332
............
..w..w..w...
.w..w..w....
...w..w..w..
..w..w..w...
""")
icon(83, """
.....h......
.h...h...h..
..h.....h...
....YhhY....
...Yh#hhY...
hh.hh#hhY.hh
...hhhhYY...
...YhhYYy...
....YYYy....
..h.....h...
.h...h...h..
.....h......
""")
icon(84, """
....0009....
..00099.....
.00#99......
.0009.......
0#099.......
00099.......
00999.......
.0999....8..
.09999..98..
..99999988..
....9888....
""")
icon(85, """
.....L......
.....4......
..34555543..
.3555555542.
.45KKKKKK42.
.45KLLKLK42.
.45KKKKKK42.
.3455554432.
..33444322..
.L3.3443.2L.
..3......2..
""")
icon(86, """
..99....99...
.9009..9009.9
90m0099000990
90MM00w00m099
90mM00w0mMM98
900m00w00Mm98
9000w0w000098
90wwW00X00098
90w0000vX0988
9000099000988
.9998..99988.
..88....888..
""")
icon(87, """
.eEEEEEEEE3.
eEaEEEEEEe03
eEEEElMEEe03
eEEElMMmEe03
eEElMlMmEe03
eEEMMlMmEe03
eEEEmMmEEe03
eEEEEGEEEe03
eEEEEGEEEe03
eeeeeeeeeeT3
.3000000003.
..33333333..
""")
icon(88, """
....4554....
.45.4554.43.
.555555554..
..55533555..
4555....5543
5553....3443
5553....3443
4555....4432
..55544444..
.544444443..
.43.4432.32.
....3322....
""")
icon(89, """
....3443....
....4aa3..C.
....4aa3.C#C
....4aa3..C.
....4aa3....
....4aa3.C..
....4wa3....
....4wa3....
...4awwa3...
...4w#ww3...
...4wwwB3...
....4BB3....
""")
icon(90, """
....LLLL....
...LL..LL...
...L....L...
.LLLL..LLLL.
LL..LLLL..LL
L...L##L...L
L...L##L...L
LL.MLLLLM.LL
.MMMM..MMMM.
....M..M....
.....MM.....
""")
icon(91, """
...000009...
..00#00099..
.000000099 8.
.0KK00KK998.
.0KK00KK998.
.000K009998.
..00009998..
..0K0K0K98..
..9090909...
...99998....
""".replace(" ", "9"))
icon(92, """
.....4444.....
....4L##L3....
.qQ.4LLLL3.Qq.
qQQq4L#LL3qQQp
.qPp4LLLM3PPp.
..pp4MMMM3pp..
....322221....
......mM......
...lM.mM......
..lMMmmM.Mm...
...Mm.mMMMlm..
......mM.mm...
......mG......
""")
icon(93, """
....344443....
..34aaaa3EE3..
.3aa#aa3EEEe3.
.4a#aaa3EEEe3.
3aaaaaa3EEeee2
3aaaaa3EEEeee2
3aaaaa3EEeeeT2
33333333333332
.222222222221.
""")
icon(94, """
............
....oOo.....
....oOo.....
.OOOO#OOOOO.
.oooOOOoooo.
.RRRoOoRRRR.
....oOo.....
....oOo.....
....RoR.....
""")
icon(95, """
.....h......
....hY......
...hY.......
..hhhhY.....
....hY..hY..
...hY..hY...
...Y..hhhhY.
........hY..
.......hY...
.......Y....
""")
icon(96, """
...355554...
..35555554..
.35KKKKKK43.
.3KcC#cccK2.
.3KC#ccccK2.
.3KccccceK2.
.35KccceK42.
.345KKKK432.
..34444432..
.O334443322.
.o3......2..
""")
icon(97, """
............
3...........
3...........
3009........3
3#09vvvvvvv.3
300vVVvvvvXX3
3444444444443
3333333333332
3..........2.
2..........1.
""")
icon(98, """
....wwwW....
..wwMMwWWw..
.wMMMMwwwWw.
.wMMMmwwMMw.
wwMMmwwwMMMB
wwwMwwwwMmwB
wwwwwMMwwwBB
.wwwMMMmwBB.
.BwwwMmwwBB.
..BBwwwBBb..
....BBbb....
""")
icon(99, """
.....00.....
....0#09....
....0009....
...00cc99...
...00cC99...
...000999...
..R0009998..
..0R009998..
.00909R9988.
.09.0998.88.
.9..YhhY..8.
.....YY.....
""")
icon(100, """
..EEE....ooo..
.EaEEe..oOooR.
.EEEEe00ooooR.
..EEe0#09ooR..
.EEEE0009oooR.
EEEEE0099ooooR
EEEE000009oooR
EEE00#00099ooR
EEE000000998oR
...000000998..
""")
_PIP = [".3443.", "345542", "455443", "454432", "344322", ".2321."]


def _pips(n, tr):
    rows = [r.translate(str.maketrans("2345", tr)) for r in _PIP]
    if n == 1:
        out = rows
    elif n == 2:
        out = [r + "." + r for r in rows]
    else:
        out = ["...." + r + "...." for r in rows[:5]] + [rows[0] + ".." + rows[0]]
        out += [r + ".." + r for r in rows[1:]]
    return "\n".join(out)


icon(101, _pips(1, "rRoO"))
icon(102, _pips(2, "2345"))
icon(103, _pips(3, "fFAH"))
icon(104, """
......h......
......#......
.....h#h.....
.Y..hh#hh..Y.
..hhh###hhh..
hh#########hh
..hhh###hhh..
.Y..hh#hh..Y.
.....h#h.....
......#......
......h......
""")
icon(105, """
.....LL.....
....LLLM....
...LLLLMM...
..LLLLLMMM..
.LLLLLLMMMM.
....LLMM....
....LLMM....
....LLMM....
....LLMM....
....MMMm....
""")
icon(106, """
....VvXX....
....vvXX....
....vvXX....
....vvXX....
....vvXX....
.VvvvvXXXXx.
..vvvvXXXx..
...vvvXXx...
....vvXx....
.....Xx.....
""")
icon(107, """
000000000099
999999999988
888888888877
""")
icon(108, """
....0.......
...09.0.0...
...09.9.9.0.
...09.9.9.9.
...09.9.9.9.
.0.0900999.
.09000009998
.09000#09998
..9000009998
..900009998.
...9009998..
....99888...
....8887....
""".replace("0999.\n", "09999\n"))
icon(109, """
.0009..0009.
.0#09..0#09.
.0009..0009.
.0009..0009.
.0009..0009.
.0009..0009.
.0009..0009.
.0998..0998.
.9998..9998.
.8888..8888.
""")
icon(110, """
..00........
..0#00......
..000000....
..00000009..
..0000000099
..0000000998
..00000998..
..000998....
..0998......
..98........
""")
icon(111, """
cc.cc.cc.cc.cc
c............c
..............
c............c
c............c
..............
c............c
c............c
..............
c............c
c............c
..............
c............c
cc.cc.cc.cc.cc
""", outline=False)

DOME_KITS = [
    # (glass ramp, accent, glyph 5x5)
    ("moss", "lime", ["..##.", ".###.", "###..", "##.#.", "...#."]),      # 33 green: leaf
    ("bone", "amber", ["#####", "#.#.#", "##.##", "#.#.#", "#####"]),     # 34 storage: box
    ("rust", "amber", ["#....", "#....", "##...", "#####", "#...#"]),     # 35 bedroom: bed
    ("copper", "bone", [".#.#.", "#####", "#####", "#####", ".###."]),    # 36 kitchen: pot
    ("violet", "mag", [".###.", "###..", "##...", "###..", ".###."]),     # 37 dark: moon
    ("steel", "cyan", [".###.", "#...#", "#.#.#", "#####", "....."]),     # 38 dome dome
    ("water", "cyan", ["....."  , ".#..#", "#.##.", ".#..#", "#.##."]),   # 39 aqua: wave
    ("teal", "amber", ["#####", "#.#.#", "#####", "#.#.#", "#####"]),     # 40 apartment: building
    ("wood", "red", ["..#..", ".###.", "#####", "#.#.#", "#.#.#"]),       # 41 barn
    ("cyan", "cyan", ["..#..", "..#..", ".###.", "#####", ".###."]),      # 42 water gen: droplet
]


def icon_dome_kit(k):
    gname, aname, glyph = DOME_KITS[k]
    g = RAMPS[gname]
    acc = RAMPS[aname]
    sp = Sprite(16, 16)
    # crate
    st = R("steel")
    box(sp, 1, 8, 14, 14, (st[0], st[1], st[2]))
    sp.hline(1, 14, 8, st[3])
    sp.hline(2, 13, 9, st[2])
    for x in (1, 14):
        sp.vline(x, 9, 14, st[0] if x == 14 else st[2])
    sp.set(2, 10, st[3])
    sp.set(13, 10, st[2])
    sp.set(2, 13, st[2])
    sp.set(13, 13, st[0])
    # glyph plate
    sp.rect(4, 9, 11, 14, R("indigo", 1))
    for y, row in enumerate(glyph):
        for x, ch in enumerate(row):
            if ch == "#":
                sp.set(5 + x + (1 if x >= 0 else 0) - 1 + 1, 9 + y + (0), acc[3] if y < 3 else acc[2])
    # dome
    for (x, y) in m_ellipse(8, 8, 6, 6.5):
        if y >= 8:
            continue
        dx, dy = (x + 0.5 - 8) / 6, (y + 0.5 - 8) / 6.5
        edge = dx * dx + dy * dy > 0.62
        v = 0.55 - dx * 0.45 - dy * 0.1
        c = g[3] if v > 0.72 else g[2] if v > 0.4 else g[1]
        if edge:
            c = st[3] if dx < 0.2 else st[2]
        sp.set(x, y, c)
    sp.vline(8, 2, 7, st[2])
    sp.set(5, 3, WHITE)
    sp.set(4, 4, g[4])
    sp.set(4, 5, g[4])
    sp.outline()
    return sp


def gen_icons():
    sh = Sprite(256, 128)
    for i in range(112):
        if 33 <= i <= 42:
            sp = icon_dome_kit(i - 33)
        elif i in ICONS:
            art, key, ol, dx, dy = ICONS[i]
            sp = ascii_sprite(art, key, outline=ol, dx=dx, dy=dy)
        else:
            raise SystemExit("missing icon %d" % i)
        sh.blit(sp, (i % 16) * 16, (i // 16) * 16)
    return sh


# --------------------------------------------------------------------------
# Domes (domes/<name>_back.png, <name>_front.png)
# --------------------------------------------------------------------------
DOMES = [("starter", 216, 80), ("green", 104, 52), ("storage", 72, 44), ("bedroom", 72, 44),
         ("kitchen", 72, 44), ("dark", 104, 52), ("domedome", 104, 52), ("aqua", 104, 52),
         ("apartment", 136, 64), ("barn", 104, 52), ("watergen", 72, 44)]
CALM = 26  # bottom rows of each backdrop kept quiet for fixtures

DOME_WALL = {
    "starter": ["131c28", "1a2a36", "223844", "2c4a54", "3a5e66"],
    "green": ["0e2220", "14302a", "1b4034", "245242", "31664e"],
    "storage": ["1a1822", "24222e", "2e2c3a", "3a3848", "4a4858"],
    "bedroom": ["20141f", "2c1c2a", "3a2636", "4c3242", "5e4050"],
    "kitchen": ["182226", "202e32", "2a3c40", "364c50", "445e62"],
    "dark": ["08060e", "0c0914", "120d1c", "181226", "201832"],
    "domedome": ["121724", "1a2030", "232c40", "2e3a52", "3c4a66"],
    "aqua": ["081428", "0b1e3c", "0f2a52", "143866", "1c4a7c"],
    "apartment": ["15132a", "1e1c36", "282646", "343258", "44426c"],
    "barn": ["1c1210", "281a16", "36241c", "463024", "583e2e"],
    "watergen": ["0c1c2a", "122838", "183646", "204656", "2a5a68"],
}
DOME_WALL = {k: [hexc(c) for c in v] for k, v in DOME_WALL.items()}


def dome_masks(W, H):
    a, b = W / 2.0, float(H)
    inner, shell = set(), set()
    for y in range(H):
        for x in range(W):
            dx, dy = x + 0.5 - W / 2.0, y + 0.5 - H
            if (dx / a) ** 2 + (dy / b) ** 2 <= 1.0:
                if (dx / (a - 2)) ** 2 + (dy / (b - 2)) ** 2 <= 1.0:
                    inner.add((x, y))
                else:
                    shell.add((x, y))
    return inner, shell


class DomeCtx:
    def __init__(self, name, W, H):
        self.name, self.W, self.H = name, W, H
        self.inner, self.shell = dome_masks(W, H)
        self.rng = random.Random(hash2(W, H, sum(ord(c) for c in name)))
        self.wall = DOME_WALL[name]
        self.sp = Sprite(W, H)
        self.sp.clip = self.inner
        self.cx = W // 2

    def hw(self, y):
        """Interior half-width at row y."""
        t = (self.H - y - 0.5) / (self.H - 2.0)
        return (self.W / 2.0 - 2) * math.sqrt(max(0.0, 1 - t * t))

    def top(self, x):
        """First interior row in column x."""
        t = (x + 0.5 - self.W / 2.0) / (self.W / 2.0 - 2)
        if abs(t) >= 1:
            return self.H
        return int(math.ceil(self.H - 0.5 - (self.H - 2.0) * math.sqrt(1 - t * t)))


def wall_base(d, grain=0.07, vert=0.3):
    sp, W, H, ramp = d.sp, d.W, d.H, d.wall
    a, b = W / 2.0 - 2, H - 2.0
    for (x, y) in d.inner:
        dx, dy = (x + 0.5 - W / 2.0) / a, (H - y - 0.5) / b
        r = math.sqrt(dx * dx + dy * dy)
        v = 0.34 + vert * dy - 0.16 * dx - 0.5 * max(0.0, r - 0.86) / 0.14 * 0.5
        v += (hash2(x // 2, y // 2, W) - 0.5) * grain
        sp.set(x, y, ramp_soft(ramp, v, x, y, 0.03))


def wall_ribs(d, lit=None, dark=None, rings=(0.36, 0.64, 0.86), spacing=17, rivets=True):
    """Meridian ribs and latitude seams of the back half of the dome."""
    sp, W, H = d.sp, d.W, d.H
    a, b = W / 2.0 - 2, H - 2.0
    lit = lit or d.wall[4]
    dark = dark or mix(d.wall[0], INK, 0.4)
    n = max(1, int(round(a / spacing)))
    ks = [i / (n + 0.5) for i in range(-n, n + 1)]
    for ry in rings:
        y = int(round(H - 0.5 - b * ry))
        for x in range(W):
            sp.set(x, y, dark)
            sp.set(x, y + 1, mix(sp.get(x, y + 1), lit, 0.35))
    for k in ks:
        lam = math.asin(max(-1, min(1, k)))
        for y in range(H):
            dy = (H - y - 0.5) / b
            if dy >= 1:
                continue
            x = W / 2.0 + a * math.sin(lam) * math.sqrt(1 - dy * dy)
            xi = int(math.floor(x))
            sp.set(xi, y, lit if k <= 0 else mix(d.wall[3], lit, 0.4))
            sp.set(xi + 1, y, dark)
    if rivets:
        for ry in rings:
            y = int(round(H - 0.5 - b * ry))
            dy = ry
            for k in ks:
                x = int(math.floor(W / 2.0 + a * math.sin(math.asin(k)) * math.sqrt(1 - dy * dy)))
                sp.set(x, y, lit)
                sp.set(x + 1, y + 1, dark)


def calm_floor(d, strength=0.5):
    """Fade the bottom CALM rows toward the dark wall tone so fixtures read."""
    sp, H = d.sp, d.H
    base = d.wall[1]
    for (x, y) in d.inner:
        if y >= H - CALM:
            t = (y - (H - CALM)) / float(CALM)
            k = strength * (0.55 + 0.45 * t)
            if y < H - CALM + 3:
                k *= (y - (H - CALM) + 1) / 4.0
            sp.px[y * sp.w + x] = mix(sp.px[y * sp.w + x], base, k)


def edge_shade(d, depth=4, k=0.45):
    """Ambient occlusion along the inside of the shell."""
    sp, W, H = d.sp, d.W, d.H
    a, b = W / 2.0 - 2, H - 2.0
    for (x, y) in d.inner:
        dx, dy = (x + 0.5 - W / 2.0) / a, (H - y - 0.5) / b
        r = math.sqrt(dx * dx + dy * dy)
        e = (1 - r) * min(a, b)
        if e < depth:
            t = 1 - e / depth
            sp.px[y * sp.w + x] = mix(sp.px[y * sp.w + x], INK, k * t * t)
    for x in range(W):
        for i, kk in enumerate((0.3, 0.15)):
            y = H - 1 - i
            if (x, y) in d.inner:
                sp.px[y * sp.w + x] = mix(sp.px[y * sp.w + x], INK, kk)


def light_cone(d, x, y, r, c, k=0.5, spread=1.0, down=True):
    """Additive pool of light on the wall below a lamp."""
    sp = d.sp
    for yy in range(int(y - r * 0.4), int(y + r) + 1):
        for xx in range(int(x - r * spread), int(x + r * spread) + 1):
            if (xx, yy) not in d.inner:
                continue
            dx, dy = (xx + 0.5 - x) / (r * spread), (yy + 0.5 - y) / r
            if down and dy < 0:
                dy *= 2.5
            dd = dx * dx + dy * dy
            if dd < 1:
                t = (1 - dd) ** 1.5 * k
                # posterise the falloff a little
                t = int(t * 10 + 0.5) / 10.0
                q = sp.px[yy * sp.w + xx]
                sp.px[yy * sp.w + xx] = (min(255, int(q[0] + c[0] * t)), min(255, int(q[1] + c[1] * t)),
                                         min(255, int(q[2] + c[2] * t)), 255)


def hang_lamp(d, x, y, w, c, cable=True, k=0.45, r=None, housing=None):
    """Lamp bar hung from the ceiling: housing, lit strip and a light pool."""
    sp = d.sp
    st = housing or R("steel")
    if cable:
        for xx in (x - w // 2 + 1, x + w // 2 - 1):
            sp.vline(xx, d.top(xx), y - 1, mix(st[0], INK, 0.3))
    sp.hline(x - w // 2, x + w // 2, y, st[2])
    sp.hline(x - w // 2, x + w // 2, y + 1, st[1])
    sp.set(x - w // 2, y, st[3])
    sp.hline(x - w // 2 + 1, x + w // 2 - 1, y + 2, c)
    sp.set(x, y + 2, mix(c, WHITE, 0.6))
    light_cone(d, x, y + 3, r or (w + 10), c, k=k, spread=1.1)


def cable(d, x0, y0, x1, y1, sag, c, bulbs=None, step=5):
    pts = curve_pts((x0, y0), ((x0 + x1) / 2.0, (y0 + y1) / 2.0 + sag * 2), (x1, y1), 40)
    for i, (x, y) in enumerate(pts):
        d.sp.set(x, y, c)
    if bulbs:
        for i, (x, y) in enumerate(pts):
            if i % step == step // 2:
                d.sp.set(x, y + 1, bulbs)
                light_cone(d, x, y + 1, 5, bulbs, k=0.22, down=False)
                d.sp.set(x, y + 1, mix(bulbs, WHITE, 0.5))


def vine(sp, rng, x, y, length, ramp=None, leaf=0.5, sway=0.35, bud=None):
    """Hanging vine: wandering stem with paired leaves."""
    ramp = ramp or R("moss")
    fx = float(x)
    drift = rng.uniform(-0.3, 0.3)
    for i in range(length):
        yy = y + i
        fx += drift + rng.uniform(-sway, sway)
        drift = drift * 0.8 + rng.uniform(-0.12, 0.12)
        xi = int(round(fx))
        sp.set(xi, yy, ramp[1] if i % 3 else ramp[2])
        if rng.random() < leaf and i > 0:
            side = rng.choice((-1, 1))
            sp.set(xi + side, yy, ramp[2])
            if rng.random() < 0.5:
                sp.set(xi + side * 2, yy + (0 if rng.random() < 0.5 else 1), ramp[3] if side < 0 else ramp[2])
    if bud:
        sp.set(int(round(fx)), y + length, bud)
    return int(round(fx)), y + length


def leaf_clump(sp, rng, cx, cy, r, ramp=None, n=None):
    """Cluster of small lit leaves."""
    ramp = ramp or R("moss")
    n = n or int(r * r * 0.9) + 3
    for i in range(n):
        ang = rng.uniform(0, math.tau)
        rr = r * math.sqrt(rng.random())
        x, y = int(round(cx + math.cos(ang) * rr)), int(round(cy + math.sin(ang) * rr * 0.8))
        v = 0.5 - (x - cx) / (2.5 * r + 0.1) - (y - cy) / (2.5 * r + 0.1)
        c = ramp[3] if v > 0.72 else ramp[2] if v > 0.4 else ramp[1]
        sp.set(x, y, c)
        if rng.random() < 0.6:
            sp.set(x + 1, y, ramp[1] if c != ramp[1] else ramp[0])
        if rng.random() < 0.35:
            sp.set(x, y + 1, ramp[1] if c == ramp[2] else ramp[0])


def moss_on_ribs(d, amount=0.5, ramp=None):
    """Moss creeping along seams: recolour some lit rib pixels."""
    sp, rng = d.sp, d.rng
    ramp = ramp or R("moss")
    lit = d.wall[4]
    n = FBM(random.Random(d.W * 7 + d.H), 4, 3, w=64)
    for (x, y) in sorted(d.inner):
        p = sp.px[y * sp.w + x]
        if p == lit and n(x * 1.3, y * 1.3) < amount:
            sp.px[y * sp.w + x] = ramp[2] if rng.random() < 0.6 else ramp[1]
            if rng.random() < 0.3:
                sp.set(x, y + 1, ramp[1])
            if rng.random() < 0.12:
                sp.set(x - 1, y, ramp[3])


def ceiling_vines(d, count, lmin=6, lmax=22, ramp=None, margin=6, buds=None):
    rng = d.rng
    for i in range(count):
        x = rng.randint(margin, d.W - 1 - margin)
        y = d.top(x)
        ln = rng.randint(lmin, lmax)
        vine(d.sp, rng, x, y, ln, ramp=ramp, bud=(rng.choice(buds) if buds and rng.random() < 0.6 else None))


def panel(sp, x0, y0, x1, y1, ramp, inset=None):
    box(sp, x0, y0, x1, y1, ramp)
    if inset:
        sp.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, inset)


def pipe_h(sp, x0, x1, y, ramp, joints=10):
    sp.hline(x0, x1, y, ramp[3])
    sp.hline(x0, x1, y + 1, ramp[1])
    for x in range(x0 + joints // 2, x1, joints):
        sp.set(x, y, ramp[4])
        sp.set(x, y - 1, ramp[2])
        sp.set(x, y + 1, ramp[2])
        sp.set(x, y + 2, ramp[0])


def pipe_v(sp, x, y0, y1, ramp, joints=9):
    sp.vline(x, y0, y1, ramp[3])
    sp.vline(x + 1, y0, y1, ramp[1])
    for y in range(y0 + joints // 2, y1, joints):
        sp.set(x - 1, y, ramp[2])
        sp.set(x, y, ramp[4])
        sp.set(x + 1, y, ramp[2])
        sp.set(x + 2, y, ramp[0])


def dim(ramp, k=0.62, to=None):
    """A ramp pushed into the background (darker, slightly desaturated)."""
    to = to or hexc("14121e")
    return [mix(c, to, 1 - k) for c in ramp]


# ---- per-type backdrops ----------------------------------------------------
def back_green(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    # trellis lattice
    lat = mix(d.wall[2], d.wall[4], 0.5)
    for (x, y) in d.inner:
        if y < H - CALM + 6 and ((x + y) % 12 == 0 or (x - y) % 12 == 0):
            sp.set(x, y, mix(sp.get(x, y), lat, 0.5))
    wall_ribs(d)
    moss_on_ribs(d, 0.62)
    cu = dim(R("copper"), 0.8)
    y = H - CALM - 3
    pipe_h(sp, 0, W, y, cu, joints=13)
    for x in range(8, W - 6, 13):
        sp.set(x + 6, y + 2, cu[2])
        sp.set(x + 6, y + 3, R("cyan", 3))
        if rng.random() < 0.7:
            sp.set(x + 6, y + 5 + rng.randint(0, 3), R("cyan", 2))
    for x, yy in ((W // 2, 7), (W // 2 - 27, 13), (W // 2 + 27, 13)):
        hang_lamp(d, x, yy, 12, hexc("ff7fd8"), k=0.2)
    for i in range(9):
        x = rng.randint(6, W - 7)
        yy = rng.randint(min(d.top(x) + 3, H - CALM), H - CALM)
        leaf_clump(sp, rng, x, yy, rng.uniform(2, 4.5), dim(R("moss"), 0.85))
    ceiling_vines(d, 16, 6, 24, buds=[R("lime", 3), R("amber", 3), R("mag", 3)])
    calm_floor(d, 0.5)
    for i in range(7):
        x = rng.randint(4, W - 5)
        leaf_clump(sp, rng, x, rng.randint(H - CALM + 4, H - 4), rng.uniform(2, 4), dim(R("moss"), 0.45))


def back_storage(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    wall_ribs(d, spacing=20)
    st = dim(R("steel"), 0.7)
    shelves = [12, 22, 32]
    for x in (9, 23, 48, 62):
        sp.vline(x, 4, H, st[2])
        sp.vline(x + 1, 4, H, st[0])
    for sy in shelves:
        sp.hline(0, W, sy, st[3])
        sp.hline(0, W, sy + 1, st[1])
        for x in range(2, W, 4):
            sp.set(x, sy + 1, st[0])
        x = 3 + rng.randint(0, 2)
        while x < W - 6:
            w = rng.randint(4, 8)
            h = rng.randint(3, 6)
            kind = rng.choice(["rust", "teal", "bone", "wood", "steel", "moss"])
            rp = dim(R(kind), 0.72)
            if rng.random() < 0.82:
                box(sp, x, sy - h, x + w - 1, sy - 1, (rp[1], rp[2], rp[3]))
                if w >= 6:
                    sp.hline(x + 1, x + w - 2, sy - h + h // 2, rp[1])
                if rng.random() < 0.4:
                    sp.set(x + w // 2, sy - h + 1, R("amber", 3))
            x += w + rng.randint(1, 3)
    # hazard stripe plate + lamp
    for i in range(12):
        sp.set(30 + i, 5, R("amber", 2) if (i // 2) % 2 else INK)
    hang_lamp(d, W // 2, 7, 8, R("amber", 4), k=0.3)
    moss_on_ribs(d, 0.4)
    ceiling_vines(d, 6, 4, 13)
    calm_floor(d, 0.55)


def back_bedroom(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    wall_ribs(d, spacing=20)
    # lockers along both sides
    lk = dim(R("rust"), 0.55, hexc("20141f"))
    for x0 in (4, 11, 18, 48, 55, 62):
        box(sp, x0, 21, x0 + 5, H, (lk[1], lk[2], lk[3]))
        sp.hline(x0 + 1, x0 + 4, 23, lk[1])
        sp.hline(x0 + 1, x0 + 4, 25, lk[1])
        sp.set(x0 + 4, 30, R("amber", 2))
    # window panel: dusk sky and the ringed moon
    wx0, wy0, wx1, wy1 = 26, 6, 45, 17
    st = dim(R("steel"), 0.9)
    box(sp, wx0 - 1, wy0 - 1, wx1 + 1, wy1 + 1, (st[0], st[2], st[3]))
    sky = [hexc("1c1a40"), hexc("3a2858"), hexc("7a3a6a"), hexc("c8644e"), hexc("f0a850")]
    for y in range(wy0, wy1 + 1):
        for x in range(wx0, wx1 + 1):
            sp.set(x, y, ramp_soft(sky, (y - wy0) / float(wy1 - wy0) * 0.95, x, y))
    for (x, y) in m_ellipse(40, 10, 2.5, 2.5):
        sp.set(x, y, R("bone", 4) if x < 40 else R("bone", 3))
    sp.hline(36, 43, 10, R("bone", 2))
    sp.set(39, 10, R("bone", 4))
    sp.set(40, 10, R("bone", 4))
    for sx, sy in ((29, 8), (33, 7), (31, 11), (35, 9)):
        sp.set(sx, sy, hexc("e8e0ff"))
    # far ridge silhouette in the window
    for x in range(wx0, wx1 + 1):
        hgt = 2 + int(1.5 * math.sin(x * 0.7) + 1.2 * math.sin(x * 1.9))
        sp.vline(x, wy1 - max(0, hgt), wy1, hexc("2a1c3a"))
    sp.vline(35, wy0, wy1, st[2])
    sp.hline(wx0, wx1, wy0 - 1, st[4])
    light_cone(d, 36, 17, 16, hexc("b05a40"), k=0.2)
    # string lights
    cable(d, 4, 19, 68, 19, 3, mix(INK, lk[0], 0.3), bulbs=R("amber", 3), step=7)
    moss_on_ribs(d, 0.35)
    ceiling_vines(d, 7, 3, 9)
    leaf_clump(sp, rng, 24, 17, 2.5, dim(R("moss"), 0.8))
    leaf_clump(sp, rng, 47, 6, 2.5, dim(R("moss"), 0.8))
    calm_floor(d, 0.5)


def back_kitchen(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    # tiles
    grout = d.wall[0]
    hl = d.wall[4]
    for (x, y) in d.inner:
        if y >= 6:
            if x % 5 == 0 or y % 5 == 1:
                sp.set(x, y, mix(sp.get(x, y), grout, 0.7))
            elif x % 5 == 1 and y % 5 == 2 and hash2(x, y, 5) < 0.5:
                sp.set(x, y, mix(sp.get(x, y), hl, 0.6))
            elif hash2(x // 5, (y - 1) // 5, 9) < 0.14:
                sp.set(x, y, mix(sp.get(x, y), R("teal", 2), 0.35))
    wall_ribs(d, spacing=40, rings=(0.9,))
    st = dim(R("steel"), 0.85)
    # extractor hood + duct
    sp.rect(34, 2, 37, 8, st[2])
    sp.vline(34, 2, 8, st[3])
    sp.vline(37, 2, 8, st[0])
    for i in range(6):
        sp.hline(36 - 6 - i, 36 + 5 + i, 9 + i, st[2] if i else st[4])
    sp.hline(24, 47, 15, st[0])
    sp.hline(25, 46, 14, st[1])
    for x in range(27, 46, 3):
        sp.set(x, 12, st[0])
    sp.set(26, 13, R("amber", 3))
    sp.set(45, 13, R("lime", 3))
    light_cone(d, 36, 16, 14, hexc("c08a50"), k=0.22)
    # rail with hanging pots, pans and ladles
    sp.hline(6, 21, 20, st[3])
    sp.hline(50, 66, 20, st[3])
    cu = dim(R("copper"), 0.85)
    for x in (8, 14, 19, 53, 58, 64):
        sp.vline(x, 21, 22, st[1])
        kind = rng.randint(0, 2)
        if kind == 0:
            for (px_, py_) in m_ellipse(x + 0.5, 25.5, 2.4, 2.4):
                sp.set(px_, py_, cu[3] if px_ <= x and py_ <= 25 else cu[2])
        elif kind == 1:
            box(sp, x - 2, 23, x + 2, 26, (cu[1], cu[2], cu[3]))
        else:
            sp.vline(x, 23, 26, st[3])
            sp.set(x, 27, st[4])
            sp.set(x + 1, 27, st[2])
    # jar shelf
    for x0 in (4, 52):
        sp.hline(x0, x0 + 14, 33, st[2])
        for i in range(4):
            c = rng.choice([R("moss", 3), R("amber", 2), R("rust", 3), R("bone", 2)])
            x = x0 + 1 + i * 4
            sp.rect(x, 30, x + 1, 32, mix(c, d.wall[1], 0.3))
            sp.set(x, 29, st[3])
            sp.set(x + 1, 29, st[1])
    moss_on_ribs(d, 0.4)
    ceiling_vines(d, 6, 3, 10)
    calm_floor(d, 0.5)


def mushroom_small(sp, x, y, h, capw, cap, stem, glowc=None):
    """Small wall/floor mushroom, base at (x, y)."""
    sp.vline(x, y - h + 1, y, stem)
    for i in range(-capw, capw + 1):
        sp.set(x + i, y - h, cap[1] if i > 0 else cap[2])
    for i in range(-capw + 1, capw):
        sp.set(x + i, y - h - 1, cap[3] if i <= 0 else cap[2])
    if capw > 2:
        sp.set(x - 1, y - h - 2, cap[3])
        sp.set(x, y - h - 2, cap[3])
        sp.set(x + 1, y - h - 2, cap[2])
    if glowc:
        sp.set(x - capw + 1, y - h + 1, glowc)
        sp.set(x + capw - 1, y - h + 1, glowc)


def back_dark(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d, grain=0.12)
    wall_ribs(d, lit=hexc("2a2042"), dark=hexc("05040a"), spacing=20)
    # mycelium threads
    my = hexc("2c2244")
    for i in range(15):
        x, y = rng.randint(6, W - 7), rng.randint(6, H - 8)
        for j in range(rng.randint(8, 18)):
            sp.set(x, y, my)
            x += rng.choice((-1, 0, 1))
            y += rng.choice((0, 1, 1))
            if rng.random() < 0.15:
                sp.set(x + rng.choice((-1, 1)), y, my)
    # UV strips
    uv = R("violet")
    uvs = [(W // 2, 6), (W // 2 - 28, 15), (W // 2 + 28, 15)]
    for x, y, w in ((W // 2, 6, 14), (W // 2 - 28, 15, 10), (W // 2 + 28, 15, 10)):
        sp.hline(x - w // 2 - 1, x + w // 2 + 1, y - 1, hexc("1c1630"))
        sp.hline(x - w // 2, x + w // 2, y, uv[4])
        sp.set(x - w // 2, y, uv[3])
        sp.set(x + w // 2, y, uv[3])
        sp.set(x, y, hexc("ead8ff"))
        light_cone(d, x, y + 1, 13, hexc("5a30a8"), k=0.32, spread=1.2)
    # glowing mushrooms growing on the wall
    caps = [dim(R("mag"), 0.9), dim(R("cyan"), 0.8), dim(R("violet"), 1.0), dim(R("lime"), 0.7)]
    ledges = [int(round(H - 0.5 - (H - 2.0) * ry)) - 1 for ry in (0.36, 0.64)] + [H - 1]
    for i in range(34):
        x = rng.randint(5, W - 6)
        y = rng.choice(ledges)
        if y < d.top(x) + 6 or any(abs(x - ux) < 9 and 0 <= y - uy < 5 for ux, uy in uvs):
            continue
        cap = rng.choice(caps)
        mushroom_small(sp, x, y, rng.randint(2, 5), rng.randint(1, 3), cap, hexc("5a5068"),
                       glowc=cap[4] if rng.random() < 0.5 else None)
        if y < H - CALM:
            light_cone(d, x, y - 3, 5, cap[2], k=0.16, down=False)
    # bracket fungi stepping up the ribs
    for i in range(14):
        x, y = rng.randint(6, W - 7), rng.randint(8, H - 6)
        cap = rng.choice(caps)
        sp.hline(x, x + 2, y, cap[2])
        sp.set(x, y, cap[3])
        sp.hline(x, x + 1, y + 1, cap[1])
    for i in range(34):
        x, y = rng.randint(4, W - 5), rng.randint(5, H - 5)
        sp.set(x, y, rng.choice([hexc("6a48b0"), hexc("3a8a9a"), hexc("8a3a8a")]))
    calm_floor(d, 0.5)
    edge_shade(d, 5, 0.6)


def robot_arm(d, x, y0, segs, c, tip):
    """Hanging articulated arm; segs: list of (dx, dy)."""
    sp = d.sp
    st = dim(R("steel"), 0.9)
    px_, py_ = x, y0
    sp.rect(x - 2, y0 - 1, x + 2, y0, st[2])
    for (dx, dy) in segs:
        nx, ny = px_ + dx, py_ + dy
        for (lx, ly) in line_pts(px_, py_, nx, ny):
            sp.set(lx, ly, st[3])
            sp.set(lx + 1, ly, st[1])
        sp.rect(px_ - 1, py_ - 1, px_ + 1, py_ + 1, c)
        sp.set(px_ - 1, py_ - 1, mix(c, WHITE, 0.5))
        sp.set(px_ + 1, py_ + 1, mix(c, INK, 0.5))
        px_, py_ = nx, ny
    # claw
    sp.rect(px_ - 1, py_ - 1, px_ + 1, py_, st[2])
    sp.set(px_ - 2, py_ + 1, st[3])
    sp.set(px_ + 2, py_ + 1, st[1])
    sp.set(px_ - 2, py_ + 2, st[3])
    sp.set(px_ + 2, py_ + 2, st[1])
    if tip:
        sp.set(px_, py_ + 2, tip)
    return px_, py_ + 2


def sparks(d, x, y, n=9, c=None):
    c = c or R("amber", 4)
    rng = d.rng
    light_cone(d, x, y, 8, hexc("b06a20"), k=0.4, down=False)
    d.sp.set(x, y, WHITE)
    for i in range(n):
        ang = rng.uniform(0.15, math.pi - 0.15)
        r = rng.uniform(2, 7)
        sx, sy = x + math.cos(ang) * r, y + math.sin(ang) * r * 0.8 - 1
        d.sp.set(sx, sy, c if r < 5 else R("amber", 2))


def back_domedome(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    wall_ribs(d)
    st = dim(R("steel"), 0.85)
    # gantry truss
    gy = 12
    sp.hline(0, W, gy, st[3])
    sp.hline(0, W, gy + 1, st[1])
    sp.hline(0, W, gy + 5, st[2])
    sp.hline(0, W, gy + 6, st[0])
    for x in range(0, W, 8):
        sp.line(x, gy + 5, x + 4, gy + 1, st[1])
        sp.line(x + 4, gy + 1, x + 8, gy + 5, st[2])
    for x in range(36, 68):
        if ((x + 0) // 3) % 2 == 0:
            sp.set(x, gy + 5, R("amber", 2))
    # trolley + arms
    for ax, segs, tip in ((30, [(0, 6), (5, 6), (3, 5)], None), (74, [(0, 5), (-6, 6), (-4, 6)], None)):
        box(sp, ax - 4, gy + 6, ax + 4, gy + 9, (st[1], st[2], st[4]))
        sp.set(ax + 3, gy + 7, R("amber", 3))
        tx, ty = robot_arm(d, ax, gy + 10, segs, dim(R("amber"), 0.9)[2], None)
    sparks(d, 38, 31)
    sparks(d, 64, 31, n=6)
    # blueprint screen
    box(sp, 44, 21, 59, 30, (st[0], st[1], st[3]))
    sp.rect(45, 22, 58, 29, hexc("0c2a4a"))
    cy_ = R("cyan", 2)
    for (x, y) in m_ellipse(52, 29.5, 5, 6):
        if y <= 29 and not ((x + 0.5 - 52) / 4.0) ** 2 + ((y + 0.5 - 29.5) / 5.0) ** 2 <= 1:
            sp.set(x, y, cy_)
    sp.hline(46, 57, 29, mix(cy_, INK, 0.3))
    sp.set(47, 23, R("cyan", 3))
    sp.set(49, 23, R("cyan", 1))
    light_cone(d, 52, 30, 9, hexc("1a5a80"), k=0.25)
    hang_lamp(d, 12, 22, 6, R("amber", 3), cable=True, k=0.2)
    hang_lamp(d, 92, 22, 6, R("amber", 3), cable=True, k=0.2)
    moss_on_ribs(d, 0.36)
    ceiling_vines(d, 7, 4, 10)
    calm_floor(d, 0.5)


def back_aqua(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    ramp = d.wall
    n = FBM(random.Random(77), 4, 3, w=128)
    a, b = W / 2.0 - 2, H - 2.0
    for (x, y) in d.inner:
        dy = (H - y - 0.5) / b
        v = 0.18 + 0.62 * dy + (n(x * 1.2, y * 2.2) - 0.5) * 0.25
        # light shafts
        s = (x - (H - y) * 0.45) % 26
        if s < 5 and dy > 0.15:
            v += 0.16 * min(1.0, dy * 1.4)
        sp.set(x, y, ramp_soft(ramp, v, x, y, 0.06))
    # kelp shadows
    kc = [hexc("061220"), hexc("0a2634"), hexc("0f3a44")]
    for i in range(16):
        x0 = rng.randint(5, W - 6)
        hgt = rng.randint(14, 34)
        ph = rng.uniform(0, 6)
        w0 = rng.choice((1, 2, 2))
        c = rng.choice(kc)
        for j in range(hgt):
            y = H - 1 - j
            x = x0 + math.sin(j * 0.32 + ph) * (1.5 + j * 0.05)
            for k in range(w0 if j < hgt - 4 else 1):
                sp.set(x + k, y, c)
            if j % 5 == 2 and j < hgt - 3:
                sd = 1 if (j // 5) % 2 else -1
                sp.set(x + sd * 2, y, c)
                sp.set(x + sd * 3, y - 1, c)
    # bubbles
    for i in range(7):
        x0 = rng.randint(8, W - 9)
        y = H - rng.randint(4, 12)
        while y > d.top(x0) + 4:
            r = rng.random()
            c = mix(R("water", 4), ramp[3], 0.35)
            if r < 0.3:
                sp.set(x0, y, c)
                sp.set(x0 + 1, y, mix(c, ramp[2], 0.5))
                sp.set(x0, y + 1, mix(c, ramp[2], 0.5))
            else:
                sp.set(x0, y, mix(c, ramp[2], 0.3))
            x0 += rng.choice((-1, 0, 0, 1))
            y -= rng.randint(4, 8)
    # tank frame
    wall_ribs(d, lit=hexc("36608a"), dark=hexc("050c18"), rings=(0.9,), spacing=26)
    # waterline ripples near the top
    wy = int(H - 0.5 - b * 0.9) + 2
    for x in range(W):
        if (x + int(2 * math.sin(x * 0.4))) % 5 < 3:
            sp.set(x, wy, mix(R("water", 4), ramp[4], 0.5))
    moss_on_ribs(d, 0.3, ramp=dim(R("teal"), 0.9))
    ceiling_vines(d, 5, 3, 8, ramp=R("teal"))
    calm_floor(d, 0.35)
    edge_shade(d, 4, 0.4)


def lit_window(d, x, y, w, h, c, frame, on=True, planter=True):
    sp, rng = d.sp, d.rng
    box(sp, x - 1, y - 1, x + w, y + h, (frame[0], frame[1], frame[3]))
    if on:
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                t = (yy - y) / float(h)
                sp.set(xx, yy, mix(mix(c, WHITE, 0.35), mix(c, hexc("803020"), 0.3), t))
        # curtain / silhouette
        k = rng.randint(0, 3)
        if k == 0:
            sp.vline(x, y, y + h - 1, mix(c, frame[0], 0.5))
            sp.vline(x + w - 1, y, y + h - 3, mix(c, frame[0], 0.5))
        elif k == 1:
            sp.rect(x + w // 2 - 1, y + h - 4, x + w // 2, y + h - 1, mix(c, INK, 0.75))
            sp.rect(x + w // 2 - 1, y + h - 5, x + w // 2, y + h - 5, mix(c, INK, 0.75))
        elif k == 2:
            sp.hline(x, x + w - 1, y + 2, mix(c, frame[0], 0.45))
        sp.vline(x + w // 2 + (1 if k != 1 else -2), y, y + h - 1, frame[1]) if k == 3 else None
        light_cone(d, x + w / 2.0, y + h / 2.0, w + 3, mix(c, hexc("402010"), 0.5), k=0.16, down=False)
    else:
        sp.rect(x, y, x + w - 1, y + h - 1, hexc("0e0e1e"))
        sp.set(x + 1, y + 1, hexc("2a2a4a"))
        sp.set(x + 2, y + 1, hexc("1e1e3a"))
    # balcony
    sp.hline(x - 2, x + w + 1, y + h + 1, frame[3])
    sp.hline(x - 2, x + w + 1, y + h + 2, frame[0])
    for xx in range(x - 2, x + w + 2, 2):
        sp.set(xx, y + h, frame[2])
    if planter:
        mo = dim(R("moss"), 0.85)
        for xx in range(x - 1, x + w + 1):
            r = rng.random()
            if r < 0.7:
                sp.set(xx, y + h, mo[2] if r < 0.4 else mo[3])
            if r < 0.3:
                sp.set(xx, y + h - 1, mo[2])
            if r < 0.12:
                sp.set(xx, y + h - 2, rng.choice([R("mag", 3), R("amber", 3), mo[3]]))
        if rng.random() < 0.6:
            vine(sp, rng, x + rng.choice((-2, w + 1)), y + h + 2, rng.randint(3, 7), mo)


def back_apartment(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    wall_ribs(d, rings=(0.9,), spacing=34)
    fr = dim(R("steel"), 0.8, hexc("15132a"))
    # floor slab between the two storeys
    fy = 35
    sp.hline(0, W, fy, fr[3])
    sp.hline(0, W, fy + 1, fr[1])
    sp.hline(0, W, fy + 2, fr[0])
    # central lift shaft
    box(sp, d.cx - 5, 5, d.cx + 4, H, (fr[0], fr[1], fr[2]))
    sp.vline(d.cx - 1, 6, H, fr[0])
    for y in range(8, H, 6):
        sp.set(d.cx - 3, y, R("cyan", 2))
        sp.set(d.cx + 2, y + 3, R("cyan", 1))
    box(sp, d.cx - 4, 22, d.cx + 3, 30, (fr[1], fr[2], fr[4]))
    sp.rect(d.cx - 3, 24, d.cx + 2, 27, hexc("ffd890"))
    sp.rect(d.cx - 3, 28, d.cx + 2, 29, hexc("c88a48"))
    light_cone(d, d.cx, 26, 10, hexc("704820"), k=0.2, down=False)
    cols = [hexc("ffc66a"), hexc("ffb050"), hexc("ffd890"), hexc("8fe0d0"), hexc("ff9a6a")]
    for row, wy in enumerate((19, 43)):
        for i in range(-4, 5):
            if i == 0:
                continue
            x = d.cx + i * 14 - 4 + (1 if i > 0 else -1)
            if abs(x + 4 - d.cx) > d.hw(wy - 2) - 7:
                continue
            on = rng.random() < 0.78
            lit_window(d, x, wy, 7, 9, rng.choice(cols), fr, on=on)
    cable(d, 8, 9 + 8, d.cx - 6, 8, 3, fr[0], bulbs=R("amber", 3), step=8)
    cable(d, d.cx + 5, 8, W - 9, 9 + 8, 3, fr[0], bulbs=R("amber", 3), step=8)
    moss_on_ribs(d, 0.4)
    ceiling_vines(d, 12, 3, 9, buds=[R("mag", 3)])
    calm_floor(d, 0.42)


def back_barn(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    ramp = d.wall
    a, b = W / 2.0 - 2, H - 2.0
    for (x, y) in d.inner:
        dx, dy = (x + 0.5 - W / 2.0) / a, (H - y - 0.5) / b
        plank = x // 5
        v = 0.36 + 0.26 * dy - 0.14 * dx + (hash2(plank, 0, 3) - 0.5) * 0.3
        v += (hash2(x, y // 3, 8) - 0.5) * 0.1
        c = ramp_soft(ramp, v, x, y)
        if x % 5 == 0:
            c = mix(ramp[0], INK, 0.35)
        elif x % 5 == 1:
            c = mix(c, ramp[4], 0.35)
        sp.set(x, y, c)
    # knots and nail heads
    for i in range(22):
        x, y = rng.randint(3, W - 4), rng.randint(4, H - 4)
        if x % 5 in (2, 3):
            sp.set(x, y, mix(ramp[0], INK, 0.2))
            sp.set(x, y - 1, ramp[4])
    wd = dim(R("wood"), 0.9)
    # loft beam and X bracing
    by = 20
    for x0, x1 in ((d.cx - 20, d.cx), (d.cx, d.cx + 20)):
        for (x, y) in line_pts(x0, by - 1, x1, 3):
            sp.set(x, y, wd[3])
            sp.set(x, y + 1, wd[1])
        for (x, y) in line_pts(x0, 3, x1, by - 1):
            sp.set(x, y, wd[3])
            sp.set(x, y + 1, wd[1])
    sp.rect(d.cx - 1, 2, d.cx, by, wd[2])
    sp.vline(d.cx - 1, 2, by, wd[4])
    sp.hline(0, W, by, wd[4])
    sp.hline(0, W, by + 1, wd[2])
    sp.hline(0, W, by + 2, wd[0])
    for x in range(4, W, 12):
        sp.set(x, by + 1, wd[0])
    # hay bales on the loft
    hay = [hexc("6a5020"), hexc("9a7a2c"), hexc("c8a440"), hexc("e8cc68")]
    for x0, w, h in ((14, 12, 6), (27, 10, 5), (18, 11, 5), (66, 11, 6), (78, 12, 6), (72, 11, 5)):
        y1 = by - 1
        if (x0, w) in ((18, 11), (72, 11)):
            y1 = by - 7
        for y in range(y1 - h + 1, y1 + 1):
            for x in range(x0, x0 + w):
                v = hash2(x, y, 4)
                c = hay[2] if v < 0.55 else hay[3] if v < 0.75 else hay[1]
                if y == y1 or x == x0 + w - 1:
                    c = hay[0]
                if y == y1 - h + 1:
                    c = hay[3]
                sp.set(x, y, c)
        sp.vline(x0 + w // 3, y1 - h + 1, y1, hay[0])
        sp.vline(x0 + 2 * w // 3, y1 - h + 1, y1, hay[0])
    for i in range(26):
        x = rng.randint(8, W - 9)
        sp.set(x, by + 3 + rng.randint(0, 2), hay[rng.randint(1, 2)])
    # lantern
    sp.vline(d.cx + 9, by + 3, by + 6, wd[0])
    box(sp, d.cx + 8, by + 7, d.cx + 10, by + 10, (R("rust", 1), R("amber", 3), R("amber", 4)))
    light_cone(d, d.cx + 9.5, by + 9, 13, hexc("a06020"), k=0.34, down=False)
    # pitchfork on the wall
    sp.vline(22, by + 5, by + 20, wd[3])
    for dx in (-2, 0, 2):
        sp.vline(22 + dx, by + 3, by + 6, dim(R("steel"), 0.9)[3])
    sp.hline(20, 24, by + 6, dim(R("steel"), 0.9)[2])
    moss_on_ribs(d, 0.3)
    ceiling_vines(d, 8, 3, 10)
    calm_floor(d, 0.45)
    edge_shade(d, 3, 0.4)


def back_watergen(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    wall_ribs(d, spacing=20)
    st = dim(R("steel"), 0.85)
    cu = dim(R("copper"), 0.85)
    cold = [hexc("143a4a"), hexc("1e6276"), hexc("3a9ab0"), hexc("7ad8e8"), hexc("c8f6ff")]
    # condenser coil: serpentine runs
    x0, x1 = 18, 53
    ys = [7, 11, 15, 19, 23]
    box(sp, x0 - 4, 5, x0 - 2, 26, (st[0], st[2], st[3]))
    box(sp, x1 + 2, 5, x1 + 4, 26, (st[0], st[2], st[3]))
    for i, y in enumerate(ys):
        rp = cold if i % 2 == 0 else cu
        sp.hline(x0, x1, y, rp[3])
        sp.hline(x0, x1, y + 1, rp[1])
        for x in range(x0 + 2, x1, 5):
            sp.set(x, y, rp[4])
        if i < len(ys) - 1:
            bx = x1 + 1 if i % 2 == 0 else x0 - 1
            sp.vline(bx, y, y + 5, rp[2])
    # fins
    for x in range(x0 + 1, x1, 3):
        for y in range(6, 26):
            if sp.get(x, y) not in (cold[3], cold[1], cu[3], cu[1], cold[4], cu[4]):
                sp.set(x, y, mix(sp.get(x, y), st[3], 0.3))
    # condensation drops
    for i in range(12):
        x = rng.randint(x0, x1)
        y = rng.choice(ys) + 2
        sp.set(x, y, cold[3])
        if rng.random() < 0.5:
            sp.set(x, y + 2 + rng.randint(0, 5), cold[2])
    light_cone(d, 36, 15, 20, hexc("104858"), k=0.3, down=False)
    # dripping down-pipes at the sides
    for x in (7, 63):
        pipe_v(sp, x, d.top(x) + 1, 30, cu, joints=8)
        sp.set(x, 31, cu[2])
        sp.set(x, 33, cold[3])
        sp.set(x, 37, cold[2])
    # gauge
    for (x, y) in m_ellipse(36, 30, 3, 3):
        sp.set(x, y, st[1])
    for (x, y) in m_ellipse(36, 30, 2, 2):
        sp.set(x, y, hexc("c8d8d0"))
    sp.set(36, 29, R("red", 3))
    sp.set(35, 30, INK)
    # frost sparkles
    for i in range(14):
        x, y = rng.randint(8, W - 9), rng.randint(4, 28)
        sp.set(x, y, mix(sp.get(x, y), cold[4], 0.55))
    moss_on_ribs(d, 0.35)
    ceiling_vines(d, 6, 3, 9)
    calm_floor(d, 0.5)


def back_starter(d):
    sp, W, H, rng = d.sp, d.W, d.H, d.rng
    wall_base(d)
    wall_ribs(d, spacing=19)
    st = dim(R("steel"), 0.85)
    bone = dim(R("bone"), 0.62, d.wall[1])
    rust = dim(R("rust"), 0.75, d.wall[1])
    wear = FBM(random.Random(5), 6, 3, w=128)
    cx = d.cx
    lx = cx - 58          # centre of the decal group on the left wall
    rx = cx + 54          # centre of the porthole on the right wall
    yo = H - 72           # vertical offset of the mid-wall band
    # old mission decals: worn stripe band and roundel on the left wall
    for (x, y) in d.inner:
        if 26 + yo <= y <= 30 + yo and x < cx - 12 and wear(x * 2.0, y * 2.0) > 0.42:
            sp.set(x, y, rust[3] if y in (27 + yo, 28 + yo, 29 + yo) else bone[3])
    ey = 28 + yo
    for (x, y) in m_ellipse(lx, ey, 10, 10):
        if wear(x * 2.5 + 40, y * 2.5) > 0.36:
            dd = math.hypot(x + 0.5 - lx, y + 0.5 - ey)
            if dd > 8:
                sp.set(x, y, bone[3])
            elif dd > 6.4:
                sp.set(x, y, d.wall[1])
            else:
                sp.set(x, y, dim(R("teal"), 0.7, d.wall[1])[3 if (x - lx) + (y - ey) < 0 else 2])
    for (dx, dy) in ((0, 4), (0, 3), (0, 2), (0, 1), (0, 0), (-1, -1), (-2, -2), (1, -1), (2, -2), (3, -2),
                     (-2, -3), (-3, -3), (2, -3), (-3, -2), (3, -3)):
        sp.set(lx + dx, ey + dy, bone[4])
    # stencil text blocks
    tx = lx - 34
    for i, w in enumerate((5, 3, 6, 2, 4)):
        x = tx + sum((5, 3, 6, 2, 4)[:i]) + i * 2
        for xx in range(x, x + w):
            if wear(xx * 3.0, 80) > 0.3:
                sp.set(xx, 21 + yo, bone[2])
                sp.set(xx, 22 + yo, bone[2])
                sp.set(xx, 23 + yo, bone[2])
    for i in range(5):
        sp.rect(lx + 14 + i * 3, 21 + yo, lx + 15 + i * 3, 23 + yo, bone[2] if i != 3 else rust[3])
    for i, w in enumerate((9, 6, 11)):
        sp.hline(tx, tx + w, 34 + yo + i * 2, mix(bone[1], d.wall[1], 0.3))
    # screens and a pipe run on the right
    sx = cx + 22
    box(sp, sx, 34 + yo, sx + 11, 41 + yo, (st[0], st[1], st[3]))
    sp.rect(sx + 1, 35 + yo, sx + 10, 40 + yo, hexc("0c2a3a"))
    for i, w in enumerate((7, 4, 6, 3)):
        sp.hline(sx + 2, sx + 1 + w, 36 + yo + i, R("cyan", 2) if i != 1 else R("lime", 2))
    light_cone(d, sx + 5, 41 + yo, 9, hexc("145868"), k=0.2)
    cu = dim(R("copper"), 0.8)
    pipe_h(sp, cx + 30, W, 14 + yo, cu, joints=11)
    pipe_v(sp, cx + 80, 15 + yo, H - CALM - 2, cu, joints=9)
    sp.set(cx + 80, H - CALM - 1, R("cyan", 3))
    # shelves with pots on the left
    for shx, shy in ((lx - 40, 40 + yo), (lx - 34, 33 + yo), (lx + 16, 40 + yo)):
        sp.hline(shx, shx + 20, shy, st[3])
        sp.hline(shx, shx + 20, shy + 1, st[0])
        for i in range(3):
            x = shx + 2 + i * 7
            box(sp, x, shy - 3, x + 3, shy - 1, (rust[1], rust[2], rust[3]))
            leaf_clump(sp, rng, x + 1.5, shy - 5, 2, dim(R("moss"), 0.8))
    # grow lights
    for x, y in ((cx - 58, 17), (cx - 28, 8), (cx + 28, 8), (cx + 58, 17)):
        hang_lamp(d, x, y, 14, hexc("ffc878") if abs(x - cx) < 40 else hexc("ff7fd8"), k=0.28)
    # porthole window on the right wall
    px_, py_ = rx, 27 + yo
    for (x, y) in m_ellipse(px_, py_, 11, 11):
        sp.set(x, y, st[3] if (x - px_) + (y - py_) < -6 else st[2] if (x - px_) + (y - py_) < 4 else st[0])
    sky = [hexc("1c1a40"), hexc("3a2858"), hexc("7a3a6a"), hexc("c8644e"), hexc("f0a850")]
    for (x, y) in m_ellipse(px_, py_, 9, 9):
        sp.set(x, y, ramp_soft(sky, (y - py_ + 9) / 18.0, x, y))
        hgt = 4 + int(2 * math.sin(x * 0.6) + 1.5 * math.sin(x * 1.7 + 1))
        if y > py_ + 8 - hgt:
            sp.set(x, y, hexc("241a36"))
    for (x, y) in m_ellipse(px_ + 3, py_ - 3, 2.5, 2.5):
        sp.set(x, y, R("bone", 4) if x < px_ + 3 else R("bone", 3))
    sp.hline(px_ - 1, px_ + 6, py_ - 3, R("bone", 2))
    sp.set(px_ + 2, py_ - 3, R("bone", 4))
    sp.set(px_ + 3, py_ - 3, R("bone", 4))
    sp.set(px_ - 5, py_ - 5, hexc("e8e0ff"))
    sp.set(px_ - 3, py_ - 1, hexc("b8b0e0"))
    sp.set(px_ - 6, py_ + 1, hexc("b8b0e0"))
    sp.vline(px_, py_ - 9, py_ + 8, st[1])
    sp.hline(px_ - 9, px_ + 8, py_ + 2, st[1])
    for ang in range(0, 360, 45):
        sp.set(px_ + math.cos(math.radians(ang + 22)) * 10.2, py_ + math.sin(math.radians(ang + 22)) * 10.2, st[4])
    light_cone(d, px_, py_ + 10, 16, hexc("a04a38"), k=0.18)
    moss_on_ribs(d, 0.6)
    for i in range(18):
        x = rng.randint(8, W - 9)
        y0 = d.top(x) + 2
        if y0 >= H - CALM - 2:
            continue
        y = rng.randint(y0, H - CALM - 2)
        leaf_clump(sp, rng, x, y, rng.uniform(2, 4.5), dim(R("moss"), 0.82))
    ceiling_vines(d, 32, 5, 28, buds=[R("lime", 3), R("amber", 3), R("cyan", 3)])
    calm_floor(d, 0.5)
    # central support pillar (drawn after the fade so it stays solid)
    pl = dim(R("steel"), 0.8)
    for y in range(2, H):
        for i, c in enumerate((pl[2], pl[4], pl[3], pl[3], pl[2], pl[2], pl[1], pl[0])):
            sp.set(cx - 4 + i, y, c)
    for y in range(8, H, 12):
        sp.hline(cx - 5, cx + 4, y, pl[4])
        sp.hline(cx - 5, cx + 4, y + 1, pl[1])
        sp.hline(cx - 5, cx + 4, y + 2, pl[0])
    # flared capital with braces into the roof
    for i in range(5):
        sp.hline(cx - 5 - i * 2, cx + 4 + i * 2, 6 - i, pl[3] if i % 2 else pl[2])
    for sgn in (-1, 1):
        ex = cx + sgn * 38
        for (x, y) in line_pts(cx + sgn * 4, 34, ex, d.top(ex) + 1):
            sp.set(x, y, pl[3])
            sp.set(x, y + 1, pl[1])
    # status strip and worn mission stripe on the pillar
    for y in range(24, 42):
        sp.set(cx - 1, y, R("amber", 3) if y % 4 else R("amber", 4))
        sp.set(cx, y, R("amber", 1))
    light_cone(d, cx, 33, 13, hexc("704818"), k=0.22, down=False)
    sp.rect(cx - 4, 48, cx + 3, 50, rust[3])
    sp.hline(cx - 4, cx + 3, 51, bone[3])
    # plants taking the pillar over
    mo = dim(R("moss"), 0.9)
    for y in range(H - 1, 12, -1):
        vx = cx + int(round(4 * math.sin(y * 0.22)))
        sp.set(vx, y, mo[2])
        if y % 3 == 0:
            sp.set(vx - 1, y, mo[3])
        if y % 4 == 1:
            sp.set(vx + 1, y - 1, mo[1])
    for y in (16, 28, 44, 57, 70):
        leaf_clump(sp, rng, cx + rng.randint(-4, 4), y, rng.uniform(2, 3.2), mo)
    edge_shade(d, 4, 0.35)


BACKS = {"starter": back_starter, "green": back_green, "storage": back_storage, "bedroom": back_bedroom,
         "kitchen": back_kitchen, "dark": back_dark, "domedome": back_domedome, "aqua": back_aqua,
         "apartment": back_apartment, "barn": back_barn, "watergen": back_watergen}


def dome_back(name, W, H):
    d = DomeCtx(name, W, H)
    BACKS[name](d)
    if name not in ("dark", "aqua", "barn", "starter"):
        edge_shade(d)
    for (x, y) in d.inner:
        p = d.sp.px[y * W + x]
        d.sp.px[y * W + x] = (p[0], p[1], p[2], 255)
    for i, p in enumerate(d.sp.px):
        if p[3] and (i % W, i // W) not in d.inner:
            d.sp.px[i] = CLEAR
    return d.sp


# ---- front (shell + sheen) -------------------------------------------------
def dome_front(name, W, H):
    d = DomeCtx(name, W, H)
    sp = Sprite(W, H)
    rng = random.Random(W * 31 + H + len(name))
    a, b = W / 2.0, float(H)
    dark = name == "dark"
    st = R("steel")
    rust = R("rust")
    moss = R("moss")
    # arc-length parameterisation for evenly spaced ribs
    N = 720
    arc = [0.0]
    for i in range(1, N + 1):
        t0, t1 = math.pi * (i - 1) / N, math.pi * i / N
        arc.append(arc[-1] + math.hypot(a * (math.cos(t1) - math.cos(t0)), b * (math.sin(t1) - math.sin(t0))))
    total = arc[-1]
    nrib = max(4, int(round(total / 14.0)))
    if nrib % 2:
        nrib += 1  # keeps a rib off the exact apex seam pair
    spacing = total / nrib
    wear = FBM(random.Random(W + 3), 8, 3, w=256)
    mossn = FBM(random.Random(W + 9), 10, 3, w=256)
    door_h = 11
    for (x, y) in d.shell:
        dx, dy = x + 0.5 - W / 2.0, y + 0.5 - H
        outer = (dx / (a - 1)) ** 2 + (dy / (b - 1)) ** 2 > 1.0
        th = math.atan2(-dy / b, dx / a)  # 0 right .. pi left
        s = arc[min(N, max(0, int(round(th / math.pi * N))))]
        rib = min(s % spacing, spacing - (s % spacing)) < 1.2 and 3 < s < total - 3
        lit = th > math.pi * 0.42  # left / top-left part of the arc
        topness = -dy / b
        if dark:
            base = [hexc("0c0914"), hexc("161026"), hexc("241a3a"), hexc("3a2a5c")]
            c = base[2] if outer and lit else base[1] if outer else base[0]
            if rib:
                c = base[3] if lit else base[2]
            if wear(x * 2.0, y * 2.0) > 0.6 and outer:
                c = mix(c, hexc("4a2a70"), 0.5)
        else:
            if rib:
                c = st[3] if (lit and outer) else st[2] if outer else st[1]
            elif outer:
                c = st[4] if (lit and topness > 0.5) else st[3] if lit else st[2]
            else:
                g = hexc("8fd6c0") if lit else hexc("4fa89c")
                c = wa(g, 150 if lit else 120)
            w = wear(x * 2.0, y * 2.0)
            if (outer or rib) and w > 0.6:
                c = rust[3] if w > 0.7 and lit else rust[2]
            elif (outer or rib) and w > 0.56:
                c = mix(c, rust[2], 0.5)
        m = mossn(x * 2.0, y * 2.0) + topness * 0.16
        if outer and m > 0.66 and y < H - door_h:
            c = moss[3] if (lit and m > 0.72) else moss[2] if lit else moss[1]
        elif not outer and m > 0.74 and y < H - door_h and not dark:
            c = moss[1]
        sp.set(x, y, c)
    # sheen over the interior
    if dark:
        for (x, y) in d.inner:
            v = 62 + int(10 * math.sin((x + y) * 0.21)) + (4 if (x + y) % 2 else 0)
            sp.set(x, y, wa(hexc("2a1648"), v))
    else:
        for (x, y) in d.inner:
            dx, dy = (x + 0.5 - W / 2.0) / (a - 2), (H - y - 0.5) / (b - 2)
            u = (x + (H - y) * 0.9)
            band = 0.0
            for off, wd, k in ((W * 0.36, 7.0, 1.0), (W * 0.36 + 11, 2.5, 0.7), (W * 0.95, 5.0, 0.55),
                               (W * 0.95 + 8, 1.5, 0.45)):
                t = abs(u - off) / wd
                if t < 1:
                    band = max(band, k * (1 - t * t))
            hi = clamp(0.35 + dy * 0.9 - dx * 0.25)
            al = band * hi * 30
            r = math.sqrt(dx * dx + dy * dy)
            rim = clamp((r - 0.9) / 0.1) * (14 if dx < 0.2 else 7)
            al = max(al, rim)
            if al >= 3:
                al = int(al)
                if al % 4 == 1 and (x + y) % 2:
                    al -= 1
                sp.set(x, y, wa(hexc("d8fff6"), min(30, al)))
    # airlock markings at both bottom ends
    for (x, y) in d.shell:
        if y >= H - door_h:
            left = x < W / 2
            r = y - (H - door_h)
            edge_out = (x == 0 or x == W - 1) or (x - 1, y) not in d.shell and left or \
                       ((x + 1, y) not in d.shell and not left)
            if r <= 1:
                c = R("cyan", 4) if r == 1 else R("cyan", 3)
                if dark:
                    c = R("violet", 4) if r == 1 else R("violet", 3)
            elif r == 2:
                c = st[0]
            else:
                c = R("amber", 3) if ((r - 3) // 2) % 2 == 0 else INK
                if y == H - 1:
                    c = st[1]
            sp.set(x, y, c)
    gc = R("violet", 4) if dark else R("cyan", 3)
    for sx in (0, 1):
        ex = 2 if sx == 0 else W - 3
        for (x, y) in d.inner:
            dd = math.hypot((x - ex) * 1.0, (y - (H - door_h + 1)) * 0.8)
            if dd < 6 and abs(x - ex) < 6:
                al = int(46 * (1 - dd / 6.0) ** 1.4)
                if al > 4:
                    sp.blend(x, y, wa(gc, al))
            if y >= H - 2 and abs(x - ex) < 9:
                sp.blend(x, y, wa(R("amber", 3), int(34 * (1 - abs(x - ex) / 9.0))))
    # vines draped over the outside of the shell
    canvas = sp
    nv = max(3, W // 22)
    used = []
    for i in range(nv * 3):
        if len(used) >= nv:
            break
        th = rng.uniform(0.18, 0.82) * math.pi
        if abs(th - math.pi / 2) < 0.2 or any(abs(th - u) < 0.16 for u in used):
            continue
        used.append(th)
        ln = rng.randint(int(H * 0.2), int(H * 0.5))
        sgn = -1 if th > math.pi / 2 else 1
        t = th
        last = None
        for j in range(ln * 3):
            x = int(math.floor(W / 2.0 + (a + 0.6) * math.cos(t)))
            y = int(math.floor(H - (b + 0.6) * math.sin(t)))
            t -= sgn * 0.012 * 100.0 / (W + H)
            if (x, y) == last:
                continue
            last = (x, y)
            if not (0 <= x < W and 1 <= y < H - door_h - 2) or len(used) < 0:
                break
            if (x, y) in d.inner:
                continue
            canvas.set(x, y, moss[2] if j % 3 else moss[3])
            if rng.random() < 0.45:
                lx, ly = x + rng.choice((-1, 1)), y + rng.choice((0, 1))
                if 0 <= lx < W and (lx, ly) not in d.inner and ly < H - door_h - 2:
                    canvas.set(lx, ly, moss[3] if rng.random() < 0.5 else moss[1])
            ln -= 1 if rng.random() < 0.4 else 0
            if ln <= 0:
                break
        # a short tendril hanging free over the glass
        if last and rng.random() < 0.8:
            x, y = last
            if sgn < 0:
                x += 1
            else:
                x -= 1
            for j in range(rng.randint(3, 7)):
                if y + j >= H - door_h - 2:
                    break
                canvas.set(x, y + j, moss[2] if j % 2 else moss[1])
                if rng.random() < 0.4:
                    canvas.set(x + rng.choice((-1, 1)), y + j, moss[3])
    # moss tufts hanging just inside the top of the shell
    for i in range(W // 9):
        x = rng.randint(6, W - 7)
        y = d.top(x)
        if rng.random() < 0.6:
            canvas.set(x, y, moss[1 if not dark else 0])
            if rng.random() < 0.5:
                canvas.set(x, y + 1, moss[2 if not dark else 1])
    return sp


def gen_domes():
    out = {}
    for name, W, H in DOMES:
        out["sprites/domes/%s_back.png" % name] = dome_back(name, W, H)
        out["sprites/domes/%s_front.png" % name] = dome_front(name, W, H)
    if PREVIEW_DIR:
        out_prev = Sprite(360, 640, (60, 66, 78, 255))
        x = y = 4
        rowh = 0
        for name, W, H in DOMES:
            if x + W > 356:
                x, y, rowh = 4, y + rowh + 6, 0
            out_prev.rect(x, y + H, x + W - 1, y + H + 2, hexc("58647a"))
            out_prev.blit(out["sprites/domes/%s_back.png" % name], x, y)
            # a stand-in player and plant so contrast can be judged
            out_prev.rect(x + W // 2 + 8, y + H - 12, x + W // 2 + 12, y + H - 1, hexc("e8dfc2"))
            out_prev.rect(x + W // 2 + 9, y + H - 11, x + W // 2 + 11, y + H - 9, hexc("5ff0ff"))
            out_prev.rect(x + W // 2 - 14, y + H - 7, x + W // 2 - 9, y + H - 1, hexc("77a648"))
            out_prev.blit(out["sprites/domes/%s_front.png" % name], x, y)
            x += W + 8
            rowh = max(rowh, H)
        write_png(os.path.join(PREVIEW_DIR, "domes_composite.png"), upscale(out_prev.crop(0, 0, 360, y + rowh + 8), 3,
                                                                           bgc=(60, 66, 78, 255)))
    return out


# --------------------------------------------------------------------------
# Props (props.png): 32x32 cells, 4 frames per row, bottom-centre anchored
# --------------------------------------------------------------------------
ST, BN, RU, MO, TE = R("steel"), R("bone"), R("rust"), R("moss"), R("teal")
WA, WD, GD, RD, VI = R("water"), R("wood"), R("gold"), R("red"), R("violet")
CY, LI, AM, MG, CU, IN = R("cyan"), R("lime"), R("amber"), R("mag"), R("copper"), R("indigo")
DARKIN = hexc("161424")  # interior shadow


def legs(sp, xs, y0, y1, ramp=None):
    ramp = ramp or ST
    for x in xs:
        sp.vline(x, y0, y1, ramp[2])
        sp.vline(x + 1, y0, y1, ramp[0])


def rivets(sp, pts, c=None):
    for (x, y) in pts:
        sp.set(x, y, c or ST[4])


def small_vine(sp, x, y, n, seed=0, ramp=None):
    ramp = ramp or MO
    rng = random.Random(seed * 131 + x * 7 + y)
    for i in range(n):
        sp.set(x, y + i, ramp[2])
        if rng.random() < 0.6:
            sp.set(x + rng.choice((-1, 1)), y + i, ramp[3])
        if rng.random() < 0.3:
            x += rng.choice((-1, 1))


def bed_base(sp, frame, mat, blanket, lit):
    # legs, frame rail, head and foot boards
    legs(sp, (4, 26), 27, 30, frame)
    sp.rect(3, 25, 28, 26, frame[2])
    sp.hline(3, 28, 25, frame[3])
    sp.hline(3, 28, 26, frame[1])
    box(sp, 2, 15, 3, 26, (frame[1], frame[2], frame[4]))
    box(sp, 28, 19, 29, 26, (frame[0], frame[2], frame[3]))
    # mattress and pillow
    sp.rect(4, 21, 27, 24, mat[3])
    sp.hline(4, 27, 21, mat[4])
    sp.hline(4, 27, 24, mat[2])
    for x in range(8, 27, 6):
        sp.set(x, 23, mat[2])
    sp.rect(5, 18, 10, 20, hexc("f4f0e6"))
    sp.hline(5, 10, 20, mat[2])
    sp.set(5, 18, WHITE)
    sp.set(10, 18, mat[3])
    sp.set(2, 16, lit)
    sp.set(3, 16, mix(lit, INK, 0.4))


def prop_bed(f):
    sp = Sprite(32, 32)
    bed_base(sp, ST, BN, TE, LI[3] if f in (0, 1, 2) else LI[1])
    # folded blanket at the foot
    box(sp, 20, 18, 27, 20, (TE[1], TE[2], TE[3]))
    sp.hline(20, 27, 20, TE[1])
    sp.hline(21, 26, 19, TE[3])
    sp.outline()
    return sp


def sleeper(sp, f, blanket, hair):
    breathe = (0, 1, 1, 0)[f]
    # head on the pillow
    skin = [hexc("8a6250"), hexc("c49878"), hexc("e4bc98")]
    sp.rect(6, 16, 9, 19, skin[1])
    sp.hline(6, 9, 16, hair)
    sp.set(6, 17, hair)
    sp.set(9, 17, skin[2])
    sp.set(8, 18, skin[0])
    sp.hline(6, 9, 19, skin[0])
    # blanket mound
    top = 17 - breathe
    for x in range(10, 28):
        t = (x - 10) / 17.0
        h = int(round((1 - (t * 2 - 0.7) ** 2 * 0.45) * (4 + breathe)))
        yt = 21 - max(1, h)
        if x < 12:
            yt = max(yt, 19)
        for y in range(yt, 22):
            sp.set(x, y, blanket[3] if y == yt else blanket[2])
        sp.set(x, 21, blanket[1])
        sp.set(x, 22, blanket[1])
    for x in range(13, 27, 5):
        sp.set(x, 20, blanket[1])
    sp.vline(27, 19, 24, blanket[1])
    sp.vline(26, 19, 23, blanket[2])
    # tiny "z"
    if f in (1, 2):
        zx, zy = (11, 12) if f == 1 else (13, 9)
        for (dx, dy) in ((0, 0), (1, 0), (2, 0), (1, 1), (0, 2), (1, 2), (2, 2)):
            sp.blend(zx + dx, zy + dy, wa(hexc("c8fcff"), 170))


def prop_bed_occupied(f):
    sp = Sprite(32, 32)
    bed_base(sp, ST, BN, TE, AM[3])
    sleeper(sp, f, TE, hexc("3a2418"))
    return soft_outline(sp)


def prop_bunk(f):
    sp = Sprite(32, 32)
    bed_base(sp, RU, BN, MO, LI[3] if f in (0, 1, 2) else LI[1])
    box(sp, 19, 18, 27, 20, (MO[1], MO[2], MO[3]))
    sp.hline(20, 26, 19, MO[3])
    # canvas strap + tag
    sp.vline(14, 21, 24, RU[2])
    sp.set(23, 23, AM[3])
    sp.outline()
    return sp


def prop_chest(f):
    sp = Sprite(32, 32)
    x0, x1 = 6, 25
    # body
    box(sp, x0, 21, x1, 30, (WD[1], WD[2], WD[3]))
    for y in (24, 27):
        sp.hline(x0 + 1, x1 - 1, y, WD[1])
    for x in (x0 + 3, x1 - 4):
        sp.vline(x, 21, 30, ST[2])
        sp.vline(x + 1, 21, 30, ST[1])
        sp.set(x, 22, ST[4])
        sp.set(x, 29, ST[3])
    sp.hline(x0, x1, 30, WD[0])
    if f == 0:
        box(sp, x0, 15, x1, 20, (WD[1], WD[3], WD[4]))
        sp.hline(x0 + 1, x1 - 1, 15, WD[4])
        sp.hline(x0, x1, 20, ST[1])
        sp.hline(x0, x1, 19, ST[3])
        for x in (x0 + 3, x1 - 4):
            sp.vline(x, 15, 20, ST[3])
            sp.vline(x + 1, 15, 20, ST[1])
        sp.rect(15, 19, 16, 22, GD[3])
        sp.set(15, 19, GD[4])
        sp.set(16, 22, GD[1])
        sp.set(16, 21, INK)
        small_vine(sp, x0 + 1, 15, 4, 3)
    elif f == 1:
        # lid ajar: tilted up at the back
        sp.hline(x0 + 1, x1 - 1, 20, DARKIN)
        sp.hline(x0 + 2, x1 - 2, 19, DARKIN)
        for i in range(x1 - x0 + 1):
            x = x0 + i
            lift = 3 - int(i * 0 + 0)
            sp.vline(x, 13, 17, WD[3] if i % 6 else WD[2])
            sp.set(x, 13, WD[4])
            sp.set(x, 18, ST[1])
            sp.set(x, 17, ST[3])
        for x in (x0 + 3, x1 - 4):
            sp.vline(x, 13, 18, ST[3])
        sp.rect(15, 17, 16, 19, GD[3])
        sp.set(14, 20, AM[3])
        sp.set(17, 20, AM[4])
    else:
        # lid upright behind, glowing contents
        box(sp, x0, 8, x1, 19, (WD[0], WD[1], WD[2]))
        sp.rect(x0 + 1, 9, x1 - 1, 18, mix(WD[1], INK, 0.45))
        for x in (x0 + 3, x1 - 4):
            sp.vline(x, 8, 19, ST[1])
            sp.set(x, 9, ST[2])
        sp.hline(x0, x1, 8, WD[3])
        sp.hline(x0 + 1, x1 - 1, 20, DARKIN)
        items = [(9, AM[3]), (11, AM[4]), (13, CY[3]), (15, AM[3]), (17, LI[3]), (19, AM[4]), (21, MG[3])]
        for i, (x, c) in enumerate(items):
            if (i + f) % 3 != 0 or f == 2:
                sp.set(x, 20, c)
            if (i * 2 + f) % 5 == 0:
                sp.set(x, 19, mix(c, WHITE, 0.5))
        sp.rect(15, 21, 16, 22, GD[3])
        sp.set(15, 21, GD[4])
    sp.outline()
    if f >= 2:
        glow_under(sp, 16, 18, 9, AM[3], 50)
    return sp


def hang_suit(sp, x, y, stripe):
    # helmet
    for (px_, py_) in m_ellipse(x + 3.5, y + 3, 3.4, 3):
        sp.set(px_, py_, BN[3] if px_ < x + 5 else BN[2])
    sp.rect(x + 2, y + 2, x + 5, y + 4, CY[1])
    sp.hline(x + 2, x + 4, y + 2, CY[3])
    sp.set(x + 2, y + 2, CY[4])
    sp.set(x + 1, y + 1, BN[4])
    # body
    sp.rect(x, y + 6, x + 7, y + 13, BN[2])
    sp.vline(x, y + 6, y + 13, BN[3])
    sp.vline(x + 1, y + 6, y + 12, BN[3])
    sp.vline(x + 7, y + 6, y + 13, BN[1])
    sp.hline(x, x + 7, y + 6, BN[4])
    sp.hline(x + 1, x + 6, y + 9, stripe)
    sp.rect(x + 3, y + 7, x + 4, y + 8, ST[2])
    # legs
    sp.rect(x + 1, y + 14, x + 2, y + 19, BN[2])
    sp.rect(x + 5, y + 14, x + 6, y + 19, BN[1])
    sp.vline(x + 1, y + 14, y + 19, BN[3])
    sp.rect(x + 1, y + 19, x + 2, y + 20, ST[1])
    sp.rect(x + 5, y + 19, x + 6, y + 20, ST[0])
    sp.rect(x + 3, y + 14, x + 4, y + 14, BN[1])


def prop_suit_rack(f):
    sp = Sprite(32, 32)
    legs(sp, (2, 28), 4, 30)
    sp.hline(1, 30, 30, ST[1])
    sp.rect(1, 3, 30, 4, ST[2])
    sp.hline(1, 30, 3, ST[4])
    sp.hline(1, 30, 4, ST[1])
    for x in (6, 15, 24):
        sp.set(x, 5, ST[3])
        sp.set(x + 1, 5, ST[1])
    stripes = [RU[3], TE[3], AM[3]]
    for i in range(3 - f):
        hang_suit(sp, 4 + i * 9, 6, stripes[i])
    for i in range(3 - f, 3):
        # empty hanger
        x = 6 + i * 9
        sp.vline(x, 6, 7, ST[2])
        sp.line(x, 8, x - 3, 10, ST[2])
        sp.line(x + 1, 8, x + 4, 10, ST[1])
    sp.set(29, 6, LI[3] if f < 3 else RD[3])
    sp.outline()
    return sp


def prop_synth(f):
    sp = Sprite(32, 32)
    box(sp, 3, 8, 28, 30, (ST[0], ST[1], ST[2]))
    sp.hline(3, 28, 8, ST[3])
    sp.rect(3, 26, 28, 30, ST[0])
    sp.hline(3, 28, 26, ST[2])
    # top hopper and exhaust
    box(sp, 6, 4, 14, 7, (ST[1], ST[2], ST[4]))
    box(sp, 21, 2, 24, 7, (CU[1], CU[2], CU[3]))
    sp.hline(20, 25, 2, CU[4])
    # chamber
    box(sp, 6, 11, 19, 23, (ST[3], ST[0], ST[0]))
    sp.rect(7, 12, 18, 22, hexc("0c2630"))
    for (x, y) in m_ellipse(12.5, 17.5, 5.5, 5):
        sp.set(x, y, mix(hexc("0c2630"), CY[2], 0.45))
    for (x, y) in m_ellipse(12.5, 17.5, 3.2, 3):
        sp.set(x, y, mix(hexc("0c2630"), CY[3], 0.7))
    # item being printed: grows with the frame
    h = (1, 2, 3, 4)[f]
    sp.rect(11, 21 - h, 14, 20, CY[4])
    sp.hline(11, 14, 21 - h, WHITE)
    sp.hline(10, 15, 21, CY[2])
    # scanning beam
    by = 13 + (f * 2) % 8
    sp.hline(7, 18, by, CY[3])
    sp.set(7 + (f * 3) % 11, by, WHITE)
    sp.set(8, 13, CY[4])
    # control panel
    box(sp, 21, 11, 26, 23, (ST[0], ST[1], ST[3]))
    for i, c in enumerate((AM[3], LI[3], RD[3], CY[3])):
        on = (i + f) % 4 != 0
        sp.set(22 + (i % 2) * 3, 13 + (i // 2) * 3, c if on else mix(c, INK, 0.7))
    sp.hline(22, 25, 19, BN[2])
    sp.hline(22, 24, 21, BN[1])
    # output tray
    sp.rect(8, 27, 17, 28, hexc("0c0a14"))
    sp.hline(8, 17, 29, ST[2])
    rivets(sp, ((4, 9), (27, 9), (4, 25), (27, 25)))
    sp.rect(22, 27, 26, 28, RU[2])
    sp.hline(22, 26, 27, RU[3])
    small_vine(sp, 27, 9, 9, 7)
    small_vine(sp, 4, 14, 6, 8)
    sp.outline()
    glow_under(sp, 12, 17, 12, CY[3], 26)
    return sp


def prop_comm(f):
    sp = Sprite(32, 32)
    # console
    box(sp, 5, 18, 22, 30, (ST[0], ST[1], ST[2]))
    sp.hline(5, 22, 18, ST[3])
    for i in range(4):
        sp.hline(4 - 0 + i, 23 - i, 17 - i, ST[2] if i else ST[3])
    # screen
    box(sp, 7, 20, 18, 27, (ST[3], ST[0], ST[0]))
    sp.rect(8, 21, 17, 26, hexc("0a2a2e"))
    for x in range(8, 18):
        y = 23.5 + 2 * math.sin((x + f * 2.4) * 0.9) * (0.4 + 0.6 * abs(math.sin((x + f) * 0.5)))
        sp.set(x, y, LI[3])
    sp.set(8, 21, CY[2])
    sp.set(20, 21, AM[3] if f % 2 else AM[1])
    sp.set(20, 24, RD[3] if f in (1, 2) else RD[1])
    sp.hline(19, 21, 27, BN[2])
    sp.hline(7, 20, 29, ST[0])
    # mast and dish
    sp.vline(24, 12, 30, ST[2])
    sp.vline(25, 12, 30, ST[0])
    sp.hline(22, 27, 30, ST[1])
    for (x, y) in m_ellipse(21.5, 7.5, 7.5, 6):
        cut = 7.5 + (x - 21.5) * 0.45
        if y + 0.5 < cut:
            continue
        e = ((x + 0.5 - 21.5) / 7.5) ** 2 + ((y + 0.5 - 7.5) / 6.0) ** 2
        rim = y + 0.5 < cut + 1.2
        sp.set(x, y, BN[4] if rim else BN[3] if e < 0.55 and x < 22 else BN[2] if e < 0.8 else BN[1])
    sp.line(20, 9, 16, 4, ST[3])
    sp.set(16, 3, AM[4] if f % 2 == 0 else RD[3])
    sp.set(15, 3, AM[3] if f % 2 == 0 else RD[2])
    # signal arcs
    for k in range(3):
        if (k + f) % 4 < 2:
            r = 3 + k * 2.5
            for a in range(200, 341, 12):
                x, y = 14.5 + math.cos(math.radians(a - 60)) * r, 5 + math.sin(math.radians(a - 60)) * r
                if y >= 1 and x >= 1 and a < 300:
                    sp.blend(x, y, wa(CY[3], 210 - k * 50))
    small_vine(sp, 6, 19, 7, 5)
    solid = Sprite(32, 32)
    solid.px = [p if p[3] == 255 else CLEAR for p in sp.px]
    solid.outline()
    for i, p in enumerate(sp.px):
        if 0 < p[3] < 255 and solid.px[i][3] == 0:
            solid.px[i] = p
    return solid


def prop_mod_bay(f):
    sp = Sprite(32, 32)
    # base plinth
    box(sp, 4, 27, 27, 30, (ST[0], ST[1], ST[3]))
    sp.hline(6, 25, 26, ST[2])
    for x in range(6, 26, 4):
        sp.set(x, 28, AM[3] if (x // 4 + f) % 2 else AM[1])
    # ring
    outer = m_ellipse(16, 14, 11.5, 12.5)
    inner = m_ellipse(16, 14, 8.5, 9.5)
    ring = outer - inner
    for (x, y) in ring:
        d = (x - 16) * -0.5 + (y - 14) * -0.6
        sp.set(x, y, ST[4] if d > 7 else ST[3] if d > 2 else ST[2] if d > -5 else ST[1])
    # running lights around the ring
    for k in range(12):
        a = math.radians(k * 30 + 15)
        x, y = 16 + math.cos(a) * 10, 14 + math.sin(a) * 11
        on = (k - f * 3) % 12 < 3
        sp.set(x - 0.5, y - 0.5, MG[3] if on else mix(MG[1], INK, 0.4))
        if on and (k - f * 3) % 12 == 1:
            sp.set(x - 0.5, y - 0.5, MG[4])
    # inner glow field
    for (x, y) in inner:
        d = math.hypot((x + 0.5 - 16) / 8.5, (y + 0.5 - 14) / 9.5)
        if d > 0.72:
            sp.blend(x, y, wa(MG[2], 70 if (x + y + f) % 2 else 40))
    # surgical arm reaching in from the right
    ay = 10 + (0, 2, 4, 2)[f]
    sp.line(27, 5, 23, ay, ST[3])
    sp.line(27, 6, 23, ay + 1, ST[1])
    sp.line(23, ay, 19, ay + 3, BN[3])
    sp.set(18, ay + 4, CY[4])
    sp.set(17, ay + 5, WHITE if f % 2 else CY[3])
    sp.rect(26, 3, 28, 6, ST[2])
    sp.set(26, 3, ST[4])
    # cradle on the left
    sp.line(6, 22, 11, 20, BN[2])
    sp.line(6, 23, 11, 21, BN[1])
    solid = Sprite(32, 32)
    solid.px = [p if p[3] == 255 else CLEAR for p in sp.px]
    solid.outline()
    for i, p in enumerate(sp.px):
        if 0 < p[3] < 255 and solid.px[i][3] == 0:
            solid.px[i] = p
    return solid


SOIL = [
    [hexc("5a4630"), hexc("86694a"), hexc("a88a64")],  # dry
    [hexc("3e2c20"), hexc("60442e"), hexc("7e5c3c")],  # damp
    [hexc("241810"), hexc("3c2818"), hexc("523a26")],  # wet
    [hexc("241810"), hexc("3c2818"), hexc("523a26")],  # wet + fertilized
]


def prop_planter(f):
    sp = Sprite(32, 32)
    x0, x1 = 4, 27
    # trough: riveted metal with a rusty rim
    box(sp, x0, 25, x1, 30, (ST[0], ST[1], ST[2]))
    sp.hline(x0, x1, 30, ST[0])
    sp.hline(x0 - 1, x1 + 1, 24, RU[3])
    sp.hline(x0 - 1, x1 + 1, 25, RU[1])
    sp.set(x0 - 1, 24, RU[4])
    for x in range(x0 + 2, x1, 5):
        sp.set(x, 27, ST[3])
    sp.rect(x0 + 8, 28, x0 + 15, 28, ST[0])
    sp.hline(x0 + 1, x1 - 1, 26, ST[2])
    # soil
    so = SOIL[f]
    for x in range(x0, x1 + 1):
        v = hash2(x, 3, 11)
        sp.set(x, 23, so[1] if v < 0.6 else so[2])
        if 0.2 < v < 0.75 and x0 < x < x1:
            sp.set(x, 22, so[2] if v > 0.5 else so[1])
    if f == 0:
        for x in (8, 15, 22):
            sp.set(x, 23, so[0])
            sp.set(x + 1, 22, so[0])
    if f >= 2:
        for x in (7, 13, 19, 24):
            sp.set(x, 23, WA[3])
        sp.set(10, 22, WA[4])
        sp.set(21, 22, WA[4])
    if f == 3:
        for x in (6, 11, 17, 23):
            sp.set(x, 22, LI[3])
        sp.set(14, 23, LI[4])
    # moisture gauge
    sp.set(x1 - 2, 28, [RD[3], AM[3], CY[3], LI[3]][f])
    small_vine(sp, x0, 25, 4, 2)
    sp.outline()
    if f == 3:
        for (x, y) in ((8, 19), (20, 18)):
            sp.blend(x, y, wa(LI[4], 200))
            sp.blend(x - 1, y, wa(LI[3], 90))
            sp.blend(x + 1, y, wa(LI[3], 90))
            sp.blend(x, y - 1, wa(LI[3], 90))
            sp.blend(x, y + 1, wa(LI[3], 90))
    return sp


def prop_dark_planter(f):
    sp = Sprite(32, 32)
    bark = [hexc("1e1420"), hexc("32222c"), hexc("4a3238"), hexc("644848")]
    # log
    for y in range(24, 31):
        for x in range(3, 29):
            t = (y - 24) / 6.0
            c = bark[3] if t < 0.2 else bark[2] if t < 0.55 else bark[1]
            if hash2(x // 3, y, 2) < 0.22:
                c = bark[1] if t < 0.55 else bark[0]
            sp.set(x, y, c)
    # cut ends
    for ex in (3, 28):
        sp.vline(ex, 24, 30, hexc("8a6c58") if ex == 3 else bark[0])
    sp.set(4, 27, hexc("b0907a"))
    sp.set(4, 26, hexc("8a6c58"))
    sp.set(4, 28, hexc("8a6c58"))
    # substrate
    so = [[hexc("4a3a44"), hexc("6a5660"), hexc("86707a")],
          [hexc("32243a"), hexc("4a3652"), hexc("624a6a")],
          [hexc("1c1228"), hexc("2c1c3e"), hexc("402a56")],
          [hexc("1c1228"), hexc("2c1c3e"), hexc("402a56")]][f]
    for x in range(4, 28):
        v = hash2(x, 5, 17)
        sp.set(x, 23, so[1] if v < 0.6 else so[2])
        if 0.25 < v < 0.8 and 4 < x < 27:
            sp.set(x, 22, so[2] if v > 0.55 else so[1])
    if f >= 2:
        for x in (7, 14, 20, 25):
            sp.set(x, 23, VI[4])
        sp.set(11, 22, hexc("d8c8ff"))
    if f == 3:
        for x in (9, 16, 22):
            sp.set(x, 22, MG[3])
    # small bracket fungus + mycelium
    sp.hline(23, 25, 27, VI[3])
    sp.set(23, 27, VI[4])
    sp.hline(23, 24, 28, VI[2])
    for x in (8, 12, 17):
        sp.set(x, 26 + (x % 3), hexc("8a7a96"))
    sp.outline()
    if f == 3:
        for (x, y) in ((10, 19), (21, 18)):
            sp.blend(x, y, wa(MG[4], 200))
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                sp.blend(x + dx, y + dy, wa(MG[3], 90))
    return sp


def water_fill(sp, x0, y0, x1, y1, f, surface=True, seed=0):
    """Water body with a rippling surface and drifting caustic glints."""
    for y in range(y0, y1 + 1):
        t = (y - y0) / max(1.0, float(y1 - y0))
        for x in range(x0, x1 + 1):
            c = WA[3] if t < 0.3 else WA[2] if t < 0.72 else WA[1]
            if 0.26 < t < 0.34 and (x + y) % 2:
                c = WA[2]
            if 0.68 < t < 0.76 and (x + y) % 2:
                c = WA[1]
            sp.set(x, y, c)
    if surface:
        for x in range(x0, x1 + 1):
            ph = (x + f * 2 + seed) % 8
            sp.set(x, y0, WA[4] if ph < 4 else mix(WA[4], WHITE, 0.6) if ph < 5 else WA[3])
    for k in range(5):
        gx = x0 + 1 + (k * 7 + f * 2 + seed * 3) % max(1, (x1 - x0 - 1))
        gy = y0 + 2 + (k * 5 + seed) % max(1, (y1 - y0 - 2))
        sp.set(gx, gy, WA[4])
        sp.set(gx + 1, gy, WA[3])


def prop_aqua_planter(f):
    sp = Sprite(32, 32)
    x0, x1, y0, y1 = 4, 27, 11, 30
    # rim and base
    box(sp, x0, y0, x1, y0 + 1, (ST[1], ST[2], ST[4]))
    box(sp, x0, y1 - 2, x1, y1, (ST[0], ST[1], ST[3]))
    # glass body
    water_fill(sp, x0 + 1, y0 + 3, x1 - 1, y1 - 3, f)
    sp.rect(x0 + 1, y0 + 2, x1 - 1, y0 + 2, hexc("1a3048"))
    sp.vline(x0, y0 + 2, y1 - 3, hexc("a8e8f0"))
    sp.vline(x1, y0 + 2, y1 - 3, hexc("3a7a90"))
    # gravel
    for x in range(x0 + 1, x1):
        v = hash2(x, 1, 23)
        sp.set(x, y1 - 3, BN[1] if v < 0.4 else BN[2] if v < 0.8 else ST[2])
        if v > 0.55:
            sp.set(x, y1 - 4, BN[1] if v < 0.8 else BN[0])
    # glass shine
    sp.vline(x0 + 2, y0 + 5, y0 + 10, hexc("c8f4ff"))
    sp.set(x0 + 3, y0 + 4, hexc("c8f4ff"))
    sp.vline(x0 + 4, y0 + 6, y0 + 8, WA[4])
    # bubbles
    for k in range(3):
        bx = 10 + k * 6
        by = y1 - 6 - ((f * 3 + k * 5) % 11)
        sp.set(bx, by, hexc("d8fcff"))
    rivets(sp, ((x0 + 1, y1 - 1), (x1 - 1, y1 - 1)), ST[3])
    sp.set(x1 - 3, y1 - 1, CY[3])
    sp.outline()
    return sp


def steam(sp, x, y, f, seed=0, n=3, c=None):
    c = c or hexc("e8f0f0")
    for k in range(n):
        ph = (f + k * 2 + seed) % 4
        yy = y - ph * 2 - k
        xx = x + (1 if (ph + k) % 2 else -1) * (1 if ph else 0) + (k - 1)
        a = 200 - ph * 45
        sp.blend(xx, yy, wa(c, a))
        if ph < 3:
            sp.blend(xx + 1, yy, wa(c, a - 70))
        if ph == 1:
            sp.blend(xx, yy - 1, wa(c, a - 90))


def soft_outline(sp):
    """Outline only fully opaque pixels; keep translucent ones (steam, glow)."""
    solid = Sprite(sp.w, sp.h)
    solid.px = [p if p[3] == 255 else CLEAR for p in sp.px]
    solid.outline()
    for i, p in enumerate(sp.px):
        if 0 < p[3] < 255 and solid.px[i][3] == 0:
            solid.px[i] = p
    return solid


def prop_cooker(f):
    sp = Sprite(32, 32)
    # body
    box(sp, 5, 17, 26, 30, (ST[0], ST[2], ST[3]))
    sp.hline(4, 27, 16, ST[4])
    sp.hline(4, 27, 17, ST[1])
    sp.rect(5, 29, 26, 30, ST[0])
    # oven window with fire
    box(sp, 8, 20, 19, 27, (ST[3], ST[0], ST[0]))
    sp.rect(9, 21, 18, 26, hexc("2a1008"))
    for x in range(9, 19):
        h = 2 + int(2.2 * abs(math.sin(x * 1.3 + f * 1.7)))
        for y in range(26 - h + 1, 27):
            sp.set(x, y, AM[4] if y > 25 else AM[3] if y > 24 else AM[2] if y > 23 else RD[2])
    # knobs
    for i, y in enumerate((20, 23, 26)):
        sp.set(22, y, BN[3])
        sp.set(23, y, BN[1])
        sp.set(24, y, RD[3] if i == f % 3 else ST[0])
    # pot (copper) and pan
    box(sp, 7, 10, 15, 15, (CU[1], CU[2], CU[3]))
    sp.hline(6, 16, 9, CU[4])
    sp.hline(6, 16, 10, CU[1])
    sp.set(5, 11, CU[2])
    sp.set(17, 11, CU[1])
    sp.set(8, 12, CU[4])
    sp.hline(7, 15, 15, CU[0])
    sp.rect(19, 13, 24, 15, ST[1])
    sp.hline(18, 25, 13, ST[3])
    sp.hline(25, 28, 13, WD[2])
    sp.hline(20, 23, 12, MO[3])
    sp.set(21, 12, AM[3])
    # chimney pipe
    steam(sp, 10, 7, f, 0)
    steam(sp, 13, 7, f, 2, n=2)
    steam(sp, 21, 10, f, 1, n=2)
    out = soft_outline(sp)
    glow_under(out, 13, 24, 8, AM[2], 30)
    return out


def prop_compost(f):
    sp = Sprite(32, 32)
    # vat: staved barrel with steel hoops
    for y in range(14, 31):
        t = (y - 14) / 16.0
        bulge = int(round(1.6 * math.sin(t * math.pi)))
        x0, x1 = 6 - bulge, 25 + bulge
        for x in range(x0, x1 + 1):
            u = (x - x0) / float(x1 - x0)
            c = WD[3] if u < 0.18 else WD[4] if u < 0.3 and y % 2 else WD[3] if u < 0.45 else WD[2] if u < 0.8 else WD[1]
            if (x - 6) % 4 == 3:
                c = WD[1] if u < 0.8 else WD[0]
            sp.set(x, y, c)
    for y in (17, 27):
        t = (y - 14) / 16.0
        bulge = int(round(1.6 * math.sin(t * math.pi)))
        for x in range(6 - bulge, 26 + bulge):
            u = (x - 6) / 20.0
            sp.set(x, y, ST[4] if u < 0.3 else ST[3] if u < 0.7 else ST[1])
            sp.set(x, y + 1, ST[1] if u < 0.7 else ST[0])
    # sludge surface
    sl = [hexc("2c3a14"), hexc("4a5a1c"), hexc("6e8428"), hexc("a0b840")]
    for x in range(7, 25):
        sp.set(x, 13, sl[2] if (x + f) % 5 else sl[3])
        sp.set(x, 14, sl[1])
    sp.hline(6, 25, 15, WD[4])
    # bubbles
    for k, bx in enumerate((9, 14, 19, 22)):
        ph = (f + k) % 4
        if ph == 0:
            sp.set(bx, 13, sl[3])
        elif ph == 1:
            sp.set(bx, 12, sl[3])
            sp.set(bx + 1, 12, sl[2])
            sp.set(bx, 11, sl[2])
            sp.set(bx + 1, 11, sl[3])
        elif ph == 2:
            for (dx, dy) in ((0, -1), (1, -1), (-1, 0), (2, 0), (0, 1), (1, 1)):
                sp.set(bx + dx, 10 + dy, sl[3] if dx <= 0 else sl[2])
        else:
            sp.blend(bx - 1, 9, wa(sl[3], 150))
            sp.blend(bx + 2, 10, wa(sl[3], 120))
            sp.blend(bx, 8, wa(sl[3], 90))
    # scraps poking out, tap, drip of fertilizer
    sp.set(11, 12, MO[3])
    sp.set(12, 11, MO[2])
    sp.set(17, 12, RU[3])
    sp.rect(26, 23, 28, 24, CU[2])
    sp.set(28, 25, CU[1])
    sp.hline(26, 28, 23, CU[4])
    sp.set(28, 26 + f % 4, sl[2]) if f % 2 else None
    out = soft_outline(sp)
    return out


def prop_assembly(f):
    sp = Sprite(32, 32)
    # ring gantry
    outer = m_ellipse(16, 16, 14.5, 14.5)
    inner = m_ellipse(16, 16, 12, 12)
    for (x, y) in outer - inner:
        if y > 28:
            continue
        d = (x - 16) * -0.5 + (y - 16) * -0.6
        c = ST[4] if d > 9 else ST[3] if d > 3 else ST[2] if d > -6 else ST[1]
        ang = math.degrees(math.atan2(y - 16, x - 16)) % 360
        if int(ang / 15) % 4 == 0:
            c = AM[2] if d > -6 else AM[1]
        sp.set(x, y, c)
    # feet
    box(sp, 3, 27, 9, 30, (ST[0], ST[1], ST[3]))
    box(sp, 22, 27, 28, 30, (ST[0], ST[1], ST[3]))
    sp.hline(9, 22, 30, ST[1])
    # tiny dome being assembled (more panes each frame)
    dome = m_ellipse(16, 29.5, 6.5, 7)
    panes = (2, 3, 4, 5)[f]
    for (x, y) in dome:
        if y > 29:
            continue
        seg = int((x - 9.5) / 13.0 * 5)
        e = ((x + 0.5 - 16) / 6.5) ** 2 + ((y + 0.5 - 29.5) / 7.0) ** 2
        if e > 0.68:
            sp.set(x, y, ST[4] if x < 16 else ST[2])
        elif seg < panes:
            sp.set(x, y, TE[4] if (x + y) % 5 == 0 else TE[3] if x < 15 else TE[2])
        if (x - 10) % 3 == 0 and e <= 0.68 and seg <= panes:
            sp.set(x, y, ST[2])
    # arms
    a1 = (0, 1, 2, 1)[f]
    sp.line(6, 9, 10, 14 + a1, ST[3])
    sp.line(10, 14 + a1, 13, 19 + a1, BN[3])
    sp.rect(5, 8, 7, 10, AM[2])
    sp.set(5, 8, AM[4])
    sp.rect(9, 13 + a1, 11, 15 + a1, ST[2])
    a2 = (2, 1, 0, 1)[f]
    sp.line(26, 9, 22, 13 + a2, ST[2])
    sp.line(22, 13 + a2, 19, 18 + a2, BN[2])
    sp.rect(25, 8, 27, 10, AM[2])
    sp.set(25, 8, AM[4])
    sp.rect(21, 12 + a2, 23, 14 + a2, ST[2])
    out = soft_outline(sp)
    # weld sparks
    sx, sy = (13, 20 + a1) if f % 2 == 0 else (19, 19 + a2)
    out.set(sx, sy, WHITE)
    for k in range(5):
        ang = k * 1.3 + f * 0.9
        out.blend(sx + math.cos(ang) * (1.5 + k % 3), sy + math.sin(ang) * (1.5 + k % 2), wa(AM[4], 230 - 30 * (k % 3)))
    return out


def prop_fish_pool(f):
    sp = Sprite(32, 32)
    x0, x1, y0, y1 = 1, 30, 19, 30
    box(sp, x0, y1 - 1, x1, y1, (ST[0], ST[1], ST[3]))
    box(sp, x0, y0, x1, y0 + 1, (ST[1], ST[2], ST[4]))
    water_fill(sp, x0 + 1, y0 + 2, x1 - 1, y1 - 2, f, seed=3)
    sp.vline(x0, y0 + 2, y1 - 2, hexc("a8e8f0"))
    sp.vline(x1, y0 + 2, y1 - 2, hexc("3a7a90"))
    for x in (10, 20):
        sp.vline(x, y0 + 2, y1 - 2, ST[2])
    # fish swimming back and forth
    fx = (5, 9, 13, 9)[f]
    d = 1 if f < 2 else -1
    for (dx, c) in ((0, RU[3]), (1, RU[4]), (2, RU[3]), (3, RU[2])):
        sp.set(fx + dx * d + (3 if d < 0 else 0), 24, c)
    sp.set(fx + (4 if d > 0 else -1), 23, RU[2])
    sp.set(fx + (4 if d > 0 else -1), 25, RU[2])
    fx2 = (24, 21, 18, 21)[f]
    sp.hline(fx2, fx2 + 2, 26, AM[3])
    sp.set(fx2 + (3 if f >= 2 else -1), 26, AM[2])
    # ripples above the rim
    sp.set(4 + f * 2, y0 + 2, WHITE)
    sp.set(22 - f * 2, y0 + 2, WHITE)
    sp.set(15 + f, y0 + 2, WHITE)
    small_vine(sp, x1 - 1, y0, 5, 4)
    sp.set(3, y1, CY[3])
    sp.outline()
    return sp


def prop_stall(f):
    sp = Sprite(32, 32)
    # straw floor
    hay = [hexc("6a5020"), hexc("9a7a2c"), hexc("c8a440"), hexc("e8cc68")]
    for x in range(3, 29):
        v = hash2(x, 0, 31)
        sp.set(x, 30, hay[1] if v < 0.5 else hay[2])
        if v > 0.35:
            sp.set(x, 29, hay[2] if v < 0.8 else hay[3])
        if v > 0.82:
            sp.set(x, 28, hay[2])
    # posts
    for x in (2, 15, 28):
        box(sp, x, 14, x + 1, 30, (WD[1], WD[3], WD[4]))
        sp.rect(x - 0, 13, x + 1, 13, WD[4])
    # rails
    for y in (17, 22, 27):
        sp.hline(1, 30, y, WD[3])
        sp.hline(1, 30, y + 1, WD[1])
        for x in range(4, 30, 7):
            sp.set(x, y, WD[2])
    for x in (2, 15, 28):
        for y in (17, 22, 27):
            sp.set(x, y, ST[4])
    # plaque + lantern hook
    box(sp, 6, 19, 11, 21, (BN[1], BN[3], BN[4]))
    sp.hline(7, 10, 20, BN[1])
    small_vine(sp, 28, 14, 6, 9)
    small_vine(sp, 16, 18, 4, 12)
    sp.outline()
    return sp


def prop_trough(f):
    sp = Sprite(32, 32)
    legs(sp, (6, 24), 26, 30, WD)
    # trapezoid trough
    for i, y in enumerate(range(21, 27)):
        x0, x1 = 4 + i // 2, 27 - i // 2
        for x in range(x0, x1 + 1):
            sp.set(x, y, WD[3] if x < x0 + 2 else WD[1] if x > x1 - 2 else WD[2])
        sp.set(x0 + 5, y, WD[1])
        sp.set(x1 - 7, y, WD[1])
    sp.hline(3, 28, 20, WD[4])
    sp.hline(3, 28, 21, WD[2])
    sp.set(4, 23, ST[4])
    sp.set(27, 23, ST[2])
    # feed
    fd = [hexc("7a6a24"), hexc("b09a38"), hexc("d8c858"), MO[3]]
    if f == 0:
        sp.hline(5, 26, 20, hexc("2a1a14"))
        sp.hline(4, 27, 19, WD[4])
        sp.set(9, 20, fd[1])
    else:
        h = f
        for x in range(5, 27):
            v = hash2(x, f, 41)
            top = 20 - h + (1 if v < 0.35 else 0) + (1 if x in (5, 26) else 0)
            for y in range(top, 21):
                sp.set(x, y, fd[2] if y == top and v < 0.7 else fd[3] if v > 0.85 else fd[1])
        sp.set(4, 20, WD[4])
        sp.set(27, 20, WD[2])
    sp.outline()
    return sp


def prop_condenser(f):
    sp = Sprite(32, 32)
    # frame
    box(sp, 6, 3, 25, 6, (ST[1], ST[2], ST[4]))
    legs(sp, (7, 23), 7, 27)
    # drip tray
    box(sp, 4, 27, 27, 30, (ST[0], ST[1], ST[3]))
    sp.hline(6, 25, 28, WA[3])
    sp.set(8 + f * 4, 28, WA[4])
    # coils
    cold = [hexc("1e6276"), hexc("3a9ab0"), hexc("7ad8e8"), hexc("c8f6ff")]
    for i, y in enumerate(range(8, 24, 3)):
        rp = cold if i % 2 == 0 else [CU[1], CU[2], CU[3], CU[4]]
        sp.hline(10, 21, y, rp[2])
        sp.hline(10, 21, y + 1, rp[0])
        sp.set(11 + (i * 3) % 8, y, rp[3])
        bx = 22 if i % 2 == 0 else 9
        sp.vline(bx, y, y + 3, rp[1]) if y + 3 < 24 else None
    for x in range(11, 22, 3):
        for y in range(8, 25):
            if sp.get(x, y)[3] == 0:
                sp.set(x, y, ST[1])
    # fan cap
    sp.hline(12, 19, 2, ST[3])
    sp.set(13 + f * 2 % 6, 2, ST[4])
    sp.set(24, 4, CY[3] if f % 2 == 0 else CY[1])
    # falling drops
    for k, x in enumerate((11, 15, 19)):
        ph = (f + k) % 4
        if ph < 3:
            sp.set(x, 24 + ph, hexc("c8f6ff") if ph == 0 else WA[4])
    sp.outline()
    return sp


def prop_berth(f):
    sp = Sprite(32, 32)
    for i, y0 in enumerate((2, 17)):
        # capsule shell
        for (x, y) in m_rect(2, y0, 29, y0 + 13):
            cx_, cy_ = min(max(x, 6), 25), y0 + 6.5
            if math.hypot(x - cx_, (y + 0.5 - cy_) * 0.62) > 4.3:
                continue
            v = (y - y0) / 13.0
            sp.set(x, y, BN[4] if v < 0.12 else BN[3] if v < 0.5 else BN[2] if v < 0.85 else BN[1])
        # window
        lit = f >= 1
        occ = f >= 2
        wc = hexc("121428") if not lit else hexc("ffd890") if not occ else hexc("f0b060")
        for (x, y) in m_rect(6, y0 + 3, 20, y0 + 9):
            cx_ = min(max(x, 8), 18)
            if math.hypot(x - cx_, y - (y0 + 6)) > 3.2:
                continue
            sp.set(x, y, wc)
        if not lit:
            sp.hline(8, 11, y0 + 4, hexc("2a2e52"))
            sp.set(8, y0 + 5, hexc("2a2e52"))
        else:
            sp.hline(8, 17, y0 + 4, mix(wc, WHITE, 0.5))
        if occ:
            # sleeper silhouette: head + shoulder under a blanket
            dk = hexc("5a3020")
            sp.rect(9, y0 + 6, 10, y0 + 7, dk)
            sp.hline(11, 18, y0 + 8, TE[2])
            sp.hline(12, 18, y0 + 7, TE[3] if (f + i) % 2 else TE[2])
            sp.hline(8, 19, y0 + 9, hexc("a06a38"))
        # hatch seam, handle, status light, number stripe
        sp.vline(22, y0 + 2, y0 + 11, BN[1])
        sp.rect(24, y0 + 5, 25, y0 + 8, ST[2])
        sp.set(24, y0 + 5, ST[4])
        sp.set(27, y0 + 4, (RD[1] if not lit else LI[3] if not occ else AM[3]))
        sp.hline(4, 7, y0 + 11, RU[3])
        sp.set(9 + i * 2, y0 + 11, RU[3])
    # rack frame between the pods and ladder rungs
    sp.rect(2, 15, 29, 16, ST[1])
    sp.hline(2, 29, 15, ST[3])
    for y in (5, 9, 20, 24, 28):
        sp.hline(29, 30, y, ST[2])
    sp.vline(30, 3, 30, ST[1])
    sp.outline()
    return sp


def prop_sprinkler(f):
    sp = Sprite(32, 32)
    # stub riser and rotating head
    sp.rect(15, 24, 16, 30, CU[2])
    sp.vline(15, 24, 30, CU[3])
    sp.hline(13, 18, 30, ST[1])
    sp.hline(14, 17, 29, ST[2])
    sp.rect(13, 21, 18, 23, ST[2])
    sp.hline(13, 18, 21, ST[4])
    sp.hline(13, 18, 23, ST[0])
    sp.set(12, 22, ST[3])
    sp.set(19, 22, ST[1])
    sp.set(15 + f % 2, 20, CY[3])
    out = soft_outline(sp)
    # spray arcs
    for side in (-1, 1):
        for k in range(7):
            t = (k + f * 0.5) / 7.0
            x = 16 + side * (3 + t * 11)
            y = 21 - math.sin(t * math.pi) * 9 + t * 8
            a = 230 - int(t * 120)
            if (k + f) % 2 == 0:
                out.blend(x, y, wa(hexc("c8f6ff"), a))
            else:
                out.blend(x, y, wa(WA[3], a - 40))
        for k in range(3):
            t = ((k * 3 + f) % 8) / 8.0
            x = 16 + side * (2 + t * 6)
            y = 20 - math.sin(t * math.pi) * 5 + t * 9
            out.blend(x, y, wa(WA[4], 170))
    return out


def prop_tap(f):
    sp = Sprite(32, 32)
    # standpipe
    sp.rect(15, 12, 17, 30, CU[2])
    sp.vline(15, 12, 30, CU[3])
    sp.vline(17, 12, 30, CU[1])
    for y in (17, 25):
        sp.hline(14, 18, y, CU[4])
        sp.hline(14, 18, y + 1, CU[1])
    sp.set(16, 21, hexc("3f8f7a"))
    sp.set(15, 22, hexc("7fcfae"))
    # elbow + spout to the left
    sp.rect(9, 12, 17, 14, CU[2])
    sp.hline(9, 17, 12, CU[4])
    sp.hline(9, 16, 14, CU[1])
    sp.rect(8, 13, 10, 16, CU[2])
    sp.vline(8, 13, 16, CU[3])
    sp.hline(7, 11, 17, ST[2])
    # valve wheel
    sp.vline(16, 9, 11, ST[2])
    sp.hline(13, 19, 8, RD[2])
    sp.hline(14, 18, 7, RD[3])
    sp.set(16, 7, RD[4])
    sp.set(13, 8, RD[3])
    # base plate with a grate
    box(sp, 5, 28, 21, 30, (ST[0], ST[1], ST[3]))
    for x in range(7, 13, 2):
        sp.set(x, 29, INK)
    out = soft_outline(sp)
    # drip
    dy = (0, 3, 6, 9)[f]
    if f < 3:
        out.set(9, 19 + dy, hexc("c8f6ff"))
        out.set(9, 20 + dy, WA[3])
    else:
        out.blend(7, 27, wa(WA[4], 220))
        out.blend(11, 27, wa(WA[4], 220))
        out.blend(9, 26, wa(hexc("c8f6ff"), 220))
    centred = Sprite(32, 32)
    centred.blit(out, 3, 0)
    return centred


def prop_lamp(f):
    sp = Sprite(32, 32)
    # base and pole
    sp.hline(12, 19, 30, ST[1])
    sp.hline(13, 18, 29, ST[2])
    sp.rect(15, 11, 16, 28, ST[2])
    sp.vline(15, 11, 28, ST[3])
    sp.set(15, 20, ST[4])
    sp.hline(14, 17, 21, ST[1])
    # lantern head
    box(sp, 12, 2, 19, 3, (ST[1], ST[2], ST[4]))
    sp.rect(13, 4, 18, 9, AM[3])
    sp.rect(14, 5, 16, 8, AM[4])
    sp.set(15, 6, WHITE)
    if f == 2:
        sp.rect(14, 5, 16, 8, mix(AM[4], AM[3], 0.5))
    sp.vline(13, 4, 9, AM[2])
    sp.vline(18, 4, 9, AM[1])
    sp.rect(12, 10, 19, 10, ST[1])
    sp.hline(12, 18, 10, ST[3])
    small_vine(sp, 16, 13, 9, 21)
    sp.outline()
    glow_under(sp, 16, 6.5, (11, 11.6, 10.2, 11.3)[f], AM[3], (95, 105, 80, 100)[f])
    return sp


def battery_flower(sp, x, y, on=True):
    """Tiny battery-flower cell: petals + glowing cell (5x5-ish)."""
    c = LI if on else [mix(q, INK, 0.5) for q in LI]
    sp.rect(x, y, x + 1, y + 3, c[3])
    sp.set(x, y, c[4] if on else c[3])
    sp.set(x + 1, y + 3, c[2])
    sp.hline(x, x + 1, y - 1, ST[3])
    sp.set(x - 1, y + 1, MG[3])
    sp.set(x + 2, y + 1, MG[2])
    sp.set(x - 1, y + 2, MG[2])
    sp.set(x + 2, y + 2, MG[1])
    sp.set(x, y + 4, MO[2])


def prop_pylon(f):
    sp = Sprite(32, 32)
    # footing
    box(sp, 8, 27, 23, 30, (ST[0], ST[1], ST[3]))
    sp.hline(10, 21, 26, ST[2])
    # lattice mast
    for y in range(6, 27):
        w = 1 + (y - 6) // 8
        sp.set(16 - w - 1, y, ST[3])
        sp.set(15 + w + 1, y, ST[1])
        if y % 4 == 0:
            sp.hline(16 - w - 1, 15 + w + 1, y, ST[2])
        elif y % 4 == 2:
            sp.set(15, y, ST[1])
            sp.set(16, y, ST[0])
    # coil head
    on = f > 0
    cc = CY if on else [mix(q, INK, 0.55) for q in CY]
    sp.rect(14, 1, 17, 5, cc[2])
    sp.vline(14, 1, 5, cc[3])
    sp.hline(14, 17, 1, cc[4] if f == 3 else cc[3])
    sp.hline(13, 18, 3, ST[3])
    sp.hline(13, 18, 6, ST[2])
    if f == 3:
        sp.set(15, 2, WHITE)
        sp.set(16, 4, WHITE)
    # rack arms with battery flowers (6 sockets)
    slots = [(6, 10), (24, 10), (4, 16), (26, 16), (6, 22), (24, 22)]
    for i, (x, y) in enumerate(slots):
        ax0, ax1 = (x + 1, 14) if x < 16 else (17, x)
        sp.hline(min(ax0, ax1), max(ax0, ax1), y + 4, ST[2])
        sp.set(x, y + 4, ST[3])
        sp.set(x + 1, y + 4, ST[1])
        if i < f * 2:
            battery_flower(sp, x, y, True)
    sp.outline()
    if f > 0:
        glow_under(sp, 16, 3, 5 + f * 1.5, CY[3], 40 + f * 22)
        for i, (x, y) in enumerate(slots[:f * 2]):
            glow_under(sp, x + 1, y + 2, 4, LI[3], 60)
    return sp


def prop_pump(f):
    sp = Sprite(32, 32)
    # body
    box(sp, 7, 18, 24, 30, (ST[0], ST[1], ST[2]))
    sp.hline(7, 24, 18, ST[4])
    sp.rect(7, 28, 24, 30, ST[0])
    sp.hline(6, 25, 30, ST[1])
    rivets(sp, ((8, 19), (23, 19), (8, 27), (23, 27)), ST[3])
    # pipe stubs
    hcyl(sp, 1, 22, 6, 25, CU)
    hcyl(sp, 25, 22, 30, 25, CU)
    sp.vline(1, 21, 26, CU[4])
    sp.vline(30, 21, 26, CU[1])
    sp.vline(6, 21, 26, CU[1])
    sp.vline(25, 21, 26, CU[3])
    # sight window with flowing water
    box(sp, 10, 21, 16, 26, (ST[3], ST[0], ST[0]))
    sp.rect(11, 22, 15, 25, WA[2])
    for k in range(3):
        sp.set(11 + (k * 2 + f) % 5, 22 + k, WA[4])
    # flywheel
    for (x, y) in m_ellipse(20.5, 23.5, 2.6, 2.6):
        sp.set(x, y, RU[2])
    sp.set(20, 23, RU[4])
    ang = f * math.pi / 2
    sp.set(20.5 + math.cos(ang) * 1.6, 23.5 + math.sin(ang) * 1.6, BN[4])
    # piston
    py_ = (0, 2, 4, 2)[f]
    box(sp, 12, 12, 19, 17, (ST[1], ST[2], ST[3]))
    sp.rect(14, 5 + py_, 17, 11, ST[3])
    sp.vline(14, 5 + py_, 11, ST[4])
    sp.vline(17, 5 + py_, 11, ST[1])
    box(sp, 12, 3 + py_, 19, 5 + py_, (RU[1], RU[3], RU[4]))
    sp.set(21, 14, LI[3] if f % 2 == 0 else LI[1])
    sp.hline(13, 18, 14, AM[2])
    sp.outline()
    return sp


def prop_tank(f):
    sp = Sprite(32, 32)
    legs(sp, (8, 22), 27, 30)
    sp.hline(6, 25, 30, ST[1])
    # tank body
    cyl(sp, 6, 6, 25, 26, ST)
    for (x, y) in m_ellipse(16, 6.5, 10, 3.5):
        if y <= 6:
            sp.set(x, y, ST[4] if x < 13 else ST[3] if x < 20 else ST[2])
    sp.hline(6, 25, 26, ST[0])
    for y in (10, 22):
        sp.hline(6, 25, y, ST[1])
    # sight glass
    sp.rect(12, 9, 16, 24, hexc("0c1a2c"))
    lvl = (1, 5, 10, 15)[f]
    for y in range(24 - lvl + 1, 25):
        sp.hline(12, 16, y, WA[3] if y > 24 - lvl + 1 else WA[4])
        sp.set(12, y, WA[4])
        sp.set(16, y, WA[2])
    sp.vline(11, 8, 25, ST[4])
    sp.vline(17, 8, 25, ST[0])
    sp.hline(11, 17, 8, ST[4])
    sp.hline(11, 17, 25, ST[0])
    for y in range(11, 24, 4):
        sp.set(18, y, BN[3])
    # valve + pipe
    hcyl(sp, 26, 20, 29, 22, CU)
    sp.set(27, 19, RD[3])
    sp.set(28, 19, RD[2])
    # droplet stencil + patina
    sp.set(21, 13, WA[3])
    sp.rect(20, 14, 22, 15, WA[3])
    sp.set(21, 16, WA[2])
    small_vine(sp, 7, 7, 8, 14)
    sp.outline()
    return sp


def prop_oxygen(f):
    sp = Sprite(32, 32)
    # base
    box(sp, 4, 25, 27, 30, (ST[0], ST[1], ST[3]))
    sp.hline(6, 25, 24, ST[2])
    for x in range(7, 25, 3):
        sp.set(x, 28, INK)
    sp.set(24, 26, LI[3] if f % 2 == 0 else LI[2])
    # algae cylinder
    alg = [hexc("123a1c"), hexc("1e5a24"), hexc("38862c"), hexc("6ab83a"), hexc("b4e868")]
    for y in range(10, 24):
        for x in range(7, 25):
            u = (x - 7) / 17.0
            c = alg[3] if u < 0.14 else alg[4] if u < 0.24 else alg[3] if u < 0.4 else alg[2] if u < 0.78 else alg[1]
            if (hash2(x, y + f * 3, 51) < 0.03):
                c = alg[4]
            sp.set(x, y, c)
    sp.vline(6, 10, 23, hexc("c8f4e0"))
    sp.vline(25, 10, 23, hexc("2a5a48"))
    for k in range(4):
        bx = 9 + k * 4 + (k % 2)
        by = 22 - ((f * 3 + k * 4) % 12)
        sp.set(bx, by, hexc("e8ffd8"))
        sp.set(bx + 1, by + 2, alg[4])
    sp.hline(7, 24, 16, ST[2])
    sp.hline(7, 12, 16, ST[4])
    # fan housing
    box(sp, 5, 3, 26, 9, (ST[0], ST[2], ST[4]))
    sp.rect(7, 4, 24, 8, hexc("101420"))
    # fan blades (side-on rotor: bars sweep across)
    for x in range(8, 24):
        ph = (x + f * 2) % 8
        hgt = 2 if abs(x - 15.5) < 6 else 1
        if ph < 3:
            sp.vline(x, 6 - hgt, 6 + hgt, BN[3] if ph == 1 else BN[2])
        elif ph == 3:
            sp.vline(x, 6 - hgt, 6 + hgt, ST[1])
    sp.rect(15, 5, 16, 7, ST[3])
    sp.set(15, 5, ST[4])
    # O2 puffs
    out = soft_outline(sp)
    for k in range(3):
        ph = (f + k) % 4
        out.blend(9 + k * 6 + (ph % 2), 1 if ph < 2 else 0, wa(hexc("c8fff0"), 60 + ph * 40))
    return out


def prop_splicer(f):
    sp = Sprite(32, 32)
    # bench
    box(sp, 3, 22, 28, 30, (ST[0], ST[1], ST[2]))
    sp.hline(2, 29, 21, ST[4])
    sp.hline(2, 29, 22, ST[1])
    sp.rect(3, 29, 28, 30, ST[0])
    for i, c in enumerate((LI[3], MG[3], CY[3], AM[3])):
        sp.set(11 + i * 3, 25, c if (i + f) % 4 else mix(c, INK, 0.6))
    sp.hline(11, 20, 27, BN[1])
    # twin seed sockets: glass bells
    for sx, seedc in ((7, LI), (24, MG)):
        for (x, y) in m_ellipse(sx + 0.5, 20.5, 4, 6.5):
            if y > 20:
                continue
            e = ((x + 0.5 - sx - 0.5) / 4.0) ** 2 + ((y + 0.5 - 20.5) / 6.5) ** 2
            sp.set(x, y, hexc("c8f4f0") if e > 0.6 and x <= sx else hexc("5aa8a4") if e > 0.6 else hexc("1c3c44"))
        sp.set(sx - 1, 16, hexc("e8ffff"))
        # seed
        sp.rect(sx, 18, sx + 1, 19, seedc[2])
        sp.set(sx, 18, seedc[4])
        sp.set(sx + 1, 20, seedc[1])
        if f % 2 == (0 if sx == 7 else 1):
            sp.set(sx, 17, seedc[4])
        sp.hline(sx - 3, sx + 4, 21, ST[3])
    # helix column
    sp.rect(14, 3, 17, 4, ST[3])
    sp.hline(13, 18, 2, ST[4])
    sp.rect(13, 20, 18, 21, ST[3])
    for y in range(5, 20):
        ph = (y + f * 1.5) * 0.8
        xa = 15.5 + math.sin(ph) * 2.6
        xb = 15.5 - math.sin(ph) * 2.6
        front_a = math.cos(ph) > 0
        if int(y + f) % 2 == 0:
            for x in range(int(min(xa, xb)) + 1, int(max(xa, xb))):
                sp.set(x, y, hexc("3a5a6a"))
        sp.set(xb if front_a else xa, y, MG[2] if front_a else CY[2])
        sp.set(xa if front_a else xb, y, CY[4] if front_a else MG[4])
    # feed tubes from sockets to column
    sp.hline(11, 13, 20, CU[3])
    sp.hline(18, 20, 20, CU[3])
    sp.outline()
    glow_under(sp, 15.5, 12, 7, CY[3], 36)
    return sp


def prop_drop_pod(f):
    sp = Sprite(32, 32)
    shell = set()
    for y in range(3, 28):
        t = (y - 3) / 24.0
        hw = 9.5 * (math.sin(min(1.0, t * 1.5) * math.pi / 2) ** 0.7) if t < 0.66 else 9.5 - (t - 0.66) * 6
        for x in range(32):
            if abs(x + 0.5 - 16) <= hw:
                shell.add((x, y))
    for (x, y) in shell:
        u = (x - 6) / 20.0
        c = BN[4] if u < 0.2 else BN[3] if u < 0.55 else BN[2] if u < 0.82 else BN[1]
        if y < 7 and u < 0.5:
            c = BN[4]
        sp.set(x, y, c)
    # scorch marks, stripes, heat shield
    for (x, y) in shell:
        if y >= 24:
            sp.set(x, y, hexc("3a2a2a") if x < 20 else hexc("241a1e"))
        elif y in (9, 10) and f < 2:
            sp.set(x, y, RU[3] if x < 19 else RU[2])
        elif hash2(x, y, 61) < 0.07 and y > 12:
            sp.set(x, y, mix(sp.get(x, y), RU[1], 0.5))
    sp.rect(15, 1, 16, 2, ST[2])
    sp.set(15, 0 + 1, RD[3] if f % 2 == 0 else RD[1])
    # legs
    for sgn in (-1, 1):
        sp.line(16 + sgn * 7, 24, 16 + sgn * 12, 30, ST[2])
        sp.line(16 + sgn * 8, 24, 16 + sgn * 13, 30, ST[0] if sgn > 0 else ST[3])
        sp.hline(16 + sgn * 12 - 1, 16 + sgn * 12 + 2, 30, ST[1])
    sp.rect(12, 28, 19, 29, hexc("241a1e"))
    # door
    door = m_rect(11, 11, 20, 23)
    if f == 0:
        for (x, y) in door:
            edge = x in (11, 20) or y in (11, 23)
            if edge:
                sp.set(x, y, BN[1])
        sp.rect(13, 13, 18, 16, CY[1])
        sp.hline(13, 17, 13, CY[3])
        sp.set(13, 13, CY[4])
        sp.rect(18, 19, 19, 20, ST[2])
    elif f == 1:
        for (x, y) in door:
            sp.set(x, y, BN[1] if x in (11, 20) or y in (11, 23) else hexc("3a2c1c"))
        sp.rect(12, 12, 14, 22, hexc("ffc878"))
        sp.rect(12, 12, 12, 22, hexc("fff0c0"))
        sp.rect(15, 12, 19, 22, BN[2])
        sp.vline(15, 12, 22, BN[4])
        sp.rect(16, 14, 18, 16, CY[1])
    else:
        for (x, y) in door:
            t = (y - 11) / 12.0
            c = hexc("fff0c0") if t < 0.2 else hexc("ffc878") if t < 0.6 else hexc("d88a40")
            if x in (11, 20) or y == 11:
                c = BN[1]
            sp.set(x, y, c)
        sp.hline(13, 18, 15, hexc("a06030"))
        sp.rect(13, 19, 15, 22, hexc("8a5028"))
        sp.rect(17, 20, 18, 22, hexc("6a3c20"))
        if f == 3:
            sp.set(14, 17, hexc("fff0c0"))
        # door dropped as a ramp
        sp.rect(10, 24, 21, 25, BN[2])
        sp.hline(10, 21, 24, BN[4])
        sp.hline(9, 22, 26, BN[1])
    sp.outline()
    if f >= 2:
        glow_under(sp, 16, 26, 9, hexc("ffc878"), 60)
    return sp


def prop_lost_pack(f):
    sp = Sprite(32, 32)
    # backpack slumped on the ground
    pk = [hexc("4a2a1c"), hexc("7a4426"), hexc("a8643a"), hexc("d08c54")]
    body = m_ellipse(15, 26, 6.5, 5.5)
    for (x, y) in body:
        if y > 30:
            continue
        v = 0.5 - (x - 15) / 14.0 - (y - 26) / 12.0
        sp.set(x, y, pk[3] if v > 0.78 else pk[2] if v > 0.42 else pk[1])
    sp.rect(9, 30, 21, 30, pk[0])
    # flap, straps, bedroll
    sp.hline(10, 19, 23, pk[1])
    sp.rect(12, 24, 13, 27, BN[2])
    sp.rect(17, 24, 18, 27, BN[1])
    sp.set(12, 27, GD[3])
    sp.set(17, 27, GD[2])
    sp.rect(9, 19, 20, 21, TE[2])
    sp.hline(9, 20, 19, TE[3])
    sp.set(9, 20, TE[4])
    sp.set(20, 20, TE[1])
    sp.vline(12, 19, 21, BN[1])
    sp.vline(17, 19, 21, BN[1])
    sp.rect(21, 26, 23, 29, pk[1])
    sp.hline(21, 23, 26, pk[2])
    # beacon antenna
    sp.vline(22, 14, 25, ST[3])
    sp.set(23, 22, ST[1])
    on = f in (0, 1)
    sp.rect(21, 12, 23, 13, RD[3] if on else RD[1])
    if on:
        sp.set(21, 12, RD[4])
    # a sprout already growing on it
    sp.set(10, 22, MO[3])
    sp.set(9, 23, MO[2])
    sp.outline()
    if on:
        glow_under(sp, 22.5, 12.5, 5 if f == 0 else 7, RD[3], 120 if f == 0 else 70)
    return sp


def prop_flag(f):
    sp = Sprite(32, 32)
    sp.rect(15, 5, 16, 30, ST[2])
    sp.vline(15, 5, 30, ST[4])
    sp.hline(13, 18, 30, ST[1])
    sp.set(15, 4, GD[3])
    sp.set(16, 4, GD[2])
    # waving pennant
    for i in range(12):
        x = 17 + i
        wob = math.sin(i * 0.7 - f * math.pi / 2) * (0.4 + i * 0.13)
        y0 = 6 + wob + i * 0.12
        h = 8 - i * 0.45
        for y in range(int(round(y0)), int(round(y0 + h))):
            shade = math.cos(i * 0.7 - f * math.pi / 2)
            c = RD[4] if shade > 0.55 else RD[3] if shade > -0.3 else RD[2]
            if y == int(round(y0 + h)) - 1:
                c = RD[2] if shade > -0.3 else RD[1]
            sp.set(x, y, c)
        if 2 <= i <= 6:
            sp.set(x, int(round(y0 + h / 2)), BN[4] if i in (3, 4, 5) else BN[3])
    sp.outline()
    return sp


def prop_crate(f):
    sp = Sprite(32, 32)
    x0, x1 = 6, 25
    box(sp, x0, 17, x1, 30, (ST[0], ST[1], ST[2]))
    sp.rect(x0 + 2, 19, x1 - 2, 28, WD[2])
    for y in (22, 25):
        sp.hline(x0 + 2, x1 - 2, y, WD[1])
    sp.hline(x0 + 2, x1 - 2, 19, WD[3])
    sp.line(x0 + 2, 28, x1 - 2, 19, ST[1])
    sp.line(x0 + 3, 28, x1 - 1, 19, ST[2])
    sp.hline(x0, x1, 17, ST[4])
    sp.vline(x0, 17, 29, ST[3])
    rivets(sp, ((x0 + 1, 18), (x1 - 1, 18), (x0 + 1, 29), (x1 - 1, 29)), ST[4])
    # arrow stencil
    sp.set(9, 24, AM[3])
    sp.hline(8, 10, 25, AM[3])
    # open top: dark inside + contents
    sp.hline(x0 + 1, x1 - 1, 16, DARKIN)
    cols = [MO[3], RU[3], AM[3], TE[3], BN[3], LI[3], MG[2]]
    if f >= 1:
        n = 5 if f == 1 else 9
        for k in range(n):
            x = x0 + 2 + (k * 7) % 16
            h = 1 if f == 1 else 2 + (k % 3)
            c = cols[k % len(cols)]
            sp.rect(x, 16 - h + 1, x + 1, 16, c)
            sp.set(x, 16 - h + 1, mix(c, WHITE, 0.45))
            sp.set(x + 1, 16, mix(c, INK, 0.4))
        if f >= 2:
            sp.set(12, 12, LI[4])
            sp.set(19, 12, AM[4])
    sp.outline()
    return sp


def prop_dome_kit(f):
    sp = Sprite(32, 32)
    # pallet crate
    box(sp, 4, 20, 27, 30, (ST[0], ST[1], ST[2]))
    sp.hline(4, 27, 20, ST[4])
    sp.rect(4, 29, 27, 30, ST[0])
    for x in (9, 22):
        sp.vline(x, 20, 30, AM[2])
        sp.vline(x + 1, 20, 30, AM[1])
    sp.rect(13, 23, 18, 27, IN[1])
    for (dx, dy) in ((2, 0), (3, 0), (1, 1), (4, 1), (0, 2), (5, 2), (0, 3), (5, 3)):
        sp.set(13 + dx, 23 + dy, CY[3])
    sp.hline(13, 18, 27, CY[2])
    # folded dome segments stacked on top
    for (x, y) in m_ellipse(16, 20, 11, 12):
        if y >= 20:
            continue
        e = ((x + 0.5 - 16) / 11.0) ** 2 + ((y + 0.5 - 20) / 12.0) ** 2
        seg = (x - 5) // 4
        c = TE[3] if (x - 5) % 4 else ST[2]
        if e > 0.8:
            c = ST[4] if x < 16 else ST[2]
        elif (x - 5) % 4:
            c = TE[4] if x < 12 else TE[3] if x < 19 else TE[2]
            # travelling shimmer
            s = (x + (20 - y)) - (6 + f * 6)
            if 0 <= s < 2:
                c = hexc("e8fffa")
            elif s == 2:
                c = mix(c, WHITE, 0.4)
        sp.set(x, y, c)
    for y in (12, 16):
        for x in range(5, 27):
            if sp.get(x, y)[3]:
                sp.set(x, y, ST[3] if x < 16 else ST[1])
    # straps
    for x in (9, 22):
        for y in range(8, 20):
            if sp.get(x, y)[3]:
                sp.set(x, y, AM[3])
                sp.set(x + 1, y, AM[1])
    sp.outline()
    return sp


PROPS = [prop_bed, prop_bed_occupied, prop_chest, prop_suit_rack, prop_synth, prop_comm, prop_mod_bay,
         prop_planter, prop_dark_planter, prop_aqua_planter, prop_cooker, prop_compost, prop_assembly,
         prop_fish_pool, prop_stall, prop_trough, prop_condenser, prop_berth, prop_sprinkler, prop_tap,
         prop_lamp, prop_pylon, prop_pump, prop_tank, prop_oxygen, prop_splicer, prop_drop_pod,
         prop_lost_pack, prop_flag, prop_crate, prop_bunk, prop_dome_kit]


def gen_props():
    sh = Sprite(128, 32 * len(PROPS))
    for r, fn in enumerate(PROPS):
        for f in range(4):
            sh.blit(fn(f), f * 32, r * 32)
    return sh


# --------------------------------------------------------------------------
# FX (fx.png): 16x16 cells, 8 columns x 8 rows
# --------------------------------------------------------------------------
def fx_bolt(f, big=False):
    sp = Sprite(16, 16)
    rng = random.Random(100 + f + (50 if big else 0))
    core = hexc("f4ffff")
    if not big:
        # glow
        for (x, y) in m_ellipse(8, 8, 6.5, 3.2):
            d = math.hypot((x + 0.5 - 8) / 6.5, (y + 0.5 - 8) / 3.2)
            sp.set(x, y, wa(CY[2], int(150 * (1 - d) ** 1.2) + 20))
        sp.hline(5 + (f % 2), 10, 7, CY[3])
        sp.hline(5, 10 - (f % 2), 9, CY[3])
        sp.hline(4, 11, 8, core)
        sp.set(12, 8, CY[4])
        sp.hline(8, 10, 7, CY[4])
        sp.set(3, 8, CY[3])
        # tail flecks
        sp.set(2 - (f % 2), 8, wa(CY[3], 170))
        sp.set(1 + (f % 3), 7 + (f % 2) * 2, wa(CY[2], 150))
        sp.set(3, 7 + ((f + 1) % 2) * 2, wa(CY[3], 200))
    else:
        pul = (0, 1, 0, -1)[f] * 0.4
        for (x, y) in m_ellipse(8, 8, 7.8, 5.2 + pul):
            d = math.hypot((x + 0.5 - 8) / 7.8, (y + 0.5 - 8) / (5.2 + pul))
            sp.set(x, y, wa(CY[2], int(170 * (1 - d) ** 1.1) + 20))
        for (x, y) in m_ellipse(8.5, 8, 5.6, 3 + pul):
            sp.set(x, y, CY[3])
        for (x, y) in m_ellipse(9, 8, 4.4, 2):
            sp.set(x, y, CY[4])
        sp.hline(5, 12, 8, core)
        sp.hline(7, 12, 7, core)
        sp.set(14, 8, CY[4])
        # crackle
        for k in range(3):
            a = rng.uniform(0, math.tau)
            sp.set(8 + math.cos(a) * 6.5, 8 + math.sin(a) * 4.2, wa(core, 230))
        sp.set(1, 8 + (f % 3) - 1, wa(CY[3], 180))
        sp.set(2, 8, CY[3])
    return sp


def fx_impact(f):
    sp = Sprite(16, 16)
    rng = random.Random(200)
    core = hexc("f4ffff")
    r = (1.5, 3.2, 4.8, 6.0, 6.8, 7.4)[f]
    fade = (255, 255, 230, 180, 120, 60)[f]
    if f < 3:
        for (x, y) in m_ellipse(8, 8, r + 1.5, r + 1.5):
            d = math.hypot(x + 0.5 - 8, y + 0.5 - 8) / (r + 1.5)
            sp.set(x, y, wa(CY[2], int(160 * (1 - d))))
        for (x, y) in m_ellipse(8, 8, r, r):
            sp.set(x, y, CY[3] if f else core)
        for (x, y) in m_ellipse(8, 8, r * 0.6, r * 0.6):
            sp.set(x, y, core)
    else:
        ring = m_ellipse(8, 8, r, r) - m_ellipse(8, 8, r - 1.3, r - 1.3)
        for (x, y) in ring:
            if hash2(x, y, f) < (0.9, 0.65, 0.4)[f - 3]:
                sp.set(x, y, wa(CY[3] if f == 3 else CY[2], fade))
    # rays / flying sparks
    for k in range(8):
        a = k * math.tau / 8 + rng.uniform(-0.25, 0.25)
        d0 = r * (0.9 if f < 3 else 0.75) + rng.uniform(0, 1.2)
        for j in range(2 if f < 4 else 1):
            x, y = 8 + math.cos(a) * (d0 + j), 8 + math.sin(a) * (d0 + j)
            if 0.5 <= x < 15.5 and 0.5 <= y < 15.5:
                sp.set(x, y, wa(core if (j == 0 and f < 4) else CY[3], fade))
    return sp


def fx_muzzle(f):
    """Flash bursting to the right from (3, 8)."""
    sp = Sprite(16, 16)
    core = hexc("f4ffff")
    ln = (6, 10, 8, 4)[f]
    wd = (2.2, 3.4, 2.4, 1.2)[f]
    al = (255, 255, 220, 150)[f]
    for x in range(2, 3 + ln):
        t = (x - 2) / float(ln)
        hw = wd * math.sin(min(1.0, t * 2.2) * math.pi / 2) * (1 - t) ** 0.6
        for y in range(16):
            dy = abs(y + 0.5 - 8)
            if dy <= hw:
                c = core if dy < hw * 0.45 else CY[4] if dy < hw * 0.75 else CY[3]
                sp.set(x, y, wa(c, al))
            elif dy <= hw + 1.2:
                sp.set(x, y, wa(CY[2], al // 3))
    if f < 3:
        # side spikes
        for sgn in (-1, 1):
            for j in range((2, 4, 3)[f]):
                sp.set(4 + j, 8 + sgn * (2 + j), wa(CY[4] if j < 2 else CY[3], al))
        sp.set(2, 8, core)
    return sp


def fx_rocket(f):
    """Thruster flame pointing down, attached at the top-centre (8, 0)."""
    sp = Sprite(16, 16)
    ln = (11, 13, 10, 14)[f]
    cols = [hexc("f4ffff"), CY[4], CY[3], CY[2], hexc("2a6ad0")]
    for y in range(0, ln):
        t = y / float(ln)
        hw = 3.2 * (1 - t) ** 0.8 * (1.0 + 0.18 * math.sin(y * 1.7 + f * 2.1)) + 0.3
        sway = math.sin(y * 0.6 + f * 1.6) * t * 1.2
        for x in range(16):
            dx = abs(x + 0.5 - 8 - sway)
            if dx <= hw:
                u = dx / max(0.5, hw)
                i = min(4, int(u * 2.4 + t * 3.2))
                sp.set(x, y, cols[i])
            elif dx <= hw + 1.1 and y < ln - 1:
                sp.set(x, y, wa(hexc("2a6ad0"), 110))
    for k in range(3):
        y = ln + ((f * 3 + k * 2) % 3)
        if y < 16:
            sp.set(7 + (k + f) % 3, y, wa(CY[3], 180 - k * 40))
    return sp


def fx_dust(f):
    sp = Sprite(16, 16)
    rng = random.Random(300)
    cols = [hexc("6e675a"), hexc("9a917c"), hexc("c4b99a"), hexc("e0d6ba")]
    al = (255, 255, 235, 190, 130, 70)[f]
    spread = (1.5, 3.0, 4.2, 5.2, 6.0, 6.6)[f]
    rr = (2.2, 2.8, 2.8, 2.4, 1.9, 1.3)[f]
    blobs = []
    for k in range(6):
        a = math.pi + k * math.pi / 5 + rng.uniform(-0.2, 0.2)
        blobs.append((8 + math.cos(a) * spread * rng.uniform(0.6, 1.0),
                      12 + math.sin(a) * spread * 0.75 * rng.uniform(0.5, 1.0) - f * 0.5, rr * rng.uniform(0.7, 1.1)))
    for (bx, by, r) in blobs:
        for (x, y) in m_ellipse(bx, by, r + 0.9, r + 0.9):
            if not sp.opaque(x, y) and 0 <= y < 16:
                sp.set(x, y, wa(cols[0], al))
    for (bx, by, r) in blobs:
        for (x, y) in m_ellipse(bx, by, r, r):
            v = 0.5 - (x + 0.5 - bx) / (2.6 * r) - (y + 0.5 - by) / (2.6 * r)
            c = cols[3] if v > 0.8 else cols[2] if v > 0.42 else cols[1]
            if f >= 4 and hash2(x, y, f) < 0.35:
                continue
            sp.set(x, y, wa(c, al))
    return sp


def fx_splash(f):
    """Water splash; bottom-centre anchored (the surface is the y=15 row)."""
    sp = Sprite(16, 16)
    wh = hexc("e8fcff")
    t = f / 5.0
    al = (255, 255, 255, 235, 190, 120)[f]
    # crown
    h = (3, 6, 7, 5, 3, 1)[f]
    w = (2, 3, 4, 5, 6, 7)[f]
    for sgn in (-1, 1):
        for i in range(h):
            x = 8 + sgn * (1 + i * w / max(1.0, h * 1.3)) - (0 if sgn > 0 else 1)
            y = 15 - i
            sp.set(x, y, wa(WA[4] if i < h - 1 else wh, al))
            if i < h - 2:
                sp.set(x - sgn, y, wa(WA[3], al))
    if f < 4:
        sp.vline(7, 15 - h - (1 if f < 3 else 0), 15, wa(WA[4], al))
        sp.vline(8, 15 - h + 1, 15, wa(WA[3], al))
        sp.set(7, 15 - h - (1 if f < 3 else 0), wh)
    # droplets on ballistic arcs
    for k, (vx, vy) in enumerate(((-2.2, 5.5), (2.0, 6.0), (-1.0, 7.0), (1.2, 6.6), (-3.0, 4.0), (3.1, 4.4))):
        tt = t * 1.9
        x = 8 + vx * tt * 2.2
        y = 14 - (vy * tt - 3.6 * tt * tt) * 2.0
        if 0 <= x < 16 and 0 <= y < 15 and f > 0:
            sp.set(x, y, wa(wh if k % 2 == 0 else WA[4], al))
            if f < 4 and k < 4:
                sp.set(x, y + 1, wa(WA[3], al * 2 // 3))
    # ripple on the surface
    rw = 2 + f * 1
    sp.hline(8 - rw, 7 + rw, 15, wa(WA[4], al))
    sp.set(8 - rw, 15, wa(wh, al))
    sp.set(7 + rw, 15, wa(WA[3], al))
    return sp


def fx_sparkle(f):
    sp = Sprite(16, 16)
    ln = (2, 5, 3, 1)[f]
    c0, c1 = hexc("fffbe0"), AM[3]
    for d in range(ln + 1):
        c = c0 if d < max(1, ln - 1) else c1
        for (dx, dy) in ((d, 0), (-d, 0), (0, d), (0, -d)):
            sp.set(8 + dx, 8 + dy, c)
    if f == 1:
        for (dx, dy) in ((1, 1), (-1, 1), (1, -1), (-1, -1)):
            sp.set(8 + dx, 8 + dy, AM[4])
        for (dx, dy) in ((2, 2), (-2, 2), (2, -2), (-2, -2)):
            sp.set(8 + dx, 8 + dy, wa(AM[3], 160))
    if f >= 2:
        for k, (dx, dy) in enumerate(((4, -3), (-4, 3), (-3, -5), (5, 4))):
            if (k + f) % 2 == 0:
                sp.set(8 + dx, 8 + dy, wa(AM[4], 220 if f == 2 else 140))
    sp.set(8, 8, WHITE)
    return sp


def fx_heal(f):
    sp = Sprite(16, 16)

    def plus(x, y, big, al):
        c0, c1 = LI[4], LI[2]
        if big:
            sp.rect(x - 2, y, x + 2, y, wa(LI[3], al))
            sp.rect(x, y - 2, x, y + 2, wa(LI[3], al))
            sp.set(x, y, wa(WHITE, al))
            sp.set(x, y - 2, wa(c0, al))
            sp.set(x - 2, y, wa(c0, al))
            sp.set(x + 2, y, wa(c1, al))
            sp.set(x, y + 2, wa(c1, al))
        else:
            sp.hline(x - 1, x + 1, y, wa(LI[3], al))
            sp.vline(x, y - 1, y + 1, wa(LI[3], al))
            sp.set(x, y, wa(c0, al))
    plus(5, 11 - f * 3, True, (255, 255, 220, 140)[f])
    plus(11, 13 - f * 2, False, (255, 255, 255, 200)[f])
    if f >= 1:
        plus(12, 16 - f * 3 + 2, False, 230) if 16 - f * 3 + 2 < 15 else None
    if f >= 2:
        plus(3, 20 - f * 3, False, 255)
    return sp


def fx_leaves(f):
    sp = Sprite(16, 16)
    rng = random.Random(500)
    al = (255, 255, 255, 240, 190, 110)[f]
    t = (0.15, 0.4, 0.62, 0.8, 0.93, 1.0)[f]
    for k in range(9):
        a = k * math.tau / 9 + rng.uniform(-0.3, 0.3)
        sp_ = rng.uniform(4.5, 7.0)
        x = 8 + math.cos(a) * sp_ * t
        y = 8 + math.sin(a) * sp_ * t * 0.9 + t * t * 2.2 - 1
        rampk = MO if k % 3 else LI
        c0, c1 = (rampk[3], rampk[2]) if k % 3 else (MO[4], MO[3])
        rot = (k + f) % 4
        if f >= 5 and k % 2:
            continue
        pts = [((0, 0), c0), ((1, 0), c1)] if rot == 0 else [((0, 0), c0), ((0, 1), c1)] if rot == 1 else \
              [((0, 0), c0), ((1, 1), c1)] if rot == 2 else [((1, 0), c0), ((0, 1), c1)]
        if k % 4 == 0 and f < 4:
            pts.append(((1, 1) if rot != 2 else (1, 0), MO[1]))
        for (dx, dy), c in pts:
            if 0 <= x + dx < 16 and 0 <= y + dy < 16:
                sp.set(x + dx, y + dy, wa(c, al))
    if f == 0:
        for (x, y) in m_ellipse(8, 8, 2.2, 2.2):
            sp.set(x, y, MO[4])
        sp.set(8, 8, WHITE)
        sp.set(7, 7, WHITE)
    elif f == 1:
        for (dx, dy) in ((0, 0), (-1, 0), (0, -1), (-1, -1)):
            sp.set(8 + dx, 8 + dy, wa(LI[4], 200))
    return sp


def fx_spark(f):
    sp = Sprite(16, 16)
    rng = random.Random(600 + f * 7)
    core = hexc("f4ffff")
    for k in range(3 if f % 2 == 0 else 4):
        x, y = 8.0, 8.0
        a = rng.uniform(0, math.tau)
        pts = [(8, 8)]
        for j in range(rng.randint(3, 5)):
            a += rng.uniform(-1.1, 1.1)
            nx = min(14.0, max(1.0, x + math.cos(a) * 2.0))
            ny = min(14.0, max(1.0, y + math.sin(a) * 2.0))
            pts += line_pts(x, y, nx, ny)[1:]
            x, y = nx, ny
        for i, (px_, py_) in enumerate(pts):
            if 0 <= px_ < 16 and 0 <= py_ < 16:
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if 0 <= px_ + dx < 16 and 0 <= py_ + dy < 16 and not sp.opaque(px_ + dx, py_ + dy):
                        sp.set(px_ + dx, py_ + dy, wa(CY[2], 90))
        for i, (px_, py_) in enumerate(pts):
            if 0 <= px_ < 16 and 0 <= py_ < 16:
                sp.set(px_, py_, core if i < len(pts) * 0.6 else CY[3])
    sp.set(8, 8, WHITE)
    return sp


def fx_shield(f):
    """A flaring arc of a shield bubble, bulging right; centred."""
    sp = Sprite(16, 16)
    al = (255, 230, 170, 100)[f]
    cx_, r = -4.0 + f * 0.4, 14.0
    for y in range(1, 15):
        for x in range(16):
            d = math.hypot(x + 0.5 - cx_, (y + 0.5 - 8) * 1.05)
            edge = 1 - abs(y + 0.5 - 8) / 7.2
            if edge <= 0:
                continue
            if abs(d - r) < 0.75:
                hot = abs(y - 8 + (f - 1.5) * 2) < 2
                sp.set(x, y, wa(hexc("f4ffff") if hot else CY[3], int(al * min(1.0, edge * 1.6))))
            elif r - 3.6 < d < r:
                a = int(al * 0.42 * (1 - (r - d) / 3.6) * edge)
                if a > 8 and ((x + y + f) % 2 == 0 or (r - d) < 1.6):
                    sp.set(x, y, wa(CY[2], a))
    # hex glints travelling along the arc
    for k in range(3):
        yy = 3 + (k * 4 + f * 2) % 10
        xx = cx_ + math.sqrt(max(0, (r - 1.8) ** 2 - ((yy + 0.5 - 8) * 1.05) ** 2))
        if f < 3:
            sp.set(xx, yy, wa(CY[4], al))
    return sp


def gen_fx():
    sh = Sprite(128, 128)
    cells = {}
    for f in range(4):
        cells[(f, 0)] = fx_bolt(f)
        cells[(4 + f, 0)] = fx_bolt(f, big=True)
        cells[(f, 2)] = fx_muzzle(f)
        cells[(4 + f, 2)] = fx_rocket(f)
        cells[(f, 5)] = fx_sparkle(f)
        cells[(4 + f, 5)] = fx_heal(f)
        cells[(f, 7)] = fx_spark(f)
        cells[(4 + f, 7)] = fx_shield(f)
    for f in range(6):
        cells[(f, 1)] = fx_impact(f)
        cells[(f, 3)] = fx_dust(f)
        cells[(f, 4)] = fx_splash(f)
        cells[(f, 6)] = fx_leaves(f)
    for (c, r), sp in cells.items():
        sh.blit(sp, c * 16, r * 16)
    return sh


# --------------------------------------------------------------------------
# Foliage (foliage.png): 16x16 cells, 8 columns x 6 rows
# --------------------------------------------------------------------------
GRASS = [hexc("143a34"), hexc("1c5248"), hexc("2a7460"), hexc("46a078"), hexc("86cc96")]
FERN = [hexc("16361e"), hexc("245228"), hexc("3a7a34"), hexc("66a644"), hexc("a8d266")]
PALE = [hexc("4a4660"), hexc("7a7694"), hexc("b0aabe"), hexc("dcd6e0"), hexc("f6f2f4")]
CORAL = [hexc("5c5648"), hexc("8f8770"), hexc("c4b99a"), hexc("e8dfc2"), hexc("f8f3e0")]
THORN = [hexc("160e20"), hexc("24162e"), hexc("3a2240"), hexc("563254"), hexc("7a4a6a")]
EMBER = [hexc("5a1420"), hexc("a82a24"), hexc("f06a20"), hexc("ffb040"), hexc("ffe890")]


def blade(sp, x0, y0, tipx, tipy, ramp, bend=0.0, thick=False):
    """Curved blade from the base (x0, y0) to its tip; lighter toward the tip."""
    mx, my = (x0 + tipx) / 2.0 + bend, (y0 + tipy) / 2.0
    pts = curve_pts((x0, y0), (x0 + bend * 0.3, my), (tipx, tipy), 16)
    n = len(pts)
    for i, (x, y) in enumerate(pts):
        t = i / max(1.0, n - 1.0)
        c = ramp[1] if t < 0.3 else ramp[2] if t < 0.7 else ramp[3]
        sp.set(x, y, c)
        if thick and t < 0.6:
            sp.set(x + 1, y, ramp[1] if t < 0.45 else ramp[2])
    return pts


def fol_outline(sp, tint=0.34, side_only=False):
    """Dark tinted rim.  side_only: just a shade pixel on the right of each
    stroke (keeps thin blades from clumping into a blob)."""
    solid = Sprite(sp.w, sp.h)
    solid.px = [p if p[3] == 255 else CLEAR for p in sp.px]
    if side_only:
        add = []
        for y in range(sp.h):
            for x in range(1, sp.w):
                if solid.px[y * sp.w + x][3] == 0 and solid.px[y * sp.w + x - 1][3]:
                    add.append((x, y, mix(INK, solid.px[y * sp.w + x - 1], tint)))
        for x, y, c in add:
            solid.px[y * sp.w + x] = c
    else:
        solid.outline(tint=tint)
    for i, p in enumerate(sp.px):
        if 0 < p[3] < 255 and solid.px[i][3] == 0:
            solid.px[i] = p
    return solid


def fol_grass(k):
    """Tufts of separate 1 px blades (2 px pitch) so the silhouette stays crisp."""
    sp = Sprite(16, 16)
    rng = random.Random(1000 + k * 17)
    specs = [  # (blades, min h, max h, lean, extra)
        (5, 3, 7, 0, None), (4, 7, 13, 0, None), (6, 4, 9, 1, None), (5, 6, 12, 0, "seed"),
        (7, 2, 5, 0, None), (5, 5, 10, 0, "curl"), (4, 8, 14, -1, "dots"), (7, 4, 9, 0, "thick"),
    ]
    n, hmin, hmax, lean, extra = specs[k]
    ramp = GRASS if k % 3 else [mix(a, b, 0.5) for a, b in zip(GRASS, FERN)]
    x_start = 8 - n + (1 if n % 2 == 0 else 0)
    for i in range(n):
        bx = x_start + i * 2
        u = (i / max(1.0, n - 1.0)) * 2 - 1
        h = int(round(hmin + (hmax - hmin) * (1 - abs(u) ** 1.5 * 0.7) * rng.uniform(0.7, 1.0)))
        h = max(2, min(15, h))
        d = lean if lean else (-1 if u < -0.3 else 1 if u > 0.3 else rng.choice((-1, 0, 1)))
        tip = None
        for j in range(h):
            t = j / max(1.0, h - 1.0)
            x = bx + (d if t > 0.62 and h > 3 else 0) + (d if t > 0.9 and h > 8 else 0)
            x = max(1, min(14, x))
            c = ramp[1] if t < 0.3 else ramp[2] if t < 0.72 else ramp[3]
            sp.set(x, 15 - j, c)
            if extra == "thick" and i % 2 == 0 and t < 0.5:
                sp.set(x + 1, 15 - j, ramp[1])
            tip = (x, 15 - j)
        tx, ty = tip
        sp.set(tx, ty, ramp[4])
        if extra == "seed" and i % 2 == 0 and ty > 2:
            sp.set(tx, ty, AM[3])
            sp.set(tx, ty - 1, AM[4])
            sp.set(tx, ty + 1, AM[1])
        elif extra == "curl" and h > 5:
            dd = d if d else 1
            if 1 <= tx + dd <= 14:
                sp.set(tx + dd, ty, ramp[4])
                sp.set(tx + dd, ty + 1, ramp[3])
                sp.set(tx, ty, ramp[3])
        elif extra == "dots" and ty > 1:
            sp.set(tx, ty, CY[4])
            sp.set(tx, ty + 1, CY[2])
    # dark root line knits the tuft together
    for x in range(x_start, x_start + n * 2 - 1):
        if not sp.opaque(x, 15):
            sp.set(x, 15, ramp[0])
        if not sp.opaque(x, 14) and (x % 3 == 0):
            sp.set(x, 14, ramp[0])
    return sp


def frond(sp, x0, y0, tipx, tipy, ramp, bend, leaf=2):
    pts = curve_pts((x0, y0), ((x0 + tipx) / 2.0 + bend, min(y0, tipy) - 1), (tipx, tipy), 16)
    n = len(pts)
    for i, (x, y) in enumerate(pts):
        t = i / max(1.0, n - 1.0)
        sp.set(x, y, ramp[1] if t < 0.5 else ramp[2])
    for i, (x, y) in enumerate(pts):
        t = i / max(1.0, n - 1.0)
        if i >= 2 and i % 2 == 0:
            ln = max(1, int(round(leaf * math.sin(min(1.0, t * 1.4 + 0.15) * math.pi))))
            for j in range(1, ln + 1):
                sp.set(x, y - j, ramp[3] if j == ln else ramp[2])
                if y + j < 15:
                    sp.set(x, y + j, ramp[2] if j < ln else ramp[1])
    sp.set(pts[-1][0], pts[-1][1], ramp[4])


def fiddlehead(sp, x0, h, ramp, d=1, r=2):
    """Stalk ending in a curled spiral."""
    top = 15 - h
    for y in range(top + r, 16):
        x = x0 + int(round(math.sin((15 - y) * 0.35) * 0.8 * d))
        sp.set(x, y, ramp[1] if y > 11 else ramp[2])
        sp.set(x + 1, y, ramp[0] if y > 11 else ramp[1]) if y > 9 else None
    cx_, cy_ = x0 + d * r, top + r
    spiral = []
    for i in range(22):
        a = math.pi + (i / 21.0) * math.pi * 2.6
        rr = r * (1 - i / 30.0)
        spiral.append((int(round(cx_ + d * math.cos(a) * rr)), int(round(cy_ + math.sin(a) * rr))))
    for i, (x, y) in enumerate(spiral):
        sp.set(x, y, ramp[2] if i < 8 else ramp[3] if i < 17 else ramp[4])
    for y in range(top + r + 2, 14, 3):
        sp.set(x0 - d, y, ramp[3])


def fol_surface(k):
    sp = Sprite(16, 16)
    rng = random.Random(1100 + k * 13)
    if k == 0:
        frond(sp, 8, 15, 2, 6, FERN, -1, 2)
        frond(sp, 8, 15, 13, 5, FERN, 1, 2)
        frond(sp, 8, 15, 7, 2, FERN, 0.5, 1)
    elif k == 1:
        fiddlehead(sp, 6, 12, FERN, 1, 3)
        blade(sp, 7, 15, 12, 10, FERN, 1.0)
    elif k == 2:
        fiddlehead(sp, 5, 9, GRASS, 1, 2)
        fiddlehead(sp, 11, 13, GRASS, -1, 2)
    elif k == 3:
        # bulb plant: fat translucent bulb with a glowing heart
        for (x, y) in m_ellipse(8, 10.5, 4.2, 4.6):
            v = 0.5 - (x - 8) / 9.0 - (y - 10.5) / 10.0
            sp.set(x, y, TE[4] if v > 0.8 else TE[3] if v > 0.5 else TE[2] if v > 0.25 else TE[1])
        for (x, y) in m_ellipse(8.3, 11, 2, 2.4):
            sp.set(x, y, LI[3])
        sp.set(8, 10, LI[4])
        sp.set(6, 8, WHITE)
        sp.vline(8, 3, 5, FERN[2])
        sp.set(7, 2, FERN[3])
        sp.set(9, 3, FERN[3])
        sp.set(10, 2, FERN[4])
        sp.set(6, 3, FERN[2])
        sp.hline(6, 10, 15, FERN[1])
        sp.set(4, 14, FERN[2])
        sp.set(12, 14, FERN[2])
    elif k == 4:
        # broad-leaf rosette
        for (tx, ty, bd) in ((2, 9, -1), (13, 8, 1), (5, 4, -0.5), (11, 3, 0.5), (8, 6, 0)):
            pts = curve_pts((8, 15), ((8 + tx) / 2.0 + bd, 11), (tx, ty), 14)
            n = len(pts)
            for i, (x, y) in enumerate(pts):
                t = i / max(1.0, n - 1.0)
                w = int(round(1.6 * math.sin(min(1.0, t * 1.2) * math.pi)))
                for j in range(-w, w + 1):
                    c = FERN[4] if j == -w and w else FERN[3] if j < 0 else FERN[2] if j == 0 else FERN[1]
                    if j == 0 and w:
                        c = FERN[1]
                    sp.set(x + j, y, c)
            sp.set(pts[-1][0], pts[-1][1], FERN[4])
    elif k == 5:
        # small glowing flowers (cyan stars)
        for (x0, h, c) in ((4, 6, CY), (8, 10, CY), (12, 7, CY)):
            blade(sp, x0 + (1 if x0 < 8 else -1 if x0 > 8 else 0), 15, x0, 15 - h + 1, GRASS, 0.5)
            sp.set(x0, 15 - h, c[4])
            for (dx, dy) in ((1, 0), (-1, 0), (0, -1), (0, 1)):
                sp.set(x0 + dx, 15 - h + dy, c[3])
            sp.set(x0 + (2 if x0 < 8 else -2), 15 - h + 4, GRASS[3])
        sp.set(6, 14, GRASS[3])
        sp.set(10, 14, GRASS[2])
    elif k == 6:
        # magenta bell flowers on an arching stem
        pts = blade(sp, 6, 15, 11, 3, FERN, 2.5)
        for i in (len(pts) - 1, len(pts) - 5, len(pts) - 9):
            x, y = pts[max(0, i)]
            sp.set(x + 1, y + 1, MG[3])
            sp.set(x + 1, y + 2, MG[2])
            sp.set(x + 2, y + 2, MG[4])
            sp.set(x, y + 2, MG[2])
            sp.set(x + 1, y + 3, AM[4])
        blade(sp, 6, 15, 2, 9, FERN, -1)
        blade(sp, 7, 15, 4, 12, FERN, 0)
    else:
        # pod plant with amber seed pods
        sp.vline(8, 6, 15, FERN[1])
        sp.vline(7, 10, 15, FERN[2])
        for (px_, py_, r) in ((5, 7, 2.2), (11, 5, 2.4), (8, 3, 2.0)):
            for (x, y) in m_ellipse(px_, py_, r, r):
                v = 0.5 - (x - px_) / 5.0 - (y - py_) / 5.0
                sp.set(x, y, AM[4] if v > 0.75 else AM[3] if v > 0.4 else AM[2])
            sp.set(px_, py_ + int(r), RU[2])
        sp.line(8, 9, 5, 9, FERN[2])
        sp.line(8, 8, 11, 7, FERN[2])
        sp.set(4, 14, FERN[3])
        sp.set(5, 15, FERN[2])
        sp.set(11, 14, FERN[3])
        sp.set(10, 15, FERN[2])
    return fol_outline(sp)


def fol_hanging(k):
    sp = Sprite(16, 16)
    rng = random.Random(1200 + k * 11)
    if k in (0, 1, 4, 5):
        stems = [(8, 13)] if k in (0, 5) else [(5, 12), (10, 8)] if k == 1 else [(5, 9), (9, 13), (12, 6)]
        for (x0, ln) in stems:
            x = float(x0)
            for y in range(ln):
                x += math.sin(y * 0.7 + x0) * 0.45
                xi = int(round(x))
                sp.set(xi, y, FERN[1] if y < ln * 0.4 else FERN[2])
                if y % 2 == (x0 % 2) and y > 0 and k != 5:
                    side = 1 if (y // 2 + x0) % 2 else -1
                    sp.set(xi + side, y, FERN[3])
                    if y % 4 < 2:
                        sp.set(xi + side * 2, y + 1, FERN[2])
            xi = int(round(x))
            if k == 4:
                sp.set(xi, ln, LI[3])
                sp.set(xi, ln + 1, LI[4])
            elif k == 5:
                # curled tendril tip
                for (dx, dy) in ((1, 0), (2, 0), (3, -1), (3, -2), (2, -3), (1, -2)):
                    sp.set(xi + dx, ln + dy, FERN[3])
                sp.set(xi + 1, ln - 2, FERN[4])
                for y in (2, 5, 8):
                    sp.set(8 + int(round(math.sin(y * 0.7 + 8) * 0.9)) + (1 if y % 2 else -1), y, FERN[3])
            else:
                sp.set(xi, ln, FERN[4])
    elif k in (2, 7):
        ramp = [hexc("2a1a14"), hexc("4a3020"), hexc("6e4a30"), hexc("96704a"), hexc("c0a070")] if k == 2 else CORAL
        roots = [(4, 9), (7, 14), (10, 11), (12, 6)] if k == 2 else [(5, 12), (8, 8), (11, 14)]
        for (x0, ln) in roots:
            x = float(x0)
            for y in range(ln):
                x += rng.uniform(-0.55, 0.55)
                xi = int(round(max(1, min(14, x))))
                sp.set(xi, y, ramp[2] if y < ln * 0.5 else ramp[3] if k == 2 else ramp[2])
                if k == 2 and y < 3:
                    sp.set(xi + 1, y, ramp[1])
                if rng.random() < 0.22 and 2 < y < ln - 2:
                    d = rng.choice((-1, 1))
                    sp.set(xi + d, y + 1, ramp[2])
                    sp.set(xi + d * 2, y + 2, ramp[1] if k == 2 else ramp[2])
            sp.set(int(round(max(1, min(14, x)))), ln, ramp[4] if k == 2 else CY[3])
    else:
        # moss curtains: ragged strands of uneven length
        ramp = [mix(a, b, 0.4) for a, b in zip(FERN, GRASS)] if k == 3 else GRASS
        for x in range(2, 14):
            ln = int(3 + 9 * (0.5 + 0.5 * math.sin(x * 1.1 + k)) * (1 - abs(x - 7.5) / 9.0) + rng.randint(0, 2))
            if x in (2, 13):
                ln = min(ln, 3)
            for y in range(ln):
                t = y / max(1.0, ln - 1.0)
                c = ramp[1] if t < 0.3 else ramp[2] if t < 0.75 else ramp[3]
                if (x + y) % 3 == 0:
                    c = ramp[2] if c == ramp[1] else ramp[3] if c == ramp[2] else ramp[4]
                if x % 2 == 0 and y > ln * 0.6 and y % 2:
                    continue
                sp.set(x, y, c)
            if k == 6 and x % 4 == 1:
                sp.set(x, ln + 1, CY[3])
                sp.set(x, ln, WA[3])
    return fol_outline(sp, tint=0.42, side_only=True)


def mush(sp, x, base, h, cw, ch, cap, stem):
    for y in range(base - h + 1, base + 1):
        sp.set(x, y, stem[3] if y < base - 1 else stem[2])
    if cw >= 2:
        sp.set(x + 1, base, stem[1])
    top = base - h
    for (px_, py_) in m_ellipse(x + 0.5, top + 0.5, cw + 0.5, ch + 0.2):
        if py_ > top:
            continue
        v = 0.5 - (px_ - x) / (2.4 * cw + 1) - (py_ - top) / (2.0 * ch + 1)
        sp.set(px_, py_, cap[4] if v > 0.85 else cap[3] if v > 0.5 else cap[2])
    for px_ in range(x - cw + 1, x + cw):
        if px_ != x:
            sp.set(px_, top + 1, cap[1])


def fol_cave(k):
    sp = Sprite(16, 16)
    halos = []
    if k == 0:
        mush(sp, 5, 15, 5, 2, 2, PALE, PALE)
        mush(sp, 10, 15, 8, 3, 3, PALE, PALE)
        mush(sp, 13, 15, 3, 1, 1, PALE, PALE)
        mush(sp, 2, 15, 2, 1, 1, PALE, PALE)
    elif k == 1:
        for (x, h) in ((4, 9), (7, 13), (10, 7), (12, 11)):
            for y in range(15 - h + 1, 16):
                sp.set(x + (1 if (y // 4) % 2 else 0) * 0, y, PALE[2] if y > 12 else PALE[3])
            sp.set(x, 15 - h, PALE[4])
            sp.set(x - 1, 15 - h + 1, PALE[3])
            sp.set(x + 1, 15 - h + 1, PALE[2])
    elif k == 2:
        blade(sp, 8, 15, 7, 7, GRASS, -1.5)
        for (x, y) in m_ellipse(7, 4.5, 3.2, 3.2):
            v = 0.5 - (x - 7) / 7.0 - (y - 4.5) / 7.0
            sp.set(x, y, CY[4] if v > 0.7 else CY[3] if v > 0.35 else CY[2])
        sp.set(6, 3, WHITE)
        sp.set(10, 13, GRASS[3])
        sp.set(11, 12, GRASS[3])
        sp.set(6, 14, GRASS[2])
        halos.append((7, 4.5, 5.5, CY[3]))
    elif k == 3:
        for (x0, tx, ty) in ((8, 3, 8), (8, 8, 4), (8, 13, 9), (8, 11, 12)):
            blade(sp, x0, 15, tx, ty, GRASS, (tx - 8) * 0.3)
            for (dx, dy) in ((0, 0), (1, 0), (0, -1), (1, -1)):
                sp.set(tx + dx - (1 if tx > 8 else 0), ty + dy, LI[3])
            sp.set(tx - (1 if tx > 8 else 0), ty - 1, LI[4])
            halos.append((tx + 0.5, ty - 0.5, 3, LI[3]))
    elif k in (4, 5):
        gem = CY if k == 4 else MG
        mo = GRASS if k == 4 else [mix(a, b, 0.5) for a, b in zip(GRASS, THORN)]
        for (x, y) in m_ellipse(8, 15.5, 6.5, 3.6):
            if y > 15:
                continue
            v = hash2(x, y, k)
            sp.set(x, y, mo[3] if v > 0.72 else mo[2] if v > 0.3 else mo[1])
        for (x0, h, w) in ((5, 7, 1), (9, 10, 2), (12, 5, 1)):
            for y in range(13 - h, 14):
                t = (y - (13 - h)) / float(h)
                for dx in range(w + (1 if t > 0.25 else 0)):
                    sp.set(x0 + dx, y, gem[4] if dx == 0 else gem[2])
                if t < 0.2:
                    sp.set(x0, y, WHITE if y == 13 - h else gem[4])
            halos.append((x0 + 0.5, 12 - h / 2.0, 3.2, gem[3]))
    elif k == 6:
        # stack of luminous shelf fungi on a stump
        sp.rect(7, 4, 8, 15, THORN[2])
        sp.vline(7, 4, 15, THORN[3])
        for i, (y, w, d) in enumerate(((13, 4, 1), (10, 5, -1), (7, 4, 1), (4, 3, -1))):
            x0 = 9 if d > 0 else 6
            for j in range(w):
                sp.set(x0 + d * j, y, LI[3] if j < w - 1 else LI[4])
                if j < w - 1:
                    sp.set(x0 + d * j, y + 1, LI[1])
            sp.set(x0, y - 1, LI[2])
            halos.append((x0 + d * w * 0.6, y, 3, LI[3]))
    else:
        # pale tendril fern with glowing tips
        for (tx, ty, bd) in ((2, 8, -1), (5, 3, -0.5), (10, 2, 0.5), (13, 7, 1)):
            pts = blade(sp, 8, 15, tx, ty, PALE, bd)
            for i, (x, y) in enumerate(pts[2:-1:2]):
                sp.set(x + (1 if tx < 8 else -1), y - 1, PALE[2])
            sp.set(tx, ty, MG[3])
            sp.set(tx, ty - 1, MG[4])
            halos.append((tx + 0.5, ty, 2.6, MG[3]))
    out = fol_outline(sp, tint=0.3)
    for (x, y, r, c) in halos:
        glow_under(out, x, y, r, c, 80)
    return out


def fol_water(k):
    sp = Sprite(16, 16)
    reed = [hexc("1a3a2c"), hexc("2a5a3a"), hexc("4a8a48"), hexc("7ab45a"), hexc("b4d880")]
    if k in (0, 1, 6):
        xs = [(4, 9, -1), (6, 13, 0), (9, 11, 1), (11, 14, 0), (13, 7, 2)] if k != 6 else \
             [(5, 12, -4), (7, 13, 5), (8, 9, -2), (9, 11, 3)]
        for (x0, h, lean) in xs:
            pts = blade(sp, x0, 15, max(1, min(14, x0 + lean)), 15 - h, reed, lean * 0.6)
            tx, ty = pts[-1]
            if k == 1 and h > 9:
                # seed head
                for j in range(3):
                    if ty + j + 1 < 15:
                        sp.set(tx, ty + j, WD[3] if j else WD[4])
                        sp.set(tx + 1, ty + j + 1, WD[2])
            else:
                sp.set(tx, ty, reed[4])
    elif k in (2, 5):
        # lily pads seen from the side, on thin stems
        for (x0, y, w) in ((4, 11, 3), (11, 9, 3), (8, 13, 2)):
            sp.vline(x0, y + 1, 15, reed[1])
            sp.hline(x0 - w, x0 + w, y, reed[3])
            sp.hline(x0 - w + 1, x0 + w - 1, y + 1, reed[1])
            sp.set(x0 - w, y, reed[4])
            sp.set(x0 + w, y, reed[2])
        fx, fy = (11, 8) if k == 2 else (4, 10)
        if k == 2:
            for (dx, dy, c) in ((0, 0, MG[3]), (-1, 0, MG[2]), (1, 0, MG[2]), (0, -1, MG[4]), (-1, -1, MG[3]),
                                (1, -1, MG[3]), (0, -2, MG[4])):
                sp.set(fx + dx, fy + dy, c)
            sp.set(fx, fy - 1, AM[4])
        else:
            sp.vline(fx, fy - 4, fy - 1, reed[2])
            for (dx, dy, c) in ((0, -5, BN[4]), (0, -6, BN[4]), (-1, -5, BN[3]), (1, -5, BN[2]), (0, -7, MG[3])):
                sp.set(fx + dx, fy + dy, c)
    elif k == 3:
        # alien cattails: stalks with glowing bulb heads
        for (x0, h, c) in ((4, 9, AM), (8, 14, AM), (12, 11, AM)):
            for y in range(15 - h + 4, 16):
                sp.set(x0, y, reed[1] if y > 11 else reed[2])
            for j in range(4):
                y = 15 - h + j
                sp.set(x0, y, c[4] if j == 1 else c[3])
                if 0 < j < 3:
                    sp.set(x0 - 1, y, c[3])
                    sp.set(x0 + 1, y, c[2])
            sp.set(x0, 15 - h - 1, reed[3])
        blade(sp, 6, 15, 2, 10, reed, -1)
        blade(sp, 10, 15, 14, 12, reed, 1)
    elif k == 4:
        # segmented horsetails
        for (x0, h) in ((4, 10), (8, 14), (12, 8)):
            for y in range(15 - h, 16):
                seg = (15 - y) % 3 == 0
                sp.set(x0, y, reed[4] if seg else reed[2] if y < 11 else reed[1])
                if seg and y < 14 and y > 15 - h:
                    sp.set(x0 - 1, y - 1, reed[3])
                    sp.set(x0 + 1, y - 1, reed[2])
                    if (15 - y) % 6 == 0:
                        sp.set(x0 - 2, y - 2, reed[3])
                        sp.set(x0 + 2, y - 2, reed[2])
            sp.set(x0, 15 - h - 1, BN[3])
    else:
        # float bulbs on drooping stems
        for (x0, tx, ty) in ((7, 3, 6), (8, 12, 4), (8, 8, 9)):
            blade(sp, x0, 15, tx, ty, reed, (tx - 8) * 0.4)
            for (x, y) in m_ellipse(tx + 0.5, ty - 0.5, 2.1, 2.1):
                v = 0.5 - (x - tx) / 5.0 - (y - ty) / 5.0
                sp.set(x, y, TE[4] if v > 0.7 else TE[3] if v > 0.35 else TE[2])
            sp.set(tx, ty - 1, WHITE)
    return fol_outline(sp, tint=0.42, side_only=k in (0, 1, 4, 6))


def branch(sp, rng, x, y, ang, ln, depth, ramp, pts=None):
    """Recursive coral/thorn branching; returns tip positions."""
    tips = pts if pts is not None else []
    fx, fy = float(x), float(y)
    for i in range(ln):
        fx += math.cos(ang)
        fy += math.sin(ang)
        if not (1 <= fx <= 14 and 1 <= fy <= 15):
            break
        sp.set(int(round(fx)), int(round(fy)), ramp[2] if depth > 1 else ramp[3])
        if depth > 1:
            sp.set(int(round(fx)) + 1, int(round(fy)), ramp[1])
    if depth > 0 and ln > 1:
        spread = rng.uniform(0.5, 0.85)
        branch(sp, rng, fx, fy, ang - spread, max(2, int(ln * 0.72)), depth - 1, ramp, tips)
        branch(sp, rng, fx, fy, ang + spread, max(2, int(ln * 0.72)), depth - 1, ramp, tips)
    else:
        tips.append((int(round(fx)), int(round(fy))))
    return tips


def fol_abyss(k):
    sp = Sprite(16, 16)
    rng = random.Random(1500 + k * 7)
    halos = []
    if k in (0, 6):
        tips = branch(sp, rng, 8, 15, -math.pi / 2, 4, 3, CORAL)
        for (x, y) in tips:
            if 1 <= x <= 14 and 1 <= y <= 14:
                sp.set(x, y, CORAL[4] if k == 0 else EMBER[3])
                if k == 6:
                    halos.append((x + 0.5, y + 0.5, 2.4, EMBER[2]))
        sp.set(7, 15, CORAL[1])
        sp.set(9, 15, CORAL[1])
    elif k == 1:
        # coral fan: ribbed half-disc
        for (x, y) in m_ellipse(8, 14, 6.5, 12):
            if y > 13:
                continue
            a = math.atan2(14 - y, x + 0.5 - 8)
            rib = int(a * 9 / math.pi)
            e = ((x + 0.5 - 8) / 6.5) ** 2 + ((y + 0.5 - 14) / 12.0) ** 2
            if (a * 9 / math.pi) % 1 < 0.42 or (e > 0.5 and 0.62 < e < 0.72):
                sp.set(x, y, CORAL[4] if e > 0.85 else CORAL[3] if x < 8 else CORAL[2])
        sp.rect(7, 13, 8, 15, CORAL[2])
        sp.set(7, 13, CORAL[3])
    elif k in (2, 3):
        stalks = [(8, 11, 0)] if k == 2 else [(5, 7, -1), (8, 12, 0), (11, 9, 1)]
        for (x0, h, lean) in stalks:
            pts = blade(sp, x0, 15, x0 + lean * 2, 15 - h, THORN[1:] + [THORN[4]], lean)
            tx, ty = pts[-1]
            r = 2.4 if k == 2 else 1.6
            for (x, y) in m_ellipse(tx + 0.5, ty - r + 1, r, r + 0.4):
                v = 0.5 - (x - tx) / 6.0 - (y - (ty - r + 1)) / 6.0
                sp.set(x, y, EMBER[4] if v > 0.72 else EMBER[3] if v > 0.42 else EMBER[2] if v > 0.15 else EMBER[1])
            halos.append((tx + 0.5, ty - r + 1, r + 2.6, EMBER[2]))
            if k == 2:
                for (px_, py_) in ((x0 - 2, 10), (x0 + 3, 12)):
                    sp.set(px_, py_, EMBER[2])
                    sp.set(px_, py_ - 1, EMBER[3])
                    sp.set(px_ + (1 if px_ < x0 else -1), py_ + 1, THORN[3])
    elif k in (4, 5):
        # thorny tendrils
        stems = [(5, 2, 4), (10, 13, 6), (8, 8, 2)] if k == 4 else [(4, 13, 4), (12, 3, 2), (9, 7, 9)]
        for (x0, tx, ty) in stems:
            if k == 5:
                pts = curve_pts((x0, 15), (x0, 5), (tx, ty), 18)
            else:
                pts = curve_pts((x0, 15), ((x0 + tx) / 2.0 + (tx - x0) * 0.5, 9), (tx, ty), 16)
            for i, (x, y) in enumerate(pts):
                if not (1 <= x <= 14 and 1 <= y <= 15):
                    continue
                t = i / float(len(pts))
                sp.set(x, y, THORN[2] if t < 0.4 else THORN[3] if t < 0.8 else THORN[4])
                if i % 3 == 1 and 2 <= x <= 13 and y > 1:
                    d = 1 if (i // 3) % 2 else -1
                    sp.set(x + d, y - 1, CORAL[3] if t > 0.3 else THORN[4])
            lx, ly = pts[-1]
            if 1 <= lx <= 14 and 1 <= ly <= 14:
                sp.set(lx, ly, MG[3] if k == 4 else EMBER[3])
    else:
        # basalt-thorn bush with ember buds
        tips = branch(sp, rng, 8, 15, -math.pi / 2, 4, 2, THORN[1:] + [THORN[4]])
        tips += branch(sp, rng, 6, 15, -math.pi / 2 - 0.5, 3, 1, THORN[1:] + [THORN[4]])
        for i, (x, y) in enumerate(tips):
            if 1 <= x <= 14 and 2 <= y <= 14 and i % 2 == 0:
                sp.set(x, y, EMBER[3])
                sp.set(x, y - 1, EMBER[4])
                halos.append((x + 0.5, y, 2.4, EMBER[2]))
            elif 1 <= x <= 14 and 1 <= y <= 14:
                sp.set(x, y, CORAL[2])
    out = fol_outline(sp, tint=0.25)
    for (x, y, r, c) in halos:
        glow_under(out, x, y, r, c, 85)
    return out


def gen_foliage():
    sh = Sprite(128, 96)
    for r, fn in enumerate((fol_grass, fol_surface, fol_hanging, fol_cave, fol_water, fol_abyss)):
        for k in range(8):
            sh.blit(fn(k), k * 16, r * 16)
    return sh


# --------------------------------------------------------------------------
# Backgrounds
# --------------------------------------------------------------------------
class Layer:
    """Small id-mask canvas (0 = empty) used to build silhouettes."""

    def __init__(self, w, h, wrapx=True, wrapy=False):
        self.w, self.h, self.wrapx, self.wrapy = w, h, wrapx, wrapy
        self.d = bytearray(w * h)

    def get(self, x, y):
        if self.wrapx:
            x %= self.w
        elif not 0 <= x < self.w:
            return 0
        if self.wrapy:
            y %= self.h
        elif not 0 <= y < self.h:
            return 0
        return self.d[y * self.w + x]

    def set(self, x, y, v=1):
        x, y = int(math.floor(x)), int(math.floor(y))
        if self.wrapx:
            x %= self.w
        elif not 0 <= x < self.w:
            return
        if self.wrapy:
            y %= self.h
        elif not 0 <= y < self.h:
            return
        self.d[y * self.w + x] = v

    def rect(self, x0, y0, x1, y1, v=1):
        for y in range(int(y0), int(y1) + 1):
            for x in range(int(x0), int(x1) + 1):
                self.set(x, y, v)

    def ellipse(self, cx, cy, rx, ry, v=1):
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                if ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1.0:
                    self.set(x, y, v)

    def ring(self, cx, cy, r0, r1, v=1, a0=0.0, a1=math.tau, sy=1.0):
        for y in range(int(cy - r1 * sy) - 1, int(cy + r1 * sy) + 2):
            for x in range(int(cx - r1) - 1, int(cx + r1) + 2):
                dx, dy = x + 0.5 - cx, (y + 0.5 - cy) / sy
                d = math.hypot(dx, dy)
                if r0 <= d <= r1:
                    a = math.atan2(-dy, dx) % math.tau
                    if a0 <= a <= a1:
                        self.set(x, y, v)

    def poly(self, pts, v=1):
        for (x, y) in m_poly(pts):
            self.set(x, y, v)

    def thick(self, x0, y0, x1, y1, r, v=1):
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for i in range(n + 1):
            t = i / float(n)
            cx, cy = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            if r <= 0.6:
                self.set(cx, cy, v)
            else:
                for yy in range(int(cy - r), int(cy + r) + 1):
                    for xx in range(int(cx - r), int(cx + r) + 1):
                        if (xx + 0.5 - cx) ** 2 + (yy + 0.5 - cy) ** 2 <= r * r:
                            self.set(xx, yy, v)

    def curve(self, p0, p1, p2, r0, r1=None, v=1, n=40):
        r1 = r0 if r1 is None else r1
        last = None
        for i in range(n + 1):
            t = i / float(n)
            x = (1 - t) ** 2 * p0[0] + 2 * t * (1 - t) * p1[0] + t * t * p2[0]
            y = (1 - t) ** 2 * p0[1] + 2 * t * (1 - t) * p1[1] + t * t * p2[1]
            if last:
                self.thick(last[0], last[1], x, y, r0 + (r1 - r0) * t, v)
            last = (x, y)


def hang_vines(L, rng, x0, x1, y, n, lmin, lmax, v):
    """Vines dropping from an edge at row y."""
    for i in range(n):
        x = rng.uniform(x0, x1)
        ln = rng.randint(lmin, lmax)
        fx = x
        for j in range(ln):
            fx += rng.uniform(-0.4, 0.4)
            L.set(fx, y + j, v)
            if rng.random() < 0.3:
                L.set(fx + rng.choice((-1, 1)), y + j, v)
        if rng.random() < 0.5:
            L.ellipse(fx, y + ln, 1.6, 1.6, v)


def megastructures(L, rng, W, base, scale=1.0):
    """Colossal far structures: arcology, broken ring, spire, viaduct."""
    s = scale
    # ground ridge
    n1 = TNoise(random.Random(11), 8)
    for x in range(W):
        h = base - 6 * s - 14 * s * n1.at(x * 8.0 / W, 0.5)
        L.rect(x, h, x, L.h - 1, 1)
    # arcology: stepped mega tower with a crown
    ax = int(W * 0.17)
    for (hw, top) in ((38, base - 70 * s), (30, base - 112 * s), (22, base - 140 * s), (12, base - 158 * s)):
        L.rect(ax - hw * s, top, ax + hw * s, base, 1)
    L.rect(ax - 2, base - 178 * s, ax + 1, base - 158 * s, 1)
    L.rect(ax - 8 * s, base - 168 * s, ax + 8 * s, base - 166 * s, 1)
    L.poly([(ax - 44 * s, base - 70 * s), (ax - 38 * s, base - 84 * s), (ax - 38 * s, base - 70 * s)], 1)
    # habitat ring around the arcology
    L.ring(ax, base - 98 * s, 46 * s, 52 * s, 1, sy=0.22)
    # colossal broken ring
    rx = int(W * 0.58)
    L.ring(rx, base + 6 * s, 86 * s, 100 * s, 1, a0=0.02, a1=math.pi * 0.62)
    L.ring(rx, base + 6 * s, 86 * s, 100 * s, 1, a0=math.pi * 0.72, a1=math.pi * 0.98)
    for a in (0.2, 0.42, 0.85):
        ang = math.pi * a
        x0, y0 = rx + math.cos(ang) * 86 * s, base + 6 * s - math.sin(ang) * 86 * s
        L.thick(x0, y0, x0 - math.cos(ang) * 26 * s, y0 + math.sin(ang) * 26 * s, 1.2, 1)
    # dangling girders at the break
    ang = math.pi * 0.62
    bx, by = rx + math.cos(ang) * 93 * s, base + 6 * s - math.sin(ang) * 93 * s
    L.thick(bx, by, bx - 14 * s, by + 10 * s, 0.6, 1)
    L.thick(bx - 3, by + 2, bx - 6, by + 22 * s, 0.6, 1)
    # needle spire with cross-arms
    sx = int(W * 0.86)
    L.poly([(sx - 9 * s, base), (sx - 4 * s, base - 150 * s), (sx + 4 * s, base - 150 * s), (sx + 9 * s, base)], 1)
    L.rect(sx - 1, base - 176 * s, sx, base - 150 * s, 1)
    for hgt, hw in ((120, 16), (96, 22), (60, 14)):
        L.rect(sx - hw * s, base - hgt * s, sx + hw * s, base - hgt * s + 2, 1)
        L.rect(sx - hw * s, base - hgt * s, sx - hw * s + 1, base - hgt * s + 9 * s, 1)
        L.rect(sx + hw * s - 1, base - hgt * s, sx + hw * s, base - hgt * s + 6 * s, 1)
    # viaduct on tall piers crossing the valley
    vy = base - 46 * s
    for x in range(int(W * 0.28), int(W * 0.50)):
        L.rect(x, vy, x, vy + 3 * s, 1)
        if x % int(22 * s) < 3 * s:
            L.rect(x, vy, x, base, 1)
        else:
            arch = abs(((x % int(22 * s)) - 3 * s) / (19.0 * s) - 0.5) * 2
            L.rect(x, vy, x, vy + 3 * s + (arch ** 3) * 8 * s, 1)
    # vines: giant creepers over the big shapes (id 2)
    for (x0, x1, y) in ((ax - 38 * s, ax + 38 * s, base - 70 * s), (ax - 30 * s, ax + 30 * s, base - 112 * s),
                        (ax - 22 * s, ax + 22 * s, base - 140 * s), (int(W * 0.28), int(W * 0.5), vy + 3 * s),
                        (sx - 22 * s, sx + 22 * s, base - 94 * s)):
        hang_vines(L, rng, x0, x1, y, int((x1 - x0) / 5), int(6 * s), int(30 * s), 2)
    for a in range(8, 110, 3):
        ang = math.radians(a)
        if rng.random() < 0.8 and not (0.62 * 180 < a < 0.72 * 180):
            x0, y0 = rx + math.cos(ang) * 86 * s, base + 6 * s - math.sin(ang) * 86 * s
            hang_vines(L, rng, x0, x0 + 1, y0, 1, int(5 * s), int(34 * s), 2)


def ruined_towers(L, rng, W, base, count, hmin, hmax, wmin, wmax, vid=2, seed=0):
    """Nearer ruined towers: broken tops, bridges, antennas, creeper bands."""
    jag = TNoise(random.Random(21 + seed), 64)
    towers = []
    slot = W / float(count)
    for i in range(count):
        w = rng.randint(wmin, wmax)
        x0 = int(i * slot + rng.uniform(0.05, 0.6) * slot)
        h = rng.randint(hmin, hmax)
        towers.append((x0, w, h))
        lean = rng.choice((0, 0, 0, 1, -1))
        for x in range(x0, x0 + w):
            jx = jag.at(x * 0.37, i * 3.1)
            top = base - h + int(jx * 16) * (1 if (x - x0) > w * 0.25 or i % 2 else 0)
            if rng.random() < 0.04:
                top -= rng.randint(4, 14)   # exposed rebar
            L.rect(x + lean * 0, top, x, L.h - 1, 1)
        # set-back upper block / water tank / dish
        k = rng.random()
        if k < 0.4:
            L.rect(x0 + w * 0.2, base - h - 14, x0 + w * 0.6, base - h + 4, 1)
        elif k < 0.6:
            L.ellipse(x0 + w * 0.5, base - h - 2, w * 0.38, 7, 1)
        if rng.random() < 0.6:
            ax = x0 + rng.randint(2, w - 2)
            L.rect(ax, base - h - rng.randint(14, 34), ax, base - h, 1)
            L.rect(ax - 2, base - h - 12, ax + 2, base - h - 12, 1)
        # window holes: punch out a few gaps (ruin) high up
        for j in range(w // 5):
            hx = x0 + 2 + rng.randint(0, max(1, w - 6))
            hy = base - h + 12 + rng.randint(0, max(1, h // 3))
            if rng.random() < 0.5:
                L.rect(hx, hy, hx + rng.randint(1, 3), hy + rng.randint(2, 6), 0)
        # creeper bands spiralling up the tower
        ph = rng.uniform(0, 6)
        if rng.random() < 0.85:
            for y in range(int(base - h), int(base)):
                cxv = x0 + w / 2.0 + math.sin(y * 0.07 + ph) * w * 0.5
                for dx in range(-2, 3):
                    if L.get(int(cxv + dx), y) == 1 or abs(dx) < 2:
                        L.set(cxv + dx, y, vid)
                if rng.random() < 0.12:
                    hang_vines(L, rng, cxv - 2, cxv + 2, y, 1, 4, 16, vid)
        # canopy growing on the roof
        if rng.random() < 0.7:
            for j in range(rng.randint(2, 5)):
                L.ellipse(x0 + rng.uniform(0, w), base - h - rng.uniform(-2, 5), rng.uniform(4, 9), rng.uniform(3, 6), vid)
        hang_vines(L, rng, x0, x0 + w, base - h + 2, w // 4, 5, 26, vid)
    # pipe bridges between neighbours
    for i in range(count):
        a, b = towers[i], towers[(i + 1) % count]
        if rng.random() < 0.6:
            y = base - min(a[2], b[2]) + rng.randint(12, 30)
            xa = a[0] + a[1]
            xb = b[0] + (W if i == count - 1 else 0)
            if xb - xa < 90:
                sag = rng.randint(2, 8)
                L.curve((xa, y), ((xa + xb) / 2.0, y + sag * 2), (xb, y), 1.0, v=1)
                hang_vines(L, rng, xa, xb, y + sag // 2, int((xb - xa) / 4), 4, 22, vid)
    return towers


def colour_layers(sp, L, pal, base=None, haze=None, lights=None, rng=None, seams=7, y0=0):
    """Paint an id-mask into the sprite.
    pal: {id: (dark, mid, light)}; haze: (colour, y_start, y_end, strength)."""
    W, H = L.w, L.h
    for y in range(H):
        for x in range(W):
            v = L.d[y * W + x]
            if not v:
                continue
            dk, md, lt = pal[v]
            up = L.get(x, y - 1) == 0
            lf = L.get(x - 1, y) == 0
            rt = L.get(x + 1, y) == 0
            c = md
            if up or lf:
                c = lt
            elif rt:
                c = dk
            elif v == 1 and seams and (y % seams == 0) and hash2(x // 9, y // seams, 3) < 0.7:
                c = dk
            elif v != 1 and hash2(x // 2, y // 2, 5) < 0.16 and hash2(x, y, 7) < 0.7:
                c = dk if hash2(x // 2, y // 2, 6) < 0.6 else lt
            if haze:
                hc, hy0, hy1, hs = haze
                t = clamp((y - hy0) / float(hy1 - hy0)) * hs
                if t > 0 and t > bayer(x, y) * 0.9:
                    c = mix(c, hc, 0.55)
            sp.set(x, y + y0, c)
    if lights and rng:
        n, cols = lights
        for i in range(n):
            x, y = rng.randint(0, W - 1), rng.randint(0, H - 1)
            if L.get(x, y) == 1 and L.get(x - 1, y) == 1 and L.get(x + 1, y) == 1 and L.get(x, y - 1) == 1:
                sp.set(x, y + y0, rng.choice(cols))


def gen_surface_far():
    W, H = 512, 256
    rng = random.Random(31)
    sp = Sprite(W, H)
    sp.wrapx = True
    base = 196
    A = Layer(W, H)
    megastructures(A, rng, W, base, 1.04)
    hazec = hexc("8ea6ac")
    colour_layers(sp, A, {1: (hexc("5a7680"), hexc("64828a"), hexc("7696a0")),
                          2: (hexc("567a72"), hexc("62887c"), hexc("729a8a"))},
                  haze=(hazec, 120, 200, 0.75), lights=(60, [hexc("7fa0a8"), hexc("4f6a74")]), rng=rng, seams=9)
    B = Layer(W, H)
    n1 = TNoise(random.Random(12), 16)
    for x in range(W):
        L_h = base - 2 - 8 * n1.at(x * 16.0 / W, 0.5)
        B.rect(x, L_h, x, H - 1, 1)
    ruined_towers(B, rng, W, base, 7, 40, 104, 16, 38)
    colour_layers(sp, B, {1: (hexc("405c66"), hexc("4a6872"), hexc("5a7a84")),
                          2: (hexc("3e5e5c"), hexc("486c66"), hexc("587e74"))},
                  haze=(hexc("6c8a92"), 150, 215, 0.6), lights=(40, [hexc("c8b088"), hexc("36505a"), hexc("36505a")]),
                  rng=rng, seams=7)
    # solid base: fade to a darker mist tone, fully opaque for the bottom 60 rows
    for y in range(H - 60, H):
        for x in range(W):
            t = (y - (H - 60)) / 60.0
            p = sp.px[y * W + x]
            if p[3] == 0:
                p = hexc("4a6872")
            sp.px[y * W + x] = mix(p, hexc("3c5862"), t * 0.8 if t * 0.8 > bayer(x, y) * 0.6 else t * 0.4)
    sp.wrapx = False
    return sp


def fern_tree(L, rng, x, base, h, v_trunk, v_leaf, scale=1.0, fronds=9):
    lean = rng.uniform(-0.25, 0.25) * h
    top = (x + lean, base - h)
    L.curve((x, base + 4), (x, base - h * 0.5), top, 3.2 * scale, 1.8 * scale, v=v_trunk)
    # trunk scars
    for i in range(fronds):
        a = math.pi * (0.04 + 0.92 * (i + rng.uniform(-0.2, 0.2)) / (fronds - 1.0))
        ln = h * rng.uniform(0.42, 0.62) * (0.75 + 0.25 * math.sin(a))
        ex = top[0] + math.cos(a) * ln
        ey = top[1] - math.sin(a) * ln * 0.55 + ln * 0.34
        mx_ = top[0] + math.cos(a) * ln * 0.55
        my_ = top[1] - math.sin(a) * ln * 0.8 - ln * 0.12
        pts = []
        n = 26
        for j in range(n + 1):
            t = j / float(n)
            pts.append(((1 - t) ** 2 * top[0] + 2 * t * (1 - t) * mx_ + t * t * ex,
                        (1 - t) ** 2 * top[1] + 2 * t * (1 - t) * my_ + t * t * ey))
        for j in range(n):
            t = j / float(n)
            L.thick(pts[j][0], pts[j][1], pts[j + 1][0], pts[j + 1][1], 1.3 * scale * (1 - t * 0.6), v_leaf)
            # hanging leaflets
            if j % 2 == 0:
                ll = int((2 + 7 * math.sin(min(1.0, t * 1.15 + 0.08) * math.pi)) * scale)
                for q in range(ll):
                    L.set(pts[j][0], pts[j][1] + q, v_leaf)
                    if q < ll - 2:
                        L.set(pts[j][0] + 1, pts[j][1] + q, v_leaf)
    L.ellipse(top[0], top[1], 4 * scale, 3 * scale, v_leaf)


def bulb_tree(L, rng, x, base, h, v_trunk, v_leaf, scale=1.0):
    top = (x + rng.uniform(-6, 6), base - h)
    L.curve((x, base + 4), (x + rng.uniform(-8, 8), base - h * 0.5), top, 2.6 * scale, 1.6 * scale, v=v_trunk)
    for i in range(rng.randint(5, 8)):
        a = rng.uniform(0, math.tau)
        r = rng.uniform(0, h * 0.22)
        L.ellipse(top[0] + math.cos(a) * r * 1.5, top[1] + math.sin(a) * r * 0.7 - 2, rng.uniform(8, 15) * scale,
                  rng.uniform(6, 11) * scale, v_leaf)
    # side branches with smaller bulbs
    for sgn in (-1, 1):
        if rng.random() < 0.8:
            by = base - h * rng.uniform(0.35, 0.6)
            ex = x + sgn * h * rng.uniform(0.2, 0.34)
            L.curve((x, by), (x + sgn * 6, by - 4), (ex, by - h * 0.12), 1.4 * scale, 1.0, v=v_trunk)
            L.ellipse(ex, by - h * 0.12 - 4, rng.uniform(6, 10) * scale, rng.uniform(5, 8) * scale, v_leaf)
    return top


def gen_surface_mid():
    W, H = 512, 256
    rng = random.Random(47)
    sp = Sprite(W, H)
    sp.wrapx = True
    base = 208
    # back sublayer: bulbous canopy and distant fern-trees
    A = Layer(W, H)
    n1 = TNoise(random.Random(13), 16)
    for x in range(W):
        A.rect(x, base - 14 - 22 * n1.at(x * 16.0 / W, 0.5), x, H - 1, 2)
    for i in range(9):
        x = i * W / 9.0 + rng.uniform(0, 30)
        if i % 3 == 1:
            fern_tree(A, rng, x, base - 8, rng.randint(70, 100), 1, 2, 0.8, 8)
        else:
            bulb_tree(A, rng, x, base - 8, rng.randint(50, 96), 1, 2, 0.9)
    colour_layers(sp, A, {1: (hexc("234844"), hexc("2a544e"), hexc("34645a")),
                          2: (hexc("265248"), hexc("2e6052"), hexc("3c7460"))},
                  haze=(hexc("4a7870"), 150, 215, 0.5), seams=0)
    # front sublayer: giant fern-trees, hanging-garden spans, undergrowth
    B = Layer(W, H)
    n2 = TNoise(random.Random(14), 32)
    for x in range(W):
        B.rect(x, base - 2 - 12 * n2.at(x * 32.0 / W, 0.5), x, H - 1, 2)
    spots = [40, 150, 262, 372, 470]
    tops = []
    for i, x in enumerate(spots):
        h = rng.randint(120, 168) if i % 2 == 0 else rng.randint(86, 120)
        if i == 3:
            tops.append(bulb_tree(B, rng, x, base, h, 1, 2, 1.25))
        else:
            fern_tree(B, rng, x, base, h, 1, 2, 1.15, 10)
            tops.append((x, base - h))
    # hanging gardens: sagging root-bridges between trunks, dripping vines and planters
    for i in range(len(spots)):
        xa = spots[i]
        xb = spots[(i + 1) % len(spots)] + (W if i == len(spots) - 1 else 0)
        y = base - rng.randint(48, 82)
        sag = rng.randint(8, 18)
        B.curve((xa, y), ((xa + xb) / 2.0, y + sag * 2), (xb, y - rng.randint(-8, 8)), 1.4, v=1, n=60)
        for j in range(int((xb - xa) / 6)):
            t = (j + rng.random()) / ((xb - xa) / 6.0)
            vx = xa + (xb - xa) * t
            vy = y + sag * 4 * t * (1 - t)
            hang_vines(B, rng, vx, vx + 1, vy, 1, 6, 30, 2)
        # garden baskets
        for t in (0.3, 0.55, 0.8):
            if rng.random() < 0.7:
                vx = xa + (xb - xa) * t
                vy = y + sag * 4 * t * (1 - t) + rng.randint(8, 16)
                B.rect(vx, vy - 12, vx, vy, 1)
                B.ellipse(vx, vy + 2, 7, 4, 2)
                B.ellipse(vx - 3, vy - 1, 4, 3, 2)
                B.ellipse(vx + 4, vy, 4, 3, 2)
                hang_vines(B, rng, vx - 6, vx + 6, vy + 3, 4, 4, 14, 2)
    # undergrowth: ferns, bulbs
    for i in range(46):
        x = rng.uniform(0, W)
        gy = base - 2 - 12 * n2.at((x % W) * 32.0 / W, 0.5)
        k = rng.random()
        if k < 0.5:
            for j in range(rng.randint(3, 6)):
                a = math.pi * rng.uniform(0.15, 0.85)
                ln = rng.uniform(8, 20)
                B.curve((x, gy + 2), (x + math.cos(a) * ln * 0.6, gy - ln), (x + math.cos(a) * ln, gy - ln * 0.5), 0.8, v=2,
                        n=14)
        elif k < 0.8:
            B.ellipse(x, gy - 3, rng.uniform(5, 10), rng.uniform(4, 8), 2)
        else:
            B.rect(x, gy - rng.randint(8, 18), x + 1, gy, 1)
            B.ellipse(x + 1, gy - 20, 3.5, 4.5, 2)
    colour_layers(sp, B, {1: (hexc("132c2e"), hexc("193638"), hexc("224646")),
                          2: (hexc("15332e"), hexc("1c3f36"), hexc("275244"))}, seams=0)
    # luminous accents
    for i in range(160):
        x, y = rng.randint(0, W - 1), rng.randint(40, base)
        if B.get(x, y) == 2 and B.get(x, y + 1) == 0:
            c = rng.choice([R("lime", 3), R("cyan", 3), R("amber", 3), R("mag", 3)])
            sp.set(x, y + 1, c)
            sp.blend(x, y + 2, wa(c, 90))
            sp.blend(x - 1, y + 1, wa(c, 70))
            sp.blend(x + 1, y + 1, wa(c, 70))
    for y in range(H - 48, H):
        for x in range(W):
            t = (y - (H - 48)) / 48.0
            p = sp.px[y * W + x]
            if p[3] == 0:
                p = hexc("193638")
            sp.px[y * W + x] = mix(p, hexc("10262a"), t * 0.7 if t * 0.7 > bayer(x, y) * 0.5 else t * 0.35)
    sp.wrapx = False
    return sp


def gen_surface():
    return {"backgrounds/surface_far.png": gen_surface_far(), "backgrounds/surface_mid.png": gen_surface_mid()}


# ---- underground -----------------------------------------------------------
S = 256


def walkers(sp, rng, n, cols, lmin, lmax, down=0.75, branch=0.06, a=255):
    """Wandering root-like lines on a wrapped canvas."""
    for i in range(n):
        x, y = rng.uniform(0, S), rng.uniform(0, S)
        ang = math.pi / 2 + rng.uniform(-0.8, 0.8)
        todo = [(x, y, ang, rng.randint(lmin, lmax), 0)]
        while todo:
            x, y, ang, ln, depth = todo.pop()
            for j in range(ln):
                ang += rng.uniform(-0.35, 0.35)
                ang = ang * (1 - 0.05) + (math.pi / 2) * 0.05 * down
                x += math.cos(ang)
                y += math.sin(ang)
                c = cols[min(len(cols) - 1, depth)]
                sp.set(x, y, c if a == 255 else wa(c, a))
                if depth == 0 and j % 3 == 0:
                    sp.set(x + 1, y, cols[min(len(cols) - 1, 1)])
                if rng.random() < branch and depth < 2:
                    todo.append((x, y, ang + rng.choice((-1, 1)) * rng.uniform(0.5, 1.1), ln // 2, depth + 1))


def strata_far(seed, ramp, vertical=False, band=13.0, warp=34.0, crack=0.5):
    rng = random.Random(seed)
    sp = Sprite(S, S)
    sp.wrap = True
    f1 = FBM(rng, 3, 3)
    f2 = FBM(rng, 8, 3)
    wor = Worley(rng, 7)
    tone = [rng.random() for _ in range(64)]
    nb = int(round(S / band))
    band = S / float(nb)
    n = len(ramp)
    for y in range(S):
        for x in range(S):
            w = f1(x, y)
            u = (x if vertical else y) + warp * (w - 0.5) * 2
            bi = int(math.floor(u / band)) % nb
            fr = (u / band) % 1.0
            v = 0.25 + tone[bi] * 0.5 + (f2(x, y) - 0.5) * 0.5
            if fr < 0.12:
                v -= 0.3          # shadowed seam under each stratum
            elif fr > 0.86:
                v += 0.16         # lit ledge
            d1, d2 = wor(x, y)
            if d2 - d1 < 1.6 * crack:
                v -= 0.35
            sp.px[y * S + x] = ramp_pick(ramp, v, x, y)
    return sp, rng, wor, f1


def mean_lum(sp):
    return sum(lum(p) for p in sp.px) / len(sp.px)


def near_finish(sp, M, body, rim, shade, texture=None):
    for y in range(S):
        for x in range(S):
            v = M.d[y * S + x]
            if not v:
                continue
            c = body
            if M.get(x - 1, y) == 0 or M.get(x, y - 1) == 0:
                c = rim
            elif M.get(x + 1, y) == 0 or M.get(x, y + 1) == 0:
                c = shade
            elif texture and hash2(x // 2, y // 2, 9) < 0.14:
                c = texture
            sp.px[y * S + x] = c


def islands(rng, thresh, base=3, stretch=1.0):
    """Sparse blobs from tileable noise."""
    M = Layer(S, S, True, True)
    f = FBM(rng, base, 3)
    for y in range(S):
        for x in range(S):
            if f(x, y * stretch) > thresh:
                M.d[y * S + x] = 1
    return M


def gen_cave():
    ramp = [hexc("0d1418"), hexc("131d22"), hexc("1a272c"), hexc("223238"), hexc("2c4044")]
    far, rng, wor, f1 = strata_far(101, ramp, band=14.0)
    # faint roots and moss flecks
    walkers(far, rng, 16, [hexc("3a4a3c"), hexc("2e3e36"), hexc("28363a")], 30, 80)
    for i in range(240):
        x, y = rng.randint(0, S - 1), rng.randint(0, S - 1)
        if f1(x, y) > 0.55:
            far.set(x, y, hexc("2c4a3a"))
            if rng.random() < 0.4:
                far.set(x + 1, y, hexc("24403a"))
    far.wrap = False
    # near: rock shelves with stalactites and dangling roots
    near = Sprite(S, S)
    M = islands(rng, 0.575, base=3, stretch=2)
    cols = []
    for x in range(S):
        for y in range(S):
            if M.d[y * S + x] and not M.d[((y + 1) % S) * S + x]:
                cols.append((x, y))
    for (x, y) in cols:
        r = hash2(x // 3, y // 9, 4)
        if r < 0.3 and x % 3 == 1:
            ln = 4 + int(hash2(x, y, 5) * 26)
            for j in range(ln):
                w = 1 if j > ln * 0.45 else 2 if j > ln * 0.15 else 3
                for dx in range(-(w // 2), w - w // 2):
                    M.set(x + dx, y + j, 1)
    R2 = Layer(S, S, True, True)
    for (x, y) in cols:
        if hash2(x, y, 6) < 0.035:
            fx = float(x)
            for j in range(10 + int(hash2(x, y, 7) * 50)):
                fx += rng.uniform(-0.5, 0.5)
                R2.set(fx, y + j, 1)
                if rng.random() < 0.08:
                    R2.set(fx + rng.choice((-1, 1)), y + j, 1)
    near_finish(near, M, hexc("0a1014"), hexc("1c2a2e"), hexc("06090c"), hexc("0e161a"))
    for i, v in enumerate(R2.d):
        if v and not M.d[i]:
            near.px[i] = hexc("0d1512")
    return far, near


def gen_deep():
    ramp = [hexc("0d0e1e"), hexc("131428"), hexc("1a1c34"), hexc("222542"), hexc("2c3052")]
    far, rng, wor, f1 = strata_far(202, ramp, band=18.0, warp=46.0, crack=0.3)
    # crystal veins along cell borders, brighter nodes where the mask is high
    vein = [hexc("28366a"), hexc("36508c"), hexc("4a7ab8"), hexc("7ab8e0")]
    f3 = FBM(random.Random(203), 2, 2)
    w2 = Worley(random.Random(204), 5)
    for y in range(S):
        for x in range(S):
            d1, d2 = w2(x, y)
            m = f3(x, y)
            if d2 - d1 < 1.1 and m > 0.5:
                far.px[y * S + x] = vein[1] if m > 0.6 and (d2 - d1) < 0.6 else vein[0]
    far.wrap = True
    for i in range(46):
        x, y = rng.randint(0, S - 1), rng.randint(0, S - 1)
        d1, d2 = w2(x, y)
        if d2 - d1 < 4 and f3(x, y) > 0.48:
            h = rng.randint(3, 7)
            c = rng.choice([vein, [hexc("3a2a6a"), hexc("563c94"), hexc("7a5ac0"), hexc("b08ae8")]])
            for j in range(h):
                w = 1 if j in (0, h - 1) else 2
                far.set(x, y - j, c[2] if j < h - 1 else c[3])
                if w == 2:
                    far.set(x + 1, y - j, c[1])
    far.wrap = False
    # near: slanted crystal columns in clusters
    near = Sprite(S, S)
    M = Layer(S, S, True, True)
    G = Layer(S, S, True, True)
    for i in range(8):
        cx_, cy_ = rng.uniform(0, S), rng.uniform(0, S)
        up = rng.choice((-1, 1))
        ncol = rng.randint(4, 7)
        for j in range(ncol):
            u = (j / (ncol - 1.0)) * 2 - 1
            bx = cx_ + u * 16 + rng.uniform(-3, 3)
            ln = rng.uniform(30, 78) * (1 - abs(u) * 0.55)
            w = rng.uniform(3.5, 8) * (1 - abs(u) * 0.3)
            tilt = u * 0.55 + rng.uniform(-0.12, 0.12)
            dx, dy = math.sin(tilt), -up * math.cos(tilt)
            nx, ny = -dy, dx
            tx, ty = bx + dx * ln, cy_ + dy * ln
            sx, sy = bx + dx * (ln - w * 1.6), cy_ + dy * (ln - w * 1.6)
            pts = [(bx - nx * w, cy_ - ny * w), (sx - nx * w, sy - ny * w), (tx - nx * w * 0.2, ty - ny * w * 0.2),
                   (sx + nx * w, sy + ny * w), (bx + nx * w, cy_ + ny * w)]
            M.poly(pts, 1)
            G.thick(bx - nx * w * 0.3, cy_ - ny * w * 0.3, sx - nx * w * 0.3, sy - ny * w * 0.3, 0.5, 1)
            G.thick(sx - nx * w * 0.3, sy - ny * w * 0.3, tx - nx * w * 0.2, ty - ny * w * 0.2, 0.5, 1)
        # rubble at the root of the cluster
        for j in range(5):
            M.ellipse(cx_ + rng.uniform(-22, 22), cy_ + up * rng.uniform(-2, 4), rng.uniform(5, 11), rng.uniform(3, 6), 1)
    near_finish(near, M, hexc("0b0c1a"), hexc("26305a"), hexc("060610"))
    for i, v in enumerate(G.d):
        if v and M.d[i] and near.px[i] == hexc("0b0c1a"):
            near.px[i] = hexc("141832")
    return far, near


def gen_abyss():
    ramp = [hexc("0c0a0e"), hexc("141014"), hexc("1c161a"), hexc("261c20"), hexc("32242a")]
    far, rng, wor, f1 = strata_far(303, ramp, vertical=True, band=16.0, warp=22.0, crack=0.25)
    # hex-ish cross joints in the basalt columns
    for y in range(S):
        for x in range(S):
            if (y + int(14 * hash2((x + int(22 * (f1(x, y) - 0.5) * 2)) // 16, 0, 2))) % 23 == 0 and hash2(x // 16, y // 23, 3) < 0.7:
                far.px[y * S + x] = ramp[0]
    # ember cracks
    emb = [hexc("3a1410"), hexc("6a2012"), hexc("a83a14"), hexc("e8701c"), hexc("ffc050")]
    f3 = FBM(random.Random(304), 2, 2)
    w2 = Worley(random.Random(305), 4)
    for y in range(S):
        for x in range(S):
            d1, d2 = w2(x, y)
            m = f3(x, y)
            e = d2 - d1
            if m > 0.52:
                k = (m - 0.52) * 9
                if e < 0.9:
                    far.px[y * S + x] = emb[min(3, 1 + int(k * 1.6))]
                elif e < 2.6:
                    far.px[y * S + x] = emb[0] if e < 1.8 or (x + y) % 2 else far.px[y * S + x]
    far.wrap = False
    # near: basalt teeth growing from slabs
    near = Sprite(S, S)
    M = islands(rng, 0.59, base=3, stretch=2)
    edges_up, edges_dn = [], []
    for x in range(S):
        for y in range(S):
            if M.d[y * S + x]:
                if not M.d[((y - 1) % S) * S + x]:
                    edges_up.append((x, y))
                if not M.d[((y + 1) % S) * S + x]:
                    edges_dn.append((x, y))
    for (lst, sgn, s0) in ((edges_up, -1, 11), (edges_dn, 1, 12)):
        for (x, y) in lst:
            if x % 7 == (y // 9) % 7 and hash2(x, y // 9, s0) < 0.75:
                ln = 8 + int(hash2(x, y, s0 + 1) * 30)
                w0 = 3 + int(hash2(x, y, s0 + 2) * 4)
                skew = (hash2(x, y, s0 + 3) - 0.5) * 0.5
                for j in range(ln):
                    w = w0 * (1 - j / float(ln))
                    for dx in range(int(-w), int(w) + 1):
                        M.set(x + dx + skew * j, y + sgn * j, 1)
    near_finish(near, M, hexc("0a080a"), hexc("2a1c1c"), hexc("050405"), hexc("0f0b0d"))
    # a few ember glints on the teeth
    for i in range(60):
        x, y = rng.randint(0, S - 1), rng.randint(0, S - 1)
        if M.get(x, y) and M.get(x, y + 1) == 0:
            near.px[y * S + x] = emb[2]
    return far, near


def fit_brightness(sp, lo=0.10, hi=0.13, target=0.115):
    m = mean_lum(sp)
    if lo <= m <= hi:
        return sp
    k = target / m
    sp.px = [(min(255, int(p[0] * k + 0.5)), min(255, int(p[1] * k + 0.5)), min(255, int(p[2] * k + 0.5)), 255)
             for p in sp.px]
    return sp


def gen_caves():
    out = {}
    for name, fn in (("cave", gen_cave), ("deep", gen_deep), ("abyss", gen_abyss)):
        far, near = fn()
        far = fit_brightness(far)
        for p in far.px:
            assert p[3] == 255
        out["backgrounds/%s_far.png" % name] = far
        out["backgrounds/%s_near.png" % name] = near
        if PREVIEW_DIR:
            t = Sprite(512, 512)
            for y in range(512):
                for x in range(512):
                    t.px[y * 512 + x] = far.px[(y % S) * S + (x % S)]
            for y in range(512):
                for x in range(512):
                    p = near.px[(y % S) * S + (x % S)]
                    if p[3]:
                        t.px[y * 512 + x] = p
            write_png(os.path.join(PREVIEW_DIR, "bg_%s_tiled.png" % name), t)
    return out


def gen_mist():
    rng = random.Random(404)
    sp = Sprite(S, S)
    f1 = FBM(rng, 2, 4, pers=0.55, basey=4)
    f2 = FBM(rng, 5, 3, basey=9)
    for y in range(S):
        for x in range(S):
            v = f1(x + 30 * (f2(x, y) - 0.5), y) * 0.8 + f2(x, y) * 0.2
            a = clamp((v - 0.42) / 0.3)
            a = a * a * (3 - 2 * a)
            al = int(a * 110)
            sp.px[y * S + x] = (255, 255, 255, al)
    return sp


# --------------------------------------------------------------------------
# Title backdrop (menu.png)
# --------------------------------------------------------------------------
def gen_menu():
    W, H = 640, 400
    rng = random.Random(900)
    sp = Sprite(W, H, INK)
    horizon = 268
    dome_x, dome_y, da, db = 320, 330, 102, 72
    # ---- sky ----
    sky = [hexc("121028"), hexc("1c1838"), hexc("2a1e4a"), hexc("43285a"), hexc("6a3262"), hexc("9a4460"),
           hexc("c8644e"), hexc("e89a54"), hexc("f8c878")]
    cl = FBM(random.Random(901), 3, 4, w=640, h=400, basey=8)
    for y in range(horizon + 8):
        for x in range(W):
            t = (y / float(horizon)) ** 1.25
            # horizon glow strongest behind the dome
            t += 0.07 * math.exp(-((x - dome_x) / 220.0) ** 2) * (y / float(horizon))
            c = cl(x, y)
            calm = 1 - 0.75 * math.exp(-((x - 320) / 210.0) ** 2 - ((y - 80) / 90.0) ** 2)
            if c > 0.52:
                t -= (c - 0.52) * 0.9 * calm
            sp.px[y * W + x] = ramp_pick(sky, t * 0.98, x, y)
    for i in range(150):
        x, y = rng.randint(0, W - 1), int(rng.random() ** 1.8 * 170)
        b = rng.random()
        sp.px[y * W + x] = hexc("f0e8ff") if b > 0.85 else hexc("9a8ac8") if b > 0.4 else hexc("5a4e8a")
        if b > 0.95:
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                if 0 <= x + dx < W and 0 <= y + dy:
                    sp.px[(y + dy) * W + x + dx] = hexc("7a6ab0")
    # ---- ringed moon ----
    mx, my, mr = 524, 74, 30
    moon = [hexc("6a5a7a"), hexc("9a88a0"), hexc("c8b8b8"), hexc("ecdcc8"), hexc("fff4e0")]
    cr = Worley(random.Random(902), 9, size=128)

    def ring_at(x, y):
        ang = -0.32
        dx, dy = x + 0.5 - mx, y + 0.5 - my
        u = dx * math.cos(ang) - dy * math.sin(ang)
        v = dx * math.sin(ang) + dy * math.cos(ang)
        d = math.hypot(u / 60.0, v / 9.5)
        return d, v

    for y in range(my - 40, my + 40):
        for x in range(mx - 66, mx + 66):
            if not (0 <= x < W and 0 <= y < H):
                continue
            d, v = ring_at(x, y)
            inmoon = math.hypot(x + 0.5 - mx, y + 0.5 - my) <= mr
            ring = 0.7 < d < 1.0 and not (0.82 < d < 0.86)
            if inmoon and not (ring and v > 0):
                dx, dy = (x + 0.5 - mx) / mr, (y + 0.5 - my) / mr
                nz = math.sqrt(max(0.0, 1 - dx * dx - dy * dy))
                lit = (-0.5 * dx - 0.55 * dy + 0.62 * nz + 0.3) / 1.3
                d1, d2 = cr(x * 1.0, y * 1.0)
                if d1 < 3.2:
                    lit -= 0.14
                elif d1 < 4.2:
                    lit += 0.06
                sp.px[y * W + x] = ramp_pick(moon, lit, x, y)
            elif ring:
                c = hexc("e8c8a8") if d < 0.82 else hexc("b890a0")
                if inmoon is False and v <= 0 and math.hypot(x + 0.5 - mx, y + 0.5 - my) <= mr:
                    continue
                if (x + y) % 2 == 0 or d < 0.8:
                    sp.px[y * W + x] = mix(sp.px[y * W + x], c, 0.8)
    # moon glow
    for y in range(my - 70, my + 70):
        for x in range(mx - 80, mx + 80):
            if 0 <= x < W and 0 <= y < H:
                d = math.hypot(x - mx, y - my)
                if mr < d < 70:
                    t = (1 - (d - mr) / 40.0)
                    if t > 0 and t * 0.3 > bayer(x, y) * 0.5:
                        sp.px[y * W + x] = mix(sp.px[y * W + x], hexc("c8a0b0"), 0.14)

    def paint(L, pal, rimc=None, hazec=None, hy0=0, hy1=1, hs=0.0, seams=8):
        for y in range(H):
            for x in range(W):
                v = L.d[y * W + x]
                if not v:
                    continue
                dk, md, lt = pal[v]
                up = L.get(x, y - 1) == 0
                toward = L.get(x + (1 if x < dome_x else -1), y) == 0
                away = L.get(x - (1 if x < dome_x else -1), y) == 0
                c = md
                if up or toward:
                    c = lt
                    if rimc and toward:
                        k = clamp(1.25 - math.hypot(x - dome_x, (y - dome_y + 30) * 1.3) / 330.0)
                        c = mix(lt, rimc, k * 0.8)
                elif away:
                    c = dk
                elif v == 1 and seams and y % seams == 0 and hash2(x // 10, y // seams, 3) < 0.6:
                    c = dk
                elif v == 2 and hash2(x // 2, y // 2, 5) < 0.14:
                    c = dk
                if hazec and hs:
                    t = clamp((y - hy0) / float(hy1 - hy0)) * hs
                    if t > bayer(x, y) * 0.9:
                        c = mix(c, hazec, 0.5)
                sp.px[y * W + x] = c

    # ---- far megastructures ----
    A = Layer(W, H, wrapx=False)
    megastructures(A, rng, W, horizon + 6, 1.18)
    paint(A, {1: (hexc("3c2c54"), hexc("47345e"), hexc("5a406c")), 2: (hexc("3a3458"), hexc("443e62"), hexc("524a70"))},
          hazec=hexc("a85a62"), hy0=150, hy1=horizon + 10, hs=0.85, seams=10)
    for i in range(90):
        x, y = rng.randint(0, W - 1), rng.randint(60, horizon)
        if A.get(x, y) == 1 and A.get(x - 1, y) == 1 and A.get(x + 1, y) == 1:
            sp.px[y * W + x] = rng.choice([hexc("f8c878"), hexc("e89a54"), hexc("2a1e4a")])
    # ---- ruined towers ----
    B = Layer(W, H, wrapx=False)
    for x in range(W):
        B.rect(x, horizon + 4, x, H - 1, 1)
    ruined_towers(B, rng, W, horizon + 8, 7, 60, 150, 20, 44, seed=5)
    # keep the centre clear for the dome
    for y in range(H):
        for x in range(dome_x - 120, dome_x + 120):
            if y < horizon + 4:
                B.d[y * W + x] = 0
    paint(B, {1: (hexc("201a3a"), hexc("282044"), hexc("342a54")), 2: (hexc("1c2c3c"), hexc("223644"), hexc("2c4852"))},
          rimc=hexc("c8704a"), hazec=hexc("6a3a5a"), hy0=200, hy1=horizon + 20, hs=0.6, seams=8)
    for i in range(50):
        x, y = rng.randint(0, W - 1), rng.randint(110, horizon)
        if B.get(x, y) == 1 and B.get(x - 1, y) == 1 and B.get(x + 1, y) == 1 and B.get(x, y - 1) == 1:
            sp.px[y * W + x] = hexc("ffc878")
            sp.px[(y + 1) * W + x] = hexc("c8704a")
    # ---- jungle behind the dome ----
    Cc = Layer(W, H, wrapx=False)
    n2 = TNoise(random.Random(905), 24)
    for x in range(W):
        Cc.rect(x, horizon + 12 - 10 * n2.at(x * 24.0 / W, 0.5), x, H - 1, 2)
    for x in (24, 96, 168, 228, 420, 474, 548, 612):
        if rng.random() < 0.5:
            fern_tree(Cc, rng, x, horizon + 14, rng.randint(70, 120), 1, 2, 1.0, 9)
        else:
            bulb_tree(Cc, rng, x, horizon + 14, rng.randint(60, 110), 1, 2, 1.0)
    paint(Cc, {1: (hexc("121c2c"), hexc("172434"), hexc("20303e")), 2: (hexc("12222c"), hexc("172c34"), hexc("203c40"))},
          rimc=hexc("e08a50"), seams=0)
    # ---- ground ----
    gn = FBM(random.Random(906), 6, 3, w=640, h=400)
    ground = [hexc("0a0e16"), hexc("0e141c"), hexc("131c24"), hexc("1a262c"), hexc("223234")]
    gy0 = horizon + 20
    for y in range(gy0, H):
        for x in range(W):
            t = (y - gy0) / float(H - gy0)
            v = 0.6 - t * 0.55 + (gn(x * 1.0, y * 3.0) - 0.5) * 0.5
            sp.px[y * W + x] = ramp_pick(ground, v, x, y)
    # ---- the biodome ----
    warm = [hexc("5a2a1c"), hexc("8a4424"), hexc("c8703a"), hexc("f0a850"), hexc("ffd890"), hexc("fff4d0")]
    for y in range(dome_y - db - 2, dome_y):
        for x in range(dome_x - da - 2, dome_x + da + 2):
            e = ((x + 0.5 - dome_x) / da) ** 2 + ((y + 0.5 - dome_y) / db) ** 2
            if e > 1:
                continue
            ei = ((x + 0.5 - dome_x) / (da - 3)) ** 2 + ((y + 0.5 - dome_y) / (db - 3)) ** 2
            if ei > 1:
                lit = x < dome_x + 20
                sp.px[y * W + x] = hexc("d8c8a8") if (lit and y < dome_y - 20) else hexc("8a7a70") if lit else hexc("4a4048")
                continue
            d = math.hypot((x - dome_x) / float(da), (y - (dome_y - 14)) / float(db))
            v = 1.0 - d * 0.78 + 0.1 * math.sin(x * 0.11)
            sp.px[y * W + x] = ramp_pick(warm, v, x, y)
    # ribs
    for k in (-0.8, -0.55, -0.28, 0, 0.28, 0.55, 0.8):
        lam = math.asin(k)
        for y in range(dome_y - db + 3, dome_y):
            dy = (dome_y - y - 0.5) / (db - 3.0)
            x = int(dome_x + (da - 3) * math.sin(lam) * math.sqrt(max(0.0, 1 - dy * dy)))
            sp.px[y * W + x] = hexc("6a3a24")
    for ry in (0.4, 0.72):
        y = int(dome_y - (db - 3) * ry)
        hwd = int((da - 3) * math.sqrt(1 - ry * ry))
        for x in range(dome_x - hwd, dome_x + hwd):
            sp.px[y * W + x] = hexc("6a3a24")
    # glass sheen
    for y in range(dome_y - db + 3, dome_y - 4):
        for x in range(dome_x - da + 3, dome_x + da - 3):
            ei = ((x + 0.5 - dome_x) / (da - 3)) ** 2 + ((y + 0.5 - dome_y) / (db - 3)) ** 2
            u = x + (dome_y - y) * 0.8
            if ei <= 1 and (abs(u - (dome_x - 34)) < 5 or abs(u - (dome_x - 22)) < 1.5):
                sp.px[y * W + x] = mix(sp.px[y * W + x], hexc("fff8e0"), 0.28)
    # plants and fixtures inside, as silhouettes against the light
    D = Layer(W, H, wrapx=False)
    for (px_, h_) in ((dome_x - 62, 30), (dome_x - 24, 46), (dome_x + 36, 38), (dome_x + 70, 22)):
        if h_ > 28:
            fern_tree(D, rng, px_, dome_y - 1, h_, 1, 1, 0.5, 8)
        else:
            bulb_tree(D, rng, px_, dome_y - 1, h_, 1, 1, 0.42)
    for (x0, x1) in ((dome_x - 84, dome_x - 48), (dome_x - 10, dome_x + 18), (dome_x + 48, dome_x + 84)):
        D.rect(x0, dome_y - 5, x1, dome_y - 1, 1)
        for x in range(x0, x1, 3):
            D.rect(x, dome_y - 6 - rng.randint(1, 7), x, dome_y - 5, 1)
    hang_vines(D, rng, dome_x - 50, dome_x + 50, dome_y - db + 6, 12, 5, 24, 1)
    # a tiny terraformer silhouette for scale
    D.rect(dome_x + 20, dome_y - 12, dome_x + 23, dome_y - 1, 1)
    D.rect(dome_x + 20, dome_y - 16, dome_x + 23, dome_y - 13, 1)
    for y in range(dome_y - db, dome_y):
        for x in range(dome_x - da, dome_x + da):
            if D.d[y * W + x]:
                ei = ((x + 0.5 - dome_x) / (da - 3)) ** 2 + ((y + 0.5 - dome_y) / (db - 3)) ** 2
                if ei <= 1:
                    sp.px[y * W + x] = hexc("3a2218") if D.get(x - 1, y) else hexc("6a3c20")
    # airlock
    for sx in (dome_x - da - 4, dome_x + da - 5):
        for y in range(dome_y - 16, dome_y):
            for x in range(sx, sx + 10):
                sp.px[y * W + x] = hexc("3a3440") if x in (sx, sx + 9) or y == dome_y - 16 else hexc("5ff0ff") if y < dome_y - 12 \
                    else hexc("201c28")
    # warm halo over everything around the dome (dithered additive)
    for y in range(dome_y - 170, H):
        for x in range(dome_x - 240, dome_x + 240):
            if not (0 <= x < W and 0 <= y < H):
                continue
            e = ((x + 0.5 - dome_x) / da) ** 2 + ((y + 0.5 - dome_y) / db) ** 2
            if e <= 1 and y < dome_y:
                continue
            d = math.hypot((x - dome_x) / 1.25, (y - (dome_y - 24)) * 1.15)
            t = clamp(1 - (d - 74) / 150.0) ** 2.2 * 0.55
            if t > 0.02:
                tq = int(t * 10 + bayer(x, y)) / 10.0
                p = sp.px[y * W + x]
                sp.px[y * W + x] = (min(255, int(p[0] + 250 * tq)), min(255, int(p[1] + 150 * tq)),
                                    min(255, int(p[2] + 60 * tq)), 255)
    # dome foundation + wet reflections
    for x in range(dome_x - da - 10, dome_x + da + 10):
        for y in range(dome_y, dome_y + 4):
            sp.px[y * W + x] = hexc("4a4450") if y == dome_y else hexc("2a2630")
    for y in range(dome_y + 4, min(H, dome_y + 64)):
        for x in range(dome_x - da - 30, dome_x + da + 30):
            t = (y - dome_y - 4) / 60.0
            wob = int(2 * math.sin(y * 0.9 + x * 0.05))
            pud = gn(x * 2.0, y * 6.0) > 0.47
            if pud and abs(x + wob - dome_x) < da * (1 - t * 0.5) and (y % 2 == 0 or t < 0.3):
                k = (1 - t) * 0.6 * (1 - abs(x + wob - dome_x) / float(da))
                sp.px[y * W + x] = mix(sp.px[y * W + x], hexc("ffc070"), clamp(k))
    # ---- foreground: giant plants and ruined machinery framing the view ----
    F = Layer(W, H, wrapx=False)
    fn = TNoise(random.Random(907), 16)
    for x in range(W):
        edge = min(x, W - 1 - x) / 320.0
        F.rect(x, H - 16 - 34 * (1 - edge) ** 2 - 8 * fn.at(x * 16.0 / W, 0.5), x, H - 1, 2)
    fern_tree(F, rng, 58, H - 20, 250, 1, 2, 2.1, 11)
    fern_tree(F, rng, 596, H - 16, 214, 1, 2, 1.9, 10)
    bulb_tree(F, rng, 150, H - 10, 96, 1, 2, 1.3)
    # toppled pylon (right) and a half-buried cog (left)
    F.thick(430, H - 14, 560, H - 78, 4, 1)
    F.thick(436, H - 6, 566, H - 70, 1.5, 1)
    for i in range(7):
        t = i / 6.0
        F.thick(430 + 130 * t, H - 14 - 64 * t, 436 + 130 * t + 6, H - 6 - 64 * t, 1, 1)
    F.thick(560, H - 78, 584, H - 104, 2, 1)
    F.thick(548, H - 96, 600, H - 84, 1.5, 1)
    hang_vines(F, rng, 440, 590, H - 70, 16, 8, 40, 2)
    F.ring(238, H + 6, 34, 52, 1, a0=0.1, a1=math.pi - 0.1)
    for i in range(9):
        a = math.pi * (0.08 + i * 0.105)
        F.thick(238 + math.cos(a) * 50, H + 6 - math.sin(a) * 50, 238 + math.cos(a) * 62, H + 6 - math.sin(a) * 62, 4.5, 1)
    F.ellipse(238, H + 6, 14, 14, 1)
    hang_vines(F, rng, 200, 280, H - 44, 8, 6, 24, 2)
    for i in range(60):
        x = rng.uniform(0, W)
        if 250 < x < 400 and rng.random() < 0.7:
            continue
        edge = min(x, W - 1 - x) / 320.0
        gy = H - 16 - 34 * (1 - edge) ** 2
        for j in range(rng.randint(3, 6)):
            a = math.pi * rng.uniform(0.15, 0.85)
            ln = rng.uniform(12, 34)
            F.curve((x, gy + 4), (x + math.cos(a) * ln * 0.6, gy - ln), (x + math.cos(a) * ln, gy - ln * 0.55), 1.0, v=2,
                    n=16)
    paint(F, {1: (hexc("07080e"), hexc("0b0d14"), hexc("14161e")), 2: (hexc("070b0e"), hexc("0a1014"), hexc("121c1e"))},
          rimc=hexc("f0a050"), seams=0)
    for i in range(80):
        x, y = rng.randint(0, W - 1), rng.randint(140, H - 4)
        if F.get(x, y) == 2 and F.get(x, y + 1) == 0:
            c = rng.choice([R("lime", 3), R("cyan", 3), R("mag", 3)])
            sp.px[(y + 1) * W + x] = c
            if y + 2 < H:
                sp.px[(y + 2) * W + x] = mix(sp.px[(y + 2) * W + x], c, 0.4)
    # ---- mist and rain ----
    mf = FBM(random.Random(908), 3, 3, w=640, h=400, basey=10)
    for y in range(horizon - 30, H):
        for x in range(W):
            band = math.exp(-((y - (horizon + 34)) / 30.0) ** 2)
            t = (mf(x, y) - 0.4) * 1.6 * band
            if t > 0 and t > bayer(x, y) * 0.8:
                sp.px[y * W + x] = mix(sp.px[y * W + x], hexc("b08a9a"), 0.16)
    for i in range(520):
        x, y = rng.randint(0, W + 60), rng.randint(-10, H)
        ln = rng.randint(4, 11)
        a = rng.choice((0.1, 0.14, 0.2))
        for j in range(ln):
            xx, yy = x - j // 3, y + j
            if 0 <= xx < W and 0 <= yy < H:
                sp.px[yy * W + xx] = mix(sp.px[yy * W + xx], hexc("c8d8f0"), a)
    sp.px = [(p[0], p[1], p[2], 255) for p in sp.px]
    return sp


# --------------------------------------------------------------------------
# Earth cargo ship (ship.png, ship_flame.png)
# --------------------------------------------------------------------------
def gen_ship_body():
    W, H = 192, 96
    sp = Sprite(W, H)
    rng = random.Random(77)
    hull = m_poly([(34, 64), (28, 46), (32, 30), (46, 17), (70, 9), (120, 8), (142, 14), (154, 28), (158, 46),
                   (156, 64), (148, 71), (42, 71)])
    wear = FBM(random.Random(78), 10, 3, w=192, h=96)
    for (x, y) in hull:
        v = 0.92 - (y - 8) / 63.0 * 0.75 + (wear(x, y) - 0.5) * 0.25
        if x > 150:
            v -= 0.15
        sp.set(x, y, ramp_soft(BN[1:], v, x, y, 0.05))
    # panel seams
    for (x, y) in hull:
        if (x - 34) % 17 == 0 or y in (36, 55):
            sp.set(x, y, mix(sp.get(x, y), BN[0], 0.55))
        elif (x - 35) % 17 == 0 or y in (37, 56):
            sp.set(x, y, mix(sp.get(x, y), WHITE, 0.25))
    # mission stripes
    for (x, y) in hull:
        if 41 <= y <= 46:
            c = RU[3] if y < 45 else RU[2]
            if wear(x * 2.0, y * 2.0) < 0.36:
                continue
            sp.set(x, y, c)
        elif 48 <= y <= 49 and wear(x * 2.0, y * 2.0 + 40) > 0.34:
            sp.set(x, y, TE[2])
    # rust streaks bleeding down from seams
    for i in range(46):
        x, y = rng.randint(30, 156), rng.choice((37, 47, 56, 20 + rng.randint(0, 30)))
        ln = rng.randint(2, 9)
        for j in range(ln):
            if (x, y + j) in hull:
                sp.set(x, y + j, mix(sp.get(x, y + j), RU[2] if j < ln * 0.6 else RU[1], 0.55 - j * 0.04))
    # scorched belly
    for (x, y) in hull:
        if y > 62:
            t = (y - 62) / 9.0
            if t > bayer(x, y) * 0.9:
                sp.set(x, y, mix(sp.get(x, y), hexc("2a2026"), 0.7))
    # cockpit windows
    for i in range(4):
        x0 = 44 + i * 9
        y0 = 24 - i
        for (x, y) in m_rect(x0, y0, x0 + 6, y0 + 6):
            if (x, y) in hull:
                sp.set(x, y, CY[1] if y > y0 + 3 else CY[2])
        sp.hline(x0, x0 + 6, y0 - 1, ST[1])
        sp.hline(x0, x0 + 6, y0 + 7, ST[3])
        sp.vline(x0 - 1, y0, y0 + 6, ST[1])
        sp.vline(x0 + 7, y0, y0 + 6, ST[2])
        sp.hline(x0 + 1, x0 + 3, y0 + 1, CY[4])
        sp.set(x0 + 1, y0 + 2, CY[4])
    # portholes
    for x0 in (62, 80, 98, 116):
        for (x, y) in m_ellipse(x0, 30, 3, 3):
            sp.set(x, y, ST[1])
        for (x, y) in m_ellipse(x0, 30, 2, 2):
            sp.set(x, y, AM[3] if x0 != 98 else CY[2])
        sp.set(x0 - 1, 29, AM[4] if x0 != 98 else CY[4])
    # Earth roundel + registry blocks
    for (x, y) in m_ellipse(134, 28, 6, 6):
        sp.set(x, y, WA[3] if (x + y) % 5 else WA[4])
    for (x, y) in ((132, 25), (133, 25), (131, 26), (132, 26), (133, 27), (136, 29), (137, 29), (136, 30), (135, 31),
                   (131, 30), (132, 31)):
        sp.set(x, y, MO[3])
    for (x, y) in m_ellipse(134, 28, 6, 6) - m_ellipse(134, 28, 5, 5):
        sp.set(x, y, BN[4] if x + y < 162 else BN[1])
    for i, w in enumerate((7, 3, 5, 2)):
        x = 62 + sum((7, 3, 5, 2)[:i]) + i * 2
        sp.rect(x, 50, x + w - 1, 53, ST[0])
    sp.rect(88, 50, 91, 53, RU[3])
    # dorsal equipment: dish, antenna, RCS blocks
    box(sp, 84, 3, 104, 7, (ST[1], ST[2], ST[4]))
    sp.vline(110, 0 + 1, 7, ST[3])
    sp.set(110, 1, RD[3])
    for (x, y) in m_ellipse(72, 6, 6, 3.4):
        if y <= 6:
            sp.set(x, y, BN[3] if x < 72 else BN[2])
    sp.line(72, 6, 72, 9, ST[2])
    box(sp, 126, 5, 132, 8, (ST[0], ST[1], ST[3]))
    # cargo door on the right with warm light and the ramp down to the ground
    door = m_rect(138, 40, 153, 66)
    for (x, y) in door:
        if (x, y) in hull:
            t = (y - 40) / 26.0
            sp.set(x, y, hexc("fff0c0") if t < 0.25 else hexc("ffc878") if t < 0.6 else hexc("d88a40"))
    sp.vline(137, 39, 67, ST[1])
    sp.hline(137, 154, 39, ST[1])
    sp.hline(140, 151, 46, hexc("a06030"))
    sp.rect(141, 56, 146, 66, hexc("8a5028"))
    sp.rect(148, 60, 151, 66, hexc("6a3c20"))
    ramp = m_poly([(146, 66), (156, 66), (190, 93), (190, 95), (178, 95)])
    for (x, y) in ramp:
        sp.set(x, y, BN[2] if (x + y) % 9 > 1 else BN[1])
    for (x, y) in line_pts(156, 66, 190, 93):
        sp.set(x, y, BN[4])
    for (x, y) in line_pts(146, 67, 178, 95):
        sp.set(x, y, BN[0])
    for i in range(5):
        t = (i + 0.5) / 5.0
        x, y = 152 + 31 * t, 67 + 27 * t
        sp.line(x - 3, y + 1, x + 3, y - 1, AM[2])
    # engine bells
    for ex in (62, 96, 128):
        for i in range(8):
            w = 5 + i
            for x in range(ex - w, ex + w + 1):
                u = (x - (ex - w)) / float(2 * w)
                sp.set(x, 72 + i, ST[3] if u < 0.25 else ST[2] if u < 0.6 else ST[1] if u < 0.85 else ST[0])
        sp.hline(ex - 12, ex + 12, 79, ST[0])
        sp.hline(ex - 5, ex + 5, 72, ST[4])
        sp.hline(ex - 9, ex + 9, 76, RU[1])
    # landing legs
    for (hx, hy, fx) in ((44, 66, 22), (150, 68, 166), (90, 71, 96)):
        if fx == 96:
            sp.rect(94, 80, 98, 92, ST[1])
            sp.vline(94, 80, 92, ST[2])
        else:
            for (x, y) in line_pts(hx, hy, fx, 92):
                sp.set(x, y, ST[3])
                sp.set(x + 1, y, ST[2])
                sp.set(x + 2, y, ST[1])
            sgn = 1 if fx < hx else -1
            for (x, y) in line_pts(hx + sgn * 14, hy + 2, fx + sgn * 4, 84):
                sp.set(x, y, ST[2])
                sp.set(x, y + 1, ST[0])
        box(sp, fx - 7, 92, fx + 8, 94, (ST[0], ST[2], ST[4]))
        sp.hline(fx - 5, fx + 6, 93, ST[1])
    sp.outline()
    # running lights
    sp.set(29, 46, RD[3])
    sp.set(157, 46, LI[3])
    return sp


def gen_ship_flame():
    sh = Sprite(256, 48)
    cols = [hexc("fffbe8"), hexc("fff0a0"), hexc("ffc04a"), hexc("f08a28"), hexc("c84a1c"), hexc("7a2418")]
    for f in range(4):
        ln = (40, 45, 38, 47)[f]
        for y in range(ln):
            t = y / float(ln)
            hw = 14.0 * (1 - t) ** 0.7 * (1 + 0.14 * math.sin(y * 0.55 + f * 1.9)) + 0.6
            sway = math.sin(y * 0.21 + f * 1.6) * t * 3.5
            for x in range(64):
                dx = abs(x + 0.5 - 32 - sway)
                if dx <= hw:
                    u = dx / hw
                    v = u * 2.6 + t * 3.4 + (0.5 if (x + y + f) % 2 and u > 0.4 else 0)
                    i = min(5, int(v))
                    c = cols[i]
                    if i == 5:
                        c = wa(c, 200 if (x + y) % 2 else 140)
                    sh.set(f * 64 + x, y, c)
                elif dx <= hw + 2 and t < 0.9:
                    sh.set(f * 64 + x, y, wa(cols[4], 90 if dx <= hw + 1 else 45))
        # shock diamonds
        for k in range(3):
            y = 5 + k * 9 + (f % 2)
            w = 5 - k
            for x in range(-w, w + 1):
                if abs(x) + 0 <= w:
                    sh.set(f * 64 + 32 + x, y + (abs(x) // 2), cols[0])
        # embers
        rng = random.Random(880 + f)
        for k in range(7):
            x, y = 32 + rng.randint(-9, 9), ln - rng.randint(0, 9) + rng.randint(0, 3)
            if 0 <= y < 48:
                sh.set(f * 64 + x, y, wa(cols[2], 220))
    return sh


def gen_ship():
    return {"sprites/ship.png": gen_ship_body(), "sprites/ship_flame.png": gen_ship_flame()}


# --------------------------------------------------------------------------
# UI ornaments (ui.png)
# --------------------------------------------------------------------------
UI_CORNER = """
KKKKKKKKKKKKKKKK
K455555544444433
K5#3K33333333332
K53KKKKKKKKKKKKK
K5K3.....MlM....
K43K....mM.mM...
K43K...mm...m...
K43K..Mm........
K43KmlM.........
K43KMm..........
K43KmM..........
K33K.m..........
K33K............
K33K............
K32K............
K32K............
"""

UI_BADGE = """
.....KKKKKK.....
...KK455554KK...
..K4553333344K..
.K453KKKKKK342K.
.K53KjjjjjjK33K.
K45KjjjjjjjjK32K
K53KjjjjjjjjK32K
K53KjjjjjjjjK32K
K53KjjjjjjjjK22K
K43KjjjjjjjjK22K
K43KjjjjjjjjK21K
.K33KjjjjjjK21K.
.K332KKKKKK221K.
..K3322222211K..
...KK222211KK...
.....KKKKKK.....
"""


def ui_slot(kind):
    sp = Sprite(16, 16)
    inset = hexc("14121e")
    sp.rect(0, 0, 15, 15, INK)
    # bevel: dark top-left (inset), light bottom-right
    sp.rect(1, 1, 14, 14, ST[1])
    sp.hline(1, 14, 1, ST[0])
    sp.vline(1, 1, 14, ST[0])
    sp.hline(1, 14, 14, ST[2])
    sp.vline(14, 1, 14, ST[2])
    sp.set(14, 14, ST[3])
    sp.rect(2, 2, 13, 13, inset)
    sp.hline(2, 13, 2, hexc("0c0a14"))
    sp.vline(2, 2, 13, hexc("0c0a14"))
    sp.set(13, 13, hexc("1e1c2c"))
    for (x, y) in ((0, 0), (15, 0), (0, 15), (15, 15)):
        sp.set(x, y, CLEAR)
    if kind == "selected":
        for i in range(1, 15):
            for (x, y) in ((i, 1), (i, 14), (1, i), (14, i)):
                sp.set(x, y, CY[3])
        for (x, y) in ((1, 1), (14, 1), (1, 14), (14, 14), (2, 1), (1, 2)):
            sp.set(x, y, CY[4])
        for i in range(2, 14):
            for (x, y) in ((i, 2), (2, i)):
                sp.set(x, y, hexc("1a4a58"))
            for (x, y) in ((i, 13), (13, i)):
                sp.set(x, y, hexc("143642"))
    elif kind == "favourite":
        for (x, y, c) in ((12, 2, GD[4]), (11, 3, GD[3]), (12, 3, GD[4]), (13, 3, GD[3]), (12, 4, GD[3]), (11, 4, GD[2]),
                          (13, 4, GD[2]), (10, 3, GD[2]), (14, 3, GD[2])):
            sp.set(x, y, c)
        sp.hline(1, 14, 14, GD[2])
        sp.vline(14, 6, 14, GD[2])
    elif kind == "hotbar":
        sp.hline(1, 14, 14, AM[2])
        sp.hline(1, 14, 1, RU[1])
        for (x, y) in ((1, 1), (2, 1), (1, 2), (14, 1), (13, 1), (14, 2), (1, 14), (2, 14), (1, 13), (14, 14),
                       (13, 14), (14, 13)):
            sp.set(x, y, AM[3])
        sp.hline(6, 9, 15, AM[3])
        sp.hline(6, 9, 14, AM[4])
    return sp


def ui_panel_tile(k):
    """Tileable 16x16 brushed-metal panel with faint moss in the seams."""
    sp = Sprite(16, 16)
    rng = random.Random(660 + k)
    base = [hexc("1c1e2c"), hexc("222536"), hexc("282c3e"), hexc("303548")]
    for y in range(16):
        streak = hash2(0, y, 70 + k)
        for x in range(16):
            v = 1 + (1 if streak > 0.72 else -1 if streak < 0.2 else 0)
            if hash2(x // 3, y, 71 + k) < 0.12:
                v += 1 if hash2(x, y, 72) < 0.5 else -1
            sp.set(x, y, base[max(0, min(3, v))])
    seam = hexc("12131e")
    if k in (0, 2, 4, 5):
        sp.hline(0, 15, 15, seam)
        sp.hline(0, 15, 0, base[3])
    if k in (1, 2, 5):
        sp.vline(15, 0, 15, seam)
        sp.vline(0, 0, 15, base[3])
    if k == 3:
        for (x, y) in ((3, 3), (12, 3), (3, 12), (12, 12)):
            sp.set(x, y, ST[2])
            sp.set(x + 1, y + 1, seam)
    if k in (2, 5):
        sp.set(1, 1, ST[2])
        sp.set(14, 14, ST[1])
    mossc = [hexc("223a2c"), hexc("2c4a34"), hexc("3a5e3c")]
    n = (5, 4, 7, 0, 9, 6)[k]
    for i in range(n):
        if k in (0, 2, 4, 5) and (i % 2 == 0 or k not in (2, 5)):
            x = rng.randint(0, 15)
            sp.set(x, 15, mossc[rng.randint(0, 2)])
            if rng.random() < 0.4:
                sp.set(x, 14, mossc[0])
        else:
            y = rng.randint(0, 15)
            sp.set(15, y, mossc[rng.randint(0, 2)])
            if rng.random() < 0.4:
                sp.set(14, y, mossc[0])
    return sp


def gen_ui():
    sh = Sprite(96, 48)
    sh.blit(ascii_sprite(UI_CORNER, outline=False), 0, 0)
    sh.blit(ascii_sprite(UI_BADGE, outline=False), 16, 0)
    # divider with a leaf motif (columns 0-7 and 24-31 are plain and stretchable)
    d = Sprite(32, 8)
    d.hline(0, 31, 3, ST[3])
    d.hline(0, 31, 4, ST[0])
    for x in (9, 22):
        d.rect(x, 2, x + 1, 5, ST[2])
        d.set(x, 2, ST[4])
        d.set(x + 1, 5, ST[0])
    stamp(d, ["..l....M..", ".lMm..MMm.", "lMMMmMMMmG", ".mMmKKmMG.", "..m.KK.G.."], 11, 1,
          {"l": MO[4], "M": MO[3], "m": MO[2], "G": MO[1], "K": ST[1]})
    d.set(15, 4, AM[4])
    d.set(16, 4, AM[2])
    sh.blit(d, 32, 0)
    d2 = Sprite(32, 8)
    d2.hline(0, 31, 3, ST[2])
    d2.hline(0, 31, 4, ST[0])
    for (dx, dy, c) in ((0, -2, ST[3]), (-1, -1, ST[3]), (0, -1, ST[4]), (1, -1, ST[2]), (-2, 0, ST[3]), (-1, 0, ST[4]),
                        (0, 0, CY[3]), (1, 0, ST[2]), (2, 0, ST[1]), (-1, 1, ST[2]), (0, 1, ST[1]), (1, 1, ST[1]),
                        (0, 2, ST[0])):
        d2.set(16 + dx, 3 + dy, c)
    sh.blit(d2, 32, 8)
    # tab / label plate
    t = Sprite(32, 16)
    t.rect(1, 0, 30, 15, INK)
    t.rect(0, 1, 31, 14, INK)
    t.rect(1, 1, 30, 14, ST[1])
    t.hline(1, 30, 1, ST[3])
    t.vline(1, 1, 14, ST[2])
    t.hline(1, 30, 14, ST[0])
    t.vline(30, 1, 14, ST[0])
    t.rect(3, 3, 28, 12, hexc("1c1e2c"))
    t.hline(3, 28, 3, hexc("12131e"))
    t.vline(3, 3, 12, hexc("12131e"))
    t.hline(4, 28, 12, ST[2])
    for (x, y) in ((2, 2), (29, 2), (2, 13), (29, 13)):
        t.set(x, y, ST[4])
    t.set(27, 1, RU[2])
    t.set(28, 1, RU[3])
    t.set(28, 2, RU[1])
    for (x, y, c) in ((1, 10, MO[2]), (1, 11, MO[3]), (2, 12, MO[2]), (1, 12, MO[1]), (0, 9, MO[3]), (2, 14, MO[2]),
                      (3, 14, MO[3]), (1, 13, MO[2])):
        t.set(x, y, c)
    sh.blit(t, 64, 0)
    for i, kind in enumerate(("normal", "selected", "favourite", "hotbar")):
        sh.blit(ui_slot(kind), i * 16, 16)
    for k in range(6):
        sh.blit(ui_panel_tile(k), k * 16, 32)
    return sh


# --------------------------------------------------------------------------
# Fonts (pixel.ttf, title.ttf)
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

GLYPHS["₂"] = [".....", ".....", ".....", ".....", "##.", "..#", ".#.", "#..", "###"]

PIXEL_FONT = dict(glyphs=GLYPHS, notdef=NOTDEF, cap=7, asc=8, desc=2, check="Ag0→°₂",
                  names={1: "Terraform Pixel", 2: "Regular", 3: "TerraformPixel-Regular-1.000",
                         4: "Terraform Pixel Regular", 5: "Version 1.000", 6: "TerraformPixel-Regular", "xh": 5})

# Title font: tall stencil capitals, 9 px cap height, 2 px stems and 1 px
# bars, with stencil cuts where bowls meet stems.  Row 0 is the cap-top row,
# row 8 sits on the baseline.  Lowercase letters are small caps (7 px).
TCAPS = {
    "A": ["..##..", ".####.", "##..##", "##..##", "##..##", "######", "##..##", "##..##", "##..##"],
    "B": ["##.##.", "##..##", "##..##", "##..##", "##.##.", "##..##", "##..##", "##..##", "##.##."],
    "C": [".####.", "##..##", "##....", "##....", "##....", "##....", "##....", "##..##", ".####."],
    "D": ["##.##.", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", "##.##."],
    "E": ["######", "##....", "##....", "##....", "##.##.", "##....", "##....", "##....", "######"],
    "F": ["######", "##....", "##....", "##....", "##.##.", "##....", "##....", "##....", "##...."],
    "G": [".####.", "##..##", "##....", "##....", "##.###", "##..##", "##..##", "##..##", ".####."],
    "H": ["##..##", "##..##", "##..##", "##..##", "######", "##..##", "##..##", "##..##", "##..##"],
    "I": ["##", "##", "##", "##", "##", "##", "##", "##", "##"],
    "J": ["....##", "....##", "....##", "....##", "....##", "....##", "##..##", "##..##", ".####."],
    "K": ["##..##", "##..##", "##.##.", "####..", "###...", "####..", "##.##.", "##..##", "##..##"],
    "L": ["##....", "##....", "##....", "##....", "##....", "##....", "##....", "##....", "######"],
    "M": ["##....##", "###..###", "########", "##.##.##", "##.##.##", "##....##", "##....##", "##....##", "##....##"],
    "N": ["##...##", "###..##", "###..##", "####.##", "##.####", "##..###", "##..###", "##...##", "##...##"],
    "O": [".####.", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", ".####."],
    "P": ["##.##.", "##..##", "##..##", "##..##", "##.##.", "##....", "##....", "##....", "##...."],
    "Q": [".####.", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", ".####.", "...##.", "....##"],
    "R": ["##.##.", "##..##", "##..##", "##..##", "##.##.", "####..", "##.##.", "##..##", "##..##"],
    "S": [".####.", "##..##", "##....", "###...", ".####.", "...###", "....##", "##..##", ".####."],
    "T": ["######", "..##..", "..##..", "..##..", "..##..", "..##..", "..##..", "..##..", "..##.."],
    "U": ["##..##", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", ".####."],
    "V": ["##..##", "##..##", "##..##", "##..##", "##..##", "##..##", ".####.", ".####.", "..##.."],
    "W": ["##....##", "##....##", "##....##", "##....##", "##.##.##", "##.##.##", "########", "###..###", "##....##"],
    "X": ["##..##", "##..##", "##..##", ".####.", "..##..", ".####.", "##..##", "##..##", "##..##"],
    "Y": ["##..##", "##..##", "##..##", ".####.", "..##..", "..##..", "..##..", "..##..", "..##.."],
    "Z": ["######", "....##", "...###", "...##.", "..##..", ".##...", "###...", "##....", "######"],
    "0": [".###.", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##", ".###."],
    "1": ["..##", ".###", "####", "..##", "..##", "..##", "..##", "..##", "..##"],
    "2": [".####.", "##..##", "....##", "....##", "...##.", "..##..", ".##...", "##....", "######"],
    "3": [".####.", "##..##", "....##", "....##", "..###.", "....##", "....##", "##..##", ".####."],
    "4": ["...###", "..####", ".##.##", "##..##", "##..##", "######", "....##", "....##", "....##"],
    "5": ["######", "##....", "##....", "#####.", "....##", "....##", "....##", "##..##", ".####."],
    "6": [".####.", "##..##", "##....", "##....", "#####.", "##..##", "##..##", "##..##", ".####."],
    "7": ["######", "....##", "....##", "...##.", "...##.", "..##..", "..##..", "..##..", "..##.."],
    "8": [".####.", "##..##", "##..##", "##..##", ".####.", "##..##", "##..##", "##..##", ".####."],
    "9": [".####.", "##..##", "##..##", "##..##", ".#####", "....##", "....##", "##..##", ".####."],
}
_THIN = set("#%&@*~^<>/\\=+")


def _title_glyphs():
    g = {" ": (0, ["...."])}
    for ch, rows in TCAPS.items():
        g[ch] = (0, rows)
        if ch.isalpha():
            small = [r for i, r in enumerate(rows[:9]) if i not in (2, 6)] + rows[9:]
            g[ch.lower()] = (2, small)
    rowmap = [0, 1, 1, 2, 3, 4, 4, 5, 6, 7, 8]
    for ch, rows in GLYPHS.items():
        if ch in g or not (32 <= ord(ch) < 127):
            continue
        w = max(len(r) for r in rows)
        full = [r.ljust(w, ".") for r in rows] + ["." * w] * (9 - len(rows))
        st = [full[rowmap[i]] for i in range(11)]
        if ch not in _THIN:
            # embolden: every stroke becomes 2 px wide
            st = ["".join("#" if (r[x] == "#" or (x > 0 and r[x - 1] == "#")) else "." for x in range(w + 1))
                  for r in [s + "." for s in st]]
        while len(st) > 1 and "#" not in st[-1]:
            st.pop()
        g[ch] = (0, st)
    return g


TGLYPHS = _title_glyphs()
TNOTDEF = ["######", "#....#", "#....#", "#....#", "#....#", "#....#", "#....#", "#....#", "######"]
assert all(chr(c) in TGLYPHS for c in range(32, 127)), [chr(c) for c in range(32, 127) if chr(c) not in TGLYPHS]
assert all(chr(c) in GLYPHS for c in range(32, 127))
TITLE_FONT = dict(glyphs=TGLYPHS, notdef=TNOTDEF, cap=9, asc=10, desc=3, check="Ag0Q",
                  names={1: "Terraform Title", 2: "Regular", 3: "TerraformTitle-Regular-1.000",
                         4: "Terraform Title Regular", 5: "Version 1.000", 6: "TerraformTitle-Regular", "xh": 7})


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


def font_preview(path, glyphs, notdef, lines, adv_extra=1, line_h=12, top=2):
    def rows_of(ch):
        g = glyphs.get(ch, notdef)
        return g if isinstance(g, tuple) else (0, g)
    Wd = 4 + max(sum(max(len(r) for r in rows_of(ch)[1]) + adv_extra for ch in ln) for ln in lines)
    Hd = line_h * len(lines) + 4
    sp = Sprite(Wd, Hd, (24, 20, 37, 255))
    for li, line in enumerate(lines):
        x = 2
        for ch in line:
            off, rows = rows_of(ch)
            for r, row in enumerate(rows):
                for cx, c in enumerate(row):
                    if c == "#":
                        sp.set(x + cx, top + li * line_h + r + off, (232, 223, 194, 255))
            x += max(len(r) for r in rows) + adv_extra
    write_png(path, upscale(sp, 3))


def font_previews(d):
    font_preview(os.path.join(d, "font_pixel.png"), GLYPHS, NOTDEF,
                 ["The quick brown fox jumps over", "the lazy dog! 0123456789 O₂ 19%",
                  "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~", "· • … × ← → ↑ ↓ ° ₂"])
    font_preview(os.path.join(d, "font_title.png"), TGLYPHS, TNOTDEF,
                 ["PLANET TERRAFORMING", "Colony Readiness 19%", "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                  "abcdefghijklmnopqrstuvwxyz", "0123456789 !?.,:;'\"-+=/()", "#$%&*<>@[\\]^_`{|}~"], line_h=15, top=2)


def gen_fonts():
    out = {}
    for fname, kw in (("pixel.ttf", PIXEL_FONT), ("title.ttf", TITLE_FONT)):
        ttf = build_ttf(**kw)
        check_ttf(ttf, kw["check"])
        out["fonts/" + fname] = ttf
    return out


# --------------------------------------------------------------------------
# Preview + main
# --------------------------------------------------------------------------
def upscale(sp, k, bgc=(30, 27, 44, 255), grid=None):
    out = Sprite(sp.w * k, sp.h * k)
    W = out.w
    for y in range(out.h):
        sy = y // k
        for x in range(W):
            p = sp.px[sy * sp.w + (x // k)]
            b = bgc
            if grid and ((x // k) // grid[0] + sy // grid[1]) % 2:
                b = (bgc[0] + 10, bgc[1] + 10, bgc[2] + 12, 255)
            if p[3] == 255:
                out.px[y * W + x] = p
            elif p[3] == 0:
                out.px[y * W + x] = b
            else:
                a = p[3] / 255.0
                out.px[y * W + x] = (int(p[0] * a + b[0] * (1 - a)), int(p[1] * a + b[1] * (1 - a)),
                                     int(p[2] * a + b[2] * (1 - a)), 255)
    return out


# name -> (generator, preview checker grid or None).  A generator returns
# {relative asset path: Sprite} (or bytes for fonts).
GENERATORS = [
    ("icons", lambda: {"sprites/icons.png": gen_icons()}, (16, 16)),
    ("domes", lambda: gen_domes(), None),
    ("props", lambda: {"sprites/props.png": gen_props()}, (32, 32)),
    ("fx", lambda: {"sprites/fx.png": gen_fx()}, (16, 16)),
    ("foliage", lambda: {"sprites/foliage.png": gen_foliage()}, (16, 16)),
    ("surface", lambda: gen_surface(), None),
    ("caves", lambda: gen_caves(), None),
    ("mist", lambda: {"backgrounds/mist.png": gen_mist()}, None),
    ("menu", lambda: {"backgrounds/menu.png": gen_menu()}, None),
    ("ship", lambda: gen_ship(), None),
    ("ui", lambda: {"sprites/ui.png": gen_ui()}, (16, 16)),
    ("fonts", lambda: gen_fonts(), None),
]


def _run(job):
    global PREVIEW_DIR
    name, preview = job
    PREVIEW_DIR = preview
    fn = dict((g[0], g[1]) for g in GENERATORS)[name]
    grid = dict((g[0], g[2]) for g in GENERATORS)[name]
    random.seed(1234)
    outs = fn()
    lines = []
    for rel, data in sorted(outs.items()):
        path = os.path.join(ASSETS, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        if isinstance(data, bytes):
            with open(path, "wb") as f:
                f.write(data)
            lines.append("wrote %s (%d bytes)" % (rel, len(data)))
            continue
        write_png(path, data)
        lines.append("wrote %s (%dx%d)" % (rel, data.w, data.h))
        if preview:
            write_png(os.path.join(preview, rel.replace("/", "_")), upscale(data, 4, grid=grid))
    if preview and name == "fonts":
        font_previews(preview)
    return lines


def main():
    args = sys.argv[1:]
    preview = None
    only = None
    if "--preview" in args:
        preview = args[args.index("--preview") + 1]
        os.makedirs(preview, exist_ok=True)
    if "--only" in args:
        only = set(args[args.index("--only") + 1].split(","))
        bad = only - set(g[0] for g in GENERATORS)
        if bad:
            raise SystemExit("unknown --only name(s): %s (known: %s)" %
                             (", ".join(sorted(bad)), ", ".join(g[0] for g in GENERATORS)))
    jobs = [(g[0], preview) for g in GENERATORS if only is None or g[0] in only]
    results = None
    if len(jobs) > 1 and "--serial" not in args:
        try:
            import multiprocessing
            with multiprocessing.Pool(min(len(jobs), os.cpu_count() or 1)) as pool:
                results = pool.map(_run, jobs, 1)
        except (ImportError, OSError):
            results = None
    if results is None:
        results = [_run(j) for j in jobs]
    for lines in results:
        for ln in lines:
            print(ln)
    if preview:
        print("4x previews written to", preview)


if __name__ == "__main__":
    main()
