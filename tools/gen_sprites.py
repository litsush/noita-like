#!/usr/bin/env python3
"""Procedural sprite generator for "Planet Terraforming" (characters, body
mods, robots, creatures, workers, animals).

Pure Python 3 standard library (zlib + struct PNG writer), fully
deterministic (no randomness, no clock).  Run from the repo root:

    python3 tools/gen_sprites.py                 # (re)write every file below
    python3 tools/gen_sprites.py --preview DIR   # also write 4x previews to DIR
                                                 # (plus mods_on_player.png)

Exit status is 1 if any self-check fails (a frame drawn outside its cell,
the suit not covering the body, ...); the warnings are printed.

Conventions for every sheet: RGBA8, transparent background, frames packed
left-to-right from x=0, rows top-to-bottom, unused frames fully transparent.
1 art pixel = 1 game cell (shown at 3x, nearest filtering).  Everything
faces RIGHT (flip horizontally for left).  Light comes from the top-left.
Sprites carry a 1px #12101a outline; glows, sparks, flames and thin insect
legs are drawn without one.  Semi-transparent pixels are used only for glow
halos, motion streaks, webs, spores and the leech's flesh.

=========================================================================
1. PLAYER LAYERS
   assets/sprites/player_body.png          192x384
   assets/sprites/player_suit.png          192x384
   assets/sprites/player_accent_body.png   192x384
   assets/sprites/player_accent_suit.png   192x384
=========================================================================
Frames of 24x32, 8 columns x 12 rows, IDENTICAL layout and pixel alignment
in all four files (all four are rendered from one pose table).  The figure
is 10 px wide and 24 px tall when standing in the suit (outline included:
x 8..17, helmet top outline on y=8), centred on x~12; on grounded frames
the outline under the boot soles is on y=31 (sole fill on y=30).  Collision
box in game: 8x22.

  player_body         the terraformer without the suit: dark teal undersuit
                      with rust knee/shoulder/cuff patches, bare head, short
                      dark hair, dark gloves and boots.
  player_suit         the exploration suit; every opaque body pixel is
                      covered by an opaque suit pixel (self-checked), so the
                      body layer may be skipped or drawn underneath.  Bone
                      panels with moss, rust gloves/ankle/collar seals, teal
                      belt, round helmet with a dark visor and a cyan visor
                      highlight, teal chest unit with a lime status light.
                      No backpack and nothing on the back/feet/hands that
                      would clash with attachments.
  player_accent_body  team-colour pixels for the body: neckerchief + belt.
  player_accent_suit  team-colour pixels for the suit: helmet stripe,
                      shoulder patch of the leading arm, a band on each
                      forearm.
  Accent layers are neutral greys (R=G=B, 150..255, shading preserved),
  everything else transparent: multiply by the player's colour and draw on
  top of the matching layer (the same pixels are light grey in the base
  layer).  Draw order: back attachment, body and/or suit, accent, the other
  attachments.

  Rows (frames used; the remaining columns are transparent):
     0  idle (4): breathing, slight arm sway
     1  run (8): full cycle
     2  cols 0-1 jump (0 rise, 1 apex); cols 2-3 fall (alternate; limbs
        trailing up)
     3  climb (4): clinging to a wall on the RIGHT whose face is at x~16-17
     4  swim (4): diagonal/horizontal paddle, centred in the frame
     5  reach forward (4): arm straight ahead holding the multitool
     6  reach up-forward (4): aiming 45 degrees up
     7  reach down-forward (4): aiming 48 degrees down
        (rows 5-7: frame 0 extending, frame 1 fully extended, frames 2-3
        the held pose with recoil/vibration - loop 2,3 while held.  The
        multitool is drawn in BOTH the body and suit layers and only in
        these rows; its glowing tip is ~3 px beyond the hand along the aim.)
     8  col 0 hurt as a pure white flash silhouette (accent layers empty),
        col 1 hurt flinch; cols 2-5 blackout: buckling, falling backwards,
        col 5 lying on the back (head to the left)
     9  sleep (2): lying on the back, head to the left, eyes closed,
        breathing; for beds
    10  fly/hover (4): upright, legs trailing together, 1px bob (rocket feet)
    11  cols 0-1 glide (leaning forward, arms spread front/back);
        cols 2-3 dash (leaning hard forward; includes semi-transparent
        motion streaks behind, in both body and suit layers)

=========================================================================
2. assets/sprites/player_anchors.json
=========================================================================
  {"frame": [24, 32], "cols": 8, "rows": 12,
   "anchors": {"head": A, "eyes": A, "torso": A, "back": A,
               "arms": A, "hands": A, "legs": A, "feet": A},
   "extra":   {"foot_front": A, "foot_back": A, "knee_front": A,
               "knee_back": A, "hand_back": A}}
  Each A is a 12x8 array indexed [row][col] of [x, y] integer pixel
  coordinates inside the 24x32 frame (0 <= x < 24, 0 <= y < 32), or null
  for unused frames.  They are computed from the same skeleton that draws
  the frames, for the SUIT silhouette (unsuited, the head top is 1 px
  lower).  Draw an attachment cell so that its pixel (8,8) lands on the
  anchor; when the player faces left mirror both: x' = 23 - x for the
  anchor and flip the cell (its pixel (7,8) then lands on the anchor).
    head   top-centre of the helmet (the helmet's outline row)
    eyes   centre of the visor / eyes
    torso  centre of the chest
    back   upper back, 1-2 px behind (left of) the suit's back outline
    arms   shoulder joint of the leading (near) arm
    hands  the leading hand (the tip of the reach in rows 5-7)
    legs   midpoint between the two knees
    feet   midpoint between the two feet, on the sole/ground row (y=31 when
           standing)
  "extra" is optional data for games that want per-limb placement (e.g.
  drawing the feet cell once per foot while running): the sole of each foot
  (same convention as "feet"), each knee joint and the far hand.

=========================================================================
3. assets/sprites/mods.png   128x848   body-mod attachments
=========================================================================
Cells of 16x16, 8 columns x 53 rows.  Row = set index (table below).
Columns: 0-1 tier I (animation frame A, B), 2-3 tier II, 4-5 tier III,
6-7 Legendary.  So cell = (tier*2 + frame, set).  Draw the cell with its
pixel (8,8) on the slot's anchor, facing right, flipped with the player.
"back" cells are drawn BEHIND the player layers, all others in front.
Tiers grow from ~5-7 px (I) to ~11-14 px (III).  Legendary cells use
luminous colours, carry a 1px semi-transparent glow halo outside their
outline (alpha ~105 in frame A, ~60 in frame B) and are clearly animated
between A and B; tiers I-III have small A/B animations (blinking lights,
flames, rotors) or identical frames.  Alternate A/B every ~0.25-0.4 s.
A cell may use its whole 16x16 area (offsets -8..7 from the anchor pixel).
Where each slot's art sits relative to the anchor pixel (0,0), x forward:
    feet   wraps the boots and sits on the ground: the art's bottom outline
           is on y=0 (the anchor/ground row); only a Legendary halo reaches
           y=1
    legs   around the knees and shins, struts beside the legs
    back   hangs behind the back, x -8..3 (the body covers x >= 1)
    torso  on the chest, at most x,y -6..6 (plus Legendary sparkles)
    head   sits on the helmet: the art's base is on y=0..2, the rest above
    eyes   over the visor, protruding forward (to the right)
    arms   on the shoulder / upper arm, tools rising above the shoulder
    hands  around the hand, emitters pointing forward
Set index, name, slot (motif):
   0 Rocket Feet       feet  (boot shell, heel thruster pod, flame)
   1 Sticky Soles      feet  (green gecko pads and goo)
   2 Spring Heels      feet  (coil springs behind heel / before toe)
   3 Hustle Treads     feet  (wheels, then tank treads)
   4 Aqua Flippers     feet  (blue fin blades)
   5 Stompers          feet  (heavy iron boot block, hazard sole)
   6 Dash Pistons      legs  (pistons with extending rods)
   7 Cargo Pants       legs  (pouches; Legendary pocket dimension)
   8 Thermal Leggings  legs  (glowing heater bands)
   9 Root Walkers      legs  (vines, leaves, flowers)
  10 Shock Absorbers   legs  (coil-spring struts, knee cap)
  11 Jackhammer Knees  legs  (yellow housing, downward chisel)
  12 O2 Backpack       back  (tank(s) with a leaf; Legendary glass tank)
  13 Glide Wings       back  (leaf-membrane glider wing)
  14 Shoulder Turret   back  (ball turret, barrel angled up-forward)
  15 Battery Pack      back  (cell box with lime charge bars)
  16 Sprinkler Pack    back  (water tank, sprinkler; Legendary raincloud)
  17 Pack Mule Frame   back  (frame, crate, horseshoe magnet)
  18 Drone Dock        back  (shelf with tiny docked drones)
  19 Beacon Rack       back  (mast with a magenta beacon, signal rings)
  20 Plating           torso (riveted armour plates)
  21 Filter Lungs      torso (respirator canister, fan)
  22 Medi-Core         torso (white panel with a cross)
  23 Reactor Heart     torso (ring reactor with a glowing core)
  24 Camo Skin         torso (mottled ghillie patch; Legendary shimmer)
  25 Synth Belly       torso (purple fabricator with a gear)
  26 Buddy Breather    torso (regulator, yellow spare regulator, bubbles)
  27 Antenna Array     head  (antenna, crossbars, dish)
  28 Trade Chip        head  (chip with gold pins, coin)
  29 Hive Mind         head  (crown of linked magenta nodes)
  30 Botanist's Bonnet head  (sprout, then straw hat with flowers)
  31 Headlamp          head  (lamp with beam; Legendary mini sun)
  32 Dome Brain        head  (little glass geodesic dome)
  33 Weather Vane      head  (vane arrow, cups; Legendary storm cloud)
  34 X-Ray Specs       eyes  (green lens goggles)
  35 Night Vision      eyes  (protruding green-tipped tubes)
  36 Threat Lens       eyes  (red lens with a projected reticle)
  37 Appraisal Monocle eyes  (gold monocle on a chain)
  38 Zoom Goggles      eyes  (brass telescoping lens)
  39 Geo Visor         eyes  (visor bar projecting a holo grid)
  40 Extendo-Arms      arms  (jointed extra arm(s) with claws)
  41 Drill Arms        arms  (yellow housing, striped drill cone)
  42 Power Lifters     arms  (pauldron with hydraulic pistons)
  43 Shield Arm        arms  (buckler, kite shield; Legendary energy disc)
  44 Farm Hands        arms  (trowel, sickle, rake)
  45 Welder Arms       arms  (torch with blue flame, gas tank)
  46 Bolt Enhancements hands (emitter rings and prongs)
  47 Midas Mitts       hands (gold mitt / gauntlet)
  48 Cryo Palms        hands (ice mitt with crystal spikes)
  49 Boom Mitts        hands (red mitt, fused bomb knuckle)
  50 Green Fingers     hands (leafy glove, sprouts, flower)
  51 Grapple Glove     hands (hook, grapnel, cable reel)
  52 Healing Hands     hands (white glove with a green cross)

=========================================================================
4. assets/sprites/mod_icons.png   256x64
=========================================================================
16x16 icons, 16 columns x 4 rows, index = set index (col = i % 16,
row = i // 16); indices 53-63 are transparent.  Each is the set's tier-II
attachment (a lower tier if that is larger than 12 px) centred on an opaque
dark rounded-square badge tinted by slot: feet red, legs blue, back moss
green, torso rust, head purple, eyes teal, arms gold, hands magenta.

=========================================================================
5. assets/sprites/robot.png, robot_accent.png   128x80 each
=========================================================================
16x16 frames, 8 columns x 5 rows; the hover drone is centred on x~8, its
shell centre on y~7-8 (it floats; it bobs +-1 px inside the frame).
    row 0  hover idle (4)
    row 1  working (4): tool arm jabbing forward with sparks
    row 2  carrying (4): crate clamped underneath, side thrusters
    row 3  charging (4): eye pulsing amber -> green, lightning glyph at
           the top right
    row 4  cols 0-1 out of power (dim, sagging 2 px lower, eye dark);
           cols 2-3 alarm (red eye, frame 2 with a red "!" at the right)
    cols 4-7 of every row are transparent.
  robot_accent.png has the identical layout and holds only the 1px stripe
  around the shell in neutral greys (>=150): multiply by the robot's colour
  and draw over robot.png.

=========================================================================
6. assets/sprites/creatures.png   192x288
=========================================================================
24x24 frames, 8 columns x 12 rows.  Walkers stand on y=23; flyers and
swimmers are centred on (12,12).
    row 0   driftmoth: fly (4)
    row 1   puffback: cols 0-3 walk, cols 4-7 graze
    row 2   skitter: cols 0-3 walk, cols 4-7 lunge (crouch, leap, full
            stretch, land)
    row 3   skitter: cols 0-3 death / curl-up (ends on its back, legs
            folded), cols 4-7 idle twitch
    row 4   webspinner: cols 0-3 walk, cols 4-7 spit (abdomen swings over
            the head; the glob leaves in col 6)
    row 5   cols 0-3 web projectile (spinning glob flying right),
            cols 4-7 web splat / hit (expanding, fading web)
    row 6   gloom leech: cols 0-3 swim / undulate, cols 4-7 latched
            (curled, pulsing, mouth to the right)
    row 7   burrow maw: cols 0-1 hidden (mound with a twitching feeler),
            cols 2-7 erupt (6); col 6 reaches the full 24 px height (its
            outline touches y=0)
    row 8   burrow maw: cols 0-3 bite loop, cols 4-7 retreat (col 7 is
            just the settling mound)
    row 9   cols 0-5 generic death puff (ichor burst, then fading spores);
            cols 6-7 transparent
    row 10  hit flashes, pure white silhouettes: col 0 skitter (of row 2
            col 0), col 1 webspinner (row 4 col 0), col 2 leech
            (row 6 col 0), col 3 maw (the emerged maw, row 8 col 0),
            col 4 puffback (row 1 col 0), col 5 driftmoth (row 0 col 0)
    row 11  transparent

   assets/sprites/brood_mother.png   256x144
64x48 frames, 4 columns x 3 rows, feet on y=47, facing right.
    row 0  walk (4)
    row 1  spawn (4): abdomen splits, eggs drop and land
    row 2  col 0 hurt as a white flash silhouette, col 1 hurt (rearing),
           col 2 death (collapsing), col 3 death (dead, deflated, legs
           curled, ichor pool)

=========================================================================
7. assets/sprites/workers.png   128x144
=========================================================================
16x24 frames, 8 columns x 6 rows, feet on y=23, centred on x~8.
    row 0  worker A (teal overalls, yellow hard hat): cols 0-5 walk (6),
           cols 6-7 idle (2)
    row 1  worker A: cols 0-3 work (hammering; spark in col 2),
           cols 4-7 carry (walking with a crate)
    row 2  worker B (rust overalls, moss shirt, red hair bun, darker
           skin): same as row 0
    row 3  worker B: same as row 1
    row 4  colonist A (pink dress, long blond hair): cols 0-3 walk,
           cols 4-7 wave / cheer (4-5 wave, 6 hop with both arms up,
           7 land)
    row 5  colonist B (blue shirt, dark trousers, grey hair): same as row 4

=========================================================================
8. assets/sprites/animals.png   128x48
=========================================================================
16x16 frames, 8 columns x 3 rows, feet on y=15 (fish centred on (8,8)).
    row 0  cluckbug: cols 0-3 idle, cols 4-7 peck
    row 1  milk grub: cols 0-3 idle (breathing), cols 4-7 wriggle
    row 2  pond fish (~8x5): cols 0-3 swim, amber variant; cols 4-7 swim,
           teal glow variant"""
import json
import math
import os
import struct
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SPRITES = os.path.join(ROOT, "assets", "sprites")


# --------------------------------------------------------------------------
# Palette
# --------------------------------------------------------------------------
def hexc(h, a=255):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


def ramp(*hs):
    return [hexc(h) for h in hs]


CLEAR = (0, 0, 0, 0)
INK = hexc("12101a")
WHITE = hexc("ffffff")

INDIGO = ramp("1d1a2e", "2a2740", "3b3857", "565378")
TEAL = ramp("163a3c", "245557", "3a7d78", "62a89c")
RUST = ramp("5a2a22", "8f4a2f", "c2703d", "e09a5a")
MOSS = ramp("2c4a2a", "46692f", "6f8f3a", "a3b85a")
BONE = ramp("6e6a5a", "a39d84", "cfc7a8", "ece6cc")
METAL = ramp("2b3040", "474f63", "6d778c", "9aa5b8")
GOLD = ramp("6e4f1f", "b08636", "e3c069", "fff0a8")
BLUE = ramp("1c2f5c", "2c5a9c", "4f93d6", "9fd4f5")
RED = ramp("5c1626", "a22633", "e0463e", "ff8a6a")
PURPLE = ramp("2e1a45", "5a3080", "9156c0", "c9a0f0")
ICE = ramp("2d5a78", "5fa3c4", "a8dcee", "e8fbff")
BROWN = ramp("3a2418", "5e3d26", "8a6038", "b88a55")
STRAW = ramp("7a5a26", "b08a3c", "d9bb62", "f2e09a")
PALE = ramp("5e5560", "948894", "c9bcc0", "efe6e2")
FLESH = ramp("4a2638", "7a3f55", "b0647a", "e09aa6")

CYAN = hexc("5ff2e6")
CYAN_D = hexc("2bb4c9")
LIME = hexc("b6f04a")
LIME_D = hexc("6fb82e")
AMBER = hexc("ffc247")
AMBER_D = hexc("d9822b")
MAGENTA = hexc("f25fd0")
MAGENTA_D = hexc("a8358f")
HOT = hexc("ff5a4a")
GLASS = ramp("0e0c1c", "1a1836", "2c2a58", "4a4a8a")


def mix(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(4))


def alpha(c, a):
    return (c[0], c[1], c[2], a)


def darker(c, t=0.4):
    m = mix(c, INK, t)
    return (m[0], m[1], m[2], c[3])


def dark_ramp(r, t=0.32):
    return [darker(c, t) for c in r]


def grey(v):
    return (v, v, v, 255)


ACC = [grey(158), grey(206), grey(250), grey(255)]


def hash2(x, y, s=0):
    """Deterministic 0..1 hash."""
    n = (x * 374761393 + y * 668265263 + s * 2246822519) & 0xFFFFFFFF
    n = ((n ^ (n >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((n ^ (n >> 16)) & 0xFFFF) / 65535.0


# --------------------------------------------------------------------------
# Canvas
# --------------------------------------------------------------------------
class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.px = [CLEAR] * (w * h)
        self.tag = set()   # accent pixels
        self.lv = {}       # shade level of the last part drawn at a pixel
        self.oob = 0       # opaque pixels drawn outside the canvas
        self.oob_at = None
        self.clip_y = None # fills below this row are dropped (ground line)

    def inb(self, x, y):
        return 0 <= x < self.w and 0 <= y < self.h

    def get(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[y * self.w + x]
        return CLEAR

    def set(self, x, y, c):
        if self.clip_y is not None and y > self.clip_y:
            return
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y * self.w + x] = c
            self.tag.discard((x, y))
        elif c[3]:
            self.oob += 1
            self.oob_at = (x, y)

    def opaque(self, x, y):
        return self.get(x, y)[3] > 0

    def blend(self, x, y, c):
        if not self.inb(x, y):
            if c[3]:
                self.oob += 1
                self.oob_at = (x, y)
            return
        q = self.px[y * self.w + x]
        a = c[3] / 255.0
        if q[3] == 0 or c[3] == 255:
            self.px[y * self.w + x] = c
            return
        qa = q[3] / 255.0
        oa = a + qa * (1 - a)
        rgb = [int(round((c[i] * a + q[i] * qa * (1 - a)) / oa)) for i in range(3)]
        self.px[y * self.w + x] = (rgb[0], rgb[1], rgb[2], int(round(oa * 255)))

    def fill(self, pts, c):
        for (x, y) in pts:
            self.set(x, y, c)

    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)

    def line(self, x0, y0, x1, y1, c):
        for (x, y) in line_pts(x0, y0, x1, y1):
            self.set(x, y, c)

    def part(self, pts, rmp, sep=True, acc=False):
        """Fill a mask with a (dark, mid, light[, hi]) ramp lit from the
        top-left; optionally cast a 1px contact shadow on what is below."""
        S = pts if isinstance(pts, set) else set(pts)
        if sep:
            for (x, y) in S:
                for q in ((x + 1, y), (x, y + 1)):
                    if q not in S and self.opaque(*q):
                        self.px[q[1] * self.w + q[0]] = darker(self.get(*q), 0.3)
                        if q in self.lv:
                            self.lv[q] = 0
        for (x, y) in S:
            up = (x, y - 1) not in S
            lf = (x - 1, y) not in S
            dn = (x, y + 1) not in S
            rt = (x + 1, y) not in S
            if (dn or rt) and not (up and lf):
                l = 0
            elif up or lf:
                l = 2
            else:
                l = 1
            if len(rmp) > 3 and up and lf:
                l = 3
            self.set(x, y, rmp[l])
            self.lv[(x, y)] = l
            if acc and self.inb(x, y):
                self.tag.add((x, y))

    def recolor(self, pts, rmp, acc=False):
        """Recolour pixels already drawn by part(), keeping the shade level."""
        for q in pts:
            if q in self.lv and self.inb(*q):
                self.set(q[0], q[1], rmp[min(self.lv[q], len(rmp) - 1)])
                if acc:
                    self.tag.add(q)

    def outline(self, c=INK, diag=False):
        add = []
        nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        if diag:
            nb += [(1, 1), (-1, 1), (1, -1), (-1, -1)]
        for y in range(-1, self.h + 1):
            for x in range(-1, self.w + 1):
                if self.get(x, y)[3]:
                    continue
                for dx, dy in nb:
                    if self.get(x + dx, y + dy)[3] == 255:
                        add.append((x, y))
                        break
        clip, self.clip_y = self.clip_y, None
        for x, y in add:
            if y < self.h:
                self.set(x, y, c)
        self.clip_y = clip

    def halo(self, c, a=90):
        """1px soft glow around everything opaque."""
        add = []
        for y in range(-1, self.h + 1):
            for x in range(-1, self.w + 1):
                if self.get(x, y)[3]:
                    continue
                n = 0
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if self.get(x + dx, y + dy)[3] == 255:
                        n += 1
                if n:
                    add.append((x, y, min(255, a + 25 * (n - 1))))
        for x, y, aa in add:
            if self.inb(x, y):
                self.px[y * self.w + x] = alpha(c, aa)

    def blit(self, o, dx, dy, flip=False):
        for y in range(o.h):
            for x in range(o.w):
                p = o.px[y * o.w + x]
                if p[3]:
                    xx = dx + (o.w - 1 - x if flip else x)
                    if p[3] == 255:
                        self.set(xx, dy + y, p)
                    else:
                        self.blend(xx, dy + y, p)

    def silhouette(self, c=WHITE):
        self.px = [c if p[3] >= 128 else CLEAR for p in self.px]
        self.tag.clear()

    def bbox(self):
        xs = [i % self.w for i, p in enumerate(self.px) if p[3]]
        ys = [i // self.w for i, p in enumerate(self.px) if p[3]]
        if not xs:
            return None
        return (min(xs), min(ys), max(xs), max(ys))

    def accent_layer(self):
        o = Canvas(self.w, self.h)
        for (x, y) in self.tag:
            p = self.get(x, y)
            v = max(150, min(255, p[0]))
            o.px[y * self.w + x] = grey(v)
        return o

    def scaled(self, k):
        o = Canvas(self.w * k, self.h * k)
        for y in range(o.h):
            row = y // k * self.w
            for x in range(o.w):
                o.px[y * o.w + x] = self.px[row + x // k]
        return o


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


# ---- masks ----------------------------------------------------------------
def m_rect(x0, y0, x1, y1):
    return {(x, y) for y in range(y0, y1 + 1) for x in range(x0, x1 + 1)}


def m_ellipse(cx, cy, rx, ry):
    pts = set()
    for y in range(int(math.floor(cy - ry)) - 1, int(math.ceil(cy + ry)) + 1):
        for x in range(int(math.floor(cx - rx)) - 1, int(math.ceil(cx + rx)) + 1):
            if ((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1.0:
                pts.add((x, y))
    return pts


def m_disc(c, r):
    return m_ellipse(c[0], c[1], r, r)


def m_capsule(a, b, r):
    pts = set()
    x0 = int(math.floor(min(a[0], b[0]) - r)) - 1
    x1 = int(math.ceil(max(a[0], b[0]) + r)) + 1
    y0 = int(math.floor(min(a[1], b[1]) - r)) - 1
    y1 = int(math.ceil(max(a[1], b[1]) + r)) + 1
    dx, dy = b[0] - a[0], b[1] - a[1]
    L2 = dx * dx + dy * dy
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            px, py = x + 0.5 - a[0], y + 0.5 - a[1]
            t = 0.0 if L2 == 0 else max(0.0, min(1.0, (px * dx + py * dy) / L2))
            ex, ey = px - t * dx, py - t * dy
            if ex * ex + ey * ey <= r * r:
                pts.add((x, y))
    return pts


def m_poly(poly):
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    pts = set()
    for y in range(int(math.floor(min(ys))) - 1, int(math.ceil(max(ys))) + 1):
        for x in range(int(math.floor(min(xs))) - 1, int(math.ceil(max(xs))) + 1):
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


def seg_t(p, a, b):
    """Projection parameter (0..1) of pixel centre p on segment a-b."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    L2 = dx * dx + dy * dy or 1.0
    return ((p[0] + 0.5 - a[0]) * dx + (p[1] + 0.5 - a[1]) * dy) / L2


# --------------------------------------------------------------------------
# PNG writer
# --------------------------------------------------------------------------
def write_png(path, cv):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    raw = bytearray()
    for y in range(cv.h):
        raw.append(0)
        for x in range(cv.w):
            raw.extend(cv.px[y * cv.w + x])

    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    data = b"\x89PNG\r\n\x1a\n"
    data += chunk(b"IHDR", struct.pack(">IIBBBBB", cv.w, cv.h, 8, 6, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    data += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(data)


PREVIEW = None
WARN = []


def save(name, cv, fw=None, fh=None, edges="tlr"):
    """Write a sheet; optionally check that no frame touches its cell edges."""
    write_png(os.path.join(SPRITES, name), cv)
    if PREVIEW:
        bg = Canvas(cv.w, cv.h)
        for i in range(cv.w * cv.h):
            x, y = i % cv.w, i // cv.w
            chk = ((x // 8) + (y // 8)) % 2
            if fw and (x % fw == 0 or y % fh == 0):
                bg.px[i] = hexc("3a2f4a")
            else:
                bg.px[i] = hexc("2b3138") if chk else hexc("353c44")
        bg.blit(cv, 0, 0)
        write_png(os.path.join(PREVIEW, name), bg.scaled(4))
    if fw and edges:
        for fy in range(cv.h // fh):
            for fx in range(cv.w // fw):
                bad = False
                for i in range(fw):
                    if "t" in edges and cv.get(fx * fw + i, fy * fh)[3]:
                        bad = True
                    if "b" in edges and cv.get(fx * fw + i, fy * fh + fh - 1)[3]:
                        bad = True
                for i in range(fh):
                    if "l" in edges and cv.get(fx * fw, fy * fh + i)[3]:
                        bad = True
                    if "r" in edges and cv.get(fx * fw + fw - 1, fy * fh + i)[3]:
                        bad = True
                if bad:
                    WARN.append("%s: frame (col %d,row %d) touches cell edge" % (name, fx, fy))
    print("wrote %-28s %dx%d" % (name, cv.w, cv.h))


def put_frame(sheet, fr, col, row, name=""):
    if fr.oob:
        WARN.append("%s: frame (col %d,row %d) drew %d px out of bounds" % (name, col, row, fr.oob))
    sheet.blit(fr, col * fr.w, row * fr.h)


# --------------------------------------------------------------------------
# Humanoid rig (player body / suit, workers, colonists)
# --------------------------------------------------------------------------
def vadd(a, b, k=1.0):
    return (a[0] + b[0] * k, a[1] + b[1] * k)


def vdir_down(deg):
    """Limb direction: 0 = straight down, +90 = forward (right)."""
    r = math.radians(deg)
    return (math.sin(r), math.cos(r))


def vdir_up(deg):
    """Spine direction: 0 = straight up, + leans forward (right)."""
    r = math.radians(deg)
    return (math.sin(r), -math.cos(r))


def ik2(o, t, l1, l2, sign):
    dx, dy = t[0] - o[0], t[1] - o[1]
    d = math.hypot(dx, dy) or 1e-6
    dc = max(abs(l1 - l2) + 0.01, min(l1 + l2 - 0.01, d))
    ux, uy = dx / d, dy / d
    a = (l1 * l1 - l2 * l2 + dc * dc) / (2 * dc)
    h = math.sqrt(max(0.0, l1 * l1 - a * a))
    j = (o[0] + ux * a - uy * h * sign, o[1] + uy * a + ux * h * sign)
    e = (o[0] + ux * dc, o[1] + uy * dc)
    return j, e


PLAYER_DIMS = dict(hip=(12.0, 21.8), thigh=4.2, shin=4.2, torso=5.6, uarm=3.2, larm=3.2,
                   head_up=3.6, head_fwd=0.2, ground=30.0, r_helmet=4.1, r_suit_torso=3.45, xo=0.5)
WORKER_DIMS = dict(hip=(8.0, 15.6), thigh=3.3, shin=3.3, torso=4.6, uarm=2.7, larm=2.6,
                   head_up=3.1, head_fwd=0.3, ground=22.0, r_helmet=3.0, r_suit_torso=2.45)


def solve(pose, D):
    """Pose -> joint positions (floats, frame pixel coordinates)."""
    sx, sy = pose.get("shift", (0, 0))
    R = pose.get("rot", 0.0)
    A = R + pose.get("lean", 0.0)
    H = A + pose.get("head", 0.0)
    hip = pose.get("hip", D["hip"])
    sx += D.get("xo", 0.0)
    hip = (hip[0] + sx, hip[1] + sy)
    up = vdir_up(A)
    fwd = (-up[1], up[0])
    J = {"hip": hip, "up": up, "fwd": fwd}
    D = dict(D)
    D["torso"] = D["torso"] + pose.get("squash", 0.0)
    chest = vadd(hip, up, D["torso"])
    uph = vdir_up(H)
    fwh = (-uph[1], uph[0])
    J["chest"] = chest
    J["tl"] = D["torso"]
    J["uph"], J["fwh"] = uph, fwh
    J["head"] = vadd(vadd(chest, uph, D["head_up"]), fwh, D["head_fwd"])

    def limb(spec, o, l1, l2, knee):
        if spec[0] == "ik":
            t = (spec[1][0] + sx, spec[1][1] + sy)
            sign = spec[2] if len(spec) > 2 else (-1 if knee else 1)
            return ik2(o, t, l1, l2, sign)
        a1 = spec[1] - R
        a2 = a1 - spec[2] if knee else a1 + spec[2]
        j = vadd(o, vdir_down(a1), l1)
        return j, vadd(j, vdir_down(a2), l2)

    for k, off in (("F", 0.4), ("B", -0.4)):
        o = vadd(hip, fwd, off)
        J["hip" + k] = o
        J["knee" + k], J["foot" + k] = limb(pose.get("leg" + k, ("a", 0, 0)), o, D["thigh"], D["shin"], True)
        # foot direction: perpendicular to the shin, pointing forward
        kx, ky = J["foot" + k][0] - J["knee" + k][0], J["foot" + k][1] - J["knee" + k][1]
        n = math.hypot(kx, ky) or 1.0
        td = (ky / n, -kx / n)
        point = pose.get("toes", 0.0)   # 0 flat .. 1 pointed along the shin
        td = (td[0] * (1 - point) + kx / n * point, td[1] * (1 - point) + ky / n * point)
        n = math.hypot(*td) or 1.0
        J["toe" + k] = (td[0] / n, td[1] / n)
        n = math.hypot(kx, ky) or 1.0
        J["shinup" + k] = (-kx / n, -ky / n)
    for k, off in (("F", 0.2), ("B", -0.3)):
        o = vadd(vadd(chest, up, -0.9), fwd, off)
        J["sh" + k] = o
        J["elbow" + k], J["hand" + k] = limb(pose.get("arm" + k, ("a", 0, 0)), o, D["uarm"], D["larm"], False)
    if pose.get("ground"):
        low = max(J["footF"][1], J["footB"][1])
        dy = D["ground"] + sy - low
        for k, v in list(J.items()):
            if k not in ("up", "fwd", "uph", "fwh", "toeF", "toeB", "shinupF", "shinupB", "tl"):
                J[k] = (v[0], v[1] + dy)
    if not pose.get("lying") and abs(R) < 20:
        for k in "FB":           # planted feet lie flat on the ground
            if J["foot" + k][1] >= D["ground"] + sy - 0.7:
                J["toe" + k] = (1.0, 0.0)
                J["shinup" + k] = (0.0, -1.0)
    J["tool"] = pose.get("tool")
    J["ink_arm"] = pose.get("ink_arm")
    return J


def fl(v):
    return int(math.floor(v))


def anchors_of(J, D):
    up, fwd = J["up"], J["fwd"]
    a = {}
    a["head"] = vadd(J["head"], J["uph"], D["r_helmet"] - 0.4)
    a["eyes"] = vadd(J["head"], J["fwh"], D["r_helmet"] * 0.5)
    a["torso"] = vadd(J["hip"], up, J["tl"] * 0.54)
    a["back"] = vadd(vadd(J["hip"], up, J["tl"] * 0.68), fwd, -(D["r_suit_torso"] + 1.6))
    a["arms"] = J["shF"]
    a["hands"] = J["handF"]
    a["legs"] = ((J["kneeF"][0] + J["kneeB"][0]) / 2, (J["kneeF"][1] + J["kneeB"][1]) / 2)
    a["feet"] = ((J["footF"][0] + J["footB"][0]) / 2 + 0.3, (J["footF"][1] + J["footB"][1]) / 2 + 1.0)
    for k in "FB":
        n = "front" if k == "F" else "back"
        a["foot_" + n] = (J["foot" + k][0] + 0.3, J["foot" + k][1] + 1.0)
        a["knee_" + n] = J["knee" + k]
    a["hand_back"] = J["handB"]
    return {k: [fl(v[0]), fl(v[1])] for k, v in a.items()}


SLOTS = ["head", "eyes", "torso", "back", "arms", "hands", "legs", "feet"]
EXTRA_ANCHORS = ["foot_front", "foot_back", "knee_front", "knee_back", "hand_back"]


def local_uv(p, o, fwd, up):
    dx, dy = p[0] + 0.5 - o[0], p[1] + 0.5 - o[1]
    return dx * fwd[0] + dy * fwd[1], dx * up[0] + dy * up[1]


def draw_limb(cv, a, j, b, r, rmp, bands=()):
    """Two-segment limb; bands = (segment 0/1, t0, t1, ramp, accent)."""
    m = [m_capsule(a, j, r), m_capsule(j, b, r)]
    cv.part(m[0] | m[1], rmp)
    ends = [(a, j), (j, b)]
    for (s, t0, t1, r2, acc) in bands:
        pts = [p for p in m[s] if t0 <= seg_t(p, *ends[s]) <= t1 and (s == 1 or p not in m[1] or True)]
        if s == 0:
            pts = [p for p in pts if p not in m[1] or seg_t(p, *ends[1]) < 0]
        cv.recolor(pts, r2, acc)
    return m


def draw_tool(cv, hand, deg, vib=0):
    """The multitool: a stubby emitter held in the leading hand."""
    r = math.radians(deg)
    d = (math.cos(r), math.sin(r))
    a = vadd(hand, d, -0.6)
    b = vadd(hand, d, 2.7 - vib * 0.6)
    m = m_capsule(a, b, 0.95)
    cv.part(m, METAL[:3])
    cv.recolor([p for p in m if 0.5 <= seg_t(p, a, b) <= 0.72], RUST[1:])
    tip = [p for p in m if seg_t(p, a, b) > 0.86]
    for p in tip:
        cv.set(p[0], p[1], CYAN if (vib % 2 == 0) else WHITE)


def draw_human(cv, J, D, st):
    """Draw one figure from solved joints in style `st` (a dict)."""
    kind = st["kind"]
    up, fwd, hip = J["up"], J["fwd"], J["hip"]
    ra, rl, rt = st["r_arm"], st["r_leg"], st["r_torso"]

    def leg(k, back):
        rmp = st["leg_b"] if back else st["leg"]
        kn, ft, o = J["knee" + k], J["foot" + k], J["hip" + k]
        bands = []
        for (s, t0, t1, r2, acc) in st.get("leg_bands", ()):
            bands.append((s, t0, t1, dark_ramp(r2) if back and not acc else r2, acc))
        m = draw_limb(cv, o, kn, ft, rl, rmp, bands)
        if st.get("moss") and not back:
            for p in m[1]:
                t = seg_t(p, kn, ft)
                if 0.25 < t < 0.8 and cv.lv.get(p) == 0 and (p[0] + p[1]) % 2 == 0:
                    cv.set(p[0], p[1], mix(st["leg"][0], MOSS[2], 0.55))
        # boot
        td, su = J["toe" + k], J["shinup" + k]
        lift, rb, ln = st["boot"]
        a = vadd(vadd(ft, su, lift), td, -0.35)
        b = vadd(vadd(ft, su, lift), td, ln)
        bm = m_capsule(a, b, rb)
        cv.part(bm, dark_ramp(st["boot_ramp"]) if back else st["boot_ramp"], sep=False)

    def arm(k, back):
        rmp = st["arm_b"] if back else st["arm"]
        o, el, hd = J["sh" + k], J["elbow" + k], J["hand" + k]
        bands = []
        for (s, t0, t1, r2, acc) in st.get("arm_bands", ()):
            bands.append((s, t0, t1, dark_ramp(r2) if back and not acc else r2, acc))
        g = m_disc(hd, st["r_hand"])
        ink = max(st.get("ink_arm", 0.0), 0.62 if J.get("ink_arm") else 0.0)
        if not back and ink:
            full = m_capsule(o, el, ra) | m_capsule(el, hd, ra) | g
            for (x, y) in full:
                for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                    if q not in full and cv.opaque(*q):
                        cv.px[q[1] * cv.w + q[0]] = darker(cv.get(*q), ink)
                        cv.tag.discard(q)
        draw_limb(cv, o, el, hd, ra, rmp, bands)
        cv.part(g, dark_ramp(st["hand_ramp"]) if back else st["hand_ramp"], sep=False)

    arm("B", True)
    leg("B", True)
    leg("F", False)

    # torso
    ta, tb = vadd(hip, up, st.get("t0", 1.3)), vadd(hip, up, J["tl"] - 1.3)
    tm = m_capsule(ta, tb, rt)
    cv.part(tm, st["torso"])
    for p in tm:
        u, v = local_uv(p, hip, fwd, up)
        for (u0, u1, v0, v1, r2, acc) in st.get("torso_bands", ()):
            if u0 <= u <= u1 and v0 <= v <= v1:
                cv.recolor([p], r2, acc)
        if st.get("moss") and u < -1.2 and v < 2.6 and (p[0] + p[1]) % 2 == 0 and cv.lv.get(p, 1) < 2:
            cv.set(p[0], p[1], mix(st["torso"][0], MOSS[2], 0.5))
    if kind == "suit":
        # chest unit with a status light and a short hose to the belt
        for p in tm:
            u, v = local_uv(p, hip, fwd, up)
            if u > 0.9 and 2.3 <= v <= 4.7:
                cv.set(p[0], p[1], TEAL[1] if u < 2.4 else TEAL[0])
        lp = vadd(vadd(hip, up, 3.9), fwd, 1.6)
        if (fl(lp[0]), fl(lp[1])) in tm:
            cv.set(fl(lp[0]), fl(lp[1]), st.get("light", LIME))
        hp = vadd(vadd(hip, up, 1.9), fwd, 1.6)
        if (fl(hp[0]), fl(hp[1])) in tm:
            cv.set(fl(hp[0]), fl(hp[1]), INDIGO[1])
    if kind == "dress":
        # skirt flaring from the hips to the knees
        kc = ((J["kneeF"][0] + J["kneeB"][0]) / 2, (J["kneeF"][1] + J["kneeB"][1]) / 2 + 0.6)
        sk = m_capsule(vadd(hip, up, 0.5), kc, rt + 0.35)
        cv.part(sk, st["torso"], sep=False)

    # head
    hc, uph, fwh = J["head"], J["uph"], J["fwh"]
    if kind == "suit":
        R = D["r_helmet"]
        hm = m_disc(hc, R)
        if st.get("sep_head", True):
            cv.part(hm, BONE[1:], sep=True)
        for p in hm:
            dx, dy = (p[0] + 0.5 - hc[0]) / R, (p[1] + 0.5 - hc[1]) / R
            lgt = -(dx * 0.62 + dy * 0.72)
            u, v = local_uv(p, hc, fwh, uph)
            rr = math.hypot(u, v)
            if v < -R + 1.15:
                c = RUST[2] if lgt > -0.3 else RUST[1]          # collar seal
            elif u > 0.0 and -2.1 < v < 1.7 and rr < R - 0.75:
                c = GLASS[1] if (v > 0.2 or u < 1.2) else GLASS[0]
                if v > 0.9 and u < 1.6:
                    c = GLASS[2]
            else:
                c = BONE[3] if lgt > 0.55 else BONE[2] if lgt > 0.0 else BONE[1] if lgt > -0.6 else BONE[0]
                if -2.4 <= u <= -0.4 and v > -1.2:
                    c = ACC[3] if lgt > 0.55 else ACC[2] if lgt > 0.0 else ACC[1]
                    cv.set(p[0], p[1], c)
                    cv.tag.add(p)
                    continue
                if u < -2.6 and v < 0.5 and (p[0] + p[1]) % 2 == 0:
                    c = mix(c, MOSS[2], 0.5)
            cv.set(p[0], p[1], c)
        vp = vadd(vadd(hc, fwh, 1.9), uph, 0.7)
        cv.set(fl(vp[0]), fl(vp[1]), CYAN)
        vp2 = vadd(vadd(hc, fwh, 1.0), uph, -1.2)
        if st.get("visor_dim", True):
            cv.set(fl(vp2[0]), fl(vp2[1]), GLASS[3])
    else:
        R = st["r_head"]
        hm = m_disc(hc, R)
        cv.part(hm, st["skin"], sep=True)
        hair = st["hair"]
        style = st.get("hair_style", "short")
        for p in hm:
            u, v = local_uv(p, hc, fwh, uph)
            is_hair = v > R * 0.34 or (u < -R * 0.42 and v > -R * 0.5)
            if style == "bald":
                is_hair = False
            if style == "cap":
                is_hair = v > R * 0.2
            if is_hair:
                l = cv.lv.get(p, 1)
                cv.set(p[0], p[1], hair[min(l, 2)] if style != "cap" else st["cap"][min(l, 2)])
            elif style == "cap" and u < -R * 0.4 and v > -R * 0.5:
                cv.set(p[0], p[1], hair[cv.lv.get(p, 1)])
            elif u < -0.3 and cv.lv.get(p) == 2:
                cv.set(p[0], p[1], st["skin"][1])
        if style == "long":
            lm = m_capsule(vadd(vadd(hc, fwh, -R + 0.6), uph, 0.5), vadd(vadd(hc, fwh, -R + 0.3), uph, -R - 1.2), 1.0)
            cv.part(lm, hair, sep=False)
        if style == "bun":
            cv.part(m_disc(vadd(vadd(hc, fwh, -R + 0.1), uph, R * 0.55), 1.2), hair, sep=False)
        if style == "cap":
            bp = vadd(vadd(hc, fwh, R + 0.2), uph, R * 0.2 + 0.3)
            cv.set(fl(bp[0]), fl(bp[1]), st["cap"][1])
        ep = vadd(vadd(hc, fwh, R * 0.5), uph, 0.0)
        cv.set(fl(ep[0]), fl(ep[1]), st.get("eye", INK))

    if kind == "body":
        # neckerchief: accent band at the neck with a small knot at the front
        na, nb = vadd(vadd(hip, up, J["tl"] - 0.1), fwd, -1.4), vadd(vadd(hip, up, J["tl"] - 0.3), fwd, 1.6)
        nm = m_capsule(na, nb, 0.75)
        kn = vadd(vadd(hip, up, J["tl"] - 1.3), fwd, 1.5)
        nm.add((fl(kn[0]), fl(kn[1])))
        cv.part(nm, ACC, sep=False, acc=True)

    arm("F", False)
    if J["tool"] is not None:
        draw_tool(cv, J["handF"], J["tool"][0], J["tool"][1])


SUIT = ramp("8d8876", "c4bca0", "e6e0c6")
SUIT_B = ramp("595764", "8a8678", "a9a38d")
SUIT_T = ramp("77766a", "a7a28c", "c9c2a6")
UNDER = ramp("1b3238", "2b4c52", "3f6a6c")
UNDER_B = ramp("141f2a", "1d3138", "294448")
SKIN_A = ramp("9a5f4a", "cf9070", "eab896")
SKIN_B = ramp("4f3226", "7a4e38", "a06c4c")
SKIN_C = ramp("8a6a48", "b89264", "dab88a")
HAIR_DK = ramp("1c1622", "2e2436", "453850")
HAIR_RED = ramp("5a2418", "8f3f22", "c2622c")
HAIR_GREY = ramp("5a5a66", "8a8a96", "b8b8c2")
HAIR_BLOND = ramp("7a5a26", "b08a3c", "d9bb62")

STYLE_BODY = dict(
    kind="body", r_arm=1.0, r_leg=1.05, r_torso=2.45, r_hand=1.0, r_head=3.0,
    arm=UNDER, arm_b=UNDER_B, leg=UNDER, leg_b=UNDER_B, torso=UNDER,
    hand_ramp=INDIGO[1:], boot=(0.0, 1.0, 1.5), boot_ramp=INDIGO[1:],
    skin=SKIN_A, hair=HAIR_DK, hair_style="short",
    arm_bands=[(0, -1.0, 0.3, RUST[:3], False), (1, 0.55, 0.8, RUST[:3], False)],
    leg_bands=[(0, 0.75, 2.0, RUST[:3], False), (1, 0.72, 0.9, RUST[:3], False)],
    torso_bands=[(-9, 9, 0.2, 1.3, ACC, True), (-9, -1.3, 1.3, 9, UNDER_B[1:] + [UNDER[1]], False)],
)
STYLE_SUIT = dict(
    kind="suit", r_arm=1.5, r_leg=1.5, r_torso=3.45, r_hand=1.45, r_head=4.1,
    arm=SUIT, arm_b=SUIT_B, leg=SUIT, leg_b=SUIT_B, torso=SUIT_T,
    hand_ramp=RUST[:3], boot=(0.45, 1.45, 1.7), boot_ramp=METAL[:3], moss=True,
    arm_bands=[(0, -1.0, 0.34, ACC, True), (1, 0.3, 0.52, ACC, True), (1, 0.72, 2.0, RUST[:3], False)],
    leg_bands=[(1, 0.6, 0.8, RUST[:3], False)], ink_arm=0.45,
    torso_bands=[(-9, 9, 0.0, 1.1, TEAL[:3], False)],
)


def render_human(pose, D, st, w, h):
    cv = Canvas(w, h)
    if not pose.get("lying"):
        cv.clip_y = h - 2
    J = solve(pose, D)
    draw_human(cv, J, D, st)
    cv.outline(INK)
    return cv, J


def fit_pose(pose, D, st, w, h, cx=True, ground=False, cy=None):
    """Shift a pose so the rendered figure is centred / grounded in the frame."""
    big = Canvas(w + 32, h + 32)
    p2 = dict(pose)
    p2["shift"] = (16, 16)
    J = solve(p2, D)
    draw_human(big, J, D, st)
    big.outline(INK)
    x0, y0, x1, y1 = big.bbox()
    sx = sy = 0
    if cx:
        sx = int(round((w - 1) / 2.0 - ((x0 + x1) / 2.0 - 16)))
    if ground:
        sy = (h - 1) - (y1 - 16)
    elif cy is not None:
        sy = int(round(cy - ((y0 + y1) / 2.0 - 16)))
    p3 = dict(pose)
    p3["shift"] = (sx, sy)
    return p3


# ---- player poses ----------------------------------------------------------
def run_leg(p, amp=38.0, lift=62.0):
    p = p % 1.0
    th = amp * math.cos(2 * math.pi * p)
    if p < 0.5:
        kn = 8 + 14 * math.sin(math.pi * p / 0.5)
    else:
        kn = 8 + lift * math.sin(math.pi * (p - 0.5) / 0.5)
    return ("a", th, kn)


def player_poses():
    """rows -> list of (pose, flags); flags: 'flash', 'streak', fit options."""
    R = [[] for _ in range(12)]
    # 0 idle
    for i in range(4):
        dy = [0, 0, 1, 1][i]
        sw = [0, 3, 6, 3][i]
        R[0].append(dict(hip=(12.0, 21.8), squash=-dy, legF=("ik", (13.5, 30.0)), legB=("ik", (10.5, 30.0)),
                         armF=("a", -10 + sw, 26), armB=("a", -24 - sw, 14), head=0))
    # 1 run
    for i in range(8):
        p = i / 8.0
        R[1].append(dict(hip=(12.0, 21.0), lean=9, ground=True,
                         legF=run_leg(p), legB=run_leg(p + 0.5),
                         armF=("a", -40 * math.cos(2 * math.pi * p), 55),
                         armB=("a", 40 * math.cos(2 * math.pi * p), 55)))
    # 2 jump (rise, apex), fall x2
    R[2].append(dict(hip=(12.0, 21.0), lean=4, legF=("a", 48, 75), legB=("a", -12, 22),
                     armF=("a", 150, 15), armB=("a", -35, 20), toes=0.5))
    R[2].append(dict(hip=(12.0, 20.4), lean=6, legF=("a", 55, 95), legB=("a", 22, 80),
                     armF=("a", 95, 20), armB=("a", -65, 15), toes=0.3))
    R[2].append(dict(hip=(12.0, 20.6), lean=-3, legF=("a", 16, 26), legB=("a", -12, 30),
                     armF=("a", 148, 12), armB=("a", -142, -14), toes=0.7))
    R[2].append(dict(hip=(12.0, 20.6), lean=-3, legF=("a", 8, 34), legB=("a", -4, 20),
                     armF=("a", 160, -8), armB=("a", -128, -20), toes=0.7))
    # 3 climb (wall on the right at x=16)
    for i in range(4):
        s = [0, 1, 0, -1][i]
        b = [0, 1, 0, 1][i]
        R[3].append(dict(hip=(10.6, 21.6 - 0.6 * b), lean=6, head=-12,
                         armF=("ik", (15.2, 14.6 - 2.6 * s)), armB=("ik", (15.2, 14.6 + 2.6 * s)),
                         legF=("ik", (14.0, 27.4 + 2.4 * s)), legB=("ik", (14.0, 27.4 - 2.4 * s)),
                         toes=0.0))
    # 4 swim
    for i in range(4):
        s = [0, 1, 0, -1][i]
        c = [1, 0, -1, 0][i]
        R[4].append(dict(hip=(11.0, 18.0), rot=38, head=-24, toes=0.9, fit=dict(cy=19.5),
                         armF=("a", 84 + 30 * c, 36 + 14 * s), armB=("a", 84 - 30 * c, 36 - 14 * s),
                         legF=("a", 16 + 18 * s, 44 + 14 * c), legB=("a", 16 - 18 * s, 44 - 14 * c)))
    # 5-7 reach forward / up / down
    for row, aim, lean in ((5, 0, 3), (6, -45, -4), (7, 48, 8)):
        r = math.radians(aim)
        d = (math.cos(r), math.sin(r))
        for i in range(4):
            ext = [3.9, 6.1, 5.4, 5.9][i]
            ln = lean + [0, 2, 1, 2][i]
            base = dict(hip=(11.0, 22.2), lean=ln, legF=("ik", (13.0, 30.0)), legB=("ik", (9.3, 30.0)),
                        armB=("a", -22, 30), head=(-10 if aim < 0 else 8 if aim > 0 else 0))
            J = solve(base, PLAYER_DIMS)
            sh = J["shF"]
            tgt = (sh[0] + d[0] * ext, sh[1] + d[1] * ext)
            base["armF"] = ("ik", tgt, 1 if aim <= 0 else -1)
            base["tool"] = (aim, [0, 0, 1, 2][i])
            base["ink_arm"] = aim != 0
            R[row].append(base)
    # 8 hurt x2, blackout x4
    R[8].append(dict(hip=(11.4, 22.0), lean=-14, head=-12, flash=True,
                     legF=("ik", (13.4, 30.0)), legB=("ik", (9.8, 30.0)),
                     armF=("a", 62, 34), armB=("a", -70, -18)))
    R[8].append(dict(hip=(11.6, 22.0), lean=-9, head=-8,
                     legF=("ik", (13.2, 30.0)), legB=("ik", (10.0, 30.0)),
                     armF=("a", 44, 40), armB=("a", -50, -10)))
    R[8].append(dict(hip=(11.8, 23.6), lean=-8, head=14,
                     legF=("ik", (13.4, 30.0)), legB=("ik", (10.4, 30.0)),
                     armF=("a", 12, 8), armB=("a", -10, 4)))
    R[8].append(dict(hip=(11.0, 25.6), rot=-28, head=20, fit=dict(cx=False, ground=True),
                     legF=("a", 62, 100), legB=("a", 40, 96),
                     armF=("a", -8, 6), armB=("a", -28, 4)))
    R[8].append(dict(hip=(13.0, 26.6), rot=-64, head=10, fit=dict(ground=True),
                     legF=("a", 78, 112), legB=("a", 60, 100),
                     armF=("a", 8, 6), armB=("a", -30, 0)))
    R[8].append(dict(hip=(14.6, 27.0), rot=-90, head=0, fit=dict(ground=True), lying=True,
                     legF=("ik", (16.2, 30.0)), legB=("ik", (17.4, 30.0)),
                     armF=("a", 2, 4), armB=("a", -14, 0)))
    # 9 sleep
    for i in range(2):
        R[9].append(dict(hip=(14.6, 27.0), rot=-90, head=0, fit=dict(ground=True), lying=True,
                         legF=("ik", (16.2, 30.0)), legB=("ik", (17.4, 30.0)),
                         armF=("a", 38 + 6 * i, 96), armB=("a", -10, 0), asleep=True, breath=i))
    # 10 fly / hover
    for i in range(4):
        dy = [0, -1, 0, 1][i]
        R[10].append(dict(hip=(12.2, 20.8 + dy), lean=5, toes=0.85,
                          legF=("a", -6 + 2 * dy, 10), legB=("a", -15 - 2 * dy, 14),
                          armF=("a", 26 - 3 * dy, 18), armB=("a", -30 + 3 * dy, 12)))
    # 11 glide x2, dash x2
    for i in range(2):
        R[11].append(dict(hip=(11.0, 20.0), rot=38, head=-24, toes=0.9, fit=dict(cy=19.0),
                          armF=("a", 96 + 6 * i, 8), armB=("a", -112 - 6 * i, -6),
                          legF=("a", 2 + 4 * i, 10), legB=("a", -8 - 3 * i, 12)))
    for i in range(2):
        R[11].append(dict(hip=(12.6, 22.0), rot=34, head=-22, ground=True, streak=i + 1, fit=dict(cx=False),
                          legF=("a", 62 - 10 * i, 28 + 30 * i), legB=("a", -18 + 6 * i, 18),
                          armF=("a", -58 + 8 * i, -12), armB=("a", -84 + 8 * i, -6)))
    return R


def gen_player():
    FW, FH = 24, 32
    sheets = {k: Canvas(192, 384) for k in ("body", "suit", "accent_body", "accent_suit")}
    rows = player_poses()
    anch = {s: [[None] * 8 for _ in range(12)] for s in SLOTS + EXTRA_ANCHORS}
    for r, poses in enumerate(rows):
        for c, pose in enumerate(poses):
            pose = dict(pose)
            fit = pose.pop("fit", None)
            if fit is not None:
                pose = fit_pose(pose, PLAYER_DIMS, STYLE_SUIT, FW, FH, **fit)
            layers = {}
            for name, st in (("body", STYLE_BODY), ("suit", STYLE_SUIT)):
                st = dict(st)
                if pose.get("asleep"):
                    st["eye"] = SKIN_A[0]
                cv, J = render_human(pose, PLAYER_DIMS, st, FW, FH)
                if pose.get("streak"):
                    k = pose["streak"]
                    bb = cv.bbox()
                    for n, yy in enumerate((bb[1] + 4, bb[1] + 9, bb[1] + 14, bb[1] + 19)):
                        x1 = bb[0] - 1 - (n + k) % 2
                        # find the silhouette's left edge on this row
                        xs = [x for x in range(FW) if cv.get(x, yy)[3]]
                        if xs:
                            x1 = min(xs) - 2
                        ln = 3 + (n * 2 + k) % 3
                        for q in range(ln):
                            x = x1 - q
                            if x >= 1:
                                cv.px[yy * FW + x] = alpha(mix(WHITE, CYAN, 0.4), max(40, 170 - q * 45))
                layers[name] = cv
            if pose.get("flash"):
                for cv in layers.values():
                    cv.silhouette(WHITE)
            # the suit must cover the body completely
            for i, p in enumerate(layers["body"].px):
                if p[3] and not layers["suit"].px[i][3] == 255:
                    if p[3] == 255:
                        WARN.append("player: suit does not cover body at frame (%d,%d) px %d,%d" % (c, r, i % FW, i // FW))
            for name in ("body", "suit"):
                put_frame(sheets[name], layers[name], c, r, "player_" + name)
                put_frame(sheets["accent_" + name], layers[name].accent_layer(), c, r)
            A = anchors_of(J, PLAYER_DIMS)
            for s in SLOTS + EXTRA_ANCHORS:
                x, y = A[s]
                anch[s][r][c] = [max(0, min(FW - 1, x)), max(0, min(FH - 1, y))]
    save("player_body.png", sheets["body"], FW, FH)
    save("player_suit.png", sheets["suit"], FW, FH)
    save("player_accent_body.png", sheets["accent_body"], FW, FH)
    save("player_accent_suit.png", sheets["accent_suit"], FW, FH)
    lines = ['{"frame": [24, 32], "cols": 8, "rows": 12,']
    for key, names in (("anchors", SLOTS), ("extra", EXTRA_ANCHORS)):
        lines.append(' "%s": {' % key)
        for si, s in enumerate(names):
            lines.append('  "%s": [' % s)
            for r in range(12):
                lines.append("    " + json.dumps(anch[s][r]) + ("," if r < 11 else ""))
            lines.append("  ]" + ("," if si < len(names) - 1 else ""))
        lines.append(" }" + ("," if key == "anchors" else ""))
    lines.append("}")
    path = os.path.join(SPRITES, "player_anchors.json")
    with open(path, "w") as f:
        f.write("\n".join(lines) + "\n")
    with open(path) as f:
        doc = json.load(f)
    assert doc["frame"] == [24, 32] and len(doc["anchors"]) == 8
    assert all(len(v) == 12 and all(len(r) == 8 for r in v) for v in doc["anchors"].values())
    print("wrote player_anchors.json")
    return sheets, anch



# --------------------------------------------------------------------------
# Body-mod attachments
# --------------------------------------------------------------------------
class G:
    """A 16x16 attachment cell.  Coordinates are relative to the anchor
    pixel (8,8): x forward (right), y down."""

    def __init__(self, t, f, rmp, glow, glow2):
        self.cv = Canvas(16, 16)
        self.t, self.f, self.L = t, f, t == 3
        self.D, self.R, self.H = rmp[0], rmp[1:], rmp[3]
        self.G, self.G2 = glow, glow2
        self.post = []

    def px(self, x, y, c):
        self.cv.set(8 + x, 8 + y, c)

    def rc(self, x0, y0, x1, y1, c):
        self.cv.rect(8 + x0, 8 + y0, 8 + x1, 8 + y1, c)

    def hl(self, x0, x1, y, c):
        self.rc(x0, y, x1, y, c)

    def vl(self, x, y0, y1, c):
        self.rc(x, y0, x, y1, c)

    def ln(self, x0, y0, x1, y1, c):
        self.cv.line(8 + x0, 8 + y0, 8 + x1, 8 + y1, c)

    def box(self, x0, y0, x1, y1, r=None, sep=True):
        self.cv.part(m_rect(8 + x0, 8 + y0, 8 + x1, 8 + y1), r or self.R, sep)

    def ell(self, cx, cy, rx, ry=None, r=None, sep=True):
        self.cv.part(m_ellipse(8.5 + cx, 8.5 + cy, rx, ry or rx), r or self.R, sep)

    def cap(self, a, b, rad, r=None, sep=True):
        self.cv.part(m_capsule((8.5 + a[0], 8.5 + a[1]), (8.5 + b[0], 8.5 + b[1]), rad), r or self.R, sep)

    def poly(self, pts, r=None, sep=True):
        self.cv.part(m_poly([(8 + x, 8 + y) for x, y in pts]), r or self.R, sep)

    def glow(self, x, y, c=None, a=255):
        self.post.append((x, y, alpha(c or self.G, a)))

    def plus(self, x, y, c=None, a=255, post=True):
        for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
            if post:
                self.glow(x + dx, y + dy, c, a if (dx, dy) == (0, 0) else a * 2 // 3)
            else:
                self.px(x + dx, y + dy, c or self.G)

    def done(self):
        self.cv.outline(INK)
        if self.L:
            self.cv.halo(self.G, 105 if self.f == 0 else 60)
        for (x, y, c) in self.post:
            if c[3] == 255:
                self.cv.set(8 + x, 8 + y, c)
            else:
                self.cv.blend(8 + x, 8 + y, c)
        return self.cv


M3 = METAL[1:]
MD = [INDIGO[0], INDIGO[1], INDIGO[2]]


def coil_strut(g, x, y0, y1, light, dark):
    """A 3px-wide coil spring between two mounting eyes (rows y0..y1)."""
    g.px(x + 1, y0, M3[2]); g.px(x + 1, y1, M3[1])
    for y in range(y0 + 1, y1):
        if (y - y0) % 2:
            c = light(y) if callable(light) else light
            g.hl(x, x + 2, y, c); g.px(x + 2, y, dark if not g.L else c)
        else:
            g.px(x + 1, y, M3[0])


# ---- feet (boot: x -2..3, y -3..-1; ground outline row y=0) ---------------
def m_rocket(g):
    t, f = g.t, g.f
    if t >= 1:
        g.box(-1, -3 - (t >= 2), 3, -1)
        g.px(3, -1, M3[2])
    else:
        g.hl(-1, 3, -2, g.R[1]); g.px(3, -2, g.R[0]); g.px(-1, -2, g.R[2])
    w = [2, 2, 3, 3][t]
    h = [3, 3, 4, 4][t]
    g.box(-2 - w, -h, -2, -1, M3)
    g.hl(-2 - w, -2, -h + 1, g.R[1])
    g.px(-2 - w, -1, g.D)
    if t >= 2:
        g.px(-4, -h - 1, g.R[1]); g.px(-3, -h - 1, g.R[2]); g.px(-3, -h - 2, g.R[1])
        g.box(0, -6, 2, -5, M3); g.px(1, -6, g.G)
    fl_c = [AMBER, WHITE] if not g.L else [CYAN, WHITE]
    x0 = -3 - w
    n = [1, 2, 2, 2][t]
    for i in range(n + (1 - f)):
        g.glow(x0 - i, -1, fl_c[1] if i == 0 else g.G if i < n else g.G2, 255 if i < n else 150)
    if t >= 1:
        g.glow(x0, -2, g.G if f == 0 else g.G2)
    if g.L:
        g.glow(x0 - 1, -2 - f, g.G2, 200); g.glow(x0 - 1 - f, -3, WHITE, 140)
        g.px(0, -3, g.G); g.px(1, -3, g.G)


def m_sticky(g):
    t, f = g.t, g.f
    g.hl(-2, 3, -1, g.R[1]); g.px(-2, -1, g.R[2]); g.px(0, -1, g.D); g.px(2, -1, g.D)
    g.px(4, -1, g.R[2]); g.px(4, -2, g.R[1]); g.px(-3, -1, g.R[1]); g.px(-3, -2, g.R[2])
    if t >= 1:
        g.px(4, -2, g.R[1]); g.px(5, -1, g.R[2]); g.px(-3, -1, g.R[1]); g.px(-3, -2, g.R[2])
        g.px(1, -2, g.R[0])
    if t >= 2:
        g.px(5, -3, g.R[2]); g.px(5, -2, g.R[1]); g.px(6, -1, g.H); g.px(-4, -1, g.R[2])
        g.px(-4, -3, g.R[1]); g.px(-3, -3, g.R[1]); g.px(-1, -3, g.R[0]); g.px(2, -3, g.R[1]); g.px(2, -2, g.R[2])
    if g.L:
        for i, x in enumerate((-4, -1, 2, 5)):
            g.glow(x, -4 - (i + f) % 2, g.G, 220)
            g.glow(x, -3, g.G2)
        g.px(0, -1, g.G); g.px(2, -1, g.G)
    elif t >= 1:
        g.glow(3, -2 - f, g.G, 200)


def m_spring(g):
    t, f = g.t, g.f
    lit = (lambda y: M3[2]) if not g.L else (lambda y: g.G if ((y + f) // 2) % 2 else WHITE)
    h = [4, 5, 5, 5][t]
    coil_strut(g, -6, -1 - h, -1, lit, M3[1])
    g.box(-3, -4, -1, -1, g.R); g.px(-3, -1 - h + 1, g.R[1]) if h > 4 else None
    g.px(-2, -3, g.D)
    if t >= 1:
        g.box(2, -2, 3, -1, g.R)
    if t >= 2:
        coil_strut(g, 4, -6, -1, lit, M3[1])
        g.px(3, -3, g.R[1]); g.px(-3, -5, g.R[2]); g.px(-4, -6, g.R[1])
    if g.L:
        g.glow(-5, -3 - h + f, g.G, 180); g.glow(5, -8 + f, g.G, 160); g.glow(0, -5 + 2 * f, WHITE, 120)


def m_tread(g):
    t, f = g.t, g.f
    if t == 0:
        for x in (-2, 2):
            g.box(x, -2, x + 1, -1, MD); g.px(x + f, -2 + f, g.R[2])
        return
    h = [0, 2, 3, 3][t]
    g.box(-3, -h, 4, -1, MD)
    g.px(-3, -h, CLEAR); g.px(4, -h, CLEAR)
    for x in range(-3 + f, 5, 2):
        g.px(x, -1, INDIGO[3])
    for x in ((-2, 3) if t == 1 else (-2, 0, 3)):
        g.px(x, -2, g.G if g.L else g.R[2])
    if t >= 2:
        g.hl(-3, 3, -h - 1, g.R[1]); g.px(-3, -h - 1, g.R[2]); g.px(4, -h, g.R[0])
        g.px(-4, -h - 1, g.R[0]); g.px(-5, -h - 2, g.R[1])
    if g.L:
        for i in range(3):
            g.glow(-5 - i - f, -1 - i, g.G, 200 - i * 50)
        g.hl(-2, 2, -h - 1, g.G)


def m_flipper(g):
    t, f = g.t, g.f
    ln = [2, 3, 3, 3][t]
    g.box(0, -3, 3, -1)
    g.px(0, -3, g.H)
    g.hl(4, 3 + ln, -1, g.R[1]); g.hl(4, 4 + ln // 2, -2, g.R[2])
    g.px(3 + ln, -2, g.H if not g.L else g.G)
    if t >= 1:
        g.px(5, -1, g.R[0]); g.hl(-2, -1, -1, g.R[0]); g.px(-2, -2, g.R[1])
    if t >= 2:
        g.hl(4, 6, -3, g.R[1]); g.px(6, -3, g.H if not g.L else g.G); g.px(6, -4, g.R[2]); g.px(4, -2, g.R[0]); g.px(6, -1, g.R[0])
        g.box(-3, -4, -2, -2, g.R); g.px(-4, -5, g.R[2]); g.px(-4, -4, g.R[1]); g.px(-5, -6, g.H)
    if g.L:
        g.px(4, -1, g.G); g.px(-4, -5, g.G)
        g.glow(7, -5 - f, g.G, 190); g.glow(6, -7 + f, WHITE, 130); g.glow(-6, -8 + f, WHITE, 140); g.glow(2, -5 - f, g.G, 120)


def m_stomper(g):
    t, f = g.t, g.f
    x0, x1, h = [(-3, 4, 3), (-3, 4, 4), (-4, 5, 5), (-4, 5, 5)][t]
    g.box(x0, -h, x1, -1, M3)
    g.hl(x0, x1, -1, MD[1]); g.hl(x0, x1, -2, MD[2])
    g.px(x0 + 1, -h, M3[2]); g.px(x1 - 1, -h + 1, g.R[2])
    if t >= 1:
        g.box(x1 - 2, -3, x1, -2, g.R)
    else:
        g.px(x1, -3, CLEAR); g.px(x0, -3, CLEAR)
    if t >= 2:
        for x in range(x0, x1 + 1, 2):
            g.px(x, -1, AMBER_D)
        g.box(x0 - 1, -4, x0, -2, MD); g.px(x0 - 1, -5, M3[2])
        g.px(x0 + 2, -h, g.R[2])
    if g.L:
        g.px(x0 + 2, -3, g.G); g.px(x0 + 3, -4, g.G); g.px(x0 + 4, -3, g.G); g.px(x0 + 5, -2, g.G)
        for i, x in enumerate((x0 - 2, x1 + 1)):
            g.glow(x - (0 if i else f), -1 - (i + f) % 2, g.G, 230)
            g.glow(x + (i * 2 - 1), -2 - f, g.G2, 160)


# ---- legs (between the knees; legs x -3..3) -------------------------------
def m_piston(g):
    t, f = g.t, g.f
    ext = f if g.L else 0

    def piston(x, top, ln, rod):
        g.box(x, top, x + 2, top + ln - 1, M3)
        g.hl(x, x + 2, top, g.R[1]); g.px(x, top, g.R[2])
        g.hl(x, x + 2, top + ln - 1, g.R[0])
        g.vl(x + 1, top + ln, top + ln + rod - 1 + ext, M3[2] if not g.L else g.G)
        g.px(x + 1, top + ln + rod + ext, g.R[1])
    if t == 0:
        piston(-5, -3, 3, 3)
    elif t == 1:
        piston(-5, -5, 4, 4); g.px(-6, -4, M3[0]); g.px(-2, -3, g.R[1]); g.px(-2, 2, g.R[1])
    else:
        piston(-6, -5, 5, 4); piston(3, -3, 3, 3)
        g.ell(0, 0, 1.7, r=g.R); g.px(0, 0, M3[2] if not g.L else g.G)
        g.px(-7, -4, M3[0]); g.px(-7, -2, M3[0])
    if g.L:
        g.glow(-8, -5 + f, g.G, 200); g.glow(-8, -2 - f, WHITE, 130); g.glow(6, -5 + f, g.G, 160)


def m_cargo(g):
    t, f = g.t, g.f

    def pouch(x0, y0, x1, y1):
        g.box(x0, y0, x1, y1)
        g.hl(x0, x1, y0, g.H); g.hl(x0, x1, y0 + 1, g.R[0])
        g.px((x0 + x1) // 2, y0 + 1, AMBER)
    if t == 0:
        pouch(-1, -1, 2, 2)
    elif t == 1:
        pouch(-2, -2, 2, 2)
        g.hl(-2, 2, -3, g.D)
    else:
        g.hl(-3, 3, -5, g.D); g.vl(-3, -4, -3, g.D)
        pouch(-2, -3, 3, 1)
        pouch(-4, 1, -1, 4)
        pouch(1, 3, 3, 5)
    if g.L:
        g.rc(-1, -1, 2, 0, PURPLE[0])
        g.px(-1 + f * 2, -1, MAGENTA); g.px(1 - f, 0, g.G); g.px(2, -1 + f, WHITE)
        g.glow(0, -5 - f, g.G, 200); g.glow(2 - 3 * f, -6, WHITE, 150)


def m_thermal(g):
    t, f = g.t, g.f
    ys = [(0,), (-2, 1), (-3, 0, 3), (-3, 0, 3)][t]
    for i, y in enumerate(ys):
        g.hl(-3, 3, y, g.R[1]); g.px(-3, y, g.R[2]); g.px(3, y, g.R[0])
        hot = AMBER if not g.L else (WHITE if (i + f) % 2 == 0 else g.G)
        g.px(-1, y, hot); g.px(1, y, AMBER_D if not g.L else g.G)
    if t == 0:
        g.hl(-3, 3, 1, g.R[0]); g.box(2, -1, 3, 1, M3); g.px(2, 0, AMBER)
    if t >= 2:
        g.box(2, -2, 4, 2, M3); g.px(3, -1, g.R[2]); g.px(3, 1, g.R[2])
    if t >= 1:
        g.px(-3, ys[0] + 1, g.R[0])
    if g.L:
        for i, x in enumerate((-4, 0, 5)):
            g.glow(x, -5 - (i + f) % 2, g.G, 170); g.glow(x + (1 if f else -1), -6 - (i + f) % 2, g.G2, 100)


def m_roots(g):
    t, f = g.t, g.f
    V = g.R
    g.ln(-3, 3, 3, 1, V[0]); g.px(-3, 3, V[1]); g.px(0, 2, V[1])
    g.px(3, 0, LIME_D); g.px(4, -1, LIME)
    if t >= 1:
        g.ln(-3, 0, 3, -2, V[0]); g.px(-1, -1, V[1]); g.px(-4, -1, LIME_D); g.px(-4, -2, LIME)
    if t >= 2:
        g.ln(-3, -3, 2, -5, V[0]); g.px(0, -4, V[1])
        g.px(3, -6, LIME); g.px(2, -6, LIME_D); g.px(-4, 4, LIME_D)
        g.plus(3, -3, MAGENTA_D, post=False); g.px(3, -3, AMBER)
    if g.L:
        g.plus(-3, 2, g.G, post=False); g.px(-3, 2, WHITE if f == 0 else AMBER)
        g.px(3, -3, WHITE if f else AMBER)
        g.glow(5, -5 + f, g.G, 180); g.glow(-5, -4 - f, g.G, 150); g.glow(0, -7 + f, WHITE, 120)


def m_absorber(g):
    t, f = g.t, g.f
    lit = (lambda y: g.R[2]) if not g.L else (lambda y: g.G if ((y + f) // 2) % 2 else WHITE)
    if t == 0:
        coil_strut(g, -5, -3, 3, lit, g.R[1])
    elif t == 1:
        coil_strut(g, -5, -4, 4, lit, g.R[1]); g.ell(2, 0, 1.6, r=M3); g.px(2, 0, g.R[2])
    else:
        coil_strut(g, -6, -4, 4, lit, g.R[1]); coil_strut(g, 4, -3, 3, lit, g.R[1])
        g.ell(1, 0, 2.2, r=M3); g.px(1, 0, g.R[2] if not g.L else g.G); g.px(0, -1, M3[2])
    if g.L:
        g.glow(-8, -1 + 2 * f, g.G, 170); g.glow(7, -5 - f, g.G, 150); g.glow(-3, -6 + f, WHITE, 110)


def m_jackhammer(g):
    t, f = g.t, g.f
    v = f if (g.L or t >= 2) else 0
    hw = [1, 1, 2, 2][t]
    g.box(3 - hw, -1 - (t >= 1), 4, 0 + (t >= 2), g.R)
    if t >= 2:
        g.px(3 - hw, -2, INK); g.px(4 - hw + 1, -1, INK); g.px(4, 0, INK)
        g.box(1 - hw, -4, 2 - hw, -2, M3)
    ln = [2, 3, 4, 4][t]
    y0 = 1 + (t >= 2)
    g.vl(3, y0 + v, y0 + ln - 1 + v, M3[2] if not g.L else g.G)
    if t >= 2:
        g.vl(4, y0 + v, y0 + ln - 2 + v, M3[0])
    if g.L:
        g.glow(2 - f, y0 + ln + 1, g.G, 200); g.glow(5 + f, y0 + ln, WHITE, 160); g.glow(3, y0 + ln + v, WHITE)
        g.px(3, -1, g.G)


# ---- back (anchor just behind the back; the body covers x >= 1) -----------
def leaf(g, x, y, d=-1, big=False, c=None):
    c = c or LIME
    g.px(x, y, LIME_D); g.px(x + d, y - 1, c)
    if big:
        g.px(x + d * 2, y - 2, c); g.px(x + d, y - 2, LIME_D); g.px(x + d * 2, y - 1, LIME_D)


def m_o2(g):
    t, f = g.t, g.f
    if t == 0:
        g.cap((-2, -1), (-2, 2), 1.5); leaf(g, -2, -4)
        g.px(-2, -3, M3[1])
    elif t == 1:
        g.cap((-2, -2), (-2, 3), 2.0); g.hl(-4, -1, 0, g.D); g.px(-2, -5, M3[2])
        leaf(g, -3, -5, big=True); g.px(0, -3, RUST[2]); g.px(1, -4, RUST[2])
    else:
        gl = g.L
        r = GLASS[1:] if gl else g.R
        g.cap((-5, -2), (-5, 3), 1.5, r); g.cap((-2, -3), (-2, 3), 2.0, r)
        g.hl(-6, -1, 1, M3[0]); g.px(-5, -4, M3[2])
        g.px(-3, 3, AMBER); leaf(g, -2, -5, big=True); leaf(g, -5, -5, d=-1)
        g.px(0, -4, RUST[2]); g.px(1, -5, RUST[2])
        if gl:
            for y in range(-2, 4):
                g.px(-2, y, g.G if (y + f) % 2 else LIME_D)
            g.px(-3, 0 - f, g.G); g.px(-1, -1 + f, g.G); g.px(-5, 1 - f, g.G); g.px(-5, -1 + f, LIME_D)
            g.glow(-6, -7 - f, g.G, 180); g.glow(0, -7 + f, WHITE, 120)


def m_wings(g):
    t, f = g.t, g.f
    d = f if g.L else 0
    tip, tr = [((-4, -4), [(-4, -1), (-1, 1)]),
               ((-6, -5), [(-6, -1), (-3, 1), (-1, 2)]),
               ((-7, -7), [(-7, -2), (-5, 1), (-3, 4), (0, 3)]),
               ((-6, -6 + 2 * d), [(-6, -1 + 2 * d), (-4, 2 + d), (-2, 4), (0, 3)])][t]
    root = (1, -2)
    pts = [(1.5, -3.5), (tip[0] + 0.5, tip[1] + 0.5)]
    prev = tip
    for q in tr:
        mx, my = (prev[0] + q[0]) / 2.0, (prev[1] + q[1]) / 2.0
        k = 0.22
        pts.append((mx + (root[0] - mx) * k + 0.5, my + (root[1] - my) * k + 0.5))
        pts.append((q[0] + 0.5, q[1] + 0.5))
        prev = q
    pts.append((1.5, 1.5))
    mem = MOSS[1:] if not g.L else [TEAL[2], CYAN_D, CYAN]
    g.poly(pts, mem)
    vein = LIME_D if not g.L else TEAL[3]
    for q in tr:
        g.ln(0, -1, q[0], q[1], vein)
        g.px(q[0], q[1], LIME if not g.L else g.G)
    g.ln(0, -2, tip[0], tip[1], g.R[1]); g.px(tip[0], tip[1], g.R[2])
    g.px(0, -2, g.R[2]); g.px(0, -1, g.R[1])
    if g.L:
        g.glow(-8, 3 - 3 * d, WHITE, 160); g.glow(-3, -8 + 3 * d, g.G, 160); g.glow(-7, -8 + d, g.G, 110)


def m_turret(g):
    t, f = g.t, g.f
    r = [1.5, 2.0, 2.5, 2.5][t]
    cx, cy = [(-3, -3), (-3, -3), (-4, -3), (-4, -3)][t]
    g.hl(cx - 1, -1, 1, M3[0]); g.vl(cx, cy + 1, 0, M3[1]); g.px(cx + 1, 0, M3[0])
    if t >= 2:
        g.box(-6, 0, -1, 2, MD); g.px(-5, 1, AMBER); g.px(-3, 1, AMBER_D)
    bc = [METAL[3], METAL[2]] if not g.L else [WHITE, g.G]
    bl = [2, 3, 3, 3][t]
    bx, by = cx + (1 if t < 2 else 2), cy - (2 if t < 2 else 2)
    for i in range(bl):
        g.px(bx + i, by - i, bc[0]); g.px(bx + i + 1, by - i, bc[1])
    if t >= 2:
        g.px(bx + bl + 1, by - bl + 2, bc[1])
    g.ell(cx, cy, r, r=[METAL[1], METAL[2], METAL[3]])
    g.px(cx, cy, HOT if (f == 0 or t == 0) else RED[1])
    if t >= 2:
        g.px(cx - 2, cy + 1, HOT if f else RED[1]); g.px(cx + 1, cy + 2, g.R[0])
    if g.L:
        tx, ty = bx + bl, by - bl + 2
        if f == 0:
            g.glow(tx + 1, ty, WHITE); g.glow(tx + 2, ty - 1, g.G, 200); g.glow(tx, ty - 1, g.G, 160); g.glow(tx + 2, ty + 1, g.G, 160)
        else:
            g.glow(tx + 1, ty, g.G, 130)


def m_battery(g):
    t, f = g.t, g.f
    x0, y0, y1 = [(-3, -2, 2), (-4, -3, 3), (-6, -4, 4), (-6, -4, 4)][t]
    g.box(x0, y0, 0, y1, M3)
    g.px(x0 + 1, y0 - 1, AMBER); g.px(-1, y0 - 1, RUST[2])
    bars = [1, 2, 3, 3][t]
    for i in range(bars):
        y = y1 - 1 - i * 2
        lit = (not g.L) or ((i + f) % 3 != 2)
        g.hl(x0 + 1, -1, y, g.G if lit else g.G2)
    if t >= 2:
        g.vl(-3, y0 + 1, y1 - 1, M3[0]); g.px(-3, y0, M3[1])
        g.px(1, y0 + 1, RUST[1]); g.px(1, y0 + 2, RUST[2])
    if g.L:
        g.glow(x0 + 1 + f * 3, y0 - 2, WHITE); g.glow(x0 + 2 + f, y0 - 3, g.G, 200); g.glow(x0 + 3 - f * 2, y0 - 2, g.G, 170)


def m_sprinkler(g):
    t, f = g.t, g.f
    x0, y0 = [(-3, 0), (-4, -1), (-5, -2), (-5, -1)][t]
    g.box(x0, y0, 0, 3)
    g.rc(x0 + 1, y0 + 1, x0 + 1, 2, g.H)
    top = [-3, -5, -6, -2][t]
    g.vl(-2, top, y0 - 1, M3[1])
    if not g.L:
        g.hl(-3 - (t >= 1), -1 + (t >= 1), top, M3[2])
        heads = [(-3,)] if t == 0 else [(-4,), (0,)] if t == 1 else [(-4,), (0,), (-2,)]
        for i, (hx,) in enumerate(heads):
            g.glow(hx - (1 if hx < -2 else -1 if hx > -2 else 0), top - 1 - (i + f) % 2, g.G, 230)
            if t >= 2:
                g.glow(hx - (2 if hx < -2 else -2 if hx > -2 else 0), top + (i + f) % 2, g.G, 150)
    else:
        g.ell(-3, -5, 3.2, 1.6, [PALE[1], PALE[2], PALE[3]])
        g.ell(-4, -6, 1.6, 1.2, [PALE[2], PALE[3], WHITE], sep=False)
        for i, x in enumerate((-6, -4, -2, 0)):
            g.glow(x, -3 + (i + f) % 2, g.G, 230)
            g.glow(x, -1 + (i + f) % 2, g.G, 120)


def m_mule(g):
    t, f = g.t, g.f
    if t >= 1:
        g.vl(-1, -3, 4, BONE[2]); g.hl(-4 - (t >= 2), -1, 4, BONE[1]); g.px(-1, -3, BONE[3])
    if t >= 2:
        g.box(-5, 0, -2, 3); g.hl(-5, -2, 1, g.D); g.px(-3, 2, AMBER)
        g.hl(-4, -2, -1, MOSS[2]); g.px(-5, -1, MOSS[1])
    my = [-2, -5, -6, -5][t]
    w = 2 if t != 1 else 1
    mc = RED[2] if not g.L else g.G
    g.hl(-2 - w, -2 + w, my + 2, RED[1]); g.vl(-2 - w, my, my + 2, mc); g.vl(-2 + w, my, my + 2, RED[1] if not g.L else g.G2)
    g.px(-2 - w, my, WHITE); g.px(-2 + w, my, WHITE)
    if t == 0:
        g.vl(-2, 1, 3, BONE[1]); g.hl(-3, -1, 3, BONE[2]); g.px(-2, my + 3, BONE[2])
    if g.L:
        for i in range(3):
            g.glow(-2 - w + i * w, my - 2 - (i + f) % 2, g.G, 210)
        g.glow(-2, my - 2 - f, WHITE, 130)


def m_dock(g):
    t, f = g.t, g.f
    b = f if g.L else 0

    def drone(x, y):
        g.hl(x + 1, x + 2, y, BONE[3])
        g.box(x, y + 1, x + 3, y + 2, BONE[1:], sep=False)
        g.px(x + 3, y + 1, CYAN if not g.L else WHITE); g.px(x + 2, y + 1, GLASS[1]); g.px(x, y + 2, TEAL[1]); g.px(x + 1, y + 2, RUST[2])
        if f == 0:
            g.glow(x - 1, y - 1, M3[2], 230); g.glow(x, y - 1, M3[2]); g.glow(x + 3, y - 1, M3[2]); g.glow(x + 4, y - 1, M3[2], 230)
        else:
            g.glow(x + 1, y - 1, M3[2]); g.glow(x + 2, y - 1, M3[2])
    if t == 0:
        g.hl(-5, 0, 3, M3[1]); g.px(0, 4, M3[0])
        drone(-5, 0)
    elif t == 1:
        g.hl(-6, 0, 3, M3[1]); g.px(0, 4, M3[0]); g.px(-6, 2, g.G2); g.px(-1, 2, g.G2); g.vl(0, 0, 2, M3[0])
        drone(-5, 0)
    else:
        g.vl(0, -4, 4, M3[0]); g.hl(-6, 0, 4, M3[1]); g.hl(-6, 0, -2, M3[1])
        g.px(-6, 3, g.G2); g.px(-6, -3, g.G2)
        drone(-5, 1 - b); drone(-5, -5 - (1 - b if g.L else 0))
        if g.L:
            g.glow(-4, 4, g.G, 200 if b else 90); g.glow(-3, -2, g.G, 90 if b else 200)
            g.glow(-8, -1 + 2 * f, g.G, 150)


def m_beacon(g):
    t, f = g.t, g.f
    top = [-3, -5, -5, -5][t]
    g.vl(-2, top + 1, 3, M3[1]); g.hl(-3, -1, 3, M3[0]); g.px(-2, 3, M3[1])
    on = f == 0
    lc = g.G if (on or g.L) else g.G2
    g.plus(-2, top, lc, post=False)
    g.px(-2, top, WHITE if on else g.G)
    if t >= 1:
        g.hl(-3, -1, top + 3, M3[2])
    if t >= 2:
        g.box(-5, 0, -1, 3, MD); g.px(-4, 1, g.G2 if on else g.G); g.px(-2, 2, AMBER)
        g.ln(-5, -1, -6, -3, M3[2]); g.px(-6, -3, g.G if not on else g.G2)
    if t >= 1 and (on or g.L):
        rr = 3 if (not g.L or f == 0) else 4
        for dx, dy in ((-rr, -1), (-rr, 1), (rr - 1, -2), (-1, -rr + 1), (1, -rr + 1)):
            if top + dy >= -8 and -2 + dx <= 0:
                g.glow(-2 + dx, top + dy, g.G, 200 if rr == 3 else 120)
    if g.L:
        g.glow(-4, top - 2 + f, g.G, 150); g.glow(0, top - 2 - f + 1, WHITE, 130)


# ---- torso (chest: x -3..3, y -2..5) ---------------------------------------
def m_plating(g):
    t, f = g.t, g.f
    if t == 0:
        g.box(-2, -1, 2, 2); g.px(-1, 0, RUST[2]); g.px(1, 2, RUST[1])
        return
    g.box(-3, -2, 3, 1 + (t == 1) * 2)
    g.vl(0, -1, 0 + (t == 1) * 2, g.H)
    g.px(-2, -1, RUST[2]); g.px(2, -1, RUST[2])
    if t >= 2:
        g.box(-2, 2, 3, 3); g.box(-2, 4, 2, 5)
        g.px(-1, 2, RUST[2]); g.px(2, 4, RUST[1])
        g.hl(-3, 2, -3, g.R[1]); g.px(-4, -2, g.R[2]); g.px(4, -1, g.R[0]); g.px(4, 0, g.R[0])
    if g.L:
        g.vl(0, -2, 5, g.G if f == 0 else WHITE); g.hl(-2, 2, 1, g.G); g.px(-4, -3, g.G); g.px(4, -2, g.G)
        g.glow(5, 1 - f, g.G, 180); g.glow(-5, 2 + f, g.G, 140)


def m_filter(g):
    t, f = g.t, g.f

    def can(cx, cy, r):
        g.ell(cx, cy, r)
        if r >= 2:
            if f == 0 or not g.L:
                g.hl(cx - 1, cx + 1, cy, g.G if g.L else g.G2); g.vl(cx, cy - 1, cy + 1, g.G if g.L else g.G2)
            else:
                for d in (-1, 1):
                    g.px(cx + d, cy + d, g.G); g.px(cx + d, cy - d, g.G)
            g.px(cx, cy, g.D if not g.L else WHITE)
        else:
            g.px(cx, cy, g.G2); g.px(cx, cy - 1 + f * 0, g.D)
    if t == 0:
        can(1, 1, 2.0)
    elif t == 1:
        can(1, -1, 1.9); can(1, 3, 1.9); g.px(-1, 1, M3[1]); g.px(-1, 2, M3[1])
    else:
        can(1, 2, 2.6); g.box(-3, -1, -2, 3, M3); g.px(-3, -1, AMBER)
        g.px(2, -1, RUST[2]); g.px(3, -2, RUST[2]); g.px(-1, -1, RUST[1])
        if g.L:
            g.glow(5, 1 + f, g.G, 180); g.glow(6, 3 - f, g.G, 110); g.glow(4, 5, g.G2, 150)


def m_medi(g):
    t, f = g.t, g.f
    W = [PALE[1], PALE[3], WHITE]
    cc = HOT if not g.L else g.G
    g.box(-2, -2, 2 + (t >= 2), 2 + (t >= 1), W)
    g.vl(0, -1, 1 + (t >= 1), cc); g.hl(-1, 1, 0, cc)
    g.px(0, 0, WHITE if (g.L and f) else cc)
    if t >= 1:
        g.px(2, 3, LIME if f == 0 else LIME_D)
    if t >= 2:
        g.vl(-3, -1, 2, LIME_D); g.px(-3, -2, M3[2]); g.px(-3, 0, LIME)
        g.box(-1, 4, 2, 5, M3); g.px(0, 4, HOT if f == 0 else RED[1])
    if g.L:
        g.glow(4 + f, -3, g.G, 200)
        if f:
            g.plus(-4, -4, g.G, 150); g.hl(-1, 1, 1, g.G)
        else:
            g.glow(-4, -3, g.G, 200)


def m_reactor(g):
    t, f = g.t, g.f
    r = [2.3, 2.6, 3.5, 3.5][t]
    g.ell(0, 1, r, r=M3)
    core = [AMBER, AMBER] if not g.L else [WHITE, g.G]
    if t == 0:
        g.px(0, 1, core[f])
        return
    g.ell(0, 1, r - 1.0, r=[g.G2, g.G, core[0]], sep=False)
    g.px(0, 1, WHITE)
    if t >= 2:
        sp = [(-2, 1), (2, 1), (0, -1), (0, 3)] if (f == 0 or not g.L) else [(-1, 0), (1, 0), (-1, 2), (1, 2)]
        for (x, y) in sp:
            g.px(x, y, M3[0])
        g.px(-3, 4, RUST[2]); g.px(-4, 5, RUST[1]); g.px(3, 4, RUST[2])
    if g.L:
        g.glow(-5, 1, g.G, 120 + 80 * f); g.glow(5, 1, g.G, 200 - 80 * f); g.glow(0, -4, WHITE, 120 + 80 * f)


def m_camo(g):
    t, f = g.t, g.f
    cols = [MOSS[0], MOSS[1], MOSS[2], BROWN[1], MOSS[3]]
    r = [(2.7, 2.3), (3.3, 2.9), (3.8, 3.8), (3.8, 3.8)][t]
    cy = [1, 1, 1.5, 1.5][t]
    for (x, y) in m_ellipse(8.5, 8.5 + cy, r[0], r[1]):
        h = hash2(x // 2 + y, y // 2 - x, 7)
        c = cols[int(h * 4.99)]
        if g.L and (x + y + f) % 3 == 0:
            g.cv.set(x, y, alpha(g.G, 255))
        else:
            g.cv.set(x, y, c)
    if t >= 1:
        leaf(g, -3, -1, d=-1, c=MOSS[3]); leaf(g, 3, 0, d=1, c=MOSS[3])
    if t >= 2:
        leaf(g, -1, -3, d=-1, big=True, c=MOSS[3]); leaf(g, 3, 4, d=1, c=LIME_D); g.px(-4, 4, MOSS[2]); g.px(-5, 5, MOSS[3])
    if g.L:
        g.glow(-5 + f, -4, WHITE, 170); g.glow(5 - f, 3, WHITE, 150); g.glow(0, 6 - f, g.G, 130)


def m_synth(g):
    t, f = g.t, g.f
    x0, y0, x1 = [(-1, 1, 2), (-2, 0, 3), (-3, -1, 3), (-3, -1, 3)][t]
    g.box(x0, y0, x1, 4)
    # gear
    gx, gy = (0, 2) if t == 0 else (0, 2)
    gc = AMBER if not g.L else WHITE
    if t == 0:
        g.px(0, 2, gc); g.px(1, 3, AMBER_D)
    else:
        if f == 0:
            g.plus(gx, gy, gc, post=False)
        else:
            for d in ((-1, -1), (1, 1), (-1, 1), (1, -1), (0, 0)):
                g.px(gx + d[0], gy + d[1], gc)
        g.px(gx, gy, g.D)
    if t >= 1:
        g.hl(x0 + 1, x1 - 1, 4, g.D)
    if t >= 2:
        g.rc(1, -1 + 1, 2, 0 + 1, GLASS[1]); g.px(2, 0, CYAN if f == 0 else CYAN_D)
        g.box(4, 3, 5, 4, M3); g.px(5, 2 - f if g.L else 2, g.G if g.L else LIME)
    if g.L:
        g.glow(6, 0 + f, g.G, 200); g.glow(5, -2 - f, WHITE, 140); g.glow(-4, -3 + f, g.G, 150)


def m_buddy(g):
    t, f = g.t, g.f
    r = [1.6, 2.1, 2.1, 2.1][t]
    g.ln(0, -2, -3, -3, g.R[1]); g.px(-3, -2, g.R[0])
    g.ell(1, 0, r, r=M3); g.px(1, 0, g.R[2] if not g.L else g.G)
    if t >= 1:
        g.px(0, 0, g.R[1]); g.px(2, 0, g.R[1]); g.px(1, -1, g.R[1]); g.px(1, 1, g.R[1])
        g.px(4, 0, MD[1])
        g.ln(0, 2, -1, 3, AMBER_D)
        g.ell(-1.5, 4.5, 1.5, r=[AMBER_D, AMBER, GOLD[3]]); g.px(-1, 4, MD[0])
    else:
        g.px(3, 0, MD[1])
    if t >= 2:
        g.cap((-3, -1), (-3, 1), 1.1, g.R); g.px(-3, -3, M3[2])
        g.px(3, 3, AMBER_D); g.px(4, 4, AMBER)
    bub = [(5, -2), (6, -4)] if t < 3 else [(5, -3), (7, 1), (-6, -4), (-6, 3), (1, -6), (3, 6)]
    for i, (x, y) in enumerate(bub):
        if g.L or t >= 1 or i == 0:
            if g.L or (i + f) % 2 == 0:
                g.glow(x, y - ((i + f) % 2), g.G if not g.L else (WHITE if (i + f) % 2 else g.G), 220 if (i + f) % 2 else 140)


# ---- head (anchor: top-centre of the helmet; helmet below y=0) ------------
def m_antenna(g):
    t, f = g.t, g.f
    h = [4, 6, 6, 6][t]
    g.hl(-1, 1, 0, M3[0]); g.px(0, 0, M3[1])
    g.vl(0, -h + 1, -1, M3[2])
    tip = (HOT if f == 0 else RED[1]) if not g.L else g.G
    g.px(0, -h, tip)
    if t >= 1:
        g.hl(-1, 1, -3, M3[1]); g.hl(-2 if t > 1 else -1, 2 if t > 1 else 1, -h + 1, M3[1])
    if t >= 2:
        g.vl(3, -3, 0, M3[1]); g.px(3, -4, LIME if f else LIME_D)
        g.vl(-3, -2, 0, M3[1])
        g.poly([(-6, -5), (-2, -4), (-3, -1)], BONE[1:]); g.px(-5, -5, BONE[3]); g.px(-3, -3, g.G if g.L else AMBER)
    if g.L:
        for i in range(2):
            rr = 2 + i + f
            g.glow(-rr, -h - 1 + i, g.G, 200 - 60 * i); g.glow(rr, -h - 1 + i, g.G, 200 - 60 * i)
        g.glow(0, -h - 1, WHITE, 200)


def m_chip(g):
    t, f = g.t, g.f
    w = [1, 2, 3, 3][t]
    g.box(-w, -2 - (t >= 1), w, 0)
    for x in range(-w, w + 1, 2):
        g.px(x, 0, GOLD[2])
    g.px(-w + 1, -2, LIME if f == 0 else LIME_D)
    if t >= 2:
        for x in range(-2, 3):
            g.px(x, -2, AMBER if (x + f) % 2 == 0 else AMBER_D)
    if t >= 1:
        cy = [-6, -5, -6, -5][t] - (f if t == 3 else 0)
        if g.L and f == 1:
            g.vl(0, cy - 1, cy + 1, GOLD[3]); g.px(0, cy, WHITE)
        elif t == 1:
            g.box(0, cy, 1, cy + 1, GOLD[1:])
        else:
            g.ell(0, cy, 1.6, r=GOLD[1:]); g.px(0, cy, GOLD[0] if not g.L else WHITE)
    if g.L:
        g.glow(-3 + f, -7, g.G, 200); g.glow(3 - f, -5, WHITE, 180); g.glow(2, -8 + f, g.G, 120)


def m_hive(g):
    t, f = g.t, g.f

    def node(x, y, on):
        g.ell(x, y, 1.5); g.px(x, y, g.G if on else g.G2)
    if t == 0:
        g.hl(-2, 2, 0, g.R[0]); node(0, -1, True)
        return
    g.hl(-4, 4, 0, g.R[0]); g.px(-4, 1, g.R[0]); g.px(4, 1, g.R[0])
    if t == 1:
        node(-3, -1, f == 0); node(3, -1, f == 0); node(0, -2, True)
        return
    g.vl(-3, -3, -1, g.R[1]); g.vl(3, -3, -1, g.R[1]); g.vl(0, -5, -1, g.R[1])
    node(-3, -3, (f == 0) or not g.L); node(3, -3, (f == 1) or not g.L); node(0, -5, True)
    g.px(-5, -1, g.R[2]); g.px(5, -1, g.R[2]); g.px(-5, -2, g.G2); g.px(5, -2, g.G2)
    if g.L:
        g.px(0, -5, WHITE if f == 0 else g.G)
        g.glow(-2, -5 + f, g.G, 170); g.glow(2, -4 - f, g.G, 170); g.glow(-5, -5 + f, WHITE, 120); g.glow(5, -5 - f + 1, WHITE, 120)
        g.glow(-1 - f, -3, g.G, 110); g.glow(1 + f, -3, g.G, 110)


def m_bonnet(g):
    t, f = g.t, g.f
    if t == 0:
        g.vl(0, -2, 0, MOSS[2]); g.px(-1, -3, LIME); g.px(1, -3, LIME); g.px(-2, -4, LIME_D); g.px(2, -4, LIME_D); g.px(0, -3, LIME_D)
        return
    bw = [0, 4, 6, 6][t]
    g.box(-2 - (t >= 2), -3 - (t >= 2), 2 + (t >= 2), 0)
    g.hl(-bw, bw, 1, g.R[1]); g.hl(-bw, 0, 1, g.R[2]); g.px(bw, 1, g.R[0])
    g.hl(-2 - (t >= 2), 2 + (t >= 2), 0, RUST[2] if not g.L else g.G)
    if t == 1:
        g.px(2, -4, LIME); g.px(1, -4, LIME_D)
    if t >= 2:
        fc = MAGENTA if not g.L else AMBER
        g.plus(3, -4, fc, post=False); g.px(3, -4, AMBER if not g.L else WHITE)
        leaf(g, 0, -4, d=-1, big=True); g.px(-3, -5, MAGENTA_D if not g.L else g.G)
    if g.L:
        for i, (x, y) in enumerate(((5, -6), (1, -6), (5, -2), (3, -7))):
            g.glow(x, y, g.G if (i + f) % 2 else WHITE, 210 if (i + f) % 2 else 110)
        g.glow(-5, -4 - f, g.G, 150)


def m_headlamp(g):
    t, f = g.t, g.f
    if g.L:
        g.hl(-3, 3, 0, M3[0])
        g.ell(0, -4, 2.6, r=[AMBER_D, AMBER, WHITE]); g.px(0, -4, WHITE); g.px(-1, -5, WHITE)
        rays = [(0, -8), (4, -4), (-4, -4), (3, -7), (-3, -7)] if f == 0 else [(2, -8), (-2, -8), (4, -6), (-4, -6), (4, -2), (-4, -2)]
        for (x, y) in rays:
            g.glow(x, y, g.G, 230)
        return
    g.hl(-3, 1, 0, g.D)
    g.box(1, -2 - (t >= 1), 3, 0, M3)
    g.px(4, -1, AMBER); g.px(3, -1, WHITE)
    if t >= 1:
        g.px(4, -2, AMBER); g.px(3, -2, WHITE)
        g.glow(5, -2, g.G, 150); g.glow(5, -1, g.G, 190); g.glow(6, -1, g.G, 90)
    if t >= 2:
        g.box(-3, -2, -1, 0, M3); g.px(-2, -1, LIME if f == 0 else LIME_D)
        g.box(0, -5, 2, -4, M3); g.px(3, -5, AMBER); g.px(2, -5, WHITE)
        g.glow(4, -5, g.G, 190); g.glow(5, -5, g.G, 100); g.glow(6, -2, g.G, 90); g.glow(5, -3, g.G, 90)


def m_dome(g):
    t, f = g.t, g.f
    w = [2, 3, 4, 4][t]
    h = [2, 3, 5, 5][t]
    gl = [alpha(ICE[1], 255), ICE[2], ICE[3]]
    pts = m_ellipse(8.5, 8.5, w + 0.5, h + 0.2)
    pts = {p for p in pts if p[1] <= 8}
    g.cv.part(pts, gl)
    inner = {p for p in pts if all(q in pts for q in ((p[0] - 1, p[1]), (p[0] + 1, p[1]), (p[0], p[1] - 1)))}
    for p in inner:
        g.cv.set(p[0], p[1], GLASS[2] if not g.L else mix(GLASS[2], g.G, 0.35))
    g.hl(-w, w, 0, g.R[1]); g.px(-w, 0, g.R[2])
    if t >= 1:
        g.vl(0, -h + 1, -1, ICE[1])
    if t >= 2:
        g.px(-2, -2, ICE[1]); g.px(2, -2, ICE[1]); g.px(-1, -3, ICE[1]); g.px(1, -3, ICE[1])
        g.px(-2, -1, LIME); g.px(2, -1, LIME_D); g.px(-3, -1, LIME_D)
        g.px(-3, -3, WHITE)
    if g.L:
        g.px(-2, -2, g.G if f else WHITE); g.px(2, -3, WHITE if f else g.G); g.px(1, -1, AMBER if f else g.G)
        g.glow(0, -h - 2 + f, g.G, 160); g.glow(-5, -3 - f, WHITE, 120); g.glow(5, -2 - f, WHITE, 120)


def m_vane(g):
    t, f = g.t, g.f
    h = [4, 4, 4, 3][t]
    g.hl(-1, 1, 0, M3[0]); g.vl(0, -h, -1, M3[1])
    if g.L:
        g.ell(0, -5, 4.2, 1.9, [INDIGO[2], INDIGO[3], PALE[2]])
        g.ell(-2, -6, 2.0, 1.4, [INDIGO[3], PALE[2], PALE[3]], sep=False)
        if f == 0:
            for (x, y) in ((1, -3), (2, -2), (1, -2), (2, -1)):
                g.glow(x, y, AMBER if (x + y) % 2 else WHITE)
        for i, x in enumerate((-4, -2, 4)):
            g.glow(x, -2 + (i + f) % 2, g.G, 220)
        return
    g.hl(-2 - (t >= 1), 2 + (t >= 1), -h, g.R[1])
    ax = 2 + (t >= 1)
    g.px(ax + 1, -h, g.R[2]); g.px(ax, -h - 1, g.R[2]); g.px(ax, -h + 1, g.R[0])
    g.px(-ax, -h - 1, g.R[1]); g.px(-ax, -h + 1, g.R[1])
    if t >= 1:
        g.hl(-1, 1, -2, M3[2]); g.px(-2, -2, g.G2); g.px(2, -2, g.G2)
    if t >= 2:
        g.vl(0, -h - 2, -h - 1, M3[2]); g.px(0, -h - 3, AMBER); g.vl(0, -1, 0, M3[2])
        cups = [(-3, -3), (3, -3)] if f == 0 else [(-2, -3), (2, -3)]
        for (x, y) in cups:
            g.px(x, y, g.R[2]); g.px(x, y + 1, g.R[0])


# ---- eyes (visor: x -1..1, y -1..2; helmet front edge x=2) ----------------
def m_xray(g):
    t, f = g.t, g.f
    r = [1.5, 1.7, 2.3, 2.3][t]
    g.ell(1, 0.5 if t >= 2 else 0, r + 0.9, r=MD)
    g.ell(1, 0.5 if t >= 2 else 0, r, r=[g.G2, g.G, g.G], sep=False)
    if t >= 1:
        g.hl(-4, -2 if t < 2 else -3, 0, g.D)
    if t >= 2:
        g.px(1, 0, g.G2); g.px(2, 1, g.G2); g.px(0, 1, WHITE if not g.L else g.G2); g.px(1, 2, g.G2) if False else None
        g.px(3, -3, M3[2]); g.px(-2, -2, M3[1])
    else:
        g.px(1, 0, WHITE)
    if g.L:
        y = -1 + f * 2
        g.hl(0, 2, y, WHITE)
        g.glow(5, y, g.G, 200); g.glow(6, y + (1 - 2 * f), g.G, 110); g.glow(5, 2 - 3 * f, WHITE, 120)


def m_nvg(g):
    t, f = g.t, g.f
    ln = [3, 4, 5, 5][t]
    ys = [(0,), (0, 2), (0, 2), (-1, 1, 3)][t]
    if t == 0:
        g.box(-1, -1, 0, 1, M3)
    if t >= 2:
        g.box(-2, -2 if t == 3 else -1, 0, 3, M3); g.hl(-5, -3, 0, g.D); g.px(-1, -1, AMBER if f == 0 else AMBER_D)
    for i, y in enumerate(ys):
        g.hl(0, ln - 1, y, M3[1]); g.px(0, y, M3[2])
        g.px(ln, y, g.G if (not g.L or (i + f) % 2 == 0) else WHITE)
        if t >= 1:
            g.px(ln - 1, y, M3[0])
    if g.L:
        for i, y in enumerate(ys):
            g.glow(ln + 1, y, g.G, 160 if (i + f) % 2 else 220)
        g.glow(ln + 2, 1 - f, g.G, 100)


def m_threat(g):
    t, f = g.t, g.f
    g.ell(1, 0, [1.3, 1.6, 2.0, 2.0][t], r=[RED[1], RED[2], HOT]); g.px(1, 0, WHITE if f == 0 else RED[3])
    g.ln(-1, -2, -2 - (t >= 1), -3, g.D)
    if t >= 1:
        g.px(-4, -3, M3[1])
    if t >= 2:
        g.box(-3, -1, -2, 1, M3); g.px(-3, 0, HOT if f else RED[1])
        cx, cy = 5, 0
        ticks = [(0, -2), (0, 2), (-2, 0), (2, 0)] if (f == 0 or not g.L) else [(-1, -2), (1, 2), (-2, 1), (2, -1)]
        for (dx, dy) in ticks:
            g.glow(cx + dx, cy + dy, g.G, 230)
        g.glow(cx, cy, WHITE if g.L else g.G, 255 if g.L else 170)
        if g.L:
            for (dx, dy) in ((-1, -1), (1, 1), (1, -1), (-1, 1)):
                g.glow(cx + dx, cy + dy, g.G, 90)


def m_monocle(g):
    t, f = g.t, g.f
    r = [2.0, 2.2, 2.5, 2.5][t]
    g.ell(1, 0, r, r=GOLD[1:])
    g.ell(1, 0, r - 1.0, r=[ICE[1], ICE[2], ICE[3]], sep=False)
    if t >= 1:
        ch = [(0, 2), (-1, 3), (-1, 4), (-2, 5)] if t == 1 else [(0, 3), (-1, 4), (-1, 5), (-2, 6), (-3, 6), (-4, 5)]
        for i, (x, y) in enumerate(ch):
            g.px(x, y, GOLD[2] if i % 2 == 0 else GOLD[1])
    if t >= 2:
        g.px(1, -3, HOT if not g.L else g.G); g.px(1, -4, GOLD[2]); g.px(3, -2, GOLD[3])
        g.px(0, -1, WHITE)
    if g.L:
        sp = [(4, -3), (-2, -3)] if f == 0 else [(4, 2), (-1, -4)]
        for (x, y) in sp:
            g.plus(x, y, WHITE, 230)
        g.px(1, 0, WHITE if f else ICE[3])


def m_zoom(g):
    t, f = g.t, g.f
    e = f if g.L else 0
    g.box(-1, -1, 1, 2)
    g.px(-1, -1, g.H)
    x = 2
    if t >= 1:
        g.box(2, -1 + (t < 2), 3, 2 - (t < 2), g.R if t >= 2 else M3); x = 4
    if t >= 2:
        g.box(4, 0, 5 + e - g.L, 1, M3); x = 6 + e - g.L
        g.hl(-4, -2, 0, g.D); g.px(0, -2, GOLD[2])
    g.vl(x, 0, 1, g.G if g.L else ICE[2]); g.px(x, 0, WHITE)
    if t == 0:
        g.px(2, -1, CLEAR)
    if g.L:
        g.glow(x + 1, 0 + f, g.G, 200); g.glow(x + 1, 1 - f, g.G, 90); g.glow(3, -3 - f, WHITE, 130)


def m_geo(g):
    t, f = g.t, g.f
    g.hl(-1 - (t >= 1) * 2, 2, 0, g.R[1]); g.px(2, 0, g.R[2])
    g.hl(-1, 2, 1, g.R[0]); g.px(0, 1, g.G); g.px(2, 1, g.G)
    if t >= 2:
        g.hl(-3, 2, -1, g.R[2])
    if t >= 2:
        g.px(-3, 1, g.R[0]); g.px(3, 0, M3[2])
        n = 3 if t == 2 else 4
        x0 = 4
        for i in range(n):
            for j in range(n):
                on = (i == 0 or j == 0 or i == n - 1 or j == n - 1) or g.L
                if not on:
                    continue
                a = 200 if (i + j + f) % 2 == 0 else 110
                if g.L and (i, j) == ((1 + f), (1 + f)):
                    g.glow(x0 + i, -2 + j, WHITE, 255)
                else:
                    g.glow(x0 + i, -2 + j, g.G, a)


# ---- arms (shoulder of the leading arm; upper arm hangs below) ------------
def m_extendo(g):
    t, f = g.t, g.f
    g.ell(0, 0, 1.6, r=M3); g.px(0, 0, g.R[2])

    def arm(pts, claw):
        for i in range(len(pts) - 1):
            g.ln(pts[i][0], pts[i][1], pts[i + 1][0], pts[i + 1][1], M3[1])
        for (x, y) in pts[1:-1]:
            g.px(x, y, g.R[2] if not g.L else g.G)
        x, y = pts[-1]
        g.px(x, y, M3[2]); g.px(x + claw, y - 1, M3[2]); g.px(x + claw, y + 1, M3[2])
    if t == 0:
        arm([(0, -1), (2, -3)], 1)
    elif t == 1:
        arm([(0, -1), (-1, -4), (3, -5)], 1)
    else:
        b = f if g.L else 0
        arm([(0, -1), (-1, -4), (3, -6)], 1)
        arm([(-1, 0), (-4, -1), (-5, 2)], -1) if t == 2 else arm([(-1, 0), (-4, -2), (-5, 1)], -1)
        if g.L:
            hx, hy = 5, -2 - b
            g.box(hx - 1, hy - 1, hx, hy, M3, sep=False); g.px(hx + 1, hy - 2, M3[2]); g.px(hx + 1, hy, M3[2])
            g.glow(hx - 2, hy + 1, g.G, 200); g.glow(hx - 3, hy + 2 + b, g.G, 120); g.glow(hx - 1, hy + 1, WHITE, 200)


def m_drill(g):
    t, f = g.t, g.f
    ln = [2, 3, 5, 4][t]
    hw = [0, 1, 1, 1][t]
    g.box(-2, -1 - hw, 1, 1 + (t >= 2), g.R)
    g.px(-1, 0, g.D); g.px(-2, -1 - hw, g.H)
    for i in range(ln):
        half = max(0, (ln - 1 - i) * (1 + (t >= 2)) // ln) if t else 0
        if t >= 2:
            half = [2, 1, 1, 0, 0][i + (5 - ln)]
        elif t == 1:
            half = [1, 0, 0][i]
        for y in range(-half, half + 1):
            stripe = (i + y + (f if (g.L or t >= 1) else 0)) % 2 == 0
            c = (M3[2] if stripe else M3[0]) if not g.L else (g.G if stripe else M3[1])
            g.px(2 + i, y, c)
    if t >= 2:
        g.px(-3, 0, M3[1]); g.px(-3, 1, M3[0]); g.px(0, -3, AMBER)
    if g.L:
        g.glow(2 + ln, 0, WHITE); g.glow(3 + ln, -1 + 2 * f, g.G, 180); g.glow(2 + ln, 2 - 4 * f, g.G, 120)


def m_lifter(g):
    t, f = g.t, g.f
    g.ell(0, -1, [2.6, 2.6, 3.2, 3.2][t], [1.9, 2.0, 2.4, 2.4][t])
    g.px(0, -1, M3[2]); g.px(-1, -2, g.H)
    if t >= 1:
        g.box(-3, 1, -2, 3, M3); g.px(-3, 1, M3[2])
        g.vl(-2, 4, 5, M3[2] if not g.L else g.G)
        g.px(-2, 6, g.R[1]); g.px(-1, 6, g.R[0])
    if t >= 2:
        g.px(1, -3, INK); g.px(2, -2, INK); g.px(-2, -3, INK) if False else None
        g.box(-4, 1, -2, 3, M3); g.px(-3, 2, g.R[2]); g.vl(-3, 4, 4, M3[2] if not g.L else g.G)
        g.ell(-0.5, 5.5, 1.5, r=g.R); g.px(-1, 5, M3[2])
        g.px(3, 1, RUST[2]); g.px(3, 2, RUST[1]); g.px(2, 3, RUST[1])
    if g.L:
        g.hl(-2, 2, -1, g.G if f == 0 else WHITE); g.px(0, -1, WHITE)
        g.glow(5, -4 + f, g.G, 170); g.glow(-5, -4 - f + 1, g.G, 130); g.glow(-6, 3 + f, WHITE, 120)


def m_shield(g):
    t, f = g.t, g.f
    if g.L:
        pts = m_ellipse(9.5, 9.5, 4.6, 5.4)
        edge = {p for p in pts if any(q not in pts for q in ((p[0] + 1, p[1]), (p[0] - 1, p[1]), (p[0], p[1] + 1), (p[0], p[1] - 1)))}
        for (x, y) in pts:
            if (x, y) in edge:
                g.glow(x - 8, y - 8, WHITE if (x + y + f) % 4 == 0 else g.G, 235)
            else:
                g.glow(x - 8, y - 8, g.G, 70 + 40 * ((x + y + f) % 2))
        g.box(0, 0, 2, 2, M3); g.px(1, 1, WHITE)
        return
    if t == 0:
        g.ell(1, 1, 2.2); g.px(1, 1, GOLD[2])
    elif t == 1:
        g.ell(1, 1, 3.2, 3.4); g.ell(1, 1, 1.2, r=GOLD[1:], sep=False)
        g.px(-1, 3, g.D); g.px(3, -1, g.H)
    else:
        g.poly([(-3, -4), (5, -4), (6, 0), (1.5, 6), (-3, 1)])
        g.vl(1, -3, 4, g.H); g.hl(-2, 4, -1, g.H); g.px(1, -1, GOLD[2])
        g.px(-2, -3, GOLD[2]); g.px(4, -3, GOLD[1])


def m_farm(g):
    t, f = g.t, g.f
    W = BROWN[1:]
    g.box(-1, -1, 1, 0, g.R)
    g.vl(0, -4, -2, W[1]); g.px(0, -5, M3[2]); g.px(0, -6, M3[2]); g.px(1, -5, M3[1])      # trowel
    if t >= 1:
        g.ln(-1, -1, -3, -5, W[1])                                                        # sickle
        bc = M3[2] if not g.L else g.G
        for (x, y) in ((-3, -6), (-2, -7), (-1, -7), (0, -7) if t < 2 else (-4, -5)):
            g.px(x, y, bc)
        g.px(-4, -6, bc) if t >= 2 else None
    if t >= 2:
        g.ln(1, -1, 4, -4, W[1])                                                          # rake
        for d in (0, 1, 2):
            g.px(4 + d - 1, -5 - (d == 1), M3[2]); g.px(3 + d, -4 + d - 1, M3[1]) if d == 0 else None
        g.px(5, -4, M3[2]); g.px(6, -3, M3[2]); g.px(5, -6, M3[2])
        leaf(g, -2, 0, d=-1, c=LIME)
    if g.L:
        g.glow(-5 + f, -7, WHITE, 220); g.glow(2, -8 + f, g.G, 180); g.glow(6, -6 - f + 1, g.G, 150)
        g.px(0, -6, g.G)


def m_welder(g):
    t, f = g.t, g.f
    g.box(-1, -1, 1, 0, M3)
    g.hl(2, 3, 0, M3[1]); g.px(3, 0, RUST[2])
    fc = [WHITE, g.G, g.G2]
    n = [1, 2, 2, 2][t]
    for i in range(n + (1 - f if t else 0)):
        g.glow(4 + i, 0, fc[min(i, 2)], 255 if i < n else 140)
    if t >= 1:
        g.cap((-3, -3), (-3, 1), 1.2, g.R); g.px(-3, -5, M3[2]); g.px(-2, -2, RUST[1])
    if t >= 2:
        g.hl(2, 3, -2, M3[1]); g.px(3, -2, RUST[2]); g.px(1, -2, M3[0])
        for i in range(n - f):
            g.glow(4 + i, -2, fc[min(i, 2)])
        g.box(-1, -4, 1, -3, MD); g.px(0, -4, ICE[1])
    if t >= 2 or f == 0:
        g.glow(5 + n, 1 + f, AMBER, 220 if t >= 2 else 0)
    if g.L:
        g.glow(5 + n, -3 + f, AMBER); g.glow(6 + n - 2, 2, WHITE, 180); g.glow(4 + n, -1, g.G, 130)


# ---- hands (leading hand: x -1..1, y -1..1; multitool forward) ------------
def m_bolt(g):
    t, f = g.t, g.f
    g.vl(2, -1, 1, M3[2]); g.px(2, -2, M3[1]); g.px(2, 2, M3[0])
    g.px(1, 0, g.G2)
    g.vl(0, -1 - (t >= 1), 1 + (t >= 1), M3[1]); g.px(0, -1 - (t >= 1), M3[2])
    if t >= 2:
        g.hl(3, 5, -2, M3[1]); g.hl(3, 5, 2, M3[0]); g.px(5, -2, M3[2]); g.px(-2, -2, M3[1]); g.px(-2, 2, M3[0])
    tip = [3, 3, 5, 5][t]
    g.glow(tip, 0, WHITE if (f == 0 or g.L) else g.G)
    if t >= 1:
        g.glow(tip + 1, 0, g.G, 200 if f == 0 else 110)
    if t >= 2:
        g.glow(tip, -1 if f else 1, g.G, 200); g.glow(tip - 1, 0, g.G, 220)
    if g.L:
        zz = [(6, -1), (7, -2), (6, 1), (7, 0)] if f == 0 else [(6, 1), (7, 2), (6, -1), (7, -1)]
        for (x, y) in zz:
            g.glow(x, y, g.G, 220)
        g.glow(4, -3 + 6 * f, WHITE, 170)


def m_midas(g):
    t, f = g.t, g.f
    R = GOLD[1:]
    g.ell(0.5, 0.5, [2.1, 2.3, 2.6, 2.6][t], r=R)
    g.px(1, -1, GOLD[3])
    if t >= 2:
        g.box(-3, -2, -2, 2, R); g.px(-3, 0, HOT if not g.L else g.G); g.px(-2, -2, GOLD[3])
        g.px(3, -1, GOLD[3]); g.px(3, 1, GOLD[1])
    if g.L:
        sp = [(3, -3), (-1, 3)] if f == 0 else [(-1, -4), (4, 2)]
        for (x, y) in sp:
            g.plus(x, y, WHITE, 230)
        g.px(0, 0, WHITE if f else GOLD[3])


def m_cryo(g):
    t, f = g.t, g.f
    g.ell(0.5, 0.5, 2.1 if t == 0 else 2.3)
    g.px(0, -1, WHITE)
    sp = [[(2, -2)], [(3, -2), (3, 1)], [(3, -3), (4, 0), (3, 3)], [(3, -3), (5, 0), (3, 3)]][t]
    for (x, y) in sp:
        g.ln(1, 0 if y == 0 else (1 if y > 0 else -1), x, y, g.R[2]); g.px(x, y, WHITE if g.L else g.H)
    if t >= 2:
        g.box(-3, -2, -2, 2, [ICE[0], ICE[1], ICE[2]]); g.px(-2, 3, g.R[1]); g.px(-2, 4, g.H)
    if g.L:
        if f == 0:
            g.plus(6, -3, WHITE, 230); g.glow(-1, -4, g.G, 160)
        else:
            g.plus(5, 4, WHITE, 230); g.glow(0, -5, g.G, 160); g.glow(7, 0, WHITE, 200)


def m_boom(g):
    t, f = g.t, g.f
    g.ell(0.5, 0.5, 2.1 if t == 0 else 2.3)
    g.px(0, -1, g.H)
    if t >= 1:
        g.px(1, -3, BROWN[2]); g.px(2, -4, BROWN[2])
        g.glow(3 - (f if t > 1 else 0), -5 + (f if t > 1 else 0), AMBER if f == 0 else WHITE)
    if t >= 2:
        g.ell(3, 0, 1.9, r=MD); g.px(3, -1, INDIGO[3])
        g.vl(-3, -2, 2, AMBER); g.px(-3, -1, INK); g.px(-3, 1, INK)
        g.px(1, -3, CLEAR); g.px(2, -4, CLEAR)
        g.px(3, -3, BROWN[2]); g.px(4, -4, BROWN[2])
        g.glow(5 - f, -5, AMBER if f == 0 else WHITE)
    if g.L:
        g.glow(6, -6 + f, g.G, 200); g.glow(4 + 2 * f, -6, HOT, 220); g.glow(5, -4, AMBER, 170)
        g.px(3, 0, g.G if f == 0 else HOT)


def m_green(g):
    t, f = g.t, g.f
    g.ell(0.5, 0, 1.9 if t == 0 else 2.0)
    leaf(g, 1, -2, d=1)
    if t >= 1:
        g.px(-1, -2, MOSS[2]); g.px(-1, -3, LIME_D); g.px(-2, -4, LIME); g.px(3, 0, LIME_D); g.px(4, -1, LIME)
    if t >= 2:
        g.hl(-3, -2, 1, MOSS[1]); g.px(-3, 0, MOSS[2]); g.px(-2, 2, MOSS[1]); g.px(-3, 2, LIME_D)
        fc = MAGENTA if not g.L else g.G
        g.plus(3, -4, fc, post=False); g.px(3, -4, AMBER if not (g.L and f) else WHITE)
        g.px(2, -2, MOSS[2])
    if g.L:
        g.glow(5 + f, -6, g.G, 190); g.glow(1, -6 - f, WHITE, 150); g.glow(5, 1 + f, g.G, 150); g.glow(-4, -5 + f, g.G, 110)


def m_grapple(g):
    t, f = g.t, g.f
    hc = M3[2] if not g.L else g.G
    g.hl(1, 3, 0, M3[1])
    g.px(4, 0, hc); g.px(4, -1, hc); g.px(3, -2, hc)
    if t >= 1:
        g.ell(-2, 1, 1.6); g.px(-2, 1, g.D); g.px(0, 1, g.R[2])
    if t >= 2:
        g.px(5, 0, hc); g.px(4, 1, hc); g.px(3, 2, hc); g.px(2, -2, M3[0]); g.px(2, 2, M3[0])
        g.ell(-2, 1, 2.1); g.px(-2, 1, g.D); g.px(-3, 0, g.H)
        g.px(-1, -1, g.R[1]); g.px(0, -1, g.R[1])
    if g.L:
        for i in range(2):
            g.glow(6 + i, 0 + ((i + f) % 2) - (i % 2), g.G, 220 - i * 70)
        g.glow(5, -3 + f, WHITE, 150); g.glow(5, 3 - f, WHITE, 150)


def m_heal(g):
    t, f = g.t, g.f
    W = [PALE[1], PALE[3], WHITE]
    g.ell(0.5, 0.5, 2.1 if t == 0 else 2.4, r=W)
    cc = LIME_D if not g.L else g.G
    g.plus(0, 0, cc, post=False)
    if t >= 1:
        g.vl(-3, -1, 1, W[0]); g.px(-3, -1, W[1])
    if t >= 2:
        g.box(-3, -2, -2, 2, W); g.px(-3, 0, HOT)
        g.px(3, -1, M3[2]); g.px(3, 1, M3[1]); g.px(4, 0, g.G)
        g.plus(3, -5 + f, g.G, 230 if not g.L else 255)
    if g.L:
        hx, hy = -3, -6 + (1 - f)
        for (dx, dy) in ((0, 0), (2, 0), (0, 1), (1, 1), (2, 1), (1, 2)):
            g.glow(hx + dx, hy + dy, MAGENTA if (dx, dy) != (0, 0) else WHITE, 230)
        g.glow(6, 2 - f, g.G, 170); g.glow(-4, 4 + f, g.G, 120)


# name, slot, motif, colour ramp, glow, dim glow
MODS = [
    ("Rocket Feet", "feet", m_rocket, RED, AMBER, AMBER_D),
    ("Sticky Soles", "feet", m_sticky, MOSS, LIME, LIME_D),
    ("Spring Heels", "feet", m_spring, GOLD, AMBER, AMBER_D),
    ("Hustle Treads", "feet", m_tread, STRAW, AMBER, AMBER_D),
    ("Aqua Flippers", "feet", m_flipper, BLUE, CYAN, CYAN_D),
    ("Stompers", "feet", m_stomper, RUST, AMBER, HOT),
    ("Dash Pistons", "legs", m_piston, BLUE, CYAN, CYAN_D),
    ("Cargo Pants", "legs", m_cargo, STRAW, MAGENTA, MAGENTA_D),
    ("Thermal Leggings", "legs", m_thermal, RED, AMBER, AMBER_D),
    ("Root Walkers", "legs", m_roots, MOSS, LIME, LIME_D),
    ("Shock Absorbers", "legs", m_absorber, ICE, CYAN, CYAN_D),
    ("Jackhammer Knees", "legs", m_jackhammer, GOLD, AMBER, AMBER_D),
    ("O2 Backpack", "back", m_o2, TEAL, LIME, LIME_D),
    ("Glide Wings", "back", m_wings, RUST, CYAN, CYAN_D),
    ("Shoulder Turret", "back", m_turret, METAL, HOT, RED[1]),
    ("Battery Pack", "back", m_battery, METAL, LIME, LIME_D),
    ("Sprinkler Pack", "back", m_sprinkler, BLUE, CYAN, CYAN_D),
    ("Pack Mule Frame", "back", m_mule, BROWN, MAGENTA, MAGENTA_D),
    ("Drone Dock", "back", m_dock, METAL, CYAN, CYAN_D),
    ("Beacon Rack", "back", m_beacon, PURPLE, MAGENTA, MAGENTA_D),
    ("Plating", "torso", m_plating, METAL, AMBER, AMBER_D),
    ("Filter Lungs", "torso", m_filter, TEAL, LIME, LIME_D),
    ("Medi-Core", "torso", m_medi, PALE, LIME, LIME_D),
    ("Reactor Heart", "torso", m_reactor, GOLD, CYAN, AMBER_D),
    ("Camo Skin", "torso", m_camo, MOSS, CYAN, CYAN_D),
    ("Synth Belly", "torso", m_synth, PURPLE, MAGENTA, MAGENTA_D),
    ("Buddy Breather", "torso", m_buddy, BLUE, CYAN, CYAN_D),
    ("Antenna Array", "head", m_antenna, METAL, CYAN, CYAN_D),
    ("Trade Chip", "head", m_chip, MOSS, AMBER, AMBER_D),
    ("Hive Mind", "head", m_hive, PURPLE, MAGENTA, MAGENTA_D),
    ("Botanist's Bonnet", "head", m_bonnet, STRAW, LIME, LIME_D),
    ("Headlamp", "head", m_headlamp, METAL, AMBER, AMBER_D),
    ("Dome Brain", "head", m_dome, TEAL, CYAN, CYAN_D),
    ("Weather Vane", "head", m_vane, RUST, CYAN, CYAN_D),
    ("X-Ray Specs", "eyes", m_xray, METAL, LIME, LIME_D),
    ("Night Vision", "eyes", m_nvg, METAL, LIME, LIME_D),
    ("Threat Lens", "eyes", m_threat, RED, HOT, RED[1]),
    ("Appraisal Monocle", "eyes", m_monocle, GOLD, AMBER, AMBER_D),
    ("Zoom Goggles", "eyes", m_zoom, BROWN, CYAN, CYAN_D),
    ("Geo Visor", "eyes", m_geo, RUST, CYAN, CYAN_D),
    ("Extendo-Arms", "arms", m_extendo, GOLD, AMBER, AMBER_D),
    ("Drill Arms", "arms", m_drill, STRAW, AMBER, AMBER_D),
    ("Power Lifters", "arms", m_lifter, STRAW, CYAN, CYAN_D),
    ("Shield Arm", "arms", m_shield, BLUE, CYAN, CYAN_D),
    ("Farm Hands", "arms", m_farm, MOSS, AMBER, AMBER_D),
    ("Welder Arms", "arms", m_welder, RUST, CYAN, BLUE[2]),
    ("Bolt Enhancements", "hands", m_bolt, METAL, CYAN, CYAN_D),
    ("Midas Mitts", "hands", m_midas, GOLD, AMBER, AMBER_D),
    ("Cryo Palms", "hands", m_cryo, ICE, CYAN, CYAN_D),
    ("Boom Mitts", "hands", m_boom, RED, AMBER, AMBER_D),
    ("Green Fingers", "hands", m_green, MOSS, LIME, LIME_D),
    ("Grapple Glove", "hands", m_grapple, RUST, CYAN, CYAN_D),
    ("Healing Hands", "hands", m_heal, PALE, LIME, LIME_D),
]
assert len(MODS) == 53


def mod_cell(i, t, f):
    name, slot, fn, rmp, glow, glow2 = MODS[i]
    g = G(t, f, rmp, glow, glow2)
    fn(g)
    cv = g.done()
    if cv.oob:
        WARN.append("mods: %s tier %d frame %d drew %d px outside the cell (last %r)" % (name, t, f, cv.oob, cv.oob_at))
    return cv


SLOT_TINT = {"feet": RED[1], "legs": BLUE[1], "back": MOSS[1], "torso": RUST[1],
             "head": PURPLE[1], "eyes": TEAL[2], "arms": GOLD[1], "hands": MAGENTA_D}


def gen_mods():
    sheet = Canvas(128, 848)
    for i in range(53):
        for t in range(4):
            for f in range(2):
                sheet.blit(mod_cell(i, t, f), (t * 2 + f) * 16, i * 16)
    save("mods.png", sheet, 16, 16, edges="")
    icons = Canvas(256, 64)
    for i in range(53):
        name, slot = MODS[i][0], MODS[i][1]
        tint = SLOT_TINT[slot]
        ic = Canvas(16, 16)
        base = [mix(INK, tint, 0.28), mix(INK, tint, 0.45), mix(INK, tint, 0.7)]
        for y in range(16):
            for x in range(16):
                corner = (x in (0, 15) and y in (0, 15))
                if corner:
                    continue
                edge = x in (0, 15) or y in (0, 15) or (x in (1, 14) and y in (1, 14))
                if edge:
                    c = base[2] if (x + y < 15) else INK
                else:
                    c = base[1] if (y < 8 and (x + y) % 2 == 0 and y > 5) or y <= 5 else base[0]
                    if y <= 5 and y > 3 and (x + y) % 2:
                        c = base[0]
                    if y <= 3:
                        c = base[1]
                ic.px[y * 16 + x] = c
        for t in (2, 1, 0):
            em = mod_cell(i, t, 0)
            bb = em.bbox()
            if bb[2] - bb[0] + 1 <= 12 and bb[3] - bb[1] + 1 <= 12:
                break
        w, h = bb[2] - bb[0] + 1, bb[3] - bb[1] + 1
        ox, oy = (16 - w) // 2 - bb[0], (16 - h + 1) // 2 - bb[1]
        for y in range(16):
            for x in range(16):
                p = em.px[y * 16 + x]
                if p[3] and 1 <= x + ox <= 14 and 1 <= y + oy <= 14:
                    ic.blend(x + ox, y + oy, p)
        icons.blit(ic, (i % 16) * 16, (i // 16) * 16)
    save("mod_icons.png", icons)



# --------------------------------------------------------------------------
# Shared organic helpers
# --------------------------------------------------------------------------
def blob(cv, cx, cy, rx, ry, rmp, lx=-0.6, ly=-0.75, clip=None):
    """Ellipsoid shaded from the top-left with band-edge dithering."""
    pts = m_ellipse(cx, cy, rx, ry)
    if clip is not None:
        pts = {p for p in pts if clip(p)}
    n = len(rmp)
    for (x, y) in pts:
        dx, dy = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
        v = (-(dx * lx + dy * ly) * -1.0)
        v = max(0.0, min(0.999, (-(dx * lx + dy * ly) + 0.95) / 1.9))
        fv = v * n
        i = int(fv)
        fr = fv - i
        if fr > 0.78 and i < n - 1 and (x + y) % 2 == 0:
            i += 1
        cv.set(x, y, rmp[i])
    return pts


def thin_leg(cv, a, foot, l1, l2, c, cj=None, sign=None):
    """1px two-segment leg with the knee bent upward."""
    if sign is None:
        sign = -1 if foot[0] >= a[0] else 1
    j, e = ik2(a, foot, l1, l2, sign)
    cv.line(a[0], a[1], j[0], j[1], c)
    cv.line(j[0], j[1], e[0], e[1], c)
    if cj:
        cv.set(int(round(j[0])), int(round(j[1])), cj)
    return j, e


def thick_leg(cv, a, foot, l1, l2, rmp, r=1.0, sign=None):
    if sign is None:
        sign = -1 if foot[0] >= a[0] else 1
    j, e = ik2(a, foot, l1, l2, sign)
    cv.part(m_capsule(a, j, r) | m_capsule(j, e, r * 0.8), rmp, sep=False)
    cv.part(m_disc(j, r + 0.35), rmp, sep=False)
    return j, e


def glow_px(cv, x, y, c, a=255):
    cv.blend(x, y, alpha(c, a)) if a < 255 else cv.set(x, y, c)


def soft_spot(cv, x, y, c, a=90):
    """A luminous pixel with a faint cross halo (drawn after outlining)."""
    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        if cv.inb(x + dx, y + dy) and cv.get(x + dx, y + dy)[3] < 255:
            cv.blend(x + dx, y + dy, alpha(c, a))
    cv.set(x, y, c)


# --------------------------------------------------------------------------
# Robots
# --------------------------------------------------------------------------
def robot_frame(mode, f):
    cv = Canvas(16, 16)
    bob = 0
    eye = [CYAN, WHITE]
    dim = False
    if mode == "idle":
        bob = [0, -1, 0, 1][f]
    elif mode == "work":
        bob = [0, 0, -1, 0][f]
    elif mode == "carry":
        bob = [-1, -1, 0, 0][f]
    elif mode == "charge":
        eye = [[AMBER_D, AMBER], [AMBER, GOLD[3]], [LIME_D, LIME], [LIME, WHITE]][f]
    elif mode == "dead":
        bob = 2
        dim = True
        eye = [INDIGO[2], INDIGO[3]] if f == 0 else [INDIGO[1], INDIGO[2]]
    elif mode == "alarm":
        bob = [0, 0][f]
        eye = [HOT, WHITE] if f == 0 else [RED[1], RED[2]]
    cy = 7 + bob
    shell = m_ellipse(8.0, cy + 0.5, 5.2, 3.9)
    cv.part(shell, BONE[1:], sep=False)
    for (x, y) in shell:
        if y == cy + 1:
            cv.recolor([(x, y)], ACC, acc=True)
        elif y > cy + 1:
            cv.recolor([(x, y)], TEAL[1:])
    # wear: rust patch and moss
    for (x, y, c) in ((4, cy - 1, RUST[2]), (5, cy - 2, RUST[1]), (4, cy, RUST[1]), (7, cy - 3, MOSS[2]), (8, cy - 3, MOSS[3]), (6, cy + 3, MOSS[1])):
        if (x, y) in shell:
            cv.set(x, y, c)
    # eye socket
    for y in range(cy - 2, cy + 1):
        for x in range(9, 13):
            if (x, y) in shell:
                cv.set(x, y, GLASS[0] if y > cy - 2 else GLASS[1])
    cv.set(10, cy - 1, eye[0]); cv.set(11, cy - 1, eye[1]); cv.set(10, cy - 2, eye[0]); cv.set(11, cy - 2, eye[0])
    if mode == "dead":
        cv.set(10, cy - 2, GLASS[1]); cv.set(11, cy - 2, GLASS[1])
    # antenna
    ax = 5
    if mode == "dead":
        cv.set(ax, cy - 4, METAL[1]); cv.set(ax - 1, cy - 4, METAL[1]); cv.set(ax - 2, cy - 3, INDIGO[2])
    else:
        cv.set(ax, cy - 4, METAL[2])
        cv.set(ax, cy - 5, (LIME if f % 2 == 0 else LIME_D) if mode != "alarm" else (HOT if f == 0 else RED[1]))
    # thruster nozzle
    cv.rect(6, cy + 4, 9, cy + 4, METAL[1]); cv.set(6, cy + 4, METAL[2])
    post = []
    if mode not in ("dead",):
        fl = [(7, 5), (8, 5), (7 + f % 2, 6)] if mode != "carry" else []
        if mode == "charge":
            fl = [(7, 5), (8, 5)] if f % 2 == 0 else []
        for (x, dy) in fl:
            post.append((x, cy + dy, WHITE if dy == 5 and (x + f) % 2 == 0 else CYAN, 255 if dy == 5 else 170))
        if mode == "carry":
            for sx in (4, 11):
                post.append((sx, cy + 4, CYAN, 255)); post.append((sx, cy + 5 + f % 2, CYAN, 150))
    # tool arm
    if mode == "work":
        ex = [12, 13, 12, 13][f]
        cv.line(11, cy + 3, 12, cy + 2, METAL[1]); cv.line(12, cy + 2, ex, cy + 2, METAL[2])
        cv.set(ex, cy + 1, METAL[3]); cv.set(ex, cy + 3, METAL[3])
        sp = [[(14, cy + 1), (13, cy + 4)], [(14, cy + 3), (14, cy)], [(13, cy), (14, cy + 4)], [(14, cy + 2), (12, cy + 4)]][f]
        for i, (x, y) in enumerate(sp):
            post.append((x, y, AMBER if i == 0 else WHITE, 255 if i == 0 else 200))
    elif mode == "carry":
        cv.part(m_rect(5, cy + 5, 10, cy + 7), BROWN[1:], sep=False)
        cv.set(7, cy + 6, AMBER_D); cv.set(8, cy + 6, AMBER_D)
        cv.set(4, cy + 5, METAL[2]); cv.set(11, cy + 5, METAL[2]); cv.set(4, cy + 6, METAL[1]); cv.set(11, cy + 6, METAL[1])
    else:
        dy = 0
        cv.line(11, cy + 3, 12, cy + 4 + dy, METAL[1]); cv.set(13, cy + 4 + dy, METAL[2]); cv.set(13, cy + 5 + dy, METAL[3])
    if dim:
        for i, p in enumerate(cv.px):
            if p[3] and (i % 16, i // 16) not in cv.tag:
                cv.px[i] = mix(p, INDIGO[0], 0.45)
        for (x, y) in list(cv.tag):
            v = cv.get(x, y)[0]
            cv.px[y * 16 + x] = grey(max(150, v - 60))
    cv.outline(INK)
    for (x, y, c, a) in post:
        glow_px(cv, x, y, c, a)
    if mode == "charge":
        bolt = [(14, 0), (13, 1), (13, 2), (14, 2), (14, 3)]
        if f in (0, 2, 3):
            for i, (x, y) in enumerate(bolt):
                glow_px(cv, x, y, AMBER if f != 3 else LIME, 255)
            glow_px(cv, 13, 1, WHITE)
        else:
            for (x, y) in bolt:
                glow_px(cv, x, y, AMBER_D, 150)
    if mode == "alarm" and f == 0:
        for (x, y) in ((14, 1), (14, 2), (14, 3), (14, 5)):
            glow_px(cv, x, y, HOT)
        soft_spot(cv, 11, cy - 1, WHITE, 120)
    return cv


def gen_robot():
    sheet = Canvas(128, 80)
    acc = Canvas(128, 80)
    rows = [("idle", 4), ("work", 4), ("carry", 4), ("charge", 4)]
    for r, (mode, n) in enumerate(rows):
        for f in range(n):
            fr = robot_frame(mode, f)
            put_frame(sheet, fr, f, r, "robot")
            put_frame(acc, fr.accent_layer(), f, r)
    for f in range(2):
        fr = robot_frame("dead", f)
        put_frame(sheet, fr, f, 4, "robot"); put_frame(acc, fr.accent_layer(), f, 4)
        fr = robot_frame("alarm", f)
        put_frame(sheet, fr, 2 + f, 4, "robot"); put_frame(acc, fr.accent_layer(), 2 + f, 4)
    save("robot.png", sheet, 16, 16, edges="lr")
    save("robot_accent.png", acc, 16, 16, edges="lr")


# --------------------------------------------------------------------------
# Creatures
# --------------------------------------------------------------------------
CHITIN = [INDIGO[1], PALE[0], PALE[1], PALE[2], PALE[3]]
CHITIN_DK = [INDIGO[0], INDIGO[1], PALE[0], PALE[1], PALE[2]]
SAC = [INDIGO[1], TEAL[1], TEAL[2], TEAL[3], ICE[2]]


def leg_len(a, foot, extra):
    d = math.hypot(foot[0] - a[0], foot[1] - a[1]) + extra
    return d * 0.45, d * 0.55


def skitter_frame(kind, f):
    cv = Canvas(24, 24)
    body = Canvas(24, 24)
    dx = dy = 0.0
    ab_dy = 0.0
    near = [[20.8, 23], [17.6, 23], [8.0, 23], [4.0, 23]]
    far = [[19.2, 23], [15.8, 23], [10.2, 23], [6.0, 23]]
    fang = 0
    eyes = 2          # 2 bright, 1 dim, 0 dead
    on_back = False
    dark = False
    if kind == "walk":
        for i in range(4):
            for k, legs in enumerate((near, far)):
                ph = 2 * math.pi * (f / 4.0 + 0.5 * ((i + k) % 2))
                legs[i][0] += 1.4 * math.cos(ph)
                legs[i][1] -= max(0.0, math.sin(ph)) * 2.2
        dy = [0, -1, 0, -1][f] * 0.5
    elif kind == "idle":
        if f == 1:
            near[0] = [21.0, 18.5]
        if f == 3:
            near[1] = [18.5, 20.5]; far[0] = [20.0, 21.0]
        ab_dy = [0, 0, -1, 0][f]
    elif kind == "lunge":
        if f == 0:
            dy = 1.2; dx = -1.0
            for l in near + far:
                l[0] += 0.8 if l[0] > 12 else -0.8
        elif f == 1:
            dx, dy, fang = 1.5, -3.0, 1
            near[0] = [22.0, 12.5]; near[1] = [21.5, 17.0]; far[0] = [21.0, 10.5]; far[1] = [20.0, 15.0]
        elif f == 2:
            dx, dy, fang = 3.0, -4.5, 2
            near[0] = [22.5, 14.0]; near[1] = [22.0, 18.5]; far[0] = [22.0, 11.5]; far[1] = [21.0, 16.5]
            near[2] = [9.0, 20.5]; near[3] = [6.0, 18.5]; far[2] = [11.0, 21.0]; far[3] = [7.5, 20.0]
        else:
            dx, dy, fang = 1.5, 0.8, 1
            near[0][0] = 21.5; far[0][0] = 20.5
    elif kind == "death":
        if f == 0:
            dy = 1.0; eyes = 2
            for l in near + far:
                l[0] += 1.5 if l[0] > 12 else -1.5
                l[1] = 22
            near[0][1] = 18; far[3][1] = 19
        elif f == 1:
            dy = 1.6; eyes = 1
            near = [[19.0, 20.5], [17.0, 22.0], [9.0, 21.5], [6.0, 20.0]]
            far = [[18.0, 19.0], [15.5, 21.0], [10.5, 20.5], [7.5, 19.0]]
        else:
            on_back = True
            dy = 2.0
            eyes = 1 if f == 2 else 0
            dark = f == 3
            if f == 2:
                near = [[18.5, 12.5], [16.0, 11.0], [11.0, 11.0], [8.0, 12.5]]
                far = [[17.5, 13.5], [14.5, 12.0], [12.0, 12.5], [9.5, 13.5]]
            else:
                near = [[16.0, 15.0], [14.5, 14.2], [12.5, 14.2], [11.0, 15.0]]
                far = [[15.5, 15.5], [14.0, 15.0], [13.0, 15.0], [11.5, 15.5]]
    rmp = CHITIN_DK if dark else CHITIN
    root = (13.8 + dx, (18.6 if not on_back else 17.8) + dy)
    for i, ft in enumerate(far):
        a = (root[0] - 0.6 + i * 0.2, root[1])
        l1, l2 = leg_len(a, ft, (9.0 if i in (0, 3) else 6.5) if not on_back else 2.0)
        thin_leg(cv, a, ft, l1, l2, INDIGO[2], INDIGO[3])
    acx, acy = 9.2 + dx, 17.8 + dy + ab_dy
    ab = blob(body, acx, acy, 3.7, 3.0, rmp)
    blob(body, 14.4 + dx, 18.6 + dy, 2.6, 2.1, rmp)
    for (x, y) in ab:                      # dark dorsal stripe
        if abs((y + 0.5 - acy) + 1.2) < 0.6 and (x + 0.5 - acx) < 2.0 and (x % 2 == 0):
            body.set(x, y, rmp[1])
    # bioluminescent warning spots on the abdomen
    spots = [(-1.5, -1.2), (0.8, -0.2), (-2.2, 0.8), (1.0, -2.0)]
    for i, (sx, sy) in enumerate(spots):
        p = (int(acx + sx), int(acy + sy))
        if p in ab:
            body.set(p[0], p[1], (MAGENTA if i < 2 else MAGENTA_D) if eyes else PURPLE[1])
    # seam between the abdomen and the thorax
    for y in range(int(acy - 2), int(acy + 3)):
        if body.opaque(int(acx + 3.4), y):
            body.set(int(acx + 3.4), y, rmp[1])
    body.outline(INK)
    ex, ey = int(16 + dx), int(18 + dy)
    eyes_px = [(ex, ey), (ex + 1, ey + 1), (ex - 1, ey + 1), (ex + 1, ey - 1), (ex, ey + 2)]
    for i, (x, y) in enumerate(eyes_px):
        if body.opaque(x, y) and body.get(x, y) != INK:
            c = [INDIGO[1], LIME_D, LIME][eyes] if i < 3 else [INDIGO[1], MOSS[2], LIME_D][eyes]
            body.set(x, y, c)
    if eyes == 2:
        body.set(ex, ey, WHITE if kind != "death" else LIME)
    # fangs
    fx, fy = int(16 + dx), int(21 + dy)
    if not on_back:
        body.set(fx, fy, PALE[3]); body.set(fx + 1 + (fang > 0), fy - (fang > 1), PALE[2])
        if fang:
            body.set(fx + 2, fy + 1 - fang, PALE[3]); body.set(fx, fy + 1, PALE[3])
    cv.blit(body, 0, 0)
    for i, ft in enumerate(near):
        a = (root[0] - 1.0 + i * 0.5, root[1])
        l1, l2 = leg_len(a, ft, (9.0 if i in (0, 3) else 6.5) if not on_back else 2.0)
        thin_leg(cv, a, ft, l1, l2, rmp[3], WHITE if not dark else rmp[4])
    return cv


def webspinner_frame(kind, f):
    cv = Canvas(24, 24)
    body = Canvas(24, 24)
    near = [[21.2, 23], [18.6, 23], [10.5, 23], [6.0, 23]]
    far = [[20.0, 23], [17.4, 23], [12.0, 23], [8.0, 23]]
    acx, acy = 8.0, 9.6
    tdy = 0.0
    spit = -1
    if kind == "walk":
        for i in range(4):
            for k, legs in enumerate((near, far)):
                ph = 2 * math.pi * (f / 4.0 + 0.5 * ((i + k) % 2))
                legs[i][0] += 1.3 * math.cos(ph)
                legs[i][1] -= max(0.0, math.sin(ph)) * 2.4
        acy += [0, 0.6, 0, -0.6][f]
        tdy = [0, -0.5, 0, -0.5][f]
    else:
        spit = f
        acx, acy = [(9.0, 8.4), (10.6, 6.8), (10.9, 6.6), (9.4, 8.0)][f]
        tdy = [0.5, 1.0, 1.0, 0.5][f]
        near[0][0] = 22.0; far[0][0] = 20.8
    tcx, tcy = 15.0, 15.0 + tdy
    root = (15.0, 16.4 + tdy)
    for i, ft in enumerate(far):
        a = (root[0] - 0.6 + i * 0.2, root[1])
        l1, l2 = leg_len(a, ft, 6.0)
        thin_leg(cv, a, ft, l1, l2, INDIGO[2], INDIGO[3])
    # waist
    body.part(m_capsule((acx + 2.5, acy + 3.0), (tcx - 1.0, tcy - 0.5), 1.3), CHITIN[1:4], sep=False)
    ab = blob(body, acx, acy, 5.4, 4.9, SAC)
    # pale ribs over the bloated abdomen
    for (x, y) in ab:
        u = (x + 0.5 - acx) / 5.4
        if abs(((u + 1.0) * 2.6) % 1.0 - 0.5) < 0.12 and (y + 0.5 - acy) < 1.0 and (x + y) % 2 == 0:
            body.set(x, y, ICE[1])
    blob(body, tcx, tcy, 2.9, 2.4, CHITIN)
    blob(body, tcx + 2.6, tcy + 1.2, 1.7, 1.6, CHITIN)
    # spinnerets
    if spit < 0:
        for (x, y) in ((acx - 5.6, acy + 2.0), (acx - 4.8, acy + 3.6)):
            body.set(int(x), int(y), PALE[1])
    else:
        sx, sy = int(acx + 4.6), int(acy - 3.2)
        body.set(sx, sy, PALE[2]); body.set(sx + 1, sy - 1, PALE[3]); body.set(sx + 1, sy, PALE[1])
    body.outline(INK)
    # luminous sacs
    sacs = [(-2.4, -1.6, LIME), (0.6, 0.4, LIME), (-0.6, 2.2, CYAN), (2.6, -2.0, CYAN), (-3.6, 1.0, LIME_D)]
    pulse = f % 2
    for i, (sx, sy, c) in enumerate(sacs):
        x, y = int(acx + sx), int(acy + sy)
        if (x, y) in ab:
            body.set(x, y, WHITE if (i + pulse) % 3 == 0 else c)
            for ddx, ddy in ((1, 0), (0, 1), (-1, 0), (0, -1)):
                q = (x + ddx, y + ddy)
                if q in ab and body.get(*q) != INK:
                    body.set(q[0], q[1], mix(body.get(*q), c, 0.45))
    # eyes and fangs
    hx, hy = int(tcx + 3), int(tcy + 0.6)
    for i, (x, y) in enumerate(((hx, hy), (hx + 1, hy + 1), (hx - 1, hy), (hx, hy - 1))):
        if body.opaque(x, y) and body.get(x, y) != INK:
            body.set(x, y, AMBER if i < 2 else AMBER_D)
    body.set(hx, hy, WHITE)
    body.set(hx, hy + 3, PALE[3]); body.set(hx + 1, hy + 3, PALE[2])
    cv.blit(body, 0, 0)
    for i, ft in enumerate(near):
        a = (root[0] - 1.0 + i * 0.5, root[1])
        l1, l2 = leg_len(a, ft, 6.0)
        thin_leg(cv, a, ft, l1, l2, CHITIN[2], CHITIN[4])
    if spit >= 1:
        sx, sy = int(acx + 5.6), int(acy - 4.2)
        if spit == 1:
            for (x, y, c) in ((sx + 1, sy, WHITE), (sx + 2, sy, PALE[3]), (sx + 1, sy - 1, PALE[3]), (sx + 2, sy - 1, WHITE)):
                cv.set(x, y, c)
        elif spit == 2:
            for (x, y, c) in ((20, 2, WHITE), (21, 2, PALE[3]), (20, 1, PALE[3]), (21, 1, WHITE), (21, 3, PALE[2])):
                cv.set(x, y, c)
            for x in range(sx + 1, 20):
                cv.blend(x, sy - 1 + (x % 2), alpha(PALE[3], 170))
        else:
            cv.blend(sx + 1, sy, alpha(PALE[3], 150)); cv.blend(sx + 2, sy + 1, alpha(PALE[3], 90))
    return cv


def web_frame(kind, f):
    cv = Canvas(24, 24)
    if kind == "shot":
        for i in range(7):                       # trailing strands
            x = 9 - i
            cv.blend(x, 12 + [0, 1, 0, -1][(i + f) % 4], alpha(PALE[3], 200 - i * 24))
            if i % 2 == 0:
                cv.blend(x, 10 + [0, -1, 0, 1][(i + f) % 4] - (i // 3), alpha(PALE[2], 150 - i * 18))
                cv.blend(x, 14 + [0, 1, 0, -1][(i + f + 1) % 4] + (i // 3), alpha(PALE[2], 150 - i * 18))
        core = Canvas(24, 24)
        blob(core, 12.5, 12.5, 2.9, 2.9, [PALE[1], PALE[2], PALE[3], WHITE])
        arms = [((1, 0), (0, 1)), ((1, 1), (-1, 1)), ((0, 1), (1, 0)), ((1, -1), (1, 1))][f]
        for (ax, ay) in arms:
            for s in (-1, 1):
                for k in (3, 4):
                    core.set(12 + ax * k * s, 12 + ay * k * s, PALE[3] if k == 3 else PALE[1])
        core.outline(INDIGO[1])
        cv.blit(core, 0, 0)
        cv.set(12 + (f % 2), 11 + (f // 2), TEAL[3]); cv.set(11, 13 - (f % 2), ICE[1])
        return cv
    R = [3.5, 6.5, 9.0, 10.0][f]
    a = [255, 240, 200, 110][f]
    for k in range(8):
        ang = k * math.pi / 4 + 0.2
        for r10 in range(0, int(R * 10), 5):
            r = r10 / 10.0
            x, y = int(12 + math.cos(ang) * r), int(12 + math.sin(ang) * r * 0.95 + (r * r * 0.012 if f == 3 else 0))
            cv.px[y * 24 + x] = alpha(PALE[3] if r < R * 0.6 else PALE[2], a)
    for ring in ((0.45, 0), (0.8, 1)):
        rr = R * ring[0]
        if rr < 2.5 or (ring[1] and f == 0):
            continue
        for k in range(8):
            a0 = k * math.pi / 4 + 0.2
            a1 = a0 + math.pi / 4
            p0 = (12 + math.cos(a0) * rr, 12 + math.sin(a0) * rr)
            p1 = (12 + math.cos(a1) * rr, 12 + math.sin(a1) * rr)
            mx, my = (p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2
            mx, my = 12 + (mx - 12) * 0.86, 12 + (my - 12) * 0.86
            for (q0, q1) in ((p0, (mx, my)), ((mx, my), p1)):
                for (x, y) in line_pts(q0[0], q0[1], q1[0], q1[1]):
                    if 0 < x < 23 and 0 < y < 23 and cv.get(x, y)[3] == 0:
                        cv.px[y * 24 + x] = alpha(PALE[2], a * 3 // 4)
    if f < 2:
        blob(cv, 12.5, 12.5, 2.4 - f, 2.4 - f, [PALE[2], PALE[3], WHITE])
    return cv


def leech_frame(kind, f):
    cv = Canvas(24, 24)
    pts = []
    if kind == "swim":
        for i in range(49):
            s = i / 48.0
            x = 2.8 + 15.4 * s
            y = 12.0 + (3.0 * (1 - 0.65 * s)) * math.sin(2 * math.pi * (1.15 * s - f / 4.0))
            r = 0.75 + 1.75 * math.sin(min(1.0, s * 1.7) * math.pi / 2)
            pts.append((x, y, r, s))
        hx, hy = pts[-1][0], pts[-1][1]
        hdir = (1.0, 0.0)
        pulse = 0
    else:
        pulse = [0, 1, 2, 1][f]
        for i in range(49):
            s = i / 48.0
            ang = math.radians(250 - 290 * s)
            R = 5.6 - 1.2 * (1 - s)
            x = 11.0 + R * math.cos(ang)
            y = 12.0 - R * math.sin(ang)
            r = (0.8 + 1.7 * math.sin(min(1.0, s * 1.7) * math.pi / 2)) * (1.0 + 0.1 * pulse * math.sin(s * math.pi))
            pts.append((x, y, r, s))
        hx, hy = pts[-1][0], pts[-1][1]
        hdir = (1.0, 0.0)
    body = Canvas(24, 24)
    full = set()
    for (x, y, r, s) in pts:
        full |= m_disc((x, y), r)
    body.part(full, [INDIGO[2], TEAL[1], TEAL[2], ICE[1]], sep=False)
    # glistening rim and the gut seen through the skin
    for i, (x, y, r, s) in enumerate(pts):
        if i % 4 == 0 and s > 0.12:
            bead = ((i // 4) + (f if kind == "swim" else f * 2)) % 3 == 0
            c = (MAGENTA if pulse else MAGENTA_D) if bead else PURPLE[1]
            if pulse == 2 and bead:
                c = HOT
            if (int(x), int(y)) in full:
                body.set(int(x), int(y), c)
    # ring mouth
    mx, my = hx + 1.2, hy
    lip = m_ellipse(mx - 0.3, my, 2.3, 3.3)
    body.part(lip, [FLESH[0], FLESH[1], FLESH[2]], sep=False)
    ring = m_ellipse(mx + 0.2, my, 1.5, 2.4)
    for (x, y) in ring:
        body.set(x, y, RED[0])
    ix, iy = int(mx + 0.2), int(my)
    for (ddx, ddy) in ((0, -2), (0, 2), (1, -1), (1, 1), (-1, -1), (-1, 1)):
        if (ix + ddx, iy + ddy) in ring:
            body.set(ix + ddx, iy + ddy, WHITE if ddx >= 0 else PALE[2])
    body.set(ix, iy, HOT if pulse else RED[1])
    body.outline(INK)
    # translucency: let the background show faintly through the flesh
    for i, p in enumerate(body.px):
        if p in (TEAL[1], TEAL[2]):
            body.px[i] = alpha(p, 215)
        elif p == INDIGO[2]:
            body.px[i] = alpha(p, 230)
    cv.blit(body, 0, 0)
    if kind != "swim" and pulse:
        for (x, y) in ((int(mx) + 2, int(my) - 2), (int(mx) + 2, int(my) + 2)):
            if cv.inb(x, y):
                cv.blend(x, y, alpha(HOT, 90 * pulse))
    return cv


def maw_frame(H, opn, lean=0.0, debris=0, mound=True, feeler=None, dust=0):
    cv = Canvas(24, 24)
    top = 24 - H
    K = 9
    y0 = top + K
    spikes = []

    def shear(x, y):
        return (x + lean * (23 - y) / 22.0, y)
    # segmented body
    for y in range(max(y0, 0), 24):
        kk = y - y0
        cx = 12.0 + lean * (23 - y) / 22.0
        w_out = 4.6 + min(0.8, kk * 0.1) + [0.2, 0.7, 0.4, 0.0][kk % 4]
        if kk % 4 == 1 and y < 21:
            spikes.append((cx, y, w_out))
        for x in range(0, 24):
            d = x + 0.5 - cx
            if abs(d) > w_out:
                continue
            u = d / w_out
            c = PALE[3] if u < -0.55 else PALE[2] if u < 0.05 else PALE[1] if u < 0.6 else PALE[0]
            if kk % 4 == 3:
                c = PALE[0] if u < 0.05 else INDIGO[2]
            elif kk % 8 == 1 and 0.15 < u < 0.5:
                c = AMBER
            elif kk % 8 == 5 and -0.6 < u < -0.3:
                c = AMBER_D
            elif hash2(x, kk, 4) > 0.86:
                c = FLESH[3] if u < 0 else PALE[0]
            cv.set(x, y, c)
    for (cx, y, w) in spikes:
        cv.set(int(cx - w - 0.6), y - 1, PALE[3]); cv.set(int(cx - w - 1.4), y - 2, WHITE)
        cv.set(int(cx + w + 0.6), y - 1, PALE[1]); cv.set(int(cx + w + 1.4), y - 2, PALE[2])
    if H >= 3:
        o = opn
        clipg = lambda pts: {p for p in pts if p[1] <= 23}
        # throat
        if o > 0.1:
            th = m_poly([shear(*p) for p in ((12 - 4.6, y0 + 0.5), (12 - 3.0 - 3.6 * o, y0 - 5), (12 - 1.5 - 4.6 * o, top + 1.5),
                                             (12 + 1.5 + 4.6 * o, top + 1.5), (12 + 3.0 + 3.6 * o, y0 - 5), (12 + 4.6, y0 + 0.5))])
            for (x, y) in clipg(th):
                dd = abs(x + 0.5 - (12 + lean * (23 - y) / 22.0))
                depth = (y - top) / float(K)
                c = RED[0] if depth < 0.45 else FLESH[0]
                if depth > 0.55 and dd < 2.2:
                    c = MAGENTA_D
                if depth > 0.75 and dd < 1.2:
                    c = MAGENTA
                cv.set(x, y, c)
            # back petal
            bp = m_poly([shear(*p) for p in ((12 - 1.6, y0 - 1), (12 - 1.0, top + 4 - 2 * o), (12, top + 2.5 - 2 * o), (12 + 1.0, top + 4 - 2 * o), (12 + 1.6, y0 - 1))])
            for (x, y) in clipg(bp):
                cv.set(x, y, PALE[0] if (x + y) % 2 else INDIGO[3])
        for side in (-1, 1):
            pts = [(12 + side * 5.0, y0 + 0.5), (12 + side * (5.2 + 2.4 * o), y0 - 4.5), (12 + side * (1.2 + 5.2 * o), top),
                   (12 + side * (0.2 + 3.4 * o), y0 - 4.0), (12 + side * (0.0 + 1.4 * o), y0 + 0.5)]
            pm = clipg(m_poly([shear(*p) for p in pts]))
            cv.part(pm, [PALE[1], PALE[2], PALE[3]] if side < 0 else [PALE[0], PALE[1], PALE[2]], sep=False)
            # teeth along the inner edge
            for (x, y) in sorted(pm):
                inner = (x - side, y)
                if inner not in pm and (y - top) % 2 == 0 and y < y0 and o > 0.1:
                    cv.set(inner[0], inner[1], WHITE)
                    if (y - top) % 4 == 0:
                        cv.set(inner[0] - side, inner[1] + 1, PALE[3])
            for (x, y) in pm:
                if (y - top) in (5, 6) and (x + y) % 3 == 0:
                    cv.set(x, y, FLESH[2])
        if o <= 0.1:
            for y in range(max(top + 1, 0), min(y0, 24)):
                x = int(12 + lean * (23 - y) / 22.0)
                cv.set(x - (y % 2), y, WHITE if y % 2 == 0 else INDIGO[2])
    if mound:
        for (x, y) in m_ellipse(12.0, 23.6, 7.5 if H > 2 else 5.0, 2.0):
            if y <= 23 and (H <= 2 or abs(x + 0.5 - 12) > 4.5 or y == 23):
                cv.set(x, y, BROWN[1] if (x + y) % 3 else BROWN[2])
        cv.set(7, 22, BROWN[2]); cv.set(16, 22, BROWN[0])
    cv.outline(INK)
    if feeler is not None:
        fx = [(12, 20), (12, 19), (12 + feeler, 18), (12 + feeler, 17), (12 + 2 * feeler, 16)]
        for i, (x, y) in enumerate(fx):
            cv.set(x, y, PALE[1] if i < 4 else AMBER)
        cv.set(13 - feeler, 21, PALE[0])
    for i in range(debris * 3):
        ang = math.radians(200 + 140 * hash2(i, debris, 3))
        r = 7 + debris * 2.0 + 4 * hash2(i, 7, debris)
        x, y = int(12 + math.cos(ang) * r), int(23 + math.sin(ang) * r * 0.8)
        if 1 <= x <= 22 and 1 <= y <= 22 and cv.get(x, y)[3] == 0:
            cv.set(x, y, BROWN[2] if i % 2 else BROWN[1])
    for i in range(dust * 3):
        x = int(5 + 14 * hash2(i, 11, dust)); y = int(22 - dust - 3 * hash2(i, 5, dust))
        if cv.get(x, y)[3] == 0:
            cv.blend(x, y, alpha(BROWN[3], 170 - dust * 30))
    return cv


def puff_frame(f):
    cv = Canvas(24, 24)
    if f == 0:
        blob(cv, 12, 12, 3.2, 3.2, [LIME_D, LIME, WHITE])
        cv.outline(MOSS[0])
    elif f in (1, 2):
        R = 5.5 if f == 1 else 8.0
        for k in range(10):
            ang = k * math.pi / 5 + 0.3
            rr = R * (0.75 + 0.25 * hash2(k, 3, 1))
            x, y = 12 + math.cos(ang) * rr, 12 + math.sin(ang) * rr + (1.5 if f == 2 else 0)
            blob(cv, x, y, 1.5 if f == 1 else 1.1, 1.5 if f == 1 else 1.1, [TEAL[1], LIME_D, LIME])
        if f == 1:
            blob(cv, 12, 12, 3.6, 3.4, [MOSS[1], LIME_D, LIME, WHITE])
            cv.outline(MOSS[0])
        else:
            for (x, y) in m_ellipse(12, 12, 4.0, 3.6):
                if (x + y) % 2 == 0:
                    cv.set(x, y, alpha(LIME_D, 200))
            cv.outline(MOSS[0])
    else:
        a = [210, 140, 70][f - 3]
        for k in range(12):
            h1, h2 = hash2(k, 1, 9), hash2(k, 2, 9)
            x = int(4 + 16 * h1 + math.sin(k + f) * 1.2)
            y = int(17 - 12 * h2 - (f - 3) * 2.5)
            if 1 <= y <= 22:
                c = LIME if k % 3 == 0 else MOSS[3] if k % 3 == 1 else TEAL[3]
                cv.blend(x, y, alpha(c, a))
                if k % 4 == 0 and f == 3:
                    cv.blend(x + 1, y, alpha(c, a // 2)); cv.blend(x, y + 1, alpha(c, a // 2))
    return cv


def m_rot_ellipse(cx, cy, rx, ry, deg):
    r = math.radians(deg)
    c, s = math.cos(r), math.sin(r)
    pts = set()
    R = int(max(rx, ry)) + 2
    for y in range(int(cy) - R, int(cy) + R + 1):
        for x in range(int(cx) - R, int(cx) + R + 1):
            dx, dy = x + 0.5 - cx, y + 0.5 - cy
            u, v = dx * c + dy * s, -dx * s + dy * c
            if (u / rx) ** 2 + (v / ry) ** 2 <= 1.0:
                pts.add((x, y))
    return pts


def driftmoth_frame(f):
    cv = Canvas(24, 24)
    th = [62, 24, -30, 24][f]
    bob = [0, -1, 0, 1][f]
    root = (11.5, 12.5 + bob)

    def wing(off, deg, rmp, edge):
        r = math.radians(deg)
        ux, uy = -math.cos(r) * 0.45, -math.sin(r)
        n = math.hypot(ux, uy)
        ux, uy = ux / n, uy / n
        ang = math.degrees(math.atan2(uy, ux))
        fore = m_rot_ellipse(root[0] + off + ux * 4.6, root[1] + uy * 4.6, 5.2, 3.0, ang)
        hind = m_rot_ellipse(root[0] + off - 2.6 + ux * 2.2, root[1] + 0.6 + uy * 2.6, 3.2, 2.2, ang + 28)
        w = Canvas(24, 24)
        w.part(hind, rmp, sep=False)
        w.part(fore, rmp, sep=True)
        w.outline(edge)
        return w, fore, hind, (ux, uy)
    wf, _, _, _ = wing(-1.2, th * 0.8 + 8, [INDIGO[2], TEAL[1], TEAL[2]], INDIGO[0])
    cv.blit(wf, 0, 0)
    body = Canvas(24, 24)
    body.part(m_capsule((9.4, 13.6 + bob), (13.4, 12.6 + bob), 1.25), [BONE[1], BONE[2], BONE[3]], sep=False)
    body.part(m_disc((14.6, 12.3 + bob), 1.35), [BONE[1], BONE[2], BONE[3]], sep=False)
    body.set(9, int(13 + bob), BONE[0]); body.set(11, int(13 + bob), BONE[1])
    body.outline(INDIGO[0])
    body.set(15, int(12 + bob), INK)
    cv.blit(body, 0, 0)
    for i in range(3):                                    # antennae
        cv.set(16 + i, int(11 + bob) - i - (1 if i == 2 else 0), BONE[2] if i < 2 else AMBER)
        cv.set(15 + i, int(10 + bob) - i, BONE[1] if i < 2 else AMBER_D)
    wn, fore, hind, (ux, uy) = wing(0.0, th, [TEAL[2], TEAL[3], ICE[2]], INDIGO[1])
    cv.blit(wn, 0, 0)
    spots = [(root[0] + ux * 5.6, root[1] + uy * 5.6, CYAN, fore), (root[0] + ux * 2.6, root[1] + uy * 2.4, MAGENTA, fore),
             (root[0] - 2.8 + ux * 2.8, root[1] + 0.8 + uy * 3.0, LIME, hind)]
    for (sx, sy, c, m) in spots:
        p = (int(sx), int(sy))
        if p in m:
            for ddx, ddy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                q = (p[0] + ddx, p[1] + ddy)
                if q in m:
                    cv.set(q[0], q[1], mix(cv.get(*q), c, 0.6))
                elif cv.inb(*q) and cv.get(*q)[3] == 0:
                    cv.blend(q[0], q[1], alpha(c, 80))
            cv.set(p[0], p[1], WHITE if c == CYAN else c)
    return cv


def puffback_frame(kind, f):
    cv = Canvas(24, 24)
    bob = [0, -1, 0, -1][f] if kind == "walk" else 0
    bcx, bcy = 10.8, 15.6 + bob
    # legs (far then near)
    lx = [6.2, 9.4, 12.8, 15.8]
    legs = Canvas(24, 24)
    for i, x in enumerate(lx):
        farleg = i % 2 == 1
        lift = sh = 0
        if kind == "walk":
            ph = (f + (0 if i in (0, 3) else 2)) % 4
            lift = 1 if ph == 1 else 0
            sh = [1, 0, -1, 0][ph]
        x0 = int(x + sh)
        legs.part(m_rect(x0, 19, x0 + 1, 22 - lift), [INDIGO[2], PALE[0], PALE[1]] if farleg else [PALE[0], PALE[1], PALE[2]], sep=False)
    legs.outline(INK)
    cv.blit(legs, 0, 0)
    body = Canvas(24, 24)
    body.part(m_capsule((3.6, 16.5 + bob), (4.6, 16.0 + bob), 1.0), [PALE[0], PALE[1], PALE[2]], sep=False)   # tail
    bm = blob(body, bcx, bcy, 7.0, 5.2, [INDIGO[2], PALE[0], PALE[1], PALE[2], BONE[3]])
    # mossy back
    for (x, y) in bm:
        dxn, dyn = (x + 0.5 - bcx) / 7.0, (y + 0.5 - bcy) / 5.2
        edge = dyn + 0.25 * math.sin(x * 1.3) + 0.12 * dxn
        if edge < -0.05:
            lgt = -(dxn * 0.6 + dyn * 0.75)
            c = MOSS[3] if lgt > 0.75 else MOSS[2] if lgt > 0.35 else MOSS[1]
            if hash2(x, y, 5) > 0.82:
                c = MOSS[3] if c != MOSS[3] else LIME_D
            if edge > -0.2 and (x + y) % 2:
                c = MOSS[0]
            body.set(x, y, c)
    # head
    if kind == "walk":
        hcx, hcy = 18.4, 17.0 + bob + ([0, 0, 1, 0][f])
    else:
        hcx, hcy = 18.8, 19.6 + [0, 0.6, 0, 0.6][f]
    blob(body, hcx, hcy, 2.6, 2.4, [PALE[0], PALE[1], PALE[2], BONE[3]])
    body.set(int(hcx - 1.6), int(hcy - 2.6), PALE[1]); body.set(int(hcx - 0.6), int(hcy - 2.9), PALE[2])   # ear nubs
    body.outline(INK)
    body.set(int(hcx + 0.9), int(hcy - 0.6), INK); body.set(int(hcx + 0.9), int(hcy - 1.4), WHITE)
    body.set(int(hcx + 2.0), int(hcy + 0.8), FLESH[2])
    if kind == "graze" and f % 2 == 1:
        body.set(int(hcx + 1.2), int(hcy + 2.0), LIME_D)
    cv.blit(body, 0, 0)
    if kind == "graze":
        for (x, y, c) in ((21, 22, MOSS[2]), (22, 21 + f % 2, LIME_D), (20, 22, MOSS[1])):
            if cv.get(x, y)[3] == 0:
                cv.set(x, y, c)
    # plants growing on the back
    sway = [0, 1, 0, -1][f] if kind == "walk" else [0, 0, 1, 0][f]
    top = lambda x: min(y for (xx, y) in bm if xx == x)
    for (px_, hgt, kindp) in ((6, 3, "sprout"), (9, 5, "flower"), (12, 4, "fern"), (14, 2, "cap")):
        ty = top(px_)
        for i in range(hgt):
            cv.set(px_ + (sway if i >= hgt - 2 and hgt > 2 else 0), ty - 1 - i, MOSS[2] if i < hgt - 1 else LIME_D)
        tx, tyy = px_ + (sway if hgt > 2 else 0), ty - hgt
        if kindp == "sprout":
            cv.set(tx - 1, tyy, LIME); cv.set(tx + 1, tyy - 1, LIME)
        elif kindp == "flower":
            for ddx, ddy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                cv.set(tx + ddx, tyy + ddy, MAGENTA)
            cv.set(tx, tyy, AMBER); cv.set(tx - 1, tyy + 2, LIME)
        elif kindp == "fern":
            cv.set(tx - 1, tyy + 1, LIME_D); cv.set(tx + 1, tyy, LIME); cv.set(tx + 1, tyy + 2, LIME_D); cv.set(tx - 1, tyy - 1, LIME)
        else:
            cv.set(tx - 1, tyy, RUST[2]); cv.set(tx, tyy, RUST[3]); cv.set(tx + 1, tyy, RUST[1])
    return cv


def gen_creatures():
    FW = FH = 24
    sheet = Canvas(192, 288)

    def put(fr, c, r):
        put_frame(sheet, fr, c, r, "creatures")
    for f in range(4):
        put(driftmoth_frame(f), f, 0)
        put(puffback_frame("walk", f), f, 1); put(puffback_frame("graze", f), 4 + f, 1)
        put(skitter_frame("walk", f), f, 2); put(skitter_frame("lunge", f), 4 + f, 2)
        put(skitter_frame("death", f), f, 3); put(skitter_frame("idle", f), 4 + f, 3)
        put(webspinner_frame("walk", f), f, 4); put(webspinner_frame("spit", f), 4 + f, 4)
        put(web_frame("shot", f), f, 5); put(web_frame("splat", f), 4 + f, 5)
        put(leech_frame("swim", f), f, 6); put(leech_frame("latched", f), 4 + f, 6)
    put(maw_frame(0, 0, feeler=-1), 0, 7)
    put(maw_frame(0, 0, feeler=1), 1, 7)
    for i, (H, o) in enumerate(((5, 0.0), (10, 0.25), (15, 0.55), (20, 0.85), (23, 1.0), (22, 1.0))):
        put(maw_frame(H, o, debris=[1, 2, 3, 3, 2, 0][i]), 2 + i, 7)
    for i, (o, ln) in enumerate(((1.0, 1.0), (0.5, 2.5), (0.0, 3.2), (0.5, 2.0))):
        put(maw_frame(22, o, lean=ln), i, 8)
    for i, (H, o) in enumerate(((17, 0.3), (11, 0.0), (5, 0.0), (0, 0.0))):
        put(maw_frame(H, o, dust=i + 1 if i >= 2 else 0), 4 + i, 8)
    for f in range(6):
        put(puff_frame(f), f, 9)
    firsts = [skitter_frame("walk", 0), webspinner_frame("walk", 0), leech_frame("swim", 0),
              maw_frame(22, 1.0, lean=1.0), puffback_frame("walk", 0), driftmoth_frame(0)]
    for i, fr in enumerate(firsts):
        fr.silhouette(WHITE)
        put(fr, i, 10)
    save("creatures.png", sheet, FW, FH, edges="")
    for r in range(12):
        for c in range(8):
            if r in (5, 7, 8, 9, 10):
                continue
            for i in range(24):
                if sheet.get(c * 24, r * 24 + i)[3] or sheet.get(c * 24 + 23, r * 24 + i)[3] or sheet.get(c * 24 + i, r * 24)[3]:
                    WARN.append("creatures: frame (col %d,row %d) touches cell edge" % (c, r))
                    break


BROOD = [INDIGO[1], FLESH[0], PALE[0], PALE[1], PALE[2], PALE[3]]
BROOD_LEG = [INDIGO[2], PALE[0], PALE[2]]
BROOD_LEG_FAR = [INDIGO[0], INDIGO[2], INDIGO[3]]


def brood_frame(kind, f):
    cv = Canvas(64, 48)
    bdx = bdy = 0.0
    near = [[58.5, 46], [52.0, 46], [33.0, 46], [24.0, 46]]
    far = [[55.0, 46], [48.5, 46], [36.5, 46], [28.0, 46]]
    ext_n = [15.0, 11.0, 11.0, 16.0]
    eyes = 2
    mouth = 0
    ary = 13.0
    arx = 16.0
    slit = 0
    eggs = []
    dead = False
    pulse = f % 2
    if kind == "walk":
        for i in range(4):
            for k, legs in enumerate((near, far)):
                ph = 2 * math.pi * (f / 4.0 + 0.5 * ((i + k) % 2))
                legs[i][0] += 2.2 * math.cos(ph)
                legs[i][1] -= max(0.0, math.sin(ph)) * 4.5
        bdy = [0, -1, 0, -1][f]
    elif kind == "spawn":
        bdy = [0, -1, -1, 0][f]
        ary = [14.0, 13.5, 13.0, 13.0][f]
        slit = [1, 3, 3, 2][f]
        near[3][0] = 22.0; far[3][0] = 26.0; near[2][0] = 35.0
        eggs = [[], [(15.5, 37.5, 0)], [(15.0, 41.5, 0), (17.5, 37.0, 0)], [(14.0, 44.5, 1), (18.0, 41.5, 0), (19.5, 44.6, 1)]][f]
        mouth = 1 if f in (1, 2) else 0
    elif kind == "hurt":
        bdx, bdy = -2.0, -1.5
        near[0] = [61.0, 28.0]; near[1] = [57.0, 37.0]; far[0] = [59.0, 24.0]; far[1] = [54.0, 34.0]
        mouth = 2
        eyes = 3
    elif kind == "death":
        dead = True
        if f == 0:
            bdy = 7.0
            eyes = 1
            mouth = 2
            near = [[61.0, 44.0], [55.0, 46.0], [31.0, 46.0], [20.0, 44.0]]
            far = [[58.0, 40.0], [50.0, 45.0], [38.0, 45.0], [27.0, 41.0]]
            ext_n = [9.0, 6.0, 6.0, 9.0]
        else:
            bdy = 10.5
            ary = 9.5
            arx = 17.0
            eyes = 0
            mouth = 1
            near = [[52.0, 30.0], [48.0, 27.0], [40.0, 27.0], [36.0, 31.0]]
            far = [[50.0, 33.0], [46.0, 30.0], [42.0, 30.0], [38.0, 33.0]]
            ext_n = [3.0, 2.5, 2.5, 3.0]
    acx, acy = 21.0 + bdx, 22.0 + bdy + (13.0 - ary)
    tdy = min(bdy, 8.8)
    flen = 1.0 if not dead else (0.55 if f == 0 else 0.3)
    tcx, tcy = 43.0 + bdx, 30.5 + tdy
    hcx, hcy = 51.5 + bdx, 33.0 + min(bdy, 7.5)
    root = (43.0 + bdx, 32.5 + tdy)
    up_legs = dead and f == 1
    for i, ft in enumerate(far):
        a = (root[0] - 3.0 + i * 1.6, root[1] - (3.5 if up_legs else 0))
        l1, l2 = leg_len(a, ft, ext_n[i] * 0.9)
        thick_leg(cv, a, ft, l1, l2, BROOD_LEG_FAR, 1.0)
    # abdomen
    ab = blob(cv, acx, acy, arx, ary, BROOD)
    for (x, y) in ab:
        dxn, dyn = (x + 0.5 - acx) / arx, (y + 0.5 - acy) / ary
        band = math.sin(dxn * 9.0 + dyn * 2.5)
        if band > 0.72 and dyn < 0.35 and (x + y) % 2 == 0:
            cv.set(x, y, MAGENTA_D if not dead or f == 0 else PURPLE[1])
        elif band < -0.86 and dyn < 0.1 and (x + 2 * y) % 3 == 0:
            cv.set(x, y, INDIGO[2])
        elif dyn > 0.55 and hash2(x, y, 2) > 0.8:
            cv.set(x, y, FLESH[1])
    # waist, thorax, head
    cv.part(m_capsule((acx + arx - 3, acy + 5.0), (tcx - 4, tcy), 3.2), BROOD[2:5], sep=False)
    th = blob(cv, tcx, tcy, 8.0, 6.3, BROOD)
    for (x, y) in th:
        if abs(((x - tcx) * 0.8) % 3.0) < 0.6 and (y + 0.5 - tcy) < -1.5 and (x + y) % 2:
            cv.set(x, y, INDIGO[2])
    hd = blob(cv, hcx, hcy, 5.0, 4.4, BROOD)
    # chelicerae and fangs
    spread = [0, 1.2, 2.6][mouth]
    for k, s in enumerate((-1, 1)):
        bx = hcx + 2.0 + s * (1.2 + spread * 0.5)
        fang = m_poly([(bx - 1.5, hcy + 2.4), (bx + 1.7, hcy + 2.4), (bx + 1.2 + s * spread * 0.4, hcy + 2.4 + 4.0 * flen),
                       (bx + s * spread * 0.9 - 0.2, hcy + 2.4 + 7.0 * flen)])
        cv.part(fang, [PALE[1], PALE[3], WHITE] if k else [PALE[0], PALE[1], PALE[2]], sep=True)
    if mouth:
        for (x, y) in m_ellipse(hcx + 2.0, hcy + 3.6, 1.2 + spread * 0.5, 1.4):
            if (x, y) in hd or cv.get(x, y)[3] == 0:
                cv.set(x, y, RED[0])
    # egg sacs
    sacs = [(-9, -4, 3.3), (-2, -8, 2.9), (5, -4, 3.1), (-12, 4, 2.9), (-4, 2, 3.5), (5, 5, 2.9), (-8, 9.5, 2.7), (1, 10.5, 2.5), (10, -9, 2.2)]
    sac_px = []
    for i, (sx, sy, r) in enumerate(sacs):
        if dead and f == 1 and i % 3 == 0:
            continue
        x, y = acx + sx * arx / 16.0, acy + sy * ary / 13.0
        rr = r + (0.35 if (kind == "spawn" and (i + f) % 2 == 0) else 0)
        cv.part(m_disc((x, y), rr + 0.9), [FLESH[0], FLESH[1], FLESH[2]], sep=False)
        blob(cv, x, y, rr, rr, [FLESH[2], FLESH[3], PALE[3], WHITE] if not (dead and f == 1) else [FLESH[0], FLESH[1], FLESH[2], PALE[1]])
        sac_px.append((int(x), int(y), i))
    # birth slit
    if slit:
        sxc, syc = acx - 5.0, acy + ary - 1.2
        for (x, y) in m_ellipse(sxc, syc, 3.2 + slit * 0.5, 0.6 + slit * 0.75):
            cv.set(x, y, RED[0] if slit > 1 else MAGENTA_D)
        for (x, y) in m_ellipse(sxc, syc, 3.2 + slit * 0.5 + 1.0, 0.6 + slit * 0.75 + 1.0):
            if cv.get(x, y) not in (RED[0], MAGENTA_D) and (x, y) in ab:
                cv.set(x, y, MAGENTA if (x + y) % 2 else FLESH[2])
    for (ex_, ey_, landed) in eggs:
        cv.part(m_ellipse(ex_, ey_, 2.0 + landed * 0.3, 2.5 - landed * 0.5), [FLESH[2], PALE[3], WHITE], sep=False)
    for i, ft in enumerate(near):
        a = (root[0] - 4.0 + i * 2.6, root[1] - (3.5 if up_legs else 0))
        l1, l2 = leg_len(a, ft, ext_n[i])
        j, e = thick_leg(cv, a, ft, l1, l2, BROOD_LEG, 1.25)
        cv.set(int(j[0]), int(j[1]) - 1, WHITE)
        # spines
        for t in (0.35, 0.65):
            sxp, syp = j[0] + (e[0] - j[0]) * t, j[1] + (e[1] - j[1]) * t
            cv.set(int(sxp - 1.6 if e[0] > j[0] else sxp + 1.6), int(syp), PALE[0])
    if kind == "death":
        for (x, y) in m_ellipse(acx + 2, 46.6, 15 + f * 7, 1.2):
            if 1 < x < 62 and y <= 46 and cv.get(x, y)[3] == 0 and (f == 1 or (x + y) % 2):
                cv.set(x, y, LIME_D if (x + y) % 3 else MOSS[2])
    cv.outline(INK)
    # ---- after the outline: eyes, glows, drips ------------------------------
    for (x, y, i) in sac_px:
        cv.set(x, y + 1, INDIGO[1]); cv.set(x + 1, y + 1, PURPLE[0])
        cv.set(x - 1, y - 1, WHITE)
        if not dead and (i + pulse) % 3 == 0:
            cv.set(x + 1, y, LIME); cv.set(x, y, LIME_D)
    eye_c = [[INDIGO[1], INDIGO[0]], [RED[0], RED[1]], [RED[2], HOT], [HOT, WHITE]][eyes]
    big = [(hcx + 1.5, hcy - 1.5), (hcx - 1.2, hcy - 2.4)]
    small = [(hcx + 3.2, hcy + 0.2), (hcx + 0.2, hcy + 0.6), (hcx - 2.6, hcy - 0.4), (hcx + 2.6, hcy - 2.8), (hcx - 0.2, hcy - 3.6), (hcx - 3.4, hcy - 2.4)]
    for (x, y) in big:
        x, y = int(x), int(y)
        cv.rect(x, y, x + 1, y + 1, eye_c[0]); cv.set(x + 1, y, eye_c[1])
        if eyes >= 2:
            cv.set(x, y, WHITE if eyes == 3 else AMBER)
    for i, (x, y) in enumerate(small):
        if (int(x), int(y)) in hd:
            cv.set(int(x), int(y), eye_c[1] if (i + pulse) % 2 == 0 else eye_c[0])
    if not dead:
        dx_ = int(hcx + 2.0)
        for k in range(2 + mouth):
            cv.blend(dx_ + (k % 2), int(hcy + 10.0 + k), alpha(LIME, 200 - 40 * k))
    if slit >= 2:
        sxc, syc = int(acx - 5.0), int(acy + ary)
        for k in range(3):
            cv.blend(sxc - 3 + k * 3, syc + 1 + (k + f) % 3, alpha(LIME, 210))
    if kind == "hurt":
        for (x, y) in ((hcx + 8, hcy - 6), (hcx + 10, hcy - 2), (acx - 3, acy - ary - 3), (acx + 9, acy - ary - 1)):
            if 0 < x < 63 and y > 0:
                cv.blend(int(x), int(y), alpha(LIME, 220)); cv.blend(int(x) + 1, int(y) - 1, alpha(LIME_D, 160))
    return cv


def gen_brood():
    sheet = Canvas(256, 144)
    for f in range(4):
        put_frame(sheet, brood_frame("walk", f), f, 0, "brood")
        put_frame(sheet, brood_frame("spawn", f), f, 1, "brood")
    fl = brood_frame("hurt", 0)
    fl.silhouette(WHITE)
    put_frame(sheet, fl, 0, 2, "brood")
    put_frame(sheet, brood_frame("hurt", 1), 1, 2, "brood")
    put_frame(sheet, brood_frame("death", 0), 2, 2, "brood")
    put_frame(sheet, brood_frame("death", 1), 3, 2, "brood")
    save("brood_mother.png", sheet, 64, 48, edges="tlr")


# --------------------------------------------------------------------------
# Workers and colonists (same rig as the player, smaller)
# --------------------------------------------------------------------------
def worker_style(overall, shirt, skin, hair, hair_style, cap=None, kind="worker", bare_arms=True):
    ob = dark_ramp(overall, 0.3)
    st = dict(
        kind=kind, r_arm=0.85, r_leg=0.95, r_torso=2.0, r_hand=0.8, r_head=2.55,
        arm=shirt, arm_b=dark_ramp(shirt, 0.3), leg=overall, leg_b=ob, torso=overall,
        hand_ramp=skin, boot=(0.0, 0.95, 1.2), boot_ramp=BROWN[:3],
        skin=skin, hair=hair, hair_style=hair_style, cap=cap or hair,
        torso_bands=[(-9, -0.4, 2.3, 9, shirt, False), (-9, 9, 3.6, 9, shirt, False)],
        arm_bands=[(1, 0.35, 2.0, skin, False)] if bare_arms else [],
    )
    return st


OVER_TEAL = ramp("1f4a50", "2f6f72", "4a9a94")
OVER_RUST = ramp("6a3324", "a3522f", "cf7a44")
SHIRT_BONE = ramp("8a8470", "c4bca0", "e6e0c6")
SHIRT_MOSS = ramp("38552c", "55793a", "7fa04a")
DRESS = ramp("7a3f55", "b0647a", "e09aa6")
SHIRT_BLUE = ramp("2c4a7c", "3f6faa", "6a9ed0")
TROUSER = ramp("2a2740", "3b3857", "565378")

WORKER_A = worker_style(OVER_TEAL, SHIRT_BONE, SKIN_A, HAIR_DK, "cap", cap=[AMBER_D, AMBER, GOLD[3]])
WORKER_B = worker_style(OVER_RUST, SHIRT_MOSS, SKIN_B, HAIR_RED, "bun")
COLONIST_A = worker_style(DRESS, DRESS, SKIN_C, HAIR_BLOND, "long", kind="dress")
COLONIST_A["leg"] = SKIN_C; COLONIST_A["leg_b"] = dark_ramp(SKIN_C, 0.3)
COLONIST_A["torso_bands"] = [(-9, 9, 0.2, 0.9, ramp("5a2a40", "8a4560", "b0647a"), False)]
COLONIST_B = worker_style(TROUSER, SHIRT_BLUE, SKIN_A, HAIR_GREY, "short", bare_arms=False)
COLONIST_B["torso"] = SHIRT_BLUE; COLONIST_B["torso_bands"] = [(-9, 9, -9, 0.9, TROUSER, False)]


def worker_poses():
    walk, idle, work, carry = [], [], [], []
    for i in range(6):
        p = i / 6.0
        walk.append(dict(hip=(8.0, 15.0), lean=3, ground=True,
                         legF=run_leg(p, 27, 38), legB=run_leg(p + 0.5, 27, 38),
                         armF=("a", -24 * math.cos(2 * math.pi * p), 18),
                         armB=("a", 24 * math.cos(2 * math.pi * p), 18)))
    for i in range(2):
        idle.append(dict(hip=(8.0, 15.6), squash=-0.7 * i, legF=("ik", (9.0, 22.0)), legB=("ik", (6.8, 22.0)),
                         armF=("a", -6 + 4 * i, 14), armB=("a", -16 - 4 * i, 10)))

    def hammer(spark):
        def prop(cv, J):
            ex, ey = J["elbowF"]
            hx, hy = J["handF"]
            n = math.hypot(hx - ex, hy - ey) or 1.0
            ux, uy = (hx - ex) / n, (hy - ey) / n
            cv.set(fl(hx + ux * 1.0), fl(hy + uy * 1.0), BROWN[2])
            tx, ty = hx + ux * 2.0, hy + uy * 2.0
            cv.part(m_capsule((tx + uy * 1.1, ty - ux * 1.1), (tx - uy * 1.1, ty + ux * 1.1), 0.85), METAL[1:], sep=False)
            J["spark"] = (tx + ux * 1.2, ty + uy * 1.2) if spark else None
        return prop
    for i, (sa, el, ln) in enumerate(((150, 40, 0), (112, 30, 3), (58, 14, 8), (80, 20, 6))):
        work.append(dict(hip=(5.6, 15.8), lean=ln, legF=("ik", (7.6, 22.0)), legB=("ik", (4.0, 22.0)),
                         armF=("a", sa, el), armB=("a", 30 if i < 2 else 44, 30), prop=hammer(i == 2), head=4 if i >= 2 else -4))

    def crate(cv, J):
        hx, hy = J["handF"]
        x0, y0 = fl(hx - 1.5), fl(hy - 2.2)
        cv.part(m_rect(x0, y0, x0 + 4, y0 + 3), BROWN[1:], sep=True)
        cv.rect(x0, y0 + 1, x0 + 4, y0 + 1, BROWN[0]); cv.set(x0 + 2, y0 + 2, AMBER_D); cv.set(x0 + 1, y0 - 1, LIME_D); cv.set(x0 + 2, y0 - 2, LIME)
    for i in range(4):
        p = i / 4.0
        base = dict(hip=(7.4, 15.0), lean=-3, ground=True, legF=run_leg(p, 22, 34), legB=run_leg(p + 0.5, 22, 34))
        J = solve(base, WORKER_DIMS)
        tgt = (J["shF"][0] + 3.2, J["shF"][1] + 2.2)
        base["hold"] = (3.2, 2.2)
        base["prop"] = crate
        carry.append(base)
    return walk, idle, work, carry


def colonist_poses():
    walk, cheer = [], []
    for i in range(4):
        p = i / 4.0
        walk.append(dict(hip=(8.0, 15.0), lean=2, ground=True,
                         legF=run_leg(p, 22, 30), legB=run_leg(p + 0.5, 22, 30),
                         armF=("a", -20 * math.cos(2 * math.pi * p), 14),
                         armB=("a", 20 * math.cos(2 * math.pi * p), 14)))
    cheer.append(dict(hip=(8.0, 15.6), legF=("ik", (9.0, 22.0)), legB=("ik", (6.8, 22.0)),
                      armF=("a", 128, -20), armB=("a", -12, 10), head=-6))
    cheer.append(dict(hip=(8.0, 15.6), legF=("ik", (9.0, 22.0)), legB=("ik", (6.8, 22.0)),
                      armF=("a", 142, 34), armB=("a", -12, 10), head=-6))
    cheer.append(dict(hip=(8.0, 14.2), legF=("a", 14, 26), legB=("a", -12, 22), toes=0.6,
                      armF=("a", 136, 10), armB=("a", -150, -10), head=-10))
    cheer.append(dict(hip=(8.0, 16.0), legF=("ik", (9.4, 22.0)), legB=("ik", (6.4, 22.0)),
                      armF=("a", 124, 34), armB=("a", -136, -30), head=-4))
    return walk, cheer


def render_small(pose, st):
    pose = dict(pose)
    if "hold" in pose:
        J0 = solve(pose, WORKER_DIMS)
        hx, hy = pose["hold"]
        t = (J0["shF"][0] + hx, J0["shF"][1] + hy)
        pose.pop("ground", None)
        pose["hip"] = (J0["hip"][0] - WORKER_DIMS.get("xo", 0.0), J0["hip"][1])
        for k in ("legF", "legB"):
            pose[k] = ("ik", J0["foot" + k[-1]], -1)
        pose["armF"] = ("ik", t, 1)
        pose["armB"] = ("ik", (t[0] - 0.6, t[1] + 0.4), 1)
        pose["flat"] = False
    cv = Canvas(16, 24)
    cv.clip_y = 22
    J = solve(pose, WORKER_DIMS)
    draw_human(cv, J, WORKER_DIMS, st)
    if pose.get("prop"):
        pose["prop"](cv, J)
    cv.outline(INK)
    sp = J.get("spark")
    if sp:
        for (dx, dy, c) in ((0, 0, WHITE), (1, -1, AMBER), (1, 1, AMBER), (-1, 1, AMBER_D), (2, 0, AMBER_D)):
            x, y = fl(sp[0]) + dx, fl(sp[1]) + dy
            if 0 < x < 15 and 0 < y < 23:
                cv.set(x, y, c)
    return cv


def gen_workers():
    sheet = Canvas(128, 144)
    walk, idle, work, carry = worker_poses()
    for v, st in enumerate((WORKER_A, WORKER_B)):
        for i, p in enumerate(walk + idle):
            put_frame(sheet, render_small(p, st), i, v * 2, "workers")
        for i, p in enumerate(work + carry):
            put_frame(sheet, render_small(p, st), i, v * 2 + 1, "workers")
    cwalk, cheer = colonist_poses()
    for v, st in enumerate((COLONIST_A, COLONIST_B)):
        for i, p in enumerate(cwalk + cheer):
            put_frame(sheet, render_small(p, st), i, 4 + v, "workers")
    save("workers.png", sheet, 16, 24, edges="tlr")


# --------------------------------------------------------------------------
# Farm animals
# --------------------------------------------------------------------------
def cluckbug_frame(kind, f):
    cv = Canvas(16, 16)
    bob = [0, 0, 1, 0][f] if kind == "idle" else 0
    hx, hy = (11.2, 8.6 + bob) if kind == "idle" else [(11.4, 9.6), (11.8, 11.4), (11.9, 12.2), (11.6, 10.6)][f]
    body = Canvas(16, 16)
    # tail tuft
    body.set(3, int(7 + bob), LIME_D); body.set(2, int(6 + bob), LIME); body.set(3, int(5 + bob), LIME_D)
    sh = blob(body, 7.0, 9.6 + bob, 4.3, 3.7, [INDIGO[2], TEAL[1], TEAL[2], TEAL[3], ICE[1]])
    for (x, y) in sh:
        d = (y + 0.5 - (9.6 + bob)) - (x - 7.0) * 0.15
        if abs(d + 0.3) < 0.5 and x > 3:
            body.set(x, y, AMBER_D if x % 2 else AMBER)
        elif d > 2.0:
            body.set(x, y, BONE[1])
    for (x, y) in ((5, int(7 + bob)), (8, int(8 + bob)), (4, int(9 + bob))):
        if (x, y) in sh:
            body.set(x, y, ICE[2])
    blob(body, hx, hy, 2.3, 2.2, [BONE[1], BONE[2], BONE[3]])
    body.outline(INK)
    ex, ey = int(hx + 0.6), int(hy - 0.6)
    body.set(ex, ey, INK); body.set(ex, ey - 1, WHITE)
    body.set(int(hx + 2.4), int(hy + 0.4), AMBER)
    # legs (thin, no outline)
    for i, x in enumerate((4, 7, 10)):
        lift = 1 if (kind == "idle" and f == 3 and i == 2) else 0
        cv.line(x, 12 + bob, x + (1 if i == 2 else -1 if i == 0 else 0), 15 - lift, INDIGO[1])
        cv.set(x + (2 if i == 2 else 0 if i == 0 else 1), 15 - lift, INDIGO[2])
    cv.blit(body, 0, 0)
    # antennae
    tw = [0, 1, 0, -1][f] if kind == "idle" else 0
    cv.set(int(hx), int(hy - 3), BONE[1]); cv.set(int(hx) + tw, int(hy - 4), BONE[2]); cv.set(int(hx) + 1 + tw, int(hy - 5), LIME)
    cv.set(int(hx - 2), int(hy - 3), BONE[0]); cv.set(int(hx - 3) + tw, int(hy - 4), LIME_D)
    if kind == "peck" and f == 2:
        cv.set(14, 15, STRAW[2]); cv.set(14, 13, STRAW[1])
    return cv


def grub_frame(kind, f):
    cv = Canvas(16, 16)
    body = Canvas(16, 16)
    CREAM = [PALE[1], BONE[2], BONE[3]]
    br = ar = sq = 0.0
    if kind == "idle":
        br = [0.0, 0.2, 0.4, 0.2][f]
    else:
        ar = [0.0, 1.2, 2.0, 0.9][f]
        sq = [0.0, 0.8, 1.5, 0.6][f]
    tail = (4.2 + sq, 12.2)
    mid = (7.4 + sq * 0.4, 11.4 - ar - br)
    head = (10.8, 11.4)
    bm = m_capsule(tail, mid, 2.2) | m_capsule(mid, head, 2.5 + br * 0.5) | m_disc(head, 2.9)
    bm = {p for p in bm if p[1] <= 14}
    body.part(bm, CREAM, sep=False)
    for (x, y) in bm:                                   # soft belly shadow and segment creases
        if y >= 13 and body.lv.get((x, y)) == 1:
            body.set(x, y, PALE[2])
    for cx_ in (5.4 + sq * 0.8, 7.6 + sq * 0.4):
        x = int(cx_)
        ys = [y for (xx, y) in bm if xx == x]
        for y in ys:
            if y > min(ys) + 1:
                body.set(x, y, PALE[1])
    # udder saddle with straps
    ux, uy = mid[0] - 0.4, mid[1] - 2.6 - br * 0.5
    sac = m_ellipse(ux, uy, 2.9, 2.0)
    body.part(sac, [FLESH[1], FLESH[2], FLESH[3]], sep=True)
    for sx in (int(ux - 2.4), int(ux + 2.0)):
        for y in range(int(uy + 1), int(uy + 4)):
            if (sx, y) in bm:
                body.set(sx, y, FLESH[1])
        body.set(sx, int(uy + 2), FLESH[3])
    body.outline(INK)
    body.set(int(ux - 1), int(uy - 1), WHITE)
    ex, ey = int(head[0] + 0.2), int(head[1] - 1.0)
    body.set(ex, ey, INK); body.set(ex + 2, ey, INK)
    body.set(ex + 1, ey + 2, FLESH[2]); body.set(ex - 1, ey + 1, FLESH[3])
    cv.blit(body, 0, 0)
    for i, x in enumerate((4, 6, 8, 10)):
        xx = int(x + (sq * (1 - i / 3.0)))
        if cv.get(xx, 15) == INK:
            cv.set(xx, 15, INDIGO[3])
    if kind == "idle" and f == 2:
        cv.blend(int(ux - 3), int(uy + 4), alpha(WHITE, 220))
    return cv


def fish_frame(f, rmp, spot, glow=None):
    cv = Canvas(16, 16)
    bob = [0, 0, 1, 1][f]
    cy = 7.5 + bob
    tail = [-1, 0, 1, 0][f]
    body = Canvas(16, 16)
    body.part(m_ellipse(8.6, cy, 3.1, 2.1), rmp, sep=False)
    body.part(m_poly([(6.2, cy), (4.0, cy - 2.2 + tail), (4.4, cy + 2.2 + tail)]), rmp, sep=False)
    body.set(4, int(cy + tail), rmp[2])
    body.set(8, int(cy - 2), rmp[2]); body.set(7, int(cy - 2), rmp[0])
    body.set(8, int(cy + 1), spot); body.set(7, int(cy), spot)
    body.outline(INK)
    body.set(10, int(cy - 1), INK)
    if glow:
        body.set(9, int(cy + 1), glow); body.set(6, int(cy), glow)
    cv.blit(body, 0, 0)
    return cv


def gen_animals():
    sheet = Canvas(128, 48)
    for f in range(4):
        put_frame(sheet, cluckbug_frame("idle", f), f, 0, "animals")
        put_frame(sheet, cluckbug_frame("peck", f), 4 + f, 0, "animals")
        put_frame(sheet, grub_frame("idle", f), f, 1, "animals")
        put_frame(sheet, grub_frame("wriggle", f), 4 + f, 1, "animals")
        put_frame(sheet, fish_frame(f, [AMBER_D, AMBER, GOLD[3]], WHITE), f, 2, "animals")
        put_frame(sheet, fish_frame(f, [TEAL[1], TEAL[2], TEAL[3]], ICE[2], CYAN), 4 + f, 2, "animals")
    save("animals.png", sheet, 16, 16, edges="tlr")


def preview_mods_on_player(sheets, anch):
    """Preview only: every set composited on the suited player."""
    order = ["back", None, "feet", "legs", "torso", "arms", "hands", "head", "eyes"]
    frames = [(0, 0, 0), (0, 0, 1), (0, 0, 2), (0, 0, 3), (2, 1, 3), (1, 5, 3), (5, 8, 2), (0, 10, 3)]
    out = Canvas(24 * len(frames), 32 * 53)
    for i in range(53):
        slot = MODS[i][1]
        for k, (c, r, t) in enumerate(frames):
            fr = Canvas(24, 32)
            ax, ay = anch[slot][r][c]
            cell = mod_cell(i, t, 0)
            if slot == "back":
                fr.blit(cell, ax - 8, ay - 8)
            for y in range(32):
                for x in range(24):
                    p = sheets["suit"].get(c * 24 + x, r * 32 + y)
                    if p[3]:
                        fr.blend(x, y, p)
            if slot != "back":
                fr.blit(cell, ax - 8, ay - 8)
            out.blit(fr, k * 24, i * 32)
    bg = Canvas(out.w, out.h)
    bg.px = [hexc("3c4852")] * (out.w * out.h)
    bg.blit(out, 0, 0)
    write_png(os.path.join(PREVIEW, "mods_on_player.png"), bg)


# --------------------------------------------------------------------------
def main():
    global PREVIEW
    args = sys.argv[1:]
    if "--preview" in args:
        PREVIEW = args[args.index("--preview") + 1]
        os.makedirs(PREVIEW, exist_ok=True)
    os.makedirs(SPRITES, exist_ok=True)
    sheets, anch = gen_player()
    gen_mods()
    gen_robot()
    gen_creatures()
    gen_brood()
    gen_workers()
    gen_animals()
    if PREVIEW:
        preview_mods_on_player(sheets, anch)
    for w in sorted(set(WARN)):
        print("WARNING:", w)
    return 1 if WARN else 0


if __name__ == "__main__":
    sys.exit(main())
