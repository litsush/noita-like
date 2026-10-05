#!/usr/bin/env python3
"""Procedural audio generator for "Planet Terraforming".

Python 3 standard library only. Subtractive / additive / FM synthesis
(band-limited oscillators, envelopes, state-variable filters, Schroeder
reverb, feedback echo, tape wow and flutter) aimed at a plant-punk, Rain
World-like mood: rain, wind, dripping caves, strange animal calls, warm analog
machine hums and sparse, slightly detuned music. Fully deterministic: every
sound seeds its own RNG from its name, so the output does not depend on which
sounds are generated together or on the number of worker processes.

Usage (from the repo root):
    python3 tools/gen_audio.py              # regenerate everything
    python3 tools/gen_audio.py jump dash    # regenerate only the named files
    python3 tools/gen_audio.py -j1 ...      # single process (default: all cores)
    python3 tools/gen_audio.py --check      # verify the files on disk, write nothing

Output: assets/audio/<name>.wav -- 16-bit PCM, mono, 22050 Hz.

Levels. One-shots peak at -1 dBFS (the game sets per-sound volume), except
ui_hover (-14 dBFS) and inv_move (-8 dBFS), which are meant to be barely there,
and music_ending (-3 dBFS). Everything marked [LOOP] is seamless (play it with
looping on) and peaks at -6 dBFS. Lengths below are exact.

After writing, every file is read back and checked: right length, not silent,
not clipped, one-shots start and end at zero, loops have no step at the seam.

Footsteps (two variants each; pick randomly per step)
  step_soft_1        0.14 s  dirt / moss: muffled thud
  step_soft_2        0.14 s  dirt / moss: muffled thud, lower
  step_hard_1        0.10 s  stone: dry tap
  step_hard_2        0.10 s  stone: dry tap, higher
  step_metal_1       0.18 s  dome plating: dull ringing clank
  step_metal_2       0.18 s  dome plating: dull ringing clank, higher
  step_wet_1         0.22 s  shallow water: small slosh
  step_wet_2         0.22 s  shallow water: small slosh, brighter

Movement
  jump               0.18 s  soft upward whoosh with a cloth rustle
  land               0.24 s  soft thump on landing
  swim_stroke        0.45 s  muffled underwater push with a few bubbles
  splash             0.75 s  body entering water

Tools
  dig_soft           0.16 s  multitool in dirt: earthy crumble
  dig_hard           0.14 s  multitool on stone: dry crack + grit
  dig_ore            0.30 s  bright metallic clink with a short ring
  dig_metal          0.09 s  short dull clank (safe to retrigger ~10x/s)
  laser_fire         0.26 s  punchy sci-fi bolt
  laser_big          0.70 s  charged shot: heavy bolt with a boom and tail
  laser_charge       1.20 s  rising whine (ends at full pitch; follow with laser_big)
  laser_hit          0.28 s  bolt impact: crack + sizzle
  laser_empty        0.55 s  fizzle, then a double lockout buzz
  battery_ready      0.22 s  small rising blip: battery recharged

Body mods
  rocket_burst       0.50 s  rocket-boots kick
  rocket_loop        1.00 s  [LOOP] sustained rocket thrust
  dash               0.28 s  fast air whoosh
  grapple_fire       0.30 s  launcher thunk + line zipping out
  grapple_hit        0.22 s  hook biting: clank + line twang
  shield_hit         0.35 s  energy shield absorbing a hit: glassy thrum
  shield_break       0.80 s  shield collapsing: shatter + falling tone
  glide_loop         1.50 s  [LOOP] wind rushing past while gliding
  stomp              0.70 s  heavy ground pound with debris
  teleport           0.55 s  inward swell, pop, shimmer
  turret_fire        0.16 s  small pew (shoulder turret)
  heal               0.90 s  soft two-note chime with a warm bloom

Suit and survival
  o2_low             0.50 s  two-tone warning beep (high, low)
  o2_refill          1.20 s  tank refilling: hiss + bubbles
  gasp               0.70 s  sharp breath in, short breath out
  hurt               0.26 s  muffled grunt + thud
  blackout           1.80 s  downward swell into darkness
  respawn            2.00 s  warm rising swell on waking in bed
  shiver             0.60 s  trembling breath + teeth chatter (cold)
  cough              0.45 s  double cough inside the helmet (toxins)

Items and UI
  pickup             0.12 s  soft pop (the game varies the pitch)
  drop               0.16 s  soft low plop
  inv_open           0.22 s  pouch opening: rising rustle + click
  inv_close          0.20 s  pouch closing: falling rustle + click
  inv_move           0.04 s  tiny tick (quiet)
  craft              1.00 s  synthesizer whirr, then a ding
  craft_big          2.20 s  longer whirr with clunks, then a two-note ding (domes, robots)
  sell               0.70 s  credits chime: quick rising tinkle
  buy                0.50 s  soft stamp + two-note confirm
  error              0.30 s  soft low double thud
  ui_hover           0.03 s  very quiet tick
  ui_click           0.07 s  soft click
  ui_toggle          0.12 s  switch click with a small ping
  toast              0.50 s  notification: two soft mallet notes
  unlock             1.40 s  blueprint unlocked: rising mallet flourish
  codex_new          1.20 s  discovery sparkle

Farming and base
  plant_harvest      0.30 s  leafy snip
  plant_sow          0.25 s  seed pressed into soil
  water_pour         0.80 s  watering by hand: pour + gurgle
  sprinkler_loop     2.00 s  [LOOP] spray hiss with a ticking head
  fertilize          0.40 s  granules scattered
  plant_grow         0.90 s  gentle maturity chime with a leafy stretch
  flower_pop         0.35 s  battery flower sprouting: pop + tiny electric zip
  dome_enter         0.60 s  soft membrane whoosh (also fine for leaving)
  dome_place         2.50 s  heavy construction: clunks, servo, hiss, settle
  dome_hum_loop      4.00 s  [LOOP] warm interior hum
  machine_on         0.70 s  relay click + motor spinning up
  machine_off        0.70 s  motor spinning down + clunk
  pylon_loop         2.00 s  [LOOP] electric hum with faint crackle
  pump_loop          2.00 s  [LOOP] pump: motor + four strokes per loop
  pipe_place         0.30 s  copper clink
  generator_loop     3.00 s  [LOOP] fan + bubbling tank
  splicer_run        2.00 s  gene-splicer warble, then a ding
  water_flow_loop    2.00 s  [LOOP] water running in pipes

Robots
  robot_ok           0.30 s  friendly rising two-note beep
  robot_error        0.40 s  sad falling two-note beep
  robot_hover_loop   1.50 s  [LOOP] small hover fan
  robot_work         0.40 s  short servo chatter
  drop_pod_land      1.80 s  delivery pod: whoosh, thud, hiss

Creatures
  moth_flutter       0.45 s  Driftmoth: soft papery wingbeats
  puff_grunt         0.40 s  Puffback: gentle low grunt
  skitter_chitter    0.60 s  Skitter: creepy clicking
  skitter_lunge      0.50 s  Skitter: hiss-screech attack
  skitter_die        0.70 s  Skitter: falling screech + crunch
  web_spit           0.30 s  Webspinner: wet spit
  web_hit            0.30 s  web landing: sticky splat
  leech_latch        0.45 s  Gloom Leech: wet suck
  maw_rumble         1.60 s  Burrow Maw: ground tremor (warning)
  maw_erupt          1.40 s  Burrow Maw: burst of rock + roar
  brood_roar         2.00 s  Brood Mother: huge low roar
  brood_spawn        0.80 s  Brood Mother spawning: wet pops
  creature_hit       0.20 s  any creature struck: fleshy thwack + squeak

Weather and ambience
  amb_surface_day   20.00 s  [LOOP] wind through leaves, sparse strange bird and insect calls
  amb_surface_night 20.00 s  [LOOP] alien crickets, distant hoots, hush
  amb_rain          12.00 s  [LOOP] steady rain on leaves and metal
  amb_mist          12.00 s  [LOOP] soft muffled wind
  amb_dome          10.00 s  [LOOP] quiet interior: hum, air, occasional drip
  amb_cave          20.00 s  [LOOP] Shallows: drips with long echoes, air movement
  amb_deep          20.00 s  [LOOP] Deeps: low drones, distant rumbles, sparse drips
  amb_abyss         20.00 s  [LOOP] Abyss: sub-bass drone, heat hiss, far-off cracks
  thunder_1          4.00 s  distant rolling thunder
  thunder_2          5.00 s  distant rolling thunder, longer and lower

Music (sparse and moody; plays quietly under the ambience)
  music_menu        40.00 s  [LOOP] wistful theme: slow pad chords + a simple mallet motif
  music_day         60.00 s  [LOOP] gentle, hopeful, lots of space: pads, piano, soft pulse
  music_night       60.00 s  [LOOP] darker and slower: low pads, detuned piano
  music_deep        60.00 s  [LOOP] uneasy drone with occasional distant notes
  music_ending      35.00 s  the ship arrives: one pad note grows into a warm swell with the menu motif, then resolves (not a loop)

Events
  ship_engine_loop   3.00 s  [LOOP] colony ship engine roar
  ship_land          3.00 s  touchdown: thrust cut, heavy settle, hiss
  fanfare_ready      2.00 s  a readiness target was met: short warm fanfare
  readiness_all      4.00 s  all targets met: bigger fanfare on the menu motif
  day_start          2.50 s  soft dawn chime
  night_start        3.00 s  low dusk tone
"""

import array
import math
import os
import random
import re
import sys
import time
import wave

SR = 22050
TAU = 2.0 * math.pi
OUT_DIR = os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "audio"))

rng = random.Random(0)
_NOTE_CACHE = {}


# ---------------------------------------------------------------------------
# Basic helpers
# ---------------------------------------------------------------------------

def N(sec):
    return max(0, int(round(sec * SR)))


def _curve(spec, n):
    """Per-sample values from a parameter spec.

    number            -> constant
    (a, b)            -> exponential slide a->b over n samples (linear if <= 0)
    (a, b, 'lin')     -> linear slide
    callable(t)       -> evaluated at t seconds
    list              -> used as-is (padded with its last value)
    """
    if isinstance(spec, (int, float)):
        return [float(spec)] * n
    if callable(spec):
        return [spec(i / SR) for i in range(n)]
    if isinstance(spec, tuple):
        a, b = spec[0], spec[1]
        mode = spec[2] if len(spec) > 2 else ("exp" if a > 0 and b > 0 else "lin")
        if n <= 1:
            return [float(a)] * n
        if mode == "exp":
            k = math.log(b / a) / (n - 1)
            e = math.exp
            return [a * e(k * i) for i in range(n)]
        step = (b - a) / (n - 1)
        return [a + step * i for i in range(n)]
    lst = list(spec)
    if len(lst) < n:
        lst += [lst[-1] if lst else 0.0] * (n - len(lst))
    return lst[:n]


def nf(name):
    """Note name -> frequency, e.g. 'A4' -> 440, 'C#5', 'Bb3'."""
    idx = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}[name[0]]
    rest = name[1:]
    while rest and rest[0] in "#b":
        idx += 1 if rest[0] == "#" else -1
        rest = rest[1:]
    midi = 12 * (int(rest) + 1) + idx
    return 440.0 * 2.0 ** ((midi - 69) / 12.0)


def zeros(n):
    return [0.0] * n


def scale(x, g):
    return [v * g for v in x]


def mul(x, y):
    out = [a * b for a, b in zip(x, y)]
    if len(out) < len(x):
        out += [0.0] * (len(x) - len(out))
    return out


def add(*sigs):
    n = max(len(s) for s in sigs)
    out = [0.0] * n
    for s in sigs:
        if len(s) == n:
            out = [a + b for a, b in zip(out, s)]
        else:
            out[:len(s)] = [a + b for a, b in zip(out, s)]
    return out


def put(buf, sig, t, g=1.0):
    """Mix sig into buf at time t (seconds), extending buf if needed."""
    o = N(t)
    end = o + len(sig)
    if end > len(buf):
        buf.extend([0.0] * (end - len(buf)))
    if g == 1.0:
        buf[o:end] = [a + b for a, b in zip(buf[o:end], sig)]
    else:
        buf[o:end] = [a + b * g for a, b in zip(buf[o:end], sig)]
    return buf


def fit(x, dur):
    n = N(dur)
    if len(x) >= n:
        return x[:n]
    return x + [0.0] * (n - len(x))


def drive(x, amt):
    k = 1.0 / math.tanh(amt)
    th = math.tanh
    return [th(amt * v) * k for v in x]


# ---------------------------------------------------------------------------
# Oscillators / noise
# ---------------------------------------------------------------------------

def osc(wave_, freq, dur, duty=0.5, vib=0.0, vib_rate=5.0, phase=0.0):
    n = N(dur)
    s = math.sin
    if wave_ == "sine" and isinstance(freq, (int, float)) and not vib:
        w = TAU * freq / SR
        p = TAU * phase
        return [s(w * i + p) for i in range(n)]
    f = _curve(freq, n)
    if vib:
        w = TAU * vib_rate / SR
        f = [v * (1.0 + vib * s(w * i)) for i, v in enumerate(f)]
    out = [0.0] * n
    ph = phase % 1.0
    inv = 1.0 / SR
    if wave_ == "sine":
        for i in range(n):
            out[i] = s(TAU * ph)
            ph += f[i] * inv
    elif wave_ == "tri":
        for i in range(n):
            out[i] = 4.0 * abs(ph - 0.5) - 1.0
            ph += f[i] * inv
            ph -= int(ph)
    elif wave_ == "saw":
        for i in range(n):
            dt = f[i] * inv
            v = 2.0 * ph - 1.0
            if ph < dt:
                x = ph / dt
                v -= x + x - x * x - 1.0
            elif ph > 1.0 - dt:
                x = (ph - 1.0) / dt
                v -= x * x + x + x + 1.0
            out[i] = v
            ph += dt
            ph -= int(ph)
    elif wave_ == "square":
        dc = 2.0 * duty - 1.0
        for i in range(n):
            dt = f[i] * inv
            v = 1.0 if ph < duty else -1.0
            if ph < dt:
                x = ph / dt
                v += x + x - x * x - 1.0
            elif ph > 1.0 - dt:
                x = (ph - 1.0) / dt
                v += x * x + x + x + 1.0
            p2 = ph - duty
            if p2 < 0:
                p2 += 1.0
            if p2 < dt:
                x = p2 / dt
                v -= x + x - x * x - 1.0
            elif p2 > 1.0 - dt:
                x = (p2 - 1.0) / dt
                v -= x * x + x + x + 1.0
            out[i] = v - dc
            ph += dt
            ph -= int(ph)
    else:
        raise ValueError(wave_)
    return out


def white(dur):
    r = rng.random
    return [r() * 2.0 - 1.0 for _ in range(N(dur))]


def brown(dur):
    r = rng.random
    y = 0.0
    out = [0.0] * N(dur)
    for i in range(len(out)):
        y = 0.98 * y + 0.2 * (r() * 2.0 - 1.0)
        out[i] = y
    return out


def slow_random(dur, rate, lo=0.0, hi=1.0):
    """Smooth random control signal (cosine-interpolated random points)."""
    n = N(dur)
    pts = [rng.random() for _ in range(int(dur * rate) + 3)]
    out = [0.0] * n
    c = math.cos
    pi = math.pi
    k_ = rate / SR
    sp = hi - lo
    for i in range(n):
        x = i * k_
        k = int(x)
        w = 0.5 - 0.5 * c(pi * (x - k))
        a = pts[k]
        out[i] = lo + sp * (a + (pts[k + 1] - a) * w)
    return out


# ---------------------------------------------------------------------------
# Envelopes
# ---------------------------------------------------------------------------

def env_pts(dur, pts):
    """Piecewise-linear envelope from [(t, v), ...] (times ascending)."""
    n = N(dur)
    out = [float(pts[0][1])] * min(n, N(pts[0][0]))
    for (t0, v0), (t1, v1) in zip(pts, pts[1:]):
        i1 = min(n, N(t1))
        m = i1 - len(out)
        if m > 0:
            st = (v1 - v0) / m
            out.extend([v0 + st * k for k in range(m)])
    if len(out) < n:
        out.extend([float(pts[-1][1])] * (n - len(out)))
    return out


def env(dur, *pts):
    return env_pts(dur, list(pts))


def adsr(dur, a=0.005, d=0.05, s=0.7, r=0.05):
    hold = max(a + d, dur - r)
    return env_pts(dur, [(0, 0.0), (a, 1.0), (a + d, s), (hold, s), (dur, 0.0)])


def decay(dur, tau, attack=0.001):
    n = N(dur)
    na = max(1, N(attack))
    e = math.exp
    c = -1.0 / (tau * SR)
    out = [e(c * i) for i in range(n)]
    for i in range(min(na, n)):
        out[i] *= i / na
    return out


def fade(x, fin=0.002, fout=0.01):
    n = len(x)
    a = min(N(fin), n)
    b = min(N(fout), n)
    for i in range(a):
        x[i] *= i / a
    for i in range(b):
        x[n - 1 - i] *= i / b
    return x


# ---------------------------------------------------------------------------
# Filters / effects
# ---------------------------------------------------------------------------

def lp1(x, fc):
    a = 1.0 - math.exp(-TAU * fc / SR)
    y = 0.0
    out = [0.0] * len(x)
    for i, v in enumerate(x):
        y += a * (v - y)
        out[i] = y
    return out


def hp1(x, fc):
    lo = lp1(x, fc)
    return [a - b for a, b in zip(x, lo)]


def svf(x, fc, q=0.707, mode="lp"):
    """TPT state-variable filter. fc may be a constant or a curve spec.
    'bp' output is normalized to unity gain at the center frequency."""
    n = len(x)
    k = 1.0 / q
    const = isinstance(fc, (int, float))
    fcs = None if const else _curve(fc, n)
    tan = math.tan
    pi_sr = math.pi / SR
    fmax = SR * 0.45

    def coefs(f):
        f = min(max(f, 10.0), fmax)
        g = tan(pi_sr * f)
        a1 = 1.0 / (1.0 + g * (g + k))
        a2 = g * a1
        return a1, a2, g * a2

    ic1 = ic2 = 0.0
    out = [0.0] * n
    m = {"lp": 0, "bp": 1, "hp": 2}[mode]
    if const:
        a1, a2, a3 = coefs(fc)
        if m == 0:
            for i in range(n):
                v3 = x[i] - ic2
                v1 = a1 * ic1 + a2 * v3
                v2 = ic2 + a2 * ic1 + a3 * v3
                ic1 = 2.0 * v1 - ic1
                ic2 = 2.0 * v2 - ic2
                out[i] = v2
        elif m == 1:
            for i in range(n):
                v3 = x[i] - ic2
                v1 = a1 * ic1 + a2 * v3
                v2 = ic2 + a2 * ic1 + a3 * v3
                ic1 = 2.0 * v1 - ic1
                ic2 = 2.0 * v2 - ic2
                out[i] = k * v1
        else:
            for i in range(n):
                v0 = x[i]
                v3 = v0 - ic2
                v1 = a1 * ic1 + a2 * v3
                v2 = ic2 + a2 * ic1 + a3 * v3
                ic1 = 2.0 * v1 - ic1
                ic2 = 2.0 * v2 - ic2
                out[i] = v0 - k * v1 - v2
        return out
    a1, a2, a3 = coefs(fcs[0]) if n else (0, 0, 0)
    for i in range(n):
        if (i & 7) == 0:
            a1, a2, a3 = coefs(fcs[i])
        v0 = x[i]
        v3 = v0 - ic2
        v1 = a1 * ic1 + a2 * v3
        v2 = ic2 + a2 * ic1 + a3 * v3
        ic1 = 2.0 * v1 - ic1
        ic2 = 2.0 * v2 - ic2
        if m == 0:
            out[i] = v2
        elif m == 1:
            out[i] = k * v1
        else:
            out[i] = v0 - k * v1 - v2
    return out


def reverb(x, size=0.84, damp=0.4, wet=0.3, tail=1.0, dry=1.0):
    """Schroeder/Freeverb-style mono reverb. Wet level is energy-matched to
    the dry signal so `wet` is a perceptual ratio."""
    src = x + [0.0] * N(tail)
    n = len(src)
    acc = [0.0] * n
    d1 = 1.0 - damp
    for L in (557, 593, 641, 677, 709, 743):
        buf = [0.0] * L
        idx = 0
        filt = 0.0
        for i in range(n):
            y = buf[idx]
            filt = y * d1 + filt * damp
            buf[idx] = src[i] + filt * size
            idx += 1
            if idx == L:
                idx = 0
            acc[i] += y
    for L in (277, 211, 163):
        buf = [0.0] * L
        idx = 0
        for i in range(n):
            b = buf[idx]
            v = acc[i]
            buf[idx] = v + b * 0.5
            idx += 1
            if idx == L:
                idx = 0
            acc[i] = b - v
    ed = sum(v * v for v in x) or 1e-12
    ew = sum(v * v for v in acc) or 1e-12
    g = wet * math.sqrt(ed / ew)
    return [dry * a + g * b for a, b in zip(src, acc)]


def echo(x, delay_s, fb=0.35, mix=0.35, tail=1.0, lp_fc=None):
    d = N(delay_s)
    src = x + [0.0] * N(tail)
    n = len(src)
    y = [0.0] * n
    out = [0.0] * n
    a = 1.0 - math.exp(-TAU * lp_fc / SR) if lp_fc else 1.0
    st = 0.0
    for i in range(n):
        dl = y[i - d] if i >= d else 0.0
        st += a * (dl - st)
        v = src[i]
        y[i] = v + fb * st
        out[i] = v + mix * st
    return out


def tape(x, wow=0.002, wow_rate=0.4, flutter=0.00005, flutter_rate=6.3, loop=True):
    """Tape wow and flutter: a slowly wandering read position (seconds of
    peak displacement). With loop=True the wobble is periodic over the buffer
    and reads wrap round, so a seamless loop stays seamless."""
    n = len(x)
    if n < 4:
        return x
    dur = n / SR
    if loop:
        wow_rate = max(1.0, round(wow_rate * dur)) / dur
        flutter_rate = max(1.0, round(flutter_rate * dur)) / dur
    a1 = wow * SR
    a2 = flutter * SR
    w1 = TAU * wow_rate / SR
    w2 = TAU * flutter_rate / SR
    s = math.sin
    fl = math.floor
    out = [0.0] * n
    last = n - 1
    for i in range(n):
        p = i + a1 * s(w1 * i) + a2 * s(w2 * i + 1.3)
        k = fl(p)
        fr = p - k
        if loop:
            a = x[k % n]
            b = x[(k + 1) % n]
        else:
            a = x[min(max(k, 0), last)]
            b = x[min(max(k + 1, 0), last)]
        out[i] = a + (b - a) * fr
    return out


def comb(x, delay_s, g=0.7):
    """Feed-forward comb: hollow, pipe-like colouring."""
    d = max(1, N(delay_s))
    return [v + g * (x[i - d] if i >= d else 0.0) for i, v in enumerate(x)]


# ---------------------------------------------------------------------------
# Loop assembly
# ---------------------------------------------------------------------------

def xfade_loop(sig, L, X, mode="power"):
    """sig has >= L+X samples. Blend the overflow sig[L:L+X] into the head so
    out[L-1] -> out[0] continues exactly like sig[L-1] -> sig[L]."""
    out = sig[:L]
    for i in range(X):
        w = i / X
        if mode == "power":
            a, b = math.sin(w * math.pi / 2), math.cos(w * math.pi / 2)
        else:
            a, b = w, 1.0 - w
        out[i] = sig[i] * a + sig[L + i] * b
    return out


def fold(sig, L):
    """Wrap everything past L samples back onto the start (event tails)."""
    out = sig[:L] + [0.0] * max(0, L - len(sig))
    k = L
    while k < len(sig):
        seg = sig[k:k + L]
        out[:len(seg)] = [a + b for a, b in zip(out, seg)]
        k += L
    return out


def nloop(L_s, X_s, make):
    """Seamless noise bed: make(T) renders T seconds; the overflow is
    crossfaded (equal power) into the head."""
    return xfade_loop(make(L_s + X_s), N(L_s), N(X_s), "power")


def steady(x, fn):
    """Filter an exactly periodic loop so the result is periodic too: run the
    filter over two copies and keep the second (settled) one."""
    return fn(x + x)[len(x):]


def q(f, L):
    """Round a frequency so it has an integer number of cycles in L seconds."""
    return max(1.0, round(f * L)) / L


def lfo(L, rate, lo=0.0, hi=1.0, phase=0.0, power=1.0):
    """Sine LFO with a whole number of cycles in L seconds, as a list."""
    w = TAU * q(rate, L) / SR
    p = TAU * phase
    s = math.sin
    h = 0.5 * (hi - lo)
    if power == 1.0:
        return [lo + h * (1.0 + s(w * i + p)) for i in range(N(L))]
    return [lo + (hi - lo) * (0.5 + 0.5 * s(w * i + p)) ** power for i in range(N(L))]


# ---------------------------------------------------------------------------
# Level helpers
# ---------------------------------------------------------------------------

def norm(x, p=1.0):
    """Scale x so its peak is p."""
    m = max((abs(v) for v in x), default=0.0) or 1.0
    g = p / m
    return [v * g for v in x]


def rms_to(x, db):
    """Scale x so its RMS level is `db` dBFS (for mixing sustained beds)."""
    r = math.sqrt(sum(v * v for v in x) / max(1, len(x))) or 1e-9
    g = 10 ** (db / 20.0) / r
    return [v * g for v in x]


def reverse(x):
    return x[::-1]


def tail_fade(x, sec=0.02):
    n = len(x)
    m = min(N(sec), max(1, int(n * 0.3)))
    for i in range(m):
        x[n - 1 - i] *= i / m
    return x


# ---------------------------------------------------------------------------
# Building blocks (each ends with a short fade so truncated decays never click)
# ---------------------------------------------------------------------------

def thump(f0, f1, dur, tau, st=0.03, attack=0.002):
    s = osc("sine", lambda t: f1 + (f0 - f1) * math.exp(-t / st), dur)
    return tail_fade(mul(s, decay(dur, tau, attack)))


def noise_burst(dur, tau, fc, q_=0.7, mode="lp", attack=0.001):
    return tail_fade(mul(svf(white(dur), fc, q_, mode), decay(dur, tau, attack)))


def partials(base, ratios, amps, taus, dur, detune=0.0, attack=0.001):
    n = N(dur)
    out = zeros(n)
    s = math.sin
    e = math.exp
    for r, a, tau in zip(ratios, amps, taus):
        for dt, g in (((1.0, 1.0), (1.0 + detune, 0.6)) if detune else ((1.0, 1.0),)):
            w = TAU * base * r * dt / SR
            p = rng.random() * TAU
            c = -1.0 / (tau * SR)
            ag = a * g
            out = [o + ag * s(w * i + p) * e(c * i) for i, o in enumerate(out)]
    na = max(1, N(attack))
    for i in range(min(na, n)):
        out[i] *= i / na
    return tail_fade(out)


def fm_bell(f, dur, ratio=3.5, index=2.0, tau=0.8, itau=None, attack=0.002):
    itau = itau or tau * 0.35
    n = N(dur)
    out = [0.0] * n
    s = math.sin
    e = math.exp
    na = max(1, N(attack))
    for i in range(n):
        t = i / SR
        m = index * e(-t / itau) * s(TAU * f * ratio * t)
        v = s(TAU * f * t + m) * e(-t / tau)
        if i < na:
            v *= i / na
        out[i] = v
    return tail_fade(out, 0.04)


def grain_cloud(dur, count, glen=(0.003, 0.01), dist_pow=1.0, amp_tau=None, start=0.0):
    buf = zeros(N(dur))
    for _ in range(count):
        t = start + (dur - start) * rng.random() ** dist_pow
        L = rng.uniform(*glen)
        g = rng.uniform(0.35, 1.0)
        if amp_tau:
            g *= math.exp(-(t - start) / amp_tau)
        put(buf, mul(white(L), decay(L, L * 0.35, 0.0005)), t, g)
    return buf[:N(dur)]


def bubble(f0, dur, rise=0.8, gain=1.0):
    s = osc("sine", lambda t: f0 * (1.0 + rise * t / dur), dur)
    return scale(tail_fade(mul(s, decay(dur, dur / 3.0, 0.002)), 0.008), gain)


def stick_slip(dur, rate_fn, jitter=0.15):
    """Impulse train with irregular spacing (creaks, scrapes, servos)."""
    buf = zeros(N(dur))
    t = 0.0
    while t < dur:
        i = N(t)
        if i < len(buf):
            buf[i] += rng.uniform(0.6, 1.0)
        r = max(5.0, rate_fn(t))
        t += (1.0 / r) * (1.0 + rng.uniform(-jitter, jitter))
    return buf


def sparkle(dur, count, f_rng=(2500, 6000), tau_rng=(0.03, 0.12), dist_pow=1.0,
            amp_tau=None, start=0.0, notes=None):
    """Scattered high sine pings (glitter, tinkles)."""
    buf = zeros(N(dur))
    for _ in range(count):
        t = start + (dur - start) * 0.9 * rng.random() ** dist_pow
        f = rng.choice(notes) if notes else rng.uniform(*f_rng)
        tau = rng.uniform(*tau_rng)
        d = min(tau * 5.0, dur)
        ping = tail_fade(mul(osc("sine", f, d, phase=rng.random()), decay(d, tau, 0.001)), 0.005)
        g = rng.uniform(0.4, 1.0) * (math.exp(-(t - start) / amp_tau) if amp_tau else 1.0)
        put(buf, ping, t, g)
    return norm(fit(buf, dur))


def swoosh(dur, fc, q_=1.2, pts=None):
    """Band-passed noise sweep (fc may be a curve spec)."""
    s = svf(white(dur), fc, q_, "bp")
    s = mul(s, env_pts(dur, pts)) if pts else s
    return norm(s)


def clank(f, dur, tau=0.05, bright=1.0):
    """Struck metal: inharmonic partials + contact click."""
    ring = partials(f, [1, 2.41, 3.93, 5.62, 7.1], [1, 0.5, 0.3 * bright, 0.18 * bright, 0.1 * bright],
                    [tau, tau * 0.75, tau * 0.5, tau * 0.35, tau * 0.25], dur, detune=0.004)
    click = noise_burst(min(dur, 0.03), 0.004, 3200, 1.0, "bp")
    return norm(add(ring, scale(norm(click), 0.35)))


def motor(dur, freq, fc=900, rough=0.3):
    """Analog motor / servo whirr: saw + sub through a low-pass, a bit of grit."""
    n = N(dur)
    f = _curve(freq, n)
    a = osc("saw", f, dur)
    b = osc("square", [v * 0.5 for v in f], dur, duty=0.4)
    s = add(a, scale(b, 0.6))
    if rough:
        s = mul(s, [1.0 - rough + rough * v for v in slow_random(dur, 40)])
    return svf(s, fc, 1.2)


VOWELS = {
    "a": [(730, 90, 1.0), (1090, 110, 0.55), (2440, 160, 0.28)],
    "o": [(570, 80, 1.0), (840, 100, 0.5), (2410, 160, 0.18)],
    "u": [(300, 70, 1.0), (870, 100, 0.3), (2240, 160, 0.1)],
    "e": [(530, 80, 1.0), (1840, 120, 0.45), (2480, 160, 0.3)],
    "i": [(270, 70, 1.0), (2290, 130, 0.4), (3010, 200, 0.3)],
}


def formant(x, vowel, shift=1.0, qs=1.0):
    """Parallel resonant band-pass bank shaping x into a vowel.
    vowel: key of VOWELS, or a (from, to) pair that morphs over the signal."""
    if isinstance(vowel, tuple):
        va, vb = VOWELS[vowel[0]], VOWELS[vowel[1]]
    else:
        va = vb = VOWELS[vowel]
    out = [0.0] * len(x)
    for (fa, ba, ga), (fb, bb, gb) in zip(va, vb):
        fc = fa * shift if fa == fb else (fa * shift, fb * shift)
        y = svf(x, fc, qs * (fa + fb) / (ba + bb), "bp")
        g = 0.5 * (ga + gb)
        out = [o + g * v for o, v in zip(out, y)]
    return out


def growl(dur, freq, vowel="o", rough_rate=24.0, rough=0.5, shift=1.0, breath=0.15, qs=0.6):
    """Animal voice: rough saw through a vowel bank plus breath noise."""
    n = N(dur)
    src = osc("saw", freq, dur)
    if rough:
        am = osc("tri", rough_rate, dur, phase=rng.random())
        src = [v * (1.0 - rough * (0.5 + 0.5 * a)) for v, a in zip(src, am)]
    src = add(src, scale(white(dur), breath))
    low = svf(src, 220 * shift, 0.7)
    return norm(add(norm(formant(src, vowel, shift, qs)), scale(norm(low), 0.5)))[:n]


# --- Instruments --------------------------------------------------------------

def pad(freqs, dur, pts=None, bright=0.3, det=0.003, voices=2):
    """Warm analog pad: detuned voices of three soft harmonics each."""
    n = N(dur)
    acc = [0.0] * n
    s = math.sin
    for f in freqs:
        g = 1.0 / (1.0 + f / 500.0)
        for v in range(voices):
            sp = (2.0 * v / (voices - 1) - 1.0) if voices > 1 else 0.0
            w = TAU * f * (1.0 + det * sp + rng.uniform(-0.0006, 0.0006)) / SR
            p1, p2, p3 = rng.random() * TAU, rng.random() * TAU, rng.random() * TAU
            b2 = bright * g
            b3 = bright * bright * 0.6 * g
            acc = [a + g * s(w * i + p1) + b2 * s(2 * w * i + p2) + b3 * s(3 * w * i + p3)
                   for i, a in enumerate(acc)]
    if pts is not None:
        acc = mul(acc, env_pts(dur, pts))
    return acc


def strings(freqs, dur, pts=None, fc=1500, detune=0.005, q_=0.8):
    """Detuned saw ensemble through a low-pass (fc may be a curve)."""
    n = N(dur)
    src = [0.0] * n
    for f in freqs:
        for d in (-1.0, 1.0):
            base = f * (1.0 + detune * d + rng.uniform(-0.001, 0.001))
            src = add(src, osc("saw", base, dur, vib=0.004, vib_rate=rng.uniform(3.8, 5.2),
                               phase=rng.random()))
    out = norm(svf(src, fc, q_))
    if pts is not None:
        out = mul(out, env_pts(dur, pts))
    return out


def _cached(key, make):
    if key not in _NOTE_CACHE:
        _NOTE_CACHE[key] = make()
    return _NOTE_CACHE[key]


def keys(f, dur=3.0, tau=1.4, bright=0.8, det=0.0018):
    """Soft felt piano: slightly inharmonic partials, two detuned strings,
    a quiet hammer thump. Peak-normalised; cached per sound."""
    def make():
        n = N(dur)
        out = [0.0] * n
        s = math.sin
        e = math.exp
        B = 0.0005
        for k in range(1, 8):
            fk = f * k * math.sqrt(1.0 + B * k * k)
            if fk > SR * 0.42:
                break
            a = bright ** (k - 1) / k ** 1.5
            c = -(1.0 + 0.45 * (k - 1)) / (tau * SR)
            for dt, g in (((1.0 - det, 1.0), (1.0 + det, 0.8)) if k <= 3 else ((1.0, 1.0),)):
                w = TAU * fk * dt / SR
                p = rng.random() * TAU
                ag = a * g
                out = [o + ag * s(w * i + p) * e(c * i) for i, o in enumerate(out)]
        na = N(0.006)
        for i in range(min(na, n)):
            out[i] *= (i / na) ** 2
        out = norm(out)
        put(out, noise_burst(0.03, 0.006, min(2500.0, f * 3), 0.8, "lp"), 0.0, 0.1)
        return norm(tail_fade(out[:n], 0.08))
    return _cached(("keys", f, dur, tau, bright, det), make)


def mallet(f, dur=2.5, tau=0.9, bright=0.6, trem=0.0):
    """Soft mallet on a tuned bar (vibraphone / kalimba family)."""
    def make():
        p = partials(f, [1.0, 2.0, 3.98, 9.8], [1.0, 0.12, 0.3 * bright, 0.07 * bright],
                     [tau, tau * 0.7, tau * 0.28, tau * 0.08], dur, detune=0.0012, attack=0.003)
        if trem:
            w = TAU * trem / SR
            p = [v * (0.8 + 0.2 * math.sin(w * i)) for i, v in enumerate(p)]
        tk = noise_burst(0.02, 0.004, min(4000.0, f * 4), 1.0, "bp")
        return norm(tail_fade(add(norm(p), scale(norm(tk), 0.08)), 0.08))
    return _cached(("mallet", f, dur, tau, bright, trem), make)


def chime(f, dur=1.0, tau=0.3):
    """Small glassy chime (UI, pickups)."""
    return norm(partials(f, [1.0, 2.0, 3.01, 4.2], [1.0, 0.35, 0.12, 0.05],
                         [tau, tau * 0.6, tau * 0.35, tau * 0.2], dur, detune=0.002, attack=0.002))


def soft_tone(freq, dur, a=0.008, r=0.04, h2=0.2, h3=0.1):
    """Rounded synth tone (sine + a little 2nd/3rd harmonic) with fades."""
    n = N(dur)
    f = _curve(freq, n)
    s = add(osc("sine", f, dur), scale(osc("sine", [v * 2 for v in f], dur), h2),
            scale(osc("sine", [v * 3 for v in f], dur), h3))
    return mul(s, adsr(dur, a, 0.03, 0.8, r))


# ---------------------------------------------------------------------------
# Footsteps and movement
# ---------------------------------------------------------------------------

def step_soft(v):
    d = 0.14
    body = thump(120 * v, 55 * v, d, 0.035, st=0.02, attack=0.003)
    nz = noise_burst(d, 0.028, 700 * v, 0.7, "lp", 0.003)
    moss = svf(grain_cloud(0.1, 7, (0.004, 0.012), 1.4, 0.04), 1800 * v, 0.8, "bp")
    return svf(add(body, scale(nz, 0.8), scale(fit(moss, d), 0.25)), 1500, 0.7)


def step_hard(v):
    d = 0.10
    click = noise_burst(d, 0.009, 2100 * v, 1.6, "bp", 0.0005)
    tap = thump(380 * v, 240 * v, d, 0.02, st=0.015)
    grit = svf(grain_cloud(0.07, 4, (0.002, 0.005), 1.5, 0.03, start=0.01), 3000, 0.8, "bp")
    return svf(add(click, scale(tap, 0.7), scale(fit(grit, d), 0.2)), 5500, 0.7)


def step_metal(v):
    d = 0.18
    ring = partials(410 * v, [1, 2.32, 3.87, 5.4], [1, 0.45, 0.22, 0.1],
                    [0.05, 0.035, 0.025, 0.015], d, detune=0.005)
    hollow = thump(190 * v, 140 * v, d, 0.03)
    click = noise_burst(d, 0.004, 2600, 1.0, "bp")
    return svf(add(scale(ring, 0.55), scale(hollow, 0.7), scale(click, 0.4)), 4500, 0.7)


def step_wet(v):
    d = 0.22
    slosh = mul(svf(white(d), lambda t: 500 * v + 2600 * v * math.exp(-t / 0.035), 1.0, "bp"),
                decay(d, 0.06, 0.004))
    buf = add(slosh, scale(thump(140, 70, d, 0.03), 0.4))
    for _ in range(4):
        put(buf, bubble(rng.uniform(500, 1100) * v, rng.uniform(0.02, 0.04), 0.9),
            rng.uniform(0.03, 0.14), rng.uniform(0.15, 0.3))
    return fit(buf, d)


def sfx_jump():
    d = 0.18
    wh = swoosh(d, (400, 1700), 1.1, [(0, 0), (0.04, 1), (d, 0)])
    cloth = svf(grain_cloud(0.1, 8, (0.004, 0.012), 1.0, 0.05), 2500, 0.8, "bp")
    push = thump(150, 90, 0.08, 0.02)
    return add(scale(wh, 0.8), scale(fit(cloth, d), 0.25), scale(fit(push, d), 0.5))


def sfx_land():
    d = 0.24
    body = thump(115, 45, d, 0.06, st=0.025)
    nz = noise_burst(d, 0.035, 500)
    grit = svf(grain_cloud(0.13, 7, amp_tau=0.04), 1500, 0.7)
    return add(body, scale(nz, 0.7), scale(fit(grit, d), 0.25))


def sfx_swim_stroke():
    d = 0.45
    push = mul(svf(white(d), lambda t: 250 + 500 * math.sin(math.pi * min(1.0, t / 0.3)), 1.5, "bp"),
               env(d, (0, 0), (0.1, 1), (0.25, 0.6), (d, 0)))
    buf = add(norm(push), scale(mul(lp1(brown(d), 180), env(d, (0, 0), (0.08, 1), (d, 0))), 0.5))
    for _ in range(6):
        put(buf, bubble(rng.uniform(350, 900), rng.uniform(0.025, 0.05), 0.9),
            rng.uniform(0.1, 0.38), rng.uniform(0.12, 0.3))
    return svf(fit(buf, d), 1800, 0.7)


def sfx_splash():
    d = 0.75
    imp = mul(svf(white(d), lambda t: 700 + 4500 * math.exp(-t / 0.05), 0.7), decay(d, 0.15, 0.003))
    buf = add(imp, scale(thump(110, 60, d, 0.06), 0.5))
    for _ in range(7):
        put(buf, bubble(rng.uniform(400, 900), rng.uniform(0.02, 0.05), 0.8),
            rng.uniform(0.05, 0.45), rng.uniform(0.1, 0.25))
    for _ in range(5):
        put(buf, bubble(rng.uniform(1500, 2500), 0.015, 0.5), rng.uniform(0.1, 0.5), 0.08)
    return fit(reverb(buf, 0.75, 0.4, 0.15, tail=0.2), d)


# ---------------------------------------------------------------------------
# Tools
# ---------------------------------------------------------------------------

def sfx_dig_soft():
    d = 0.16
    cr = svf(grain_cloud(0.13, 16, (0.004, 0.012), 1.3, 0.05), 900, 0.8, "bp")
    body = thump(140, 70, 0.1, 0.03)
    nz = noise_burst(d, 0.03, 600)
    return add(fit(norm(cr), d), scale(fit(body, d), 0.7), scale(nz, 0.5))


def sfx_dig_hard():
    d = 0.14
    crack = noise_burst(d, 0.008, 2600, 1.4, "bp", 0.0005)
    knock = thump(520, 300, 0.06, 0.012, st=0.01)
    grit = svf(grain_cloud(0.11, 10, (0.002, 0.006), 1.4, 0.04, start=0.01), 3200, 0.9, "bp")
    return add(crack, scale(fit(knock, d), 0.7), scale(fit(grit, d), 0.45))


def sfx_dig_ore():
    d = 0.30
    f = 2350.0
    ring = partials(f, [1, 1.52, 2.74, 4.1], [1, 0.6, 0.35, 0.15], [0.08, 0.06, 0.035, 0.02], d,
                    detune=0.003)
    tick = noise_burst(0.03, 0.003, 5000, 1.0, "bp", 0.0003)
    knock = thump(600, 350, 0.05, 0.01)
    return add(norm(ring), scale(fit(tick, d), 0.5), scale(fit(knock, d), 0.4))


def sfx_dig_metal():
    d = 0.09
    ring = partials(820, [1, 2.41, 3.93], [1, 0.5, 0.25], [0.018, 0.013, 0.009], d, detune=0.004)
    tick = noise_burst(0.02, 0.003, 3000, 1.0, "bp", 0.0003)
    return add(norm(ring), scale(fit(tick, d), 0.5), scale(fit(thump(260, 180, 0.05, 0.012), d), 0.5))


def _bolt(d, f_hi, f_lo, st, tau, boom):
    sw = osc("saw", lambda t: f_lo + f_hi * math.exp(-t / st), d)
    s2 = osc("sine", lambda t: 2 * f_lo + 1.8 * f_hi * math.exp(-t / (st * 0.7)), d)
    body = svf(add(sw, scale(s2, 0.6)), lambda t: 500 + 5500 * math.exp(-t / (st * 1.4)), 1.6)
    body = mul(body, decay(d, tau, 0.001))
    snap = noise_burst(min(d, 0.06), 0.012, 3500, 0.8, "hp", 0.0005)
    low = thump(190, 60, d, tau * boom, st=0.04)
    return drive(add(norm(body), scale(fit(norm(snap), d), 0.35), scale(norm(low), 0.6 * boom)), 1.5)


def sfx_laser_fire():
    return _bolt(0.26, 2000, 210, 0.045, 0.07, 0.8)


def sfx_laser_big():
    d = 0.70
    b = _bolt(d, 2600, 120, 0.09, 0.16, 1.4)
    crackle = svf(grain_cloud(0.5, 30, (0.001, 0.004), 1.6, 0.15), 3500, 1.2, "bp")
    b = add(b, scale(fit(norm(crackle), d), 0.2))
    return fade(fit(reverb(b, 0.8, 0.5, 0.25, tail=0.0), d), 0.001, 0.2)


def sfx_laser_charge():
    d = 1.20
    n = N(d)
    f = [180.0 * (1700.0 / 180.0) ** ((i / n) ** 1.4) for i in range(n)]
    whine = add(osc("saw", f, d), scale(osc("sine", [v * 2.01 for v in f], d), 0.5))
    whine = svf(whine, [v * 3.0 for v in f], 2.0)
    trem = [0.75 + 0.25 * math.sin(TAU * (6.0 * t + 14.0 * t * t)) for t in (i / SR for i in range(n))]
    whine = mul(mul(whine, trem), [0.15 + 0.85 * (i / n) ** 1.5 for i in range(n)])
    air = mul(svf(white(d), (1500, 7000), 1.5, "bp"), [(i / n) ** 2 for i in range(n)])
    return fade(add(norm(whine), scale(norm(air), 0.25)), 0.03, 0.015)


def sfx_laser_hit():
    d = 0.28
    crack = noise_burst(d, 0.02, 2400, 1.0, "bp", 0.0005)
    zap = mul(osc("saw", (1400, 300), 0.12), decay(0.12, 0.03))
    sizz = mul(svf(white(d), 4500, 0.7, "hp"), decay(d, 0.08, 0.01))
    sparks = svf(grain_cloud(0.22, 14, (0.001, 0.003), 1.6, 0.08), 4000, 1.5, "bp")
    return add(norm(crack), scale(fit(svf(zap, 3000, 1.0), d), 0.4), scale(norm(sizz), 0.3),
               scale(fit(norm(sparks), d), 0.4), scale(fit(thump(160, 70, 0.1, 0.03), d), 0.5))


def sfx_laser_empty():
    d = 0.55
    buf = zeros(N(d))
    gate = [v ** 2 for v in slow_random(0.22, 45)]
    fz = mul(mul(svf(white(0.22), (3500, 500), 1.6, "bp"), gate), decay(0.22, 0.09, 0.002))
    put(buf, norm(fz), 0.0, 0.8)
    put(buf, mul(osc("saw", (600, 120), 0.18), decay(0.18, 0.05)), 0.0, 0.25)
    for t0 in (0.27, 0.40):
        bz = svf(osc("square", 98, 0.09, duty=0.35), 700, 1.0)
        put(buf, mul(bz, adsr(0.09, 0.004, 0.02, 0.8, 0.02)), t0, 0.7)
    return fit(buf, d)


def sfx_battery_ready():
    d = 0.22
    buf = zeros(N(d))
    put(buf, soft_tone(nf("E5"), 0.09, 0.004, 0.03), 0.0, 0.8)
    put(buf, soft_tone(nf("B5"), 0.13, 0.004, 0.08), 0.085, 1.0)
    return fit(buf, d)


# ---------------------------------------------------------------------------
# Body mods
# ---------------------------------------------------------------------------

def _thrust(T, flutter=22.0):
    roar = svf(white(T), 520, 0.8)
    mid = svf(white(T), 1500, 0.9, "bp")
    hiss = svf(white(T), 4000, 0.7, "hp")
    s = add(scale(roar, 1.6), scale(mid, 0.5), scale(hiss, 0.12))
    return mul(s, slow_random(T, flutter, 0.7, 1.0))


def sfx_rocket_burst():
    d = 0.50
    s = mul(_thrust(d), env(d, (0, 0), (0.02, 1), (0.15, 0.7), (d, 0)))
    s = svf(s, lambda t: 900 + 4000 * math.exp(-t / 0.12), 0.8)
    return drive(add(norm(s), scale(thump(140, 55, d, 0.07), 0.7)), 1.4)


def sfx_rocket_loop():
    L = 1.0
    bed = nloop(L, 0.25, lambda T: drive(norm(_thrust(T)), 1.3))
    sub = mul(osc("sine", q(62, L), L), lfo(L, 9, 0.7, 1.0))
    return add(norm(bed), scale(sub, 0.25))


def sfx_dash():
    d = 0.28
    fc = lambda t: 500 + 2100 * math.sin(math.pi * min(1.0, t / d)) ** 1.5
    wh = mul(svf(white(d), fc, 1.4, "bp"), env(d, (0, 0), (0.06, 1), (0.14, 0.7), (d, 0)))
    body = mul(svf(white(d), 700), env(d, (0, 0), (0.05, 1), (d, 0)))
    return add(wh, scale(body, 0.4))


def sfx_grapple_fire():
    d = 0.30
    buf = zeros(N(d))
    put(buf, thump(220, 90, 0.08, 0.02), 0.0, 0.9)
    put(buf, noise_burst(0.04, 0.008, 2200, 1.0, "bp"), 0.0, 0.6)
    imp = stick_slip(0.26, lambda t: 120 + 900 * t / 0.26, 0.1)
    zipr = add(svf(imp, 2400, 3.0, "bp"), scale(svf(imp, 1100, 2.0, "bp"), 0.6))
    put(buf, mul(norm(zipr), env(0.26, (0, 0), (0.03, 1), (0.2, 0.7), (0.26, 0))), 0.03, 0.6)
    put(buf, swoosh(0.24, (800, 3000), 1.5, [(0, 0), (0.05, 1), (0.24, 0)]), 0.03, 0.3)
    return fit(buf, d)


def sfx_grapple_hit():
    d = 0.22
    buf = scale(clank(640, d, 0.035), 0.9)
    put(buf, thump(200, 110, 0.08, 0.02), 0.0, 0.6)
    twang = mul(osc("saw", (240, 300), 0.18, vib=0.03, vib_rate=38), decay(0.18, 0.05, 0.004))
    put(buf, svf(twang, 1400, 2.0, "bp"), 0.02, 0.5)
    return fit(buf, d)


def sfx_shield_hit():
    d = 0.35
    thr = fm_bell(260, d, ratio=2.01, index=3.0, tau=0.1, itau=0.05)
    hi = fm_bell(1180, d, ratio=1.41, index=1.2, tau=0.07)
    rip = mul(svf(white(d), (4000, 900), 2.5, "bp"), decay(d, 0.05, 0.002))
    return add(thr, scale(hi, 0.35), scale(norm(rip), 0.35), scale(thump(120, 70, d, 0.04), 0.4))


def sfx_shield_break():
    d = 0.80
    buf = zeros(N(d))
    put(buf, noise_burst(0.2, 0.03, 3000, 0.8, "hp", 0.0005), 0.0, 0.7)
    put(buf, sparkle(0.6, 36, (1800, 6500), (0.02, 0.08), 1.8, amp_tau=0.2), 0.0, 0.7)
    fall = mul(add(osc("sine", (900, 110), 0.6), scale(osc("saw", (452, 56), 0.6), 0.4)),
               decay(0.6, 0.2, 0.004))
    put(buf, svf(fall, 2500, 0.8), 0.0, 0.7)
    put(buf, thump(150, 45, 0.3, 0.09), 0.0, 0.7)
    return fade(fit(reverb(fit(buf, d), 0.8, 0.4, 0.25, tail=0.0), d), 0.001, 0.15)


def sfx_glide_loop():
    L = 1.5

    def make(T):
        lo = mul(svf(white(T), [500 + 500 * v for v in slow_random(T, 1.5)], 1.3, "bp"),
                 slow_random(T, 2.0, 0.6, 1.0))
        hi = mul(svf(white(T), 2800, 0.8, "bp"), slow_random(T, 7.0, 0.3, 1.0))
        return add(rms_to(lo, -14), rms_to(hi, -26), rms_to(lp1(brown(T), 160), -22))
    return nloop(L, 0.4, make)


def sfx_stomp():
    d = 0.70
    buf = zeros(N(d))
    put(buf, thump(95, 30, 0.6, 0.16, st=0.05), 0.0, 1.2)
    put(buf, noise_burst(0.25, 0.04, 700, 0.8), 0.0, 0.8)
    put(buf, noise_burst(0.03, 0.006, 2500, 1.0, "bp"), 0.0, 0.5)
    deb = svf(grain_cloud(0.5, 22, (0.004, 0.014), 1.6, 0.14, start=0.04), 1400, 0.8, "bp")
    put(buf, norm(deb), 0.0, 0.3)
    put(buf, mul(lp1(brown(0.6), 120), env(0.6, (0, 0), (0.03, 1), (0.6, 0))), 0.0, 0.8)
    buf = drive(fit(buf, d), 1.6)
    return fade(fit(reverb(buf, 0.8, 0.5, 0.2, tail=0.0), d), 0.001, 0.2)


def sfx_teleport():
    d = 0.55
    buf = zeros(N(d))
    n1 = N(0.22)
    swell = mul(svf(white(0.22), (600, 5000), 2.0, "bp"), [(i / n1) ** 2.5 for i in range(n1)])
    tone = mul(osc("sine", (300, 1900), 0.22), [(i / n1) ** 2 for i in range(n1)])
    put(buf, add(norm(swell), scale(tone, 0.5)), 0.0, 0.7)
    put(buf, thump(420, 90, 0.1, 0.025, st=0.015), 0.22, 1.0)
    put(buf, noise_burst(0.03, 0.006, 3000, 1.0, "bp"), 0.22, 0.5)
    put(buf, sparkle(0.3, 14, (2500, 6000), (0.02, 0.06), 1.5, amp_tau=0.1), 0.23, 0.4)
    put(buf, fm_bell(1320, 0.3, ratio=2.0, index=1.0, tau=0.08), 0.23, 0.3)
    return fit(buf, d)


def sfx_turret_fire():
    d = 0.16
    s = add(osc("saw", lambda t: 500 + 2400 * math.exp(-t / 0.025), d),
            scale(osc("sine", lambda t: 900 + 3000 * math.exp(-t / 0.02), d), 0.6))
    s = mul(svf(s, lambda t: 900 + 5000 * math.exp(-t / 0.04), 1.4), decay(d, 0.04, 0.001))
    return add(norm(s), scale(fit(noise_burst(0.03, 0.006, 4000, 0.8, "hp"), d), 0.25))


def sfx_heal():
    d = 0.90
    buf = zeros(N(d))
    put(buf, chime(nf("E5"), 0.8, 0.28), 0.0, 0.7)
    put(buf, chime(nf("B5"), 0.7, 0.3), 0.14, 0.6)
    put(buf, pad([nf("E4"), nf("B4"), nf("G#5")], 0.85, [(0, 0), (0.25, 1), (0.85, 0)], 0.2), 0.0, 0.12)
    return fade(fit(reverb(fit(buf, d), 0.82, 0.4, 0.3, tail=0.0), d), 0.002, 0.2)


# ---------------------------------------------------------------------------
# Suit and survival
# ---------------------------------------------------------------------------

def sfx_o2_low():
    d = 0.50
    buf = zeros(N(d))
    put(buf, soft_tone(880, 0.16, 0.006, 0.03, 0.1, 0.25), 0.0, 1.0)
    put(buf, soft_tone(660, 0.2, 0.006, 0.05, 0.1, 0.25), 0.22, 1.0)
    return svf(fit(buf, d), 2600, 0.8)


def sfx_o2_refill():
    d = 1.20
    hiss = add(svf(white(d), 3200, 0.7, "hp"), scale(svf(white(d), (1200, 2600), 2.0, "bp"), 0.8))
    hiss = mul(hiss, env(d, (0, 0), (0.05, 1), (0.8, 0.8), (1.05, 0.25), (d, 0)))
    buf = scale(norm(hiss), 0.7)
    for _ in range(16):
        put(buf, bubble(rng.uniform(300, 1000), rng.uniform(0.03, 0.06), 1.0),
            rng.uniform(0.1, 1.05), rng.uniform(0.15, 0.4))
    put(buf, thump(300, 180, 0.06, 0.015), 0.0, 0.4)
    put(buf, noise_burst(0.02, 0.003, 2500, 1.2, "bp"), 1.07, 0.4)
    return fit(buf, d)


def sfx_gasp():
    d = 0.70
    buf = zeros(N(d))
    n = white(0.32)
    inh = add(svf(n, (700, 1800), 3.0, "bp"), scale(svf(n, 2600, 4.0, "bp"), 0.4))
    put(buf, mul(inh, env(0.32, (0, 0), (0.22, 1), (0.28, 0.8), (0.32, 0))), 0.0, 1.0)
    exh = mul(svf(white(0.3), 900, 2.0, "bp"), env(0.3, (0, 0), (0.03, 1), (0.3, 0)))
    put(buf, exh, 0.38, 0.5)
    return svf(fit(buf, d), 300, 0.7, "hp")


def sfx_hurt():
    d = 0.26
    vox = growl(0.22, (210, 130), "a", 30, 0.4, 0.9, 0.2)
    vox = mul(vox, env(0.22, (0, 0), (0.012, 1), (0.08, 0.7), (0.22, 0)))
    buf = zeros(N(d))
    put(buf, svf(vox, 1800, 0.7), 0.0, 1.0)
    put(buf, thump(150, 80, 0.12, 0.04), 0.0, 0.7)
    put(buf, noise_burst(0.03, 0.01, 1800), 0.0, 0.4)
    return fit(buf, d)


def sfx_blackout():
    d = 1.80
    n = N(d)
    gl = [2.0 ** (-1.6 * (i / n) ** 0.8) for i in range(n)]
    acc = zeros(n)
    for f in (nf("D3"), nf("A3"), nf("F4"), nf("D3") * 1.006):
        acc = add(acc, osc("saw", [f * g for g in gl], d, phase=rng.random()))
    acc = svf(acc, (2400, 120), 1.0)
    acc = mul(acc, env(d, (0, 0), (0.08, 1), (0.9, 0.8), (d, 0)))
    sub = mul(osc("sine", (70, 34), d), env(d, (0, 0), (0.3, 1), (1.3, 0.8), (d, 0)))
    air = mul(svf(white(d), (3000, 200), 1.0, "bp"), env(d, (0, 0), (0.1, 1), (d, 0)))
    buf = add(norm(acc), scale(sub, 0.7), scale(norm(air), 0.2))
    put(buf, thump(80, 40, 0.3, 0.09), 0.02, 0.6)
    return fade(fit(reverb(buf, 0.86, 0.5, 0.3, tail=0.0), d), 0.002, 0.3)


def sfx_respawn():
    d = 2.00
    ch = [nf(x) for x in ("D3", "A3", "D4", "F#4", "E5")]
    p = pad(ch, d, [(0, 0), (1.1, 1), (1.5, 0.8), (d, 0)], 0.35)
    p = svf(p, (300, 2600), 0.8)
    buf = scale(norm(p), 0.8)
    air = mul(svf(white(1.2), (400, 3500), 1.2, "bp"), env(1.2, (0, 0), (1.0, 1), (1.2, 0)))
    put(buf, norm(air), 0.0, 0.08)
    put(buf, mallet(nf("A5"), 0.9, 0.35), 1.05, 0.3)
    put(buf, mallet(nf("D6"), 0.8, 0.3), 1.2, 0.22)
    return fade(fit(reverb(fit(buf, d), 0.86, 0.4, 0.35, tail=0.0), d), 0.01, 0.4)


def sfx_shiver():
    d = 0.60
    n = N(d)
    br = add(svf(white(d), 1100, 2.5, "bp"), scale(svf(white(d), 2300, 3.0, "bp"), 0.5))
    trem = [0.35 + 0.65 * (0.5 + 0.5 * math.sin(TAU * 13.0 * i / SR)) ** 2 for i in range(n)]
    br = mul(mul(br, trem), env(d, (0, 0), (0.08, 1), (0.4, 0.8), (d, 0)))
    buf = scale(norm(br), 0.8)
    t = 0.05
    while t < 0.5:
        put(buf, noise_burst(0.012, 0.002, 2800, 2.0, "bp", 0.0003), t, rng.uniform(0.25, 0.5))
        t += rng.uniform(0.05, 0.085)
    return svf(fit(buf, d), 300, 0.7, "hp")


def sfx_cough():
    d = 0.45
    buf = zeros(N(d))
    for t0, ln, g in ((0.0, 0.16, 1.0), (0.2, 0.2, 0.8)):
        nz = add(scale(white(ln), 0.8), scale(osc("saw", (150, 100), ln), 0.5))
        c = formant(nz, ("a", "o"), 0.95, 0.5)
        put(buf, mul(norm(c), env(ln, (0, 0), (0.008, 1), (0.05, 0.5), (ln, 0))), t0, g)
        put(buf, thump(130, 80, 0.08, 0.025), t0, 0.5 * g)
    return svf(fit(buf, d), 2200, 0.7)


# ---------------------------------------------------------------------------
# Items and UI
# ---------------------------------------------------------------------------

def sfx_pickup():
    d = 0.12
    pop = mul(osc("sine", (420, 980), 0.09), decay(0.09, 0.03, 0.003))
    pop = add(pop, scale(mul(osc("sine", (840, 1960), 0.09), decay(0.09, 0.02, 0.003)), 0.2))
    return add(fit(pop, d), scale(fit(noise_burst(0.015, 0.003, 1800, 1.0, "bp"), d), 0.15))


def sfx_drop():
    d = 0.16
    pop = mul(osc("sine", (520, 190), 0.12), decay(0.12, 0.035, 0.003))
    return add(fit(pop, d), scale(fit(thump(130, 70, 0.1, 0.03), d), 0.7),
               scale(fit(noise_burst(0.03, 0.008, 900), d), 0.3))


def _pouch(d, f0, f1, click_t):
    rustle = svf(grain_cloud(d * 0.8, 16, (0.004, 0.012)), (f0, f1), 1.0, "bp")
    rustle = mul(fit(rustle, d), env(d, (0, 0), (0.03, 1), (d * 0.7, 0.6), (d, 0)))
    wh = swoosh(d, (f0 * 0.6, f1 * 0.6), 1.5, [(0, 0), (d * 0.3, 1), (d, 0)])
    buf = add(norm(rustle), scale(wh, 0.5))
    put(buf, thump(330, 200, 0.04, 0.01), click_t, 0.6)
    put(buf, noise_burst(0.015, 0.002, 2400, 1.5, "bp"), click_t, 0.5)
    return fit(buf, d)


def sfx_inv_open():
    return _pouch(0.22, 900, 3200, 0.0)


def sfx_inv_close():
    return _pouch(0.20, 3000, 800, 0.15)


def sfx_inv_move():
    d = 0.04
    return add(mul(osc("sine", (1300, 900), d), decay(d, 0.007, 0.0005)),
               scale(noise_burst(d, 0.002, 2600, 1.2, "bp", 0.0003), 0.4))


def _assembler(d, clunks):
    f = lambda t: 70 + 150 * math.sin(math.pi * min(1.0, t / d)) ** 0.6 + 12 * math.sin(TAU * 7 * t)
    m = mul(motor(d, f, 1100, 0.35), env(d, (0, 0), (0.06, 1), (d - 0.1, 0.9), (d, 0)))
    tk = stick_slip(d, lambda t: 18 + 10 * math.sin(TAU * 1.3 * t), 0.2)
    tk = svf(tk, 2600, 3.0, "bp")
    buf = add(norm(m), scale(norm(tk), 0.35))
    for t0 in clunks:
        put(buf, thump(170, 90, 0.1, 0.03), t0, 0.7)
        put(buf, clank(520, 0.12, 0.03, 0.6), t0, 0.35)
    return svf(fit(buf, d), 3200, 0.7)


def sfx_craft():
    d = 1.00
    buf = scale(_assembler(0.62, (0.0,)), 0.6)
    put(buf, chime(nf("E6"), 0.38, 0.11), 0.62, 0.8)
    put(buf, chime(nf("B6"), 0.36, 0.09), 0.63, 0.25)
    return fit(buf, d)


def sfx_craft_big():
    d = 2.20
    buf = scale(_assembler(1.5, (0.0, 0.5, 1.05, 1.42)), 0.6)
    hiss = mul(svf(white(0.3), 3500, 0.7, "hp"), env(0.3, (0, 0), (0.02, 1), (0.3, 0)))
    put(buf, norm(hiss), 1.42, 0.2)
    put(buf, chime(nf("B5"), 0.5, 0.16), 1.55, 0.7)
    put(buf, chime(nf("E6"), 0.62, 0.2), 1.72, 0.8)
    put(buf, chime(nf("B6"), 0.45, 0.15), 1.73, 0.2)
    return fade(fit(reverb(fit(buf, d), 0.78, 0.4, 0.15, tail=0.0), d), 0.002, 0.1)


def sfx_sell():
    d = 0.70
    buf = zeros(N(d))
    for i, nm in enumerate(("G5", "C6", "E6", "G6")):
        put(buf, chime(nf(nm), 0.4, 0.09), 0.055 * i, 0.5 + 0.1 * i)
    put(buf, chime(nf("C7"), 0.45, 0.14), 0.24, 0.5)
    for t0 in (0.0, 0.06, 0.13):
        put(buf, partials(rng.uniform(3200, 4200), [1, 2.76], [1, 0.3], [0.03, 0.015], 0.1), t0, 0.15)
    return fade(fit(reverb(fit(buf, d), 0.78, 0.35, 0.2, tail=0.0), d), 0.001, 0.12)


def sfx_buy():
    d = 0.50
    buf = zeros(N(d))
    put(buf, thump(210, 110, 0.09, 0.025), 0.0, 0.8)
    put(buf, noise_burst(0.03, 0.006, 1800, 1.0, "bp"), 0.0, 0.4)
    put(buf, chime(nf("E6"), 0.3, 0.08), 0.07, 0.6)
    put(buf, chime(nf("C6"), 0.36, 0.11), 0.17, 0.7)
    return fit(buf, d)


def sfx_error():
    d = 0.30
    buf = zeros(N(d))
    for t0, g in ((0.0, 1.0), (0.13, 0.85)):
        put(buf, thump(170, 95, 0.14, 0.04, st=0.03, attack=0.004), t0, g)
        put(buf, mul(svf(osc("square", 96, 0.1, duty=0.4), 420, 0.8), adsr(0.1, 0.005, 0.03, 0.5, 0.04)),
            t0, 0.35 * g)
    return fit(buf, d)


def sfx_ui_hover():
    d = 0.03
    return svf(mul(osc("sine", 1500, d), decay(d, 0.005, 0.0008)), 3500, 0.7)


def sfx_ui_click():
    d = 0.07
    a = mul(osc("sine", (900, 520), d), decay(d, 0.014, 0.0008))
    b = noise_burst(d, 0.003, 2600, 1.0, "bp")
    c = mul(osc("sine", 280, d), decay(d, 0.01, 0.001))
    return add(a, scale(b, 0.25), scale(c, 0.4))


def sfx_ui_toggle():
    d = 0.12
    buf = zeros(N(d))
    put(buf, noise_burst(0.02, 0.002, 2400, 1.2, "bp"), 0.0, 0.4)
    put(buf, thump(400, 240, 0.06, 0.012), 0.0, 0.5)
    put(buf, chime(1320, 0.11, 0.03), 0.004, 0.45)
    return fit(buf, d)


def sfx_toast():
    d = 0.50
    buf = zeros(N(d))
    put(buf, mallet(nf("G5"), 0.4, 0.14), 0.0, 0.8)
    put(buf, mallet(nf("C6"), 0.38, 0.16), 0.11, 0.9)
    return fit(buf, d)


def sfx_unlock():
    d = 1.40
    buf = zeros(N(d))
    for i, nm in enumerate(("D5", "F#5", "A5", "D6", "F#6")):
        put(buf, mallet(nf(nm), 1.0, 0.4), 0.085 * i, 0.5 + 0.08 * i)
    put(buf, pad([nf("D4"), nf("A4"), nf("F#5")], 1.3, [(0, 0), (0.4, 1), (1.3, 0)], 0.3), 0.05, 0.08)
    put(buf, sparkle(0.8, 14, (3000, 6500), (0.03, 0.1), 1.3, amp_tau=0.3), 0.4, 0.12)
    return fade(fit(reverb(fit(buf, d), 0.84, 0.4, 0.3, tail=0.0), d), 0.002, 0.3)


def sfx_codex_new():
    d = 1.20
    buf = zeros(N(d))
    sc = [nf(x) for x in ("E6", "G6", "A6", "B6", "D7", "E7")]
    put(buf, sparkle(0.9, 22, tau_rng=(0.04, 0.14), dist_pow=1.2, amp_tau=0.4, notes=sc), 0.0, 0.5)
    put(buf, keys(nf("E5"), 1.0, 0.5), 0.0, 0.5)
    put(buf, keys(nf("B5"), 1.0, 0.5), 0.16, 0.45)
    return fade(fit(reverb(fit(buf, d), 0.86, 0.35, 0.35, tail=0.0), d), 0.002, 0.3)


# ---------------------------------------------------------------------------
# Farming and base
# ---------------------------------------------------------------------------

def sfx_plant_harvest():
    d = 0.30
    buf = zeros(N(d))
    for t0, f in ((0.0, 3600), (0.022, 2700)):
        put(buf, noise_burst(0.02, 0.003, f, 2.0, "bp", 0.0003), t0, 0.9)
    put(buf, thump(700, 380, 0.03, 0.007), 0.02, 0.4)
    leaf = svf(grain_cloud(0.24, 26, (0.004, 0.014), 1.3, 0.09), 3400, 0.7, "bp")
    put(buf, norm(leaf), 0.03, 0.55)
    return fit(buf, d)


def sfx_plant_sow():
    d = 0.25
    buf = zeros(N(d))
    put(buf, noise_burst(0.015, 0.003, 2600, 2.0, "bp"), 0.0, 0.4)
    put(buf, thump(130, 65, 0.14, 0.035, attack=0.004), 0.05, 0.9)
    soil = svf(grain_cloud(0.16, 14, (0.004, 0.012), 1.3, 0.06), 800, 0.8, "bp")
    put(buf, norm(soil), 0.05, 0.6)
    return fit(buf, d)


def sfx_water_pour():
    d = 0.80
    e = env(d, (0, 0), (0.08, 1), (0.55, 0.9), (d, 0))
    fl = mul(svf(white(d), [1200 + 1400 * v for v in slow_random(d, 14)], 1.6, "bp"), e)
    sp = mul(svf(white(d), 4200, 0.7, "hp"), e)
    buf = add(norm(fl), scale(norm(sp), 0.2))
    for _ in range(22):
        put(buf, bubble(rng.uniform(450, 1500), rng.uniform(0.02, 0.05), 1.0),
            rng.uniform(0.05, 0.68), rng.uniform(0.15, 0.4))
    return fit(buf, d)


def sfx_sprinkler_loop():
    L = 2.0
    n = N(L)
    sweep = lfo(L, 1.0, 0.45, 1.0)
    hiss = nloop(L, 0.4, lambda T: add(svf(white(T), 5200, 0.8, "hp"),
                                       scale(svf(white(T), 2400, 1.0, "bp"), 0.7)))
    hiss = mul(hiss, sweep)
    ev = zeros(n)
    for k in range(12):
        t0 = k * L / 12
        put(ev, noise_burst(0.02, 0.003, 2100, 2.5, "bp", 0.0003), t0, 0.5 if k % 3 else 0.9)
        put(ev, thump(520, 320, 0.02, 0.005), t0, 0.3)
    drops = zeros(n)
    for _ in range(40):
        put(drops, noise_burst(0.012, 0.003, rng.uniform(1500, 4000), 1.5, "bp"),
            rng.uniform(0, L - 0.02), rng.uniform(0.2, 0.6))
    return add(rms_to(hiss, -17), scale(norm(fold(ev, n)), 0.3), scale(norm(fold(drops, n)), 0.15))


def sfx_fertilize():
    d = 0.40
    g = svf(grain_cloud(0.34, 46, (0.002, 0.007), 1.2, 0.14), 2300, 0.8, "bp")
    puff = mul(svf(white(d), 700, 0.8), env(d, (0, 0), (0.04, 1), (d, 0)))
    return add(fit(norm(g), d), scale(norm(puff), 0.35))


def sfx_plant_grow():
    d = 0.90
    buf = zeros(N(d))
    imp = stick_slip(0.3, lambda t: 40 + 160 * t / 0.3, 0.2)
    st = add(svf(imp, (500, 1300), 4.0, "bp"), scale(svf(imp, 2100, 3.0, "bp"), 0.4))
    put(buf, mul(norm(st), env(0.3, (0, 0), (0.05, 1), (0.3, 0))), 0.0, 0.25)
    put(buf, mallet(nf("A5"), 0.75, 0.3), 0.12, 0.6)
    put(buf, mallet(nf("E6"), 0.65, 0.3), 0.26, 0.6)
    leaf = svf(grain_cloud(0.3, 12, (0.004, 0.012)), 3600, 0.7, "bp")
    put(buf, norm(leaf), 0.05, 0.12)
    return fade(fit(reverb(fit(buf, d), 0.8, 0.4, 0.25, tail=0.0), d), 0.002, 0.2)


def _zap(d, lo, hi):
    n = N(d)
    f = []
    while len(f) < n:
        f += [rng.uniform(lo, hi)] * N(0.006)
    return svf(osc("saw", f[:n], d), 2200, 0.8, "hp")


def sfx_flower_pop():
    d = 0.35
    buf = zeros(N(d))
    put(buf, bubble(260, 0.07, 1.8), 0.0, 1.0)
    put(buf, noise_burst(0.02, 0.004, 1500, 1.0, "bp"), 0.0, 0.4)
    z = mul(_zap(0.2, 900, 3600), [v ** 2 for v in slow_random(0.2, 50)])
    put(buf, mul(norm(z), decay(0.2, 0.07, 0.003)), 0.07, 0.4)
    put(buf, chime(nf("A6"), 0.24, 0.06), 0.08, 0.3)
    return fit(buf, d)


def sfx_dome_enter():
    d = 0.60
    fc = lambda t: 1500 - 1100 * math.sin(math.pi * min(1.0, t / d))
    wh = mul(svf(white(d), fc, 1.2, "bp"), env(d, (0, 0), (0.18, 1), (0.36, 0.8), (d, 0)))
    bw = mul(osc("sine", lambda t: 150 - 70 * math.sin(math.pi * min(1.0, t / 0.4)), d),
             env(d, (0, 0), (0.12, 1), (0.3, 0.6), (d, 0)))
    air = mul(svf(white(d), 3000, 0.7, "hp"), env(d, (0, 0), (0.3, 1), (d, 0)))
    return add(norm(wh), scale(bw, 0.5), scale(norm(air), 0.08))


def sfx_dome_place():
    d = 2.50
    buf = zeros(N(d))
    for t0, f, g in ((0.0, 300, 0.9), (0.38, 240, 0.7)):
        put(buf, clank(f, 0.4, 0.09, 0.7), t0, 0.5 * g)
        put(buf, thump(120, 50, 0.3, 0.08), t0, 0.9 * g)
        put(buf, noise_burst(0.08, 0.02, 900), t0, 0.5 * g)
    sv = motor(0.75, lambda t: 90 + 120 * min(1.0, t / 0.5), 1300, 0.3)
    put(buf, mul(norm(sv), env(0.75, (0, 0), (0.05, 1), (0.65, 0.9), (0.75, 0))), 0.45, 0.3)
    hs = add(svf(white(0.95), 3000, 0.7, "hp"), scale(svf(white(0.95), (2500, 900), 1.5, "bp"), 0.8))
    put(buf, mul(norm(hs), env(0.95, (0, 0), (0.03, 1), (0.5, 0.6), (0.95, 0))), 1.15, 0.35)
    put(buf, thump(85, 32, 0.55, 0.17, st=0.06), 1.9, 1.3)
    put(buf, noise_burst(0.3, 0.06, 500), 1.9, 0.6)
    put(buf, clank(180, 0.5, 0.14, 0.5), 1.9, 0.3)
    deb = svf(grain_cloud(0.4, 14, (0.004, 0.012), 1.5, 0.12), 1500, 0.8, "bp")
    put(buf, norm(deb), 1.93, 0.18)
    buf = drive(norm(fit(buf, d)), 1.3)
    return fade(fit(reverb(buf, 0.84, 0.5, 0.22, tail=0.0), d), 0.001, 0.25)


def sfx_dome_hum_loop():
    L = 4.0
    hum = add(osc("sine", q(60, L), L), scale(osc("sine", q(60, L) + 0.5, L, phase=0.3), 0.6),
              scale(osc("sine", q(120, L), L, phase=0.1), 0.45),
              scale(osc("sine", q(120, L) + 0.25, L, phase=0.6), 0.3),
              scale(osc("sine", q(180, L), L, phase=0.4), 0.14),
              scale(osc("sine", q(240, L) - 0.25, L, phase=0.8), 0.06))
    hum = mul(hum, lfo(L, 0.25, 0.85, 1.0))
    air = nloop(L, 1.0, lambda T: mul(svf(white(T), 480, 0.7), slow_random(T, 0.8, 0.7, 1.0)))
    return add(rms_to(hum, -14), rms_to(air, -30))


def sfx_machine_on():
    d = 0.70
    buf = zeros(N(d))
    put(buf, thump(360, 200, 0.05, 0.012), 0.0, 0.7)
    put(buf, noise_burst(0.02, 0.003, 2600, 1.5, "bp"), 0.0, 0.6)
    m = motor(0.66, lambda t: 40 + 150 * (1 - math.exp(-t / 0.2)), (300, 1300), 0.25)
    put(buf, mul(norm(m), env(0.66, (0, 0), (0.1, 0.8), (0.45, 1), (0.66, 0))), 0.04, 0.7)
    return fit(buf, d)


def sfx_machine_off():
    d = 0.70
    buf = zeros(N(d))
    m = motor(0.55, lambda t: 40 + 150 * math.exp(-t / 0.18), (1300, 250), 0.25)
    put(buf, mul(norm(m), env(0.55, (0, 0), (0.02, 1), (0.3, 0.7), (0.55, 0))), 0.0, 0.7)
    put(buf, thump(200, 80, 0.14, 0.035), 0.5, 0.9)
    put(buf, noise_burst(0.03, 0.006, 1500, 1.2, "bp"), 0.5, 0.5)
    return fit(buf, d)


def sfx_pylon_loop():
    L = 2.0
    n = N(L)
    f = q(100, L)
    hum = add(osc("sine", f, L), scale(steady(osc("saw", f, L), lambda x: svf(x, 700, 1.0)), 0.5),
              scale(osc("sine", 3 * f + 0.5, L), 0.15), scale(osc("sine", 2 * f - 0.5, L), 0.3))
    hum = mul(hum, lfo(L, 0.5, 0.8, 1.0))
    bz = mul(steady(osc("square", f, L, duty=0.2), lambda x: svf(x, 2600, 1.5, "bp")), lfo(L, 1.5, 0.2, 1.0, 0.3, 2.0))
    cr = nloop(L, 0.3, lambda T: mul(svf(white(T), 5000, 0.8, "hp"),
                                     [v ** 6 for v in slow_random(T, 60)]))
    ev = zeros(n)
    for _ in range(5):
        z = mul(_zap(0.05, 1500, 5000), decay(0.05, 0.012, 0.001))
        put(ev, z, rng.uniform(0, L - 0.06), rng.uniform(0.4, 1.0))
    return add(rms_to(hum, -15), rms_to(bz, -32), rms_to(cr, -34), scale(norm(fold(ev, n)), 0.07))


def sfx_pump_loop():
    L = 2.0
    n = N(L)
    f = q(52, L)
    mot = add(osc("sine", f, L), scale(steady(osc("saw", f, L), lambda x: svf(x, 420, 1.0)), 0.6),
              scale(osc("sine", 2 * f + 0.5, L), 0.3))
    ev = zeros(n)
    for k in range(4):
        t0 = k * 0.5
        put(ev, thump(110, 55, 0.2, 0.05, attack=0.004), t0 + 0.02, 1.0)
        put(ev, clank(310, 0.08, 0.02, 0.4), t0 + 0.02, 0.12)
        sw = swoosh(0.3, (300, 900), 1.6, [(0, 0), (0.18, 1), (0.3, 0)])
        put(ev, sw, t0 + 0.12, 0.35)
        put(ev, bubble(rng.uniform(260, 420), 0.05, 0.8), t0 + 0.3, 0.2)
    return add(rms_to(mot, -18), scale(norm(fold(ev, n)), 0.42))


def sfx_pipe_place():
    d = 0.30
    f = 1480.0
    ring = partials(f, [1, 2.71, 5.2, 8.3], [1, 0.5, 0.22, 0.08], [0.09, 0.06, 0.03, 0.015], d,
                    detune=0.003)
    tube = thump(330, 250, 0.08, 0.025)
    tick = noise_burst(0.02, 0.003, 4200, 1.0, "bp", 0.0003)
    return add(norm(ring), scale(fit(tube, d), 0.45), scale(fit(tick, d), 0.4))


def sfx_generator_loop():
    L = 3.0
    n = N(L)
    blade = lfo(L, 11.0, 0.55, 1.0)
    fan = nloop(L, 0.5, lambda T: add(svf(white(T), 700, 0.9, "bp"), scale(svf(white(T), 2400, 0.8, "bp"), 0.3)))
    fan = mul(fan, blade)
    hum = add(osc("sine", q(73, L), L), scale(osc("sine", q(146, L) + 1 / 3, L), 0.35),
              scale(osc("sine", q(219, L), L), 0.1))
    ev = zeros(n)
    for _ in range(34):
        put(ev, bubble(rng.uniform(180, 620), rng.uniform(0.04, 0.09), rng.uniform(0.5, 1.1)),
            rng.uniform(0, L), rng.uniform(0.3, 1.0))
    ev = steady(fold(ev, n), lambda x: svf(x, 1500, 0.7))
    return add(rms_to(fan, -19), rms_to(hum, -19), scale(norm(ev), 0.22))


def sfx_splicer_run():
    d = 2.00
    w = 1.6
    n = N(w)
    wob = slow_random(w, 9, 0.0, 1.0)
    f1 = [320 + 380 * v + 60 * math.sin(TAU * 5.5 * i / SR) for i, v in enumerate(wob)]
    a = osc("sine", f1, w)
    b = osc("sine", [v * 1.503 for v in f1], w)
    c = svf(osc("saw", [v * 0.5 for v in f1], w), [v * 3 for v in f1], 3.0, "bp")
    s = mul(add(a, scale(b, 0.5), scale(c, 0.5)),
            [0.6 + 0.4 * math.sin(TAU * 11 * i / SR) for i in range(n)])
    s = mul(s, env(w, (0, 0), (0.15, 1), (1.35, 0.9), (w, 0)))
    buf = scale(norm(s), 0.5)
    for _ in range(20):
        put(buf, bubble(rng.uniform(500, 1600), rng.uniform(0.02, 0.05), 1.0),
            rng.uniform(0.1, 1.45), rng.uniform(0.1, 0.25))
    buf = fit(buf, d)
    put(buf, chime(nf("F#6"), 0.44, 0.13), 1.55, 0.7)
    put(buf, chime(nf("C#7"), 0.4, 0.1), 1.56, 0.2)
    return fade(fit(reverb(fit(buf, d), 0.75, 0.4, 0.15, tail=0.0), d), 0.002, 0.08)


def sfx_water_flow_loop():
    L = 2.0
    n = N(L)

    def make(T):
        a = mul(svf(white(T), [500 + 700 * v for v in slow_random(T, 7)], 2.2, "bp"),
                slow_random(T, 5, 0.5, 1.0))
        b = svf(white(T), 260, 0.8)
        return comb(add(rms_to(a, -16), rms_to(b, -20)), 0.0031, 0.5)
    bed = nloop(L, 0.4, make)
    ev = zeros(n)
    for _ in range(26):
        put(ev, bubble(rng.uniform(250, 900), rng.uniform(0.03, 0.07), rng.uniform(0.6, 1.2)),
            rng.uniform(0, L), rng.uniform(0.3, 1.0))
    return add(steady(bed, lambda x: svf(x, 2600, 0.7)), scale(norm(fold(ev, n)), 0.12))


# ---------------------------------------------------------------------------
# Robots
# ---------------------------------------------------------------------------

def _robot_note(f0, f1, dur):
    s = soft_tone((f0, f1), dur, 0.008, 0.04, 0.3, 0.15)
    v = osc("tri", (f0 * 0.5, f1 * 0.5), dur)
    return svf(add(s, scale(mul(v, adsr(dur, 0.008, 0.03, 0.8, 0.04)), 0.3)), 2800, 1.2)


def sfx_robot_ok():
    d = 0.30
    buf = zeros(N(d))
    put(buf, _robot_note(nf("E5"), nf("E5"), 0.1), 0.0, 0.9)
    put(buf, _robot_note(nf("A5") * 0.97, nf("A5"), 0.17), 0.11, 1.0)
    return fit(buf, d)


def sfx_robot_error():
    d = 0.40
    buf = zeros(N(d))
    put(buf, _robot_note(nf("A4"), nf("A4"), 0.14), 0.0, 1.0)
    put(buf, _robot_note(nf("F4"), nf("F4") * 0.9, 0.23), 0.16, 1.0)
    return fit(buf, d)


def sfx_robot_hover_loop():
    L = 1.5
    f = q(138, L)
    fan = add(osc("sine", f, L), scale(steady(osc("saw", f, L), lambda x: svf(x, 900, 1.0)), 0.45),
              scale(osc("sine", 2 * f + 2 / 3, L), 0.35), scale(osc("sine", 3 * f - 2 / 3, L), 0.12))
    fan = mul(fan, lfo(L, 2.0, 0.8, 1.0))
    air = nloop(L, 0.3, lambda T: mul(svf(white(T), 1700, 0.9, "bp"), slow_random(T, 9, 0.5, 1.0)))
    air = mul(air, lfo(L, 23.0, 0.6, 1.0))
    return add(rms_to(fan, -15), rms_to(air, -26))


def sfx_robot_work():
    d = 0.40
    buf = zeros(N(d))
    t = 0.0
    for _ in range(5):
        ln = rng.uniform(0.04, 0.08)
        f0 = rng.uniform(140, 380)
        f1 = f0 * rng.choice((0.7, 1.3, 1.6))
        m = motor(ln, (f0, f1), 1800, 0.2)
        put(buf, mul(norm(m), adsr(ln, 0.005, 0.01, 0.9, 0.012)), t, rng.uniform(0.6, 1.0))
        put(buf, noise_burst(0.01, 0.002, 3000, 1.5, "bp"), t + ln, 0.3)
        t += ln + rng.uniform(0.005, 0.02)
        if t > 0.31:
            break
    return fit(buf, d)


def sfx_drop_pod_land():
    d = 1.80
    buf = zeros(N(d))
    n1 = N(0.7)
    wh = mul(svf(white(0.7), (3500, 500), 1.2, "bp"), [(i / n1) ** 2.2 for i in range(n1)])
    rr = mul(svf(white(0.7), 500, 0.8), [(i / n1) ** 1.5 for i in range(n1)])
    put(buf, add(norm(wh), scale(norm(rr), 0.6)), 0.0, 0.6)
    put(buf, thump(100, 30, 0.6, 0.16, st=0.05), 0.7, 1.3)
    put(buf, noise_burst(0.3, 0.05, 650), 0.7, 0.9)
    put(buf, clank(230, 0.4, 0.1, 0.6), 0.7, 0.35)
    deb = svf(grain_cloud(0.5, 18, (0.004, 0.012), 1.5, 0.14), 1500, 0.8, "bp")
    put(buf, norm(deb), 0.74, 0.22)
    hs = mul(svf(white(1.0), (5000, 1800), 0.8, "hp"), env(1.0, (0, 0), (0.04, 1), (0.4, 0.5), (1.0, 0)))
    put(buf, norm(hs), 0.8, 0.28)
    buf = drive(norm(fit(buf, d)), 1.4)
    return fade(fit(reverb(buf, 0.82, 0.5, 0.2, tail=0.0), d), 0.002, 0.2)


# ---------------------------------------------------------------------------
# Creatures
# ---------------------------------------------------------------------------

def sfx_moth_flutter():
    d = 0.45
    n = N(d)
    rate = lambda t: 24.0 + 8.0 * math.sin(TAU * 2.2 * t)
    ph = 0.0
    am = [0.0] * n
    for i in range(n):
        ph += rate(i / SR) / SR
        x = ph - int(ph)
        am[i] = math.exp(-x / 0.18) * min(1.0, x / 0.04)
    nz = add(svf(white(d), 1900, 1.0, "bp"), scale(svf(white(d), 4200, 1.2, "bp"), 0.4))
    s = mul(mul(nz, am), env(d, (0, 0), (0.06, 1), (0.3, 0.8), (d, 0)))
    return svf(s, 5000, 0.7)


def sfx_puff_grunt():
    d = 0.40
    v = growl(0.34, lambda t: 92 - 22 * t / 0.34 + 6 * math.sin(TAU * 9 * t), "u", 17, 0.35, 0.85, 0.12)
    v = mul(v, env(0.34, (0, 0), (0.05, 1), (0.16, 0.8), (0.34, 0)))
    buf = zeros(N(d))
    put(buf, svf(v, 900, 0.7), 0.0, 1.0)
    br = mul(svf(white(0.25), 700, 1.5, "bp"), env(0.25, (0, 0), (0.05, 1), (0.25, 0)))
    put(buf, norm(br), 0.12, 0.15)
    return fit(buf, d)


def _click(f):
    c = partials(f, [1, 1.7, 2.9], [1, 0.5, 0.3], [0.005, 0.004, 0.003], 0.03)
    return add(c, scale(noise_burst(0.03, 0.002, min(9000.0, f * 1.6), 2.0, "bp", 0.0003), 0.8))


def sfx_skitter_chitter():
    d = 0.60
    buf = zeros(N(d))
    t = 0.01
    gap = 0.05
    while t < 0.54:
        put(buf, _click(rng.uniform(1700, 3400)), t, rng.uniform(0.5, 1.0))
        gap = max(0.014, gap * rng.uniform(0.7, 1.05))
        if gap < 0.018 and rng.random() < 0.5:
            gap = rng.uniform(0.05, 0.09)
        t += gap
    hs = mul(svf(white(d), 5200, 1.0, "hp"), [0.3 + 0.7 * v for v in slow_random(d, 10)])
    buf = add(fit(buf, d), scale(mul(hs, env(d, (0, 0), (0.1, 1), (0.5, 0.8), (d, 0))), 0.06))
    return fit(reverb(buf, 0.7, 0.3, 0.15, tail=0.0), d)


def _screech(d, f0, f1, f2, vr=42.0):
    f = lambda t: (f0 + (f1 - f0) * min(1.0, t / (d * 0.35)) if t < d * 0.35
                   else f1 + (f2 - f1) * (t - d * 0.35) / (d * 0.65))
    s = osc("saw", f, d, vib=0.06, vib_rate=vr)
    s2 = osc("square", lambda t: f(t) * 1.49, d, duty=0.3, vib=0.04, vib_rate=vr * 1.3)
    v = formant(add(s, scale(s2, 0.5), scale(white(d), 0.3)), ("e", "i"), 1.5, 0.5)
    return drive(norm(v), 2.0)


def sfx_skitter_lunge():
    d = 0.50
    hs = mul(svf(white(d), (2500, 6000), 0.8, "hp"), env(d, (0, 0), (0.05, 1), (0.25, 0.6), (d, 0)))
    sc = mul(_screech(0.42, 900, 1900, 1300), env(0.42, (0, 0), (0.04, 1), (0.25, 0.8), (0.42, 0)))
    buf = scale(norm(hs), 0.55)
    put(buf, sc, 0.05, 0.7)
    put(buf, _click(2600), 0.0, 0.5)
    put(buf, _click(3100), 0.03, 0.4)
    return fit(buf, d)


def sfx_skitter_die():
    d = 0.70
    sc = _screech(0.55, 1700, 1100, 260, 30.0)
    sc = mul(mul(sc, env(0.55, (0, 0), (0.01, 1), (0.2, 0.7), (0.55, 0))),
             [0.6 + 0.4 * math.sin(TAU * 27 * i / SR) for i in range(N(0.55))])
    buf = zeros(N(d))
    put(buf, sc, 0.0, 0.8)
    put(buf, thump(200, 80, 0.12, 0.03), 0.02, 0.8)
    cr = svf(grain_cloud(0.2, 14, (0.003, 0.009), 1.5, 0.06), 2200, 0.9, "bp")
    put(buf, norm(cr), 0.02, 0.6)
    for t0 in (0.36, 0.45, 0.57):
        put(buf, _click(rng.uniform(1500, 2400)), t0, 0.35)
    return fit(buf, d)


def sfx_web_spit():
    d = 0.30
    buf = zeros(N(d))
    sp = mul(svf(white(0.2), (3200, 700), 2.0, "bp"), decay(0.2, 0.05, 0.004))
    put(buf, norm(sp), 0.0, 1.0)
    put(buf, thump(260, 110, 0.06, 0.018), 0.0, 0.6)
    put(buf, bubble(520, 0.04, 1.4), 0.01, 0.4)
    st = mul(svf(white(0.22), (1500, 4500), 4.0, "bp"), env(0.22, (0, 0), (0.05, 0.6), (0.22, 0)))
    put(buf, norm(st), 0.06, 0.25)
    return fit(buf, d)


def sfx_web_hit():
    d = 0.30
    buf = zeros(N(d))
    put(buf, noise_burst(0.12, 0.025, 1300, 0.8), 0.0, 1.0)
    put(buf, thump(170, 80, 0.1, 0.03), 0.0, 0.6)
    sq = mul(svf(white(0.24), [700 + 900 * v for v in slow_random(0.24, 28)], 4.0, "bp"),
             decay(0.24, 0.07, 0.01))
    put(buf, norm(sq), 0.02, 0.6)
    for _ in range(4):
        put(buf, bubble(rng.uniform(500, 1200), 0.03, 1.2), rng.uniform(0.03, 0.2), 0.25)
    return fit(buf, d)


def sfx_leech_latch():
    d = 0.45
    n1 = N(0.32)
    gate = [0.45 + 0.55 * v ** 2 for v in slow_random(0.32, 34)]
    suck = svf(white(0.32), (280, 1900), 5.0, "bp")
    suck = mul(mul(suck, gate), [(i / n1) ** 1.3 for i in range(n1)])
    lo = mul(svf(white(0.32), 400, 1.0), [(i / n1) for i in range(n1)])
    buf = zeros(N(d))
    put(buf, tail_fade(add(norm(suck), scale(norm(lo), 0.4)), 0.004), 0.0, 0.8)
    put(buf, bubble(380, 0.06, 2.2), 0.32, 1.0)
    put(buf, thump(170, 80, 0.1, 0.03), 0.32, 0.7)
    put(buf, noise_burst(0.04, 0.01, 1500, 1.0, "bp"), 0.32, 0.4)
    for _ in range(4):
        put(buf, bubble(rng.uniform(500, 1100), 0.03, 1.0), rng.uniform(0.04, 0.3), 0.2)
    return fit(buf, d)


def sfx_maw_rumble():
    d = 1.60
    e = env(d, (0, 0), (0.5, 0.5), (1.2, 1.0), (d, 0))
    rb = mul(mul(svf(brown(d), 110, 0.8), e), slow_random(d, 13, 0.45, 1.0))
    sub = mul(osc("sine", lambda t: 40 + 4 * math.sin(TAU * 0.9 * t), d), e)
    mid = mul(mul(svf(white(d), 260, 1.5, "bp"), e), slow_random(d, 16, 0.2, 1.0))
    buf = add(norm(rb), scale(sub, 0.55), scale(norm(mid), 0.3))
    for _ in range(10):
        t0 = rng.uniform(0.3, 1.45)
        put(buf, noise_burst(0.03, 0.006, rng.uniform(900, 2200), 1.0, "bp"), t0, rng.uniform(0.05, 0.16))
    return fade(fit(drive(buf, 1.4), d), 0.01, 0.15)


def sfx_maw_erupt():
    d = 1.40
    buf = zeros(N(d))
    body = svf(white(0.8), lambda t: 200 + 3800 * math.exp(-t / 0.08), 0.8)
    put(buf, mul(body, decay(0.8, 0.22, 0.002)), 0.0, 1.0)
    put(buf, thump(110, 34, 0.8, 0.3, st=0.1), 0.0, 1.1)
    deb = svf(grain_cloud(0.9, 36, (0.004, 0.014), 1.5, 0.3, start=0.05), 1800, 0.8, "bp")
    put(buf, norm(deb), 0.0, 0.35)
    roar = growl(1.15, lambda t: 62 + 70 * math.exp(-t / 0.25) + 5 * math.sin(TAU * 6 * t),
                 ("a", "o"), 23, 0.55, 0.75, 0.3)
    put(buf, mul(roar, env(1.15, (0, 0), (0.08, 1), (0.6, 0.8), (1.15, 0))), 0.12, 0.9)
    buf = drive(norm(fit(buf, d)), 2.0)
    return fade(fit(reverb(buf, 0.84, 0.5, 0.25, tail=0.0), d), 0.001, 0.25)


def sfx_brood_roar():
    d = 2.00
    f = lambda t: 44 + 22 * math.exp(-((t - 0.45) / 0.35) ** 2) - 6 * t / d + 2.5 * math.sin(TAU * 5 * t)
    a = growl(d, f, ("o", "a"), 19, 0.6, 0.6, 0.25, 0.5)
    b = growl(d, lambda t: f(t) * 1.507, ("u", "o"), 27, 0.5, 0.55, 0.15, 0.5)
    c = svf(osc("saw", lambda t: f(t) * 0.5, d), 160, 1.0)
    e = env(d, (0, 0), (0.18, 0.8), (0.5, 1.0), (1.3, 0.85), (d, 0))
    v = mul(add(a, scale(b, 0.55), scale(norm(c), 0.8)), e)
    rasp = mul(mul(svf(white(d), (900, 500), 1.0, "bp"), slow_random(d, 30, 0.2, 1.0)), e)
    sub = mul(osc("sine", lambda t: 36 + 6 * math.exp(-t / 0.5), d), e)
    buf = drive(norm(add(v, scale(norm(rasp), 0.35), scale(sub, 1.0))), 2.6)
    return fade(fit(reverb(buf, 0.9, 0.55, 0.35, tail=0.0), d), 0.005, 0.35)


def sfx_brood_spawn():
    d = 0.80
    buf = zeros(N(d))
    t = 0.0
    for _ in range(7):
        f0 = rng.uniform(140, 420)
        put(buf, bubble(f0, rng.uniform(0.05, 0.09), 1.6), t, rng.uniform(0.6, 1.0))
        put(buf, noise_burst(0.07, 0.018, rng.uniform(700, 1600), 2.0, "bp", 0.004), t, 0.5)
        put(buf, thump(f0 * 0.6, f0 * 0.3, 0.08, 0.025), t, 0.4)
        t += rng.uniform(0.06, 0.13)
    sq = mul(svf(white(0.7), [500 + 800 * v for v in slow_random(0.7, 20)], 3.5, "bp"),
             env(0.7, (0, 0), (0.05, 1), (0.5, 0.6), (0.7, 0)))
    put(buf, norm(sq), 0.0, 0.25)
    return fit(svf(fit(buf, d), 2600, 0.7), d)


def sfx_creature_hit():
    d = 0.20
    buf = zeros(N(d))
    put(buf, thump(210, 85, 0.12, 0.03), 0.0, 1.0)
    put(buf, noise_burst(0.08, 0.016, 1500, 0.8), 0.0, 0.8)
    sq = mul(osc("sine", (1500, 850), 0.1, vib=0.05, vib_rate=45), decay(0.1, 0.03, 0.004))
    put(buf, sq, 0.02, 0.3)
    return fit(buf, d)


# ---------------------------------------------------------------------------
# Animal calls and other ambience events
# ---------------------------------------------------------------------------

def _whistle(f, d, h2=0.12):
    """f: callable(t) -> Hz. A pure, slightly breathy whistle note."""
    s = osc("sine", f, d)
    s = add(s, scale(osc("sine", lambda t: 2 * f(t), d), h2))
    return mul(s, env(d, (0, 0), (d * 0.2, 1), (d * 0.65, 0.8), (d, 0)))


def call_trill(base):
    """Falling run of short bent whistles."""
    buf = zeros(1)
    t = 0.0
    k = rng.randint(3, 6)
    for i in range(k):
        f0 = base * (1.0 - 0.07 * i)
        ln = rng.uniform(0.07, 0.11)
        put(buf, _whistle(lambda tt, f0=f0, ln=ln: f0 * (1.0 + 0.18 * math.sin(math.pi * tt / ln)), ln),
            t, 1.0 - 0.1 * i)
        t += ln + rng.uniform(0.02, 0.05)
    return buf


def call_two_tone(base):
    """Slow 'pee-ooo' with an odd upward hook at the end."""
    a = _whistle(lambda t: base * (1.0 + 0.02 * math.sin(TAU * 7 * t)), 0.28)
    b = _whistle(lambda t: base * 0.76 * (1.0 + 0.25 * max(0.0, t - 0.4) ** 2 * 6), 0.55)
    buf = zeros(1)
    put(buf, a, 0.0)
    put(buf, b, 0.34, 0.9)
    return buf


def call_warble(base):
    """Wobbling alien song: fast FM warble that slows down."""
    d = rng.uniform(0.6, 0.9)
    f = lambda t: base * (1.0 + 0.12 * math.sin(TAU * (26 * t - 12 * t * t)) - 0.15 * t / d)
    return _whistle(f, d, 0.2)


def call_insect(f, d, rate=48.0):
    """Dry insect buzz-trill."""
    n = N(d)
    w = TAU * rate / SR
    s = osc("sine", f, d)
    s = add(s, scale(osc("sine", f * 1.31, d), 0.5))
    am = [max(0.0, math.sin(w * i)) ** 3 for i in range(n)]
    return mul(mul(s, am), env(d, (0, 0), (d * 0.3, 1), (d * 0.8, 0.9), (d, 0)))


def call_croak(base):
    """Low rattling croak in two or three pulses."""
    buf = zeros(1)
    t = 0.0
    for _ in range(rng.randint(2, 3)):
        ln = rng.uniform(0.09, 0.14)
        g = growl(ln, (base, base * 0.85), "o", 38, 0.8, 1.1, 0.1)
        put(buf, mul(g, env(ln, (0, 0), (0.01, 1), (ln, 0))), t)
        t += ln + rng.uniform(0.04, 0.08)
    return buf


def call_hoot(base, count=2):
    """Soft hollow hoots."""
    buf = zeros(1)
    t = 0.0
    for i in range(count):
        ln = 0.42 if i == 0 else 0.3
        f = lambda tt, ln=ln: base * (1.0 + 0.05 * math.sin(math.pi * tt / ln) - 0.04 * tt / ln)
        h = _whistle(f, ln, 0.25)
        br = mul(svf(white(ln), base * 2, 3.0, "bp"), env(ln, (0, 0), (ln * 0.3, 1), (ln, 0)))
        put(buf, add(h, scale(norm(br), 0.08)), t, 1.0 if i == 0 else 0.8)
        t += ln + (0.28 if i == 0 else 0.1)
    return buf


def call_moan(base, d=2.4):
    """Far-off, whale-like rising and falling call."""
    f = lambda t: base * (1.0 + 0.22 * math.sin(math.pi * t / d) ** 2 + 0.006 * math.sin(TAU * 5 * t))
    s = add(osc("sine", f, d), scale(osc("saw", lambda t: f(t) * 0.5, d), 0.25))
    s = svf(s, base * 2.5, 1.5)
    return mul(s, env(d, (0, 0), (d * 0.35, 1), (d * 0.7, 0.7), (d, 0)))


def _drip(f0, dd=0.035):
    s = mul(osc("sine", lambda tt: f0 * (1 + 1.4 * tt / dd), dd), decay(dd, 0.012, 0.001))
    return tail_fade(s, 0.004)


def _drips(L, gap_rng, f_rng):
    ev = zeros(N(L))
    t = rng.uniform(0.2, 1.0)
    while t < L:
        put(ev, _drip(rng.uniform(*f_rng)), t, rng.uniform(0.4, 1.0))
        if rng.random() < 0.25:
            put(ev, _drip(rng.uniform(*f_rng) * 1.3), t + rng.uniform(0.08, 0.16), 0.4)
        t += rng.uniform(*gap_rng)
    return ev


def _wind(T, f_lo, f_hi, rate=0.3, q_=1.2, gust=(0.3, 1.0)):
    fc = [f_lo + (f_hi - f_lo) * v for v in slow_random(T, rate)]
    return mul(svf(white(T), fc, q_, "bp"), slow_random(T, rate * 0.8, gust[0], gust[1]))


def _amb(L, XN, periodic=None, noise=None, events=None):
    """Sum an exactly periodic part (already L long), a noise bed renderer
    (crossfaded over XN seconds) and an event buffer (tails folded round)."""
    n = N(L)
    out = [0.0] * n
    if periodic is not None:
        assert len(periodic) == n
        out = add(out, periodic)
    if noise is not None:
        out = add(out, nloop(L, XN, noise))
    if events is not None:
        out = add(out, fold(events, n))
    return out


# ---------------------------------------------------------------------------
# Weather and ambience
# ---------------------------------------------------------------------------

def amb_surface_day():
    L = 20.0

    def noise(T):
        gust = slow_random(T, 0.22, 0.25, 1.0)
        body = mul(svf(white(T), [350 + 500 * g for g in gust], 1.1, "bp"), gust)
        leaves = mul(mul(svf(white(T), 4200, 0.7, "bp"), [g * g for g in gust]),
                     slow_random(T, 8.0, 0.35, 1.0))
        low = lp1(brown(T), 140)
        return add(rms_to(body, -23), rms_to(leaves, -31), rms_to(low, -30))

    ev = zeros(N(L))
    calls = [
        (1.2, call_two_tone(1450), 0.55), (4.6, call_trill(2900), 0.4),
        (6.9, call_warble(2100), 0.3), (9.8, call_two_tone(1380), 0.3),
        (12.4, call_croak(170), 0.35), (14.1, call_trill(3300), 0.3),
        (16.7, call_warble(1750), 0.45), (18.3, call_trill(2600), 0.2),
    ]
    for t0, c, g in calls:
        put(ev, c, t0 + rng.uniform(-0.2, 0.2), g)
    for t0 in (3.1, 10.9, 15.4):
        put(ev, call_insect(rng.uniform(4300, 5200), rng.uniform(0.5, 1.1), rng.uniform(38, 60)), t0, 0.12)
    ev = svf(ev, 4500, 0.7)
    ev = reverb(ev, 0.9, 0.35, 0.9, tail=3.0)
    return _amb(L, 2.0, noise=noise, events=scale(norm(ev), 0.2))


def amb_surface_night():
    L = 20.0

    def noise(T):
        hush = mul(svf(white(T), [220 + 200 * v for v in slow_random(T, 0.15)], 0.9, "bp"),
                   slow_random(T, 0.2, 0.5, 1.0))
        air = mul(svf(white(T), 5200, 0.7, "hp"), slow_random(T, 0.3, 0.5, 1.0))
        return add(rms_to(hush, -27), rms_to(air, -44), rms_to(lp1(brown(T), 110), -30))

    ev = zeros(N(L))
    # two cricket species: steady pulsing chirps, each a short train of ticks
    for f, period, ticks, tick_gap, g, lo, hi in ((3650, 0.5, 3, 0.034, 0.5, 0.0, 20.0),
                                                 (4480, 0.8, 5, 0.022, 0.3, 2.4, 13.6)):
        tick = mul(osc("sine", f, 0.02), env(0.02, (0, 0), (0.004, 1), (0.02, 0)))
        t = lo
        while t < hi - 0.01:
            a = g * rng.uniform(0.7, 1.0) * min(1.0, (t - lo) / 1.5 + 0.3, (hi - t) / 1.5 + 0.3)
            for k in range(ticks):
                put(ev, tick, t + k * tick_gap, a)
            t += period
    far = zeros(N(L))
    put(far, call_hoot(310, 2), 2.6, 0.9)
    put(far, call_hoot(262, 3), 11.3, 0.7)
    put(far, call_hoot(330, 2), 16.9, 0.45)
    put(far, call_moan(190, 2.6), 6.4, 0.5)
    put(far, call_croak(120), 14.6, 0.3)
    far = reverb(svf(far, 1800, 0.7), 0.92, 0.4, 1.2, tail=3.0)
    ev = reverb(scale(ev, 0.09), 0.8, 0.3, 0.4, tail=1.0)
    ev = add(ev, scale(norm(far), 0.2))
    return _amb(L, 2.0, noise=noise, events=ev)


def amb_rain():
    L = 12.0

    def noise(T):
        hi = mul(svf(white(T), 3800, 0.6, "hp"), slow_random(T, 45, 0.55, 1.0))
        mid = mul(svf(white(T), 1500, 0.7, "bp"), slow_random(T, 0.4, 0.75, 1.0))
        low = svf(white(T), 320, 0.7)
        return add(rms_to(hi, -24), rms_to(mid, -24), rms_to(low, -28))

    ev = zeros(N(L))
    for _ in range(420):       # drops on leaves: soft, mid-pitched pats
        put(ev, noise_burst(0.02, 0.005, rng.uniform(700, 2600), 1.2, "bp", 0.0005),
            rng.uniform(0, L), rng.uniform(0.15, 0.7))
    for _ in range(34):        # drops on dome plating and old metal: short pings
        f = rng.choice((1320, 1710, 2080, 2540, 3100)) * rng.uniform(0.97, 1.03)
        put(ev, partials(f, [1, 2.4, 4.1], [1, 0.4, 0.15], [0.05, 0.03, 0.015], 0.2),
            rng.uniform(0, L), rng.uniform(0.1, 0.4))
    for _ in range(16):        # fat drips off leaves into puddles
        put(ev, _drip(rng.uniform(600, 1300), 0.04), rng.uniform(0, L), rng.uniform(0.3, 0.7))
    ev = reverb(ev, 0.7, 0.5, 0.25, tail=1.0)
    return _amb(L, 1.5, noise=noise, events=scale(norm(ev), 0.22))


def amb_mist():
    L = 12.0

    def noise(T):
        a = mul(svf(white(T), [200 + 260 * v for v in slow_random(T, 0.2)], 0.8),
                slow_random(T, 0.25, 0.45, 1.0))
        b = mul(svf(white(T), [500 + 300 * v for v in slow_random(T, 0.3)], 2.5, "bp"),
                slow_random(T, 0.2, 0.0, 1.0))
        c = mul(svf(white(T), 3000, 0.7, "bp"), slow_random(T, 0.3, 0.3, 1.0))
        return add(rms_to(a, -19), rms_to(b, -32), rms_to(c, -44))

    ev = zeros(N(L))
    for t0 in (3.7, 9.2):
        put(ev, _drip(rng.uniform(700, 1000)), t0, 1.0)
    put(ev, call_moan(150, 3.0), 5.0, 0.6)
    ev = reverb(svf(ev, 1200, 0.7), 0.92, 0.5, 1.2, tail=3.0)
    return _amb(L, 2.0, noise=noise, events=scale(norm(ev), 0.07))


def amb_dome():
    L = 10.0
    hum = add(osc("sine", q(60, L), L), scale(osc("sine", q(60, L) + 0.3, L, phase=0.2), 0.6),
              scale(osc("sine", q(120, L), L, phase=0.5), 0.4),
              scale(osc("sine", q(120, L) + 0.2, L, phase=0.9), 0.25),
              scale(osc("sine", q(180, L) - 0.1, L, phase=0.7), 0.1))
    hum = mul(hum, lfo(L, 0.1, 0.8, 1.0))

    def noise(T):
        vent = mul(svf(white(T), 520, 0.7), slow_random(T, 0.3, 0.7, 1.0))
        air = svf(white(T), 2400, 0.8, "bp")
        return add(rms_to(vent, -29), rms_to(air, -45))

    ev = zeros(N(L))
    for t0, f in ((2.3, 980), (6.1, 1240), (6.32, 1500), (8.8, 860)):
        put(ev, _drip(f), t0, rng.uniform(0.6, 1.0))
    ev = echo(ev, 0.19, 0.3, 0.3, tail=1.0, lp_fc=3000)
    tk = zeros(N(L))
    for t0 in (4.4, 4.52):
        put(tk, noise_burst(0.012, 0.002, 2400, 2.0, "bp"), t0, 0.5)
    ev = reverb(add(ev, tk), 0.8, 0.35, 0.5, tail=1.5)
    return _amb(L, 1.5, periodic=rms_to(hum, -22), noise=noise, events=scale(norm(ev), 0.16))


def amb_cave():
    L = 20.0
    tone = add(osc("sine", q(55, L), L), scale(osc("sine", q(55, L) + 0.15, L, phase=0.3), 0.7),
               scale(osc("sine", q(82.5, L), L, phase=0.6), 0.25))
    tone = mul(tone, lfo(L, 0.1, 0.6, 1.0))

    def noise(T):
        hollow = mul(svf(white(T), [220 + 240 * v for v in slow_random(T, 0.25)], 5.0, "bp"),
                     slow_random(T, 0.2, 0.2, 1.0))
        whistle = mul(svf(white(T), [760 + 90 * v for v in slow_random(T, 0.4)], 18.0, "bp"),
                      [v ** 3 for v in slow_random(T, 0.18)])
        return add(rms_to(hollow, -25), rms_to(whistle, -38), rms_to(lp1(brown(T), 120), -28))

    ev = _drips(L, (0.8, 2.6), (800, 1700))
    ev = echo(ev, 0.37, 0.5, 0.45, tail=2.5, lp_fc=2400)
    for t0 in (7.3, 16.2):    # a pebble somewhere
        for k in range(3):
            put(ev, noise_burst(0.02, 0.004, rng.uniform(1200, 2200), 1.5, "bp"),
                t0 + 0.13 * k * k + rng.uniform(0, 0.03), 0.3 * 0.6 ** k)
    ev = reverb(ev, 0.93, 0.3, 0.9, tail=3.5)
    return _amb(L, 2.0, periodic=rms_to(tone, -27), noise=noise, events=scale(norm(ev), 0.3))


def amb_deep():
    L = 20.0
    f0 = q(41.2, L)
    drone = add(osc("sine", f0, L), scale(osc("sine", f0 + 0.1, L, phase=0.3), 0.8),
                scale(osc("sine", q(61.7, L), L, phase=0.5), 0.45),
                scale(osc("sine", q(82.4, L) + 0.15, L, phase=0.2), 0.3),
                scale(osc("sine", q(123.5, L), L, phase=0.8), 0.12),
                scale(osc("sine", q(174.6, L), L, phase=0.1), 0.05))
    drone = mul(drone, lfo(L, 0.1, 0.65, 1.0, 0.25))
    eerie = mul(add(osc("sine", q(466.2, L), L), osc("sine", q(466.2, L) + 0.35, L)),
                lfo(L, 0.05, 0.0, 1.0, 0.6, 3.0))

    def noise(T):
        air = mul(svf(white(T), [150 + 150 * v for v in slow_random(T, 0.2)], 3.0, "bp"),
                  slow_random(T, 0.2, 0.3, 1.0))
        return add(rms_to(air, -30), rms_to(lp1(brown(T), 90), -25))

    ev = zeros(N(L))
    for t0, g in ((2.5, 0.8), (10.4, 1.0), (15.8, 0.6)):   # distant rumbles
        ln = rng.uniform(3.5, 5.0)
        r = mul(mul(lp1(brown(ln), 80), slow_random(ln, 6, 0.5, 1.0)),
                env(ln, (0, 0), (ln * 0.35, 1), (ln, 0)))
        put(ev, norm(r), t0, g)
    dr = zeros(N(L))
    for t0 in (1.4, 6.6, 8.9, 13.7, 18.2):
        put(dr, _drip(rng.uniform(600, 1100)), t0, rng.uniform(0.5, 1.0))
    dr = echo(dr, 0.52, 0.55, 0.5, tail=3.0, lp_fc=1600)
    put(dr, call_moan(98, 3.4), 11.8, 0.25)
    dr = reverb(dr, 0.94, 0.4, 1.0, tail=4.0)
    ev = add(scale(ev, 0.22), scale(norm(dr), 0.16))
    return _amb(L, 2.0, periodic=add(rms_to(drone, -19), rms_to(eerie, -44)), noise=noise, events=ev)


def amb_abyss():
    L = 20.0
    f0 = q(32.7, L)
    sub = add(osc("sine", f0, L), scale(osc("sine", f0 + 0.05, L, phase=0.4), 0.8),
              scale(osc("sine", 2 * f0 + 0.1, L, phase=0.1), 0.5),
              scale(osc("sine", 3 * f0, L, phase=0.7), 0.3),
              scale(osc("sine", q(46.2, L), L, phase=0.2), 0.35))
    sub = drive(norm(mul(sub, lfo(L, 0.15, 0.6, 1.0))), 1.6)

    def noise(T):
        hiss = mul(svf(white(T), 5200, 0.7, "hp"), slow_random(T, 0.5, 0.35, 1.0))
        sizz = mul(svf(white(T), 3000, 1.0, "bp"), [v ** 4 for v in slow_random(T, 25)])
        heat = mul(svf(white(T), [300 + 300 * v for v in slow_random(T, 0.3)], 1.5, "bp"),
                   slow_random(T, 0.25, 0.3, 1.0))
        return add(rms_to(hiss, -35), rms_to(sizz, -37), rms_to(heat, -31), rms_to(lp1(brown(T), 70), -24))

    ev = zeros(N(L))
    for t0, g in ((3.4, 1.0), (9.7, 0.6), (13.1, 0.8), (17.6, 0.45)):   # far-off cracks
        c = add(noise_burst(0.25, 0.03, rng.uniform(600, 1100), 1.2, "bp", 0.0005),
                scale(thump(140, 50, 0.25, 0.08), 0.8))
        put(ev, c, t0, g)
        put(ev, noise_burst(0.15, 0.03, 700, 1.0, "bp"), t0 + rng.uniform(0.18, 0.3), g * 0.4)
    ev = echo(ev, 0.61, 0.45, 0.4, tail=3.0, lp_fc=900)
    for _ in range(7):         # lava blups
        put(ev, bubble(rng.uniform(70, 130), rng.uniform(0.12, 0.2), 0.7), rng.uniform(0, L), 0.35)
    ev = reverb(svf(ev, 1400, 0.7), 0.94, 0.5, 1.0, tail=4.0)
    return _amb(L, 2.0, periodic=rms_to(sub, -17), noise=noise, events=scale(norm(ev), 0.3))


def _thunder(d, peaks, fc):
    e = env_pts(d, [(0, 0)] + peaks + [(d, 0)])
    mid = mul(svf(white(d), [fc * (0.5 + v) for v in slow_random(d, 2.5)], 0.7),
              slow_random(d, 11, 0.4, 1.0))
    low = mul(lp1(brown(d), 90), slow_random(d, 5, 0.5, 1.0))
    s = mul(add(rms_to(mid, -20), rms_to(low, -14)), e)
    sub = mul(osc("sine", lambda t: 38 + 6 * math.sin(TAU * 0.4 * t), d), e)
    buf = add(s, rms_to(sub, -22))
    for t0, v in peaks:
        if v >= 0.7 and t0 < d - 0.4:
            put(buf, noise_burst(0.3, 0.06, fc * 2, 0.8, "lp", 0.01), max(0.0, t0 - 0.05), 0.12 * v)
    buf = fit(reverb(drive(norm(fit(buf, d)), 1.3), 0.92, 0.6, 0.6, tail=0.0), d)
    return fade(buf, 0.03, 1.2)


def sfx_thunder_1():
    return _thunder(4.0, [(0.18, 1.0), (0.6, 0.4), (0.95, 0.75), (1.5, 0.3), (2.0, 0.5), (2.9, 0.18)], 320)


def sfx_thunder_2():
    return _thunder(5.0, [(0.5, 0.45), (0.9, 0.3), (1.4, 1.0), (2.0, 0.45), (2.5, 0.7), (3.3, 0.3),
                          (3.8, 0.4), (4.4, 0.12)], 230)


# ---------------------------------------------------------------------------
# Music. Bars of slow pad chords with a sparse lead on top; everything goes
# through one reverb whose tail folds round the loop, then a tape wobble that
# is periodic over the loop.
# ---------------------------------------------------------------------------

# The theme (A minor, 8 beats to the bar). Each note is (beat, name, gain).
MENU_CHORDS = [
    ["A2", "E3", "B3", "C4", "E4"],      # Am(add9)
    ["F2", "C3", "A3", "C4", "E4"],      # Fmaj7
    ["C3", "G3", "D4", "E4"],            # C(add9)
    ["G2", "D3", "G3", "B3", "D4"],      # G
    ["A2", "E3", "A3", "C4", "E4"],      # Am
    ["F2", "C3", "E3", "A3", "C4"],      # Fmaj7
    ["D3", "A3", "C4", "E4", "F4"],      # Dm9
    ["E2", "B2", "E3", "A3", "B3"],      # Esus4 (left open; falls back to Am)
]
MENU_LEAD = [
    [(0, "E5", 1.0), (1, "A5", 0.9), (2.5, "G5", 0.8), (3.5, "E5", 0.9)],   # the motif
    [(0, "C5", 0.9), (2, "D5", 0.7), (3, "E5", 0.8)],
    [(0, "E5", 0.9), (1, "G5", 0.9), (2.5, "E5", 0.8), (3.5, "D5", 0.8)],
    [(0, "B4", 0.9), (3, "D5", 0.6)],
    [(0, "E5", 1.0), (1, "A5", 0.9), (2.5, "G5", 0.8), (3.5, "E5", 0.9)],
    [(0, "C5", 0.9), (2, "A4", 0.7), (3, "C5", 0.8)],
    [(0, "D5", 0.9), (1, "F5", 0.9), (2.5, "E5", 0.8), (3.5, "C5", 0.8)],
    [(0, "B4", 0.9), (2.5, "A4", 0.5), (4, "E4", 0.6)],
]


def _pad_track(T, chords, bar, over, bright, att):
    buf = zeros(N(T))
    for b, ch in enumerate(chords):
        ln = bar + over
        put(buf, pad([nf(x) for x in ch], ln, [(0, 0), (att, 1), (bar, 0.85), (ln, 0)], bright), b * bar)
    return buf


def _lead_track(T, bars, bar, beat, inst, octave=1.0, human=0.012, **kw):
    buf = zeros(N(T))
    for b, notes in enumerate(bars):
        for bt, nm, g in notes:
            t = max(0.0, b * bar + bt * beat + rng.uniform(-human, human))
            put(buf, inst(nf(nm) * octave, **kw), t, g * rng.uniform(0.85, 1.0))
    return buf


def music_menu():
    L, bar = 40.0, 5.0
    beat = bar / 8
    T = L + 8.0
    pads = svf(_pad_track(T, MENU_CHORDS, bar, 3.0, 0.3, 1.8), 1500, 0.7)
    lead = _lead_track(T, MENU_LEAD, bar, beat, mallet, dur=3.0, tau=0.9, trem=4.5)
    low = _lead_track(T, [[], [], [], [], MENU_LEAD[4][:1], MENU_LEAD[5][:1], MENU_LEAD[6][:1],
                          MENU_LEAD[7][:1]], bar, beat, keys, octave=0.5, dur=4.0, tau=1.8, bright=0.6)
    lead = echo(add(lead, scale(low, 0.5)), beat * 3, fb=0.35, mix=0.3, tail=0.0, lp_fc=1800)
    mix = add(rms_to(pads, -21), scale(norm(lead), 0.32))
    mix = reverb(mix, 0.9, 0.45, 0.5, tail=0.0)
    return tape(fold(mix, N(L)))


def music_day():
    L, bar = 60.0, 7.5
    beat = bar / 8
    T = L + 9.0
    chords = [["F2", "C3", "G3", "A3", "E4"], ["E2", "C3", "G3", "D4", "E4"],
              ["D2", "A2", "F3", "C4", "E4"], ["Bb1", "F2", "A3", "D4", "F4"],
              ["F2", "C3", "G3", "A3", "E4"], ["A2", "E3", "G3", "C4", "E4"],
              ["Bb1", "F2", "A3", "D4", "F4"], ["C2", "G2", "F3", "G3", "C4"]]
    lead_bars = [
        [(0, "A4", 0.9), (2, "C5", 0.8), (3, "G5", 1.0)],
        [(1, "E5", 0.8), (2, "D5", 0.7)],
        [(0, "F5", 0.9), (1, "E5", 0.8), (3, "C5", 0.8), (5, "A4", 0.6)],
        [(2, "D5", 0.7), (6, "F5", 0.45)],
        [(0, "A4", 0.9), (2, "C5", 0.8), (3, "A5", 1.0)],
        [(0, "G5", 0.9), (2, "E5", 0.8)],
        [(0, "D5", 0.8), (1, "F5", 0.9), (3, "G5", 0.9), (5, "F5", 0.6)],
        [(1, "C5", 0.8), (4, "G4", 0.55)],
    ]
    pads = svf(_pad_track(T, chords, bar, 3.5, 0.32, 2.5), 1700, 0.7)
    lead = _lead_track(T, lead_bars, bar, beat, keys, dur=4.5, tau=1.7, bright=0.75)
    lead = echo(lead, beat * 1.5, fb=0.3, mix=0.22, tail=0.0, lp_fc=2000)
    # gentle pulse: a soft low mallet on the chord's fifth, breathing in and out
    pulse = zeros(N(T))
    pat = (1.0, 0.0, 0.45, 0.0, 0.7, 0.0, 0.45, 0.3)
    for b, ch in enumerate(chords):
        f = nf(ch[1]) * 2
        for k, g in enumerate(pat):
            t = b * bar + k * beat
            sw = 0.5 - 0.5 * math.cos(TAU * 2 * t / L)
            if g and sw > 0.05:
                put(pulse, mallet(f, 1.2, 0.35, 0.3), t, g * sw)
    mix = add(rms_to(pads, -21), scale(norm(lead), 0.3), scale(norm(svf(pulse, 900, 0.7)), 0.07))
    mix = reverb(mix, 0.9, 0.4, 0.5, tail=0.0)
    return tape(fold(mix, N(L)), wow=0.0018)


def music_night():
    L, bar = 60.0, 10.0
    beat = bar / 8
    T = L + 10.0
    chords = [["D2", "A2", "E3", "F3", "A3"], ["Bb1", "F2", "A2", "D3", "E3"],
              ["G1", "D2", "Bb2", "D3", "A3"], ["A1", "E2", "C3", "F3"],
              ["D2", "A2", "D3", "F3", "A3"], ["E2", "Bb2", "D3", "G3"]]
    lead_bars = [
        [(0, "A4", 0.9), (2, "F4", 0.8), (3, "E4", 0.8)],
        [(1, "D4", 0.8), (4, "A3", 0.6)],
        [(0, "Bb4", 0.9), (1, "A4", 0.8), (3, "F4", 0.8)],
        [(2, "E4", 0.8), (5, "D4", 0.6)],
        [(0, "D5", 0.7), (2, "A4", 0.8)],
        [(1, "G4", 0.8), (3, "E4", 0.8), (6, "D4", 0.5)],
    ]
    pads = svf(_pad_track(T, chords, bar, 4.0, 0.28, 3.5), 800, 0.7)
    lead = _lead_track(T, lead_bars, bar, beat, keys, human=0.03, dur=5.0, tau=2.0, bright=0.6,
                       det=0.0045)
    lead = svf(echo(lead, beat * 3, fb=0.4, mix=0.3, tail=0.0, lp_fc=1200), 1500, 0.7)
    breath = fit(nloop(L, 2.0, lambda TT: mul(svf(white(TT), 300, 0.8), slow_random(TT, 0.15, 0.2, 1.0))), T)
    mix = add(rms_to(pads, -20), scale(norm(lead), 0.26), rms_to(breath, -40))
    mix = reverb(mix, 0.93, 0.5, 0.6, tail=0.0)
    return tape(fold(mix, N(L)), wow=0.003, wow_rate=0.3)


def music_deep():
    L = 60.0
    f0 = q(nf("D1"), L)
    drone = add(osc("sine", f0, L), scale(osc("sine", f0 + 1 / 15, L, phase=0.3), 0.9),
                scale(osc("sine", 2 * f0 + 1 / 20, L, phase=0.7), 0.6),
                scale(osc("sine", q(nf("A1"), L), L, phase=0.2), 0.5),
                scale(osc("sine", q(nf("A1"), L) + 0.1, L, phase=0.9), 0.4),
                scale(osc("sine", 3 * f0, L, phase=0.5), 0.25),
                scale(osc("sine", 4 * f0 - 1 / 12, L, phase=0.15), 0.15))
    drone = mul(drone, lfo(L, 1 / 20, 0.6, 1.0))
    # two uneasy neighbours that swell in and out against the drone
    un1 = mul(add(osc("sine", q(nf("Eb3"), L), L), osc("sine", q(nf("Eb3"), L) + 0.25, L)),
              lfo(L, 1 / 30, 0.0, 1.0, 0.75, 3.0))
    un2 = mul(add(osc("sine", q(nf("G#2"), L), L), osc("sine", q(nf("G#2"), L) + 0.2, L)),
              lfo(L, 1 / 20, 0.0, 1.0, 0.2, 3.0))
    breath = nloop(L, 3.0, lambda T: mul(svf(white(T), [180 + 220 * v for v in slow_random(T, 0.1)], 2.0, "bp"),
                                         slow_random(T, 0.12, 0.1, 1.0)))
    ev = zeros(N(L))
    notes = [(4.0, "D4", 0.8), (9.5, "Eb4", 0.5), (17.0, "A4", 0.8), (19.2, "Bb4", 0.5),
             (27.5, "D5", 0.6), (34.0, "C#4", 0.7), (41.5, "A3", 0.8), (44.0, "F4", 0.5),
             (52.0, "Eb5", 0.45), (55.5, "D4", 0.6)]
    for t0, nm, g in notes:
        inst = keys(nf(nm), 5.0, 2.2, 0.55, 0.005) if g >= 0.6 else mallet(nf(nm), 4.0, 1.4, 0.4)
        put(ev, inst, t0 + rng.uniform(-0.3, 0.3), g)
    ev = echo(ev, 1.1, fb=0.5, mix=0.45, tail=5.0, lp_fc=1000)
    for t0 in (23.0, 48.0):    # something very large, very far away
        put(ev, mul(lp1(brown(5.0), 70), env(5.0, (0, 0), (2.0, 1), (5.0, 0))), t0, 2.0)
    ev = reverb(svf(ev, 1600, 0.7), 0.95, 0.5, 1.3, tail=5.0)
    mix = add(rms_to(drone, -19), rms_to(un1, -37), rms_to(un2, -34), rms_to(breath, -34),
              fold(scale(norm(ev), 0.22), N(L)))
    return tape(mix, wow=0.003, wow_rate=0.25)


def _drum(f=70.0, d=1.6, tau=0.45):
    return norm(add(thump(f * 1.7, f, d, tau, st=0.03, attack=0.004),
                    scale(noise_burst(0.12, 0.03, 300, 0.8), 0.3)))


def music_ending():
    d, bar, beat = 35.0, 4.0, 0.5
    pads = zeros(N(d))
    put(pads, pad([nf("A3")], 7.0, [(0, 0), (3.0, 1), (5.0, 1), (7.0, 0)], 0.2), 0.0, 1.4)
    put(pads, pad([nf("E4")], 5.5, [(0, 0), (2.5, 1), (4.0, 1), (5.5, 0)], 0.2), 2.0, 0.9)
    prog = [(4.0, 4.0, MENU_CHORDS[0], 0.4), (8.0, 4.0, MENU_CHORDS[1], 0.55),
            (12.0, 4.0, MENU_CHORDS[2], 0.75), (16.0, 4.0, MENU_CHORDS[3], 0.9),
            (20.0, 4.0, ["A1", "A2", "E3", "A3", "C4", "E4"], 1.1),
            (24.0, 2.0, ["F1", "F2", "C3", "A3", "C4", "F4"], 1.15),
            (26.0, 2.0, ["G1", "G2", "D3", "G3", "B3", "D4"], 1.2)]
    for t0, ln, ch, g in prog:
        put(pads, pad([nf(x) for x in ch], ln + 2.5, [(0, 0), (1.2, 1), (ln, 0.9), (ln + 2.5, 0)], 0.32), t0, g)
    final = ["C2", "G2", "C3", "E3", "G3", "D4", "E4", "G4"]
    put(pads, pad([nf(x) for x in final], 7.0, [(0, 0), (0.4, 1), (3.0, 0.7), (7.0, 0)], 0.35), 28.0, 1.25)
    pads = svf(pads, env(d, (0, 700), (14, 1100), (22, 2400), (28, 2800), (d, 900)), 0.7)

    st = zeros(N(d))            # strings: brighter layer for the swell
    for t0, ln, ch, g in ((16.0, 4.0, ["G3", "B3", "D4"], 0.35), (20.0, 4.0, ["A3", "C4", "E4", "A4"], 0.7),
                          (24.0, 2.0, ["A3", "C4", "F4", "A4"], 0.85), (26.0, 2.0, ["B3", "D4", "G4", "B4"], 0.95),
                          (28.0, 6.5, ["C4", "E4", "G4", "C5", "E5"], 1.0)):
        e = [(0, 0), (0.9, 1), (ln, 0.85), (ln + 0.5, 0)] if ln < 6 else [(0, 0), (0.4, 1), (2.5, 0.6), (ln, 0)]
        put(st, strings([nf(x) for x in ch], ln + 0.5 if ln < 6 else ln, e, fc=2200), t0, g)

    lead = zeros(N(d))

    def play(t0, notes, inst, g, octave=1.0, **kw):
        for bt, nm, gg in notes:
            put(lead, inst(nf(nm) * octave, **kw), t0 + bt * beat + rng.uniform(-0.008, 0.008), g * gg)
    pk = dict(dur=4.0, tau=1.6, bright=0.75)
    mk = dict(dur=3.0, tau=0.9)
    play(8.0, MENU_LEAD[0], keys, 0.35, **pk)
    play(12.0, MENU_LEAD[1], keys, 0.4, **pk)
    play(16.0, MENU_LEAD[2], keys, 0.5, **pk)
    play(16.0, MENU_LEAD[2], mallet, 0.3, **mk)
    play(20.0, MENU_LEAD[0], keys, 0.8, **pk)
    play(20.0, MENU_LEAD[0], mallet, 0.6, 2.0, **mk)
    play(20.0, MENU_LEAD[0], keys, 0.4, 0.5, **pk)
    play(24.0, [(0, "C6", 1.0), (2, "D6", 0.9), (3, "E6", 1.0)], mallet, 0.6, **mk)
    play(24.0, [(0, "C5", 1.0), (2, "D5", 0.9), (3, "E5", 1.0)], keys, 0.8, **pk)
    play(26.0, [(0, "G5", 1.0), (2, "D5", 0.8), (3, "B5", 0.9)], keys, 0.8, **pk)
    play(26.0, [(0, "G6", 1.0), (3, "B5", 0.8)], mallet, 0.6, **mk)
    play(28.0, [(0, "C5", 1.0), (1, "E5", 0.9), (2, "G5", 0.9), (3.5, "C6", 1.0)], keys, 0.85, **pk)
    play(28.0, [(0, "C6", 0.9), (3.5, "E6", 0.8), (6, "G6", 0.5)], mallet, 0.6, **mk)
    play(31.0, [(0, "E5", 0.6), (2, "G4", 0.5), (4.5, "C5", 0.45)], keys, 0.6, **pk)

    perc = zeros(N(d))
    for t0, g in ((16.0, 0.35), (20.0, 0.8), (22.0, 0.4), (24.0, 0.7), (26.0, 0.7), (27.5, 0.5), (28.0, 1.0)):
        put(perc, _drum(), t0, g)
    for t_end, ln, g in ((20.0, 3.0, 0.5), (28.0, 2.5, 0.8)):
        n = N(ln)
        sw = mul(svf(white(ln), (800, 5000), 0.8, "bp"), [(i / n) ** 3 for i in range(n)])
        put(perc, norm(sw), t_end - ln, 0.12 * g)
    put(perc, sparkle(4.0, 30, (2500, 6000), (0.08, 0.3), 1.4, amp_tau=1.5), 28.0, 0.1)

    mix = add(rms_to(pads, -19), rms_to(st, -27), scale(norm(fit(lead, d)), 0.42),
              scale(fit(perc, d), 0.22))
    mix = fit(reverb(fit(mix, d), 0.9, 0.42, 0.45, tail=0.0), d)
    return fade(tape(mix, wow=0.0015, loop=False), 0.5, 2.5)


# ---------------------------------------------------------------------------
# Events
# ---------------------------------------------------------------------------

def _engine(T):
    roar = mul(svf(brown(T), 380, 0.8), slow_random(T, 14, 0.7, 1.0))
    mid = mul(svf(white(T), [700 + 500 * v for v in slow_random(T, 3)], 0.9, "bp"),
              slow_random(T, 9, 0.5, 1.0))
    hiss = svf(white(T), 3500, 0.7, "hp")
    return add(rms_to(roar, -12), rms_to(mid, -20), rms_to(hiss, -34))


def sfx_ship_engine_loop():
    L = 3.0
    bed = nloop(L, 0.6, _engine)
    f = q(37, L)
    sub = add(osc("sine", f, L), scale(osc("sine", q(55.5, L), L, phase=0.3), 0.6),
              scale(steady(osc("saw", f, L), lambda x: svf(x, 240, 1.0)), 0.6))
    sub = mul(sub, lfo(L, 7.0, 0.75, 1.0))
    return drive(norm(add(bed, rms_to(sub, -15))), 1.5)


def sfx_ship_land():
    d = 3.00
    buf = zeros(N(d))
    th = mul(_engine(0.9), env(0.9, (0, 0.9), (0.35, 1.0), (0.6, 0.5), (0.9, 0)))
    th = svf(th, (3000, 180), 0.8)
    put(buf, norm(th), 0.0, 0.8)
    put(buf, mul(osc("sine", (60, 28), 0.9), env(0.9, (0, 0.8), (0.5, 0.7), (0.9, 0))), 0.0, 0.5)
    put(buf, thump(80, 28, 0.9, 0.25, st=0.08), 0.85, 1.4)
    put(buf, noise_burst(0.4, 0.08, 450), 0.85, 0.8)
    put(buf, clank(140, 0.8, 0.22, 0.5), 0.86, 0.3)
    put(buf, thump(70, 35, 0.4, 0.12), 1.18, 0.5)
    imp = stick_slip(0.5, lambda t: 30 + 25 * math.sin(math.pi * t / 0.5), 0.25)
    cr = add(svf(imp, 260, 5, "bp"), scale(svf(imp, 540, 6, "bp"), 0.5))
    put(buf, mul(norm(cr), env(0.5, (0, 0), (0.15, 1), (0.5, 0))), 1.0, 0.2)
    hs = add(svf(white(1.9), (5000, 2200), 0.8, "hp"), scale(svf(white(1.9), (2400, 700), 1.5, "bp"), 0.7))
    put(buf, mul(norm(hs), env(1.9, (0, 0), (0.05, 1), (0.5, 0.55), (1.9, 0))), 1.1, 0.45)
    buf = drive(norm(fit(buf, d)), 1.4)
    return fade(fit(reverb(buf, 0.86, 0.5, 0.25, tail=0.0), d), 0.004, 0.4)


def sfx_fanfare_ready():
    d = 2.00
    buf = zeros(N(d))
    for i, (nm, t0) in enumerate((("G4", 0.0), ("C5", 0.13), ("E5", 0.26), ("G5", 0.42))):
        put(buf, mallet(nf(nm), 1.5, 0.6), t0, 0.5 + 0.12 * i)
        put(buf, keys(nf(nm), 1.5, 0.8), t0, 0.25)
    ch = [nf(x) for x in ("C3", "G3", "E4", "G4")]
    put(buf, norm(pad(ch, 1.6, [(0, 0), (0.45, 1), (0.9, 0.8), (1.6, 0)], 0.35)), 0.3, 0.3)
    put(buf, strings(ch[1:], 1.5, [(0, 0), (0.4, 1), (0.8, 0.7), (1.5, 0)], fc=1800), 0.4, 0.14)
    return fade(fit(reverb(fit(buf, d), 0.86, 0.4, 0.35, tail=0.0), d), 0.002, 0.4)


def sfx_readiness_all():
    d = 4.00
    buf = zeros(N(d))
    for nm, t0, g in (("E5", 0.0, 0.8), ("A5", 0.2, 0.8), ("G5", 0.5, 0.7), ("E5", 0.7, 0.8)):
        put(buf, mallet(nf(nm), 1.6, 0.6), t0, 0.6 * g)
        put(buf, keys(nf(nm), 2.0, 1.0), t0, 0.45 * g)
    for t0, ln, ch, g in ((0.0, 0.95, ("A2", "E3", "A3", "C4"), 0.22), (0.95, 0.75, ("F2", "C3", "A3", "C4"), 0.28),
                          (1.7, 2.3, ("C2", "G2", "C3", "E3", "G3", "D4"), 0.4)):
        e = [(0, 0), (0.3, 1), (ln, 0.85), (ln + 0.4, 0)] if ln < 2 else [(0, 0), (0.25, 1), (1.0, 0.7), (ln, 0)]
        put(buf, norm(pad([nf(x) for x in ch], ln + 0.4 if ln < 2 else ln, e, 0.35)), t0, g)
    put(buf, strings([nf(x) for x in ("G3", "C4", "E4", "G4", "C5")], 2.3,
                     [(0, 0), (0.3, 1), (1.0, 0.6), (2.3, 0)], fc=2400), 1.7, 0.22)
    put(buf, _drum(65, 1.4, 0.4), 1.7, 0.6)
    for i, nm in enumerate(("C5", "E5", "G5", "C6", "E6")):
        put(buf, mallet(nf(nm), 2.0, 0.8), 1.7 + 0.11 * i, 0.5)
        put(buf, keys(nf(nm), 2.0, 1.0), 1.7 + 0.11 * i, 0.3)
    put(buf, sparkle(1.8, 22, (2800, 6500), (0.06, 0.25), 1.4, amp_tau=0.7), 1.75, 0.1)
    return fade(fit(reverb(fit(buf, d), 0.88, 0.4, 0.4, tail=0.0), d), 0.002, 0.8)


def sfx_day_start():
    d = 2.50
    buf = zeros(N(d))
    p = pad([nf(x) for x in ("A3", "E4", "C#5")], 2.4, [(0, 0), (0.9, 1), (1.4, 0.8), (2.4, 0)], 0.3)
    put(buf, svf(norm(p), (500, 2600), 0.7), 0.0, 0.3)
    for i, (nm, t0) in enumerate((("A4", 0.1), ("E5", 0.32), ("A5", 0.54), ("C#6", 0.82))):
        put(buf, mallet(nf(nm), 1.6, 0.6, trem=5.0), t0, 0.5 + 0.06 * i)
    return fade(fit(reverb(fit(buf, d), 0.88, 0.4, 0.4, tail=0.0), d), 0.005, 0.7)


def sfx_night_start():
    d = 3.00
    buf = zeros(N(d))
    p = pad([nf(x) for x in ("D2", "A2", "F3", "E4")], 2.9, [(0, 0), (0.8, 1), (1.5, 0.8), (2.9, 0)], 0.3)
    put(buf, svf(norm(p), (1400, 300), 0.7), 0.0, 0.7)
    for nm, t0, g in (("A3", 0.15, 0.5), ("F3", 0.75, 0.45), ("D3", 1.4, 0.5)):
        put(buf, svf(keys(nf(nm), 1.6, 1.2, 0.6, 0.004), 1400, 0.7), t0, g)
    return fade(fit(reverb(fit(buf, d), 0.9, 0.5, 0.4, tail=0.0), d), 0.01, 0.9)


# ---------------------------------------------------------------------------
# Registry / output
# ---------------------------------------------------------------------------

SFX, LOOP = "sfx", "loop"
SFX_DB, LOOP_DB = -1.0, -6.0

# (name, generator, kind, seconds, peak dBFS override or None)
SOUNDS = [
    ("step_soft_1", lambda: step_soft(1.0), SFX, 0.14, None),
    ("step_soft_2", lambda: step_soft(0.84), SFX, 0.14, None),
    ("step_hard_1", lambda: step_hard(1.0), SFX, 0.10, None),
    ("step_hard_2", lambda: step_hard(1.18), SFX, 0.10, None),
    ("step_metal_1", lambda: step_metal(1.0), SFX, 0.18, None),
    ("step_metal_2", lambda: step_metal(1.13), SFX, 0.18, None),
    ("step_wet_1", lambda: step_wet(1.0), SFX, 0.22, None),
    ("step_wet_2", lambda: step_wet(1.2), SFX, 0.22, None),
    ("jump", sfx_jump, SFX, 0.18, None),
    ("land", sfx_land, SFX, 0.24, None),
    ("swim_stroke", sfx_swim_stroke, SFX, 0.45, None),
    ("splash", sfx_splash, SFX, 0.75, None),
    ("dig_soft", sfx_dig_soft, SFX, 0.16, None),
    ("dig_hard", sfx_dig_hard, SFX, 0.14, None),
    ("dig_ore", sfx_dig_ore, SFX, 0.30, None),
    ("dig_metal", sfx_dig_metal, SFX, 0.09, None),
    ("laser_fire", sfx_laser_fire, SFX, 0.26, None),
    ("laser_big", sfx_laser_big, SFX, 0.70, None),
    ("laser_charge", sfx_laser_charge, SFX, 1.20, None),
    ("laser_hit", sfx_laser_hit, SFX, 0.28, None),
    ("laser_empty", sfx_laser_empty, SFX, 0.55, None),
    ("battery_ready", sfx_battery_ready, SFX, 0.22, None),
    ("rocket_burst", sfx_rocket_burst, SFX, 0.50, None),
    ("rocket_loop", sfx_rocket_loop, LOOP, 1.00, None),
    ("dash", sfx_dash, SFX, 0.28, None),
    ("grapple_fire", sfx_grapple_fire, SFX, 0.30, None),
    ("grapple_hit", sfx_grapple_hit, SFX, 0.22, None),
    ("shield_hit", sfx_shield_hit, SFX, 0.35, None),
    ("shield_break", sfx_shield_break, SFX, 0.80, None),
    ("glide_loop", sfx_glide_loop, LOOP, 1.50, None),
    ("stomp", sfx_stomp, SFX, 0.70, None),
    ("teleport", sfx_teleport, SFX, 0.55, None),
    ("turret_fire", sfx_turret_fire, SFX, 0.16, None),
    ("heal", sfx_heal, SFX, 0.90, None),
    ("o2_low", sfx_o2_low, SFX, 0.50, None),
    ("o2_refill", sfx_o2_refill, SFX, 1.20, None),
    ("gasp", sfx_gasp, SFX, 0.70, None),
    ("hurt", sfx_hurt, SFX, 0.26, None),
    ("blackout", sfx_blackout, SFX, 1.80, None),
    ("respawn", sfx_respawn, SFX, 2.00, None),
    ("shiver", sfx_shiver, SFX, 0.60, None),
    ("cough", sfx_cough, SFX, 0.45, None),
    ("pickup", sfx_pickup, SFX, 0.12, None),
    ("drop", sfx_drop, SFX, 0.16, None),
    ("inv_open", sfx_inv_open, SFX, 0.22, None),
    ("inv_close", sfx_inv_close, SFX, 0.20, None),
    ("inv_move", sfx_inv_move, SFX, 0.04, -8.0),
    ("craft", sfx_craft, SFX, 1.00, None),
    ("craft_big", sfx_craft_big, SFX, 2.20, None),
    ("sell", sfx_sell, SFX, 0.70, None),
    ("buy", sfx_buy, SFX, 0.50, None),
    ("error", sfx_error, SFX, 0.30, None),
    ("ui_hover", sfx_ui_hover, SFX, 0.03, -14.0),
    ("ui_click", sfx_ui_click, SFX, 0.07, None),
    ("ui_toggle", sfx_ui_toggle, SFX, 0.12, None),
    ("toast", sfx_toast, SFX, 0.50, None),
    ("unlock", sfx_unlock, SFX, 1.40, None),
    ("codex_new", sfx_codex_new, SFX, 1.20, None),
    ("plant_harvest", sfx_plant_harvest, SFX, 0.30, None),
    ("plant_sow", sfx_plant_sow, SFX, 0.25, None),
    ("water_pour", sfx_water_pour, SFX, 0.80, None),
    ("sprinkler_loop", sfx_sprinkler_loop, LOOP, 2.00, None),
    ("fertilize", sfx_fertilize, SFX, 0.40, None),
    ("plant_grow", sfx_plant_grow, SFX, 0.90, None),
    ("flower_pop", sfx_flower_pop, SFX, 0.35, None),
    ("dome_enter", sfx_dome_enter, SFX, 0.60, None),
    ("dome_place", sfx_dome_place, SFX, 2.50, None),
    ("dome_hum_loop", sfx_dome_hum_loop, LOOP, 4.00, None),
    ("machine_on", sfx_machine_on, SFX, 0.70, None),
    ("machine_off", sfx_machine_off, SFX, 0.70, None),
    ("pylon_loop", sfx_pylon_loop, LOOP, 2.00, None),
    ("pump_loop", sfx_pump_loop, LOOP, 2.00, None),
    ("pipe_place", sfx_pipe_place, SFX, 0.30, None),
    ("generator_loop", sfx_generator_loop, LOOP, 3.00, None),
    ("splicer_run", sfx_splicer_run, SFX, 2.00, None),
    ("water_flow_loop", sfx_water_flow_loop, LOOP, 2.00, None),
    ("robot_ok", sfx_robot_ok, SFX, 0.30, None),
    ("robot_error", sfx_robot_error, SFX, 0.40, None),
    ("robot_hover_loop", sfx_robot_hover_loop, LOOP, 1.50, None),
    ("robot_work", sfx_robot_work, SFX, 0.40, None),
    ("drop_pod_land", sfx_drop_pod_land, SFX, 1.80, None),
    ("moth_flutter", sfx_moth_flutter, SFX, 0.45, None),
    ("puff_grunt", sfx_puff_grunt, SFX, 0.40, None),
    ("skitter_chitter", sfx_skitter_chitter, SFX, 0.60, None),
    ("skitter_lunge", sfx_skitter_lunge, SFX, 0.50, None),
    ("skitter_die", sfx_skitter_die, SFX, 0.70, None),
    ("web_spit", sfx_web_spit, SFX, 0.30, None),
    ("web_hit", sfx_web_hit, SFX, 0.30, None),
    ("leech_latch", sfx_leech_latch, SFX, 0.45, None),
    ("maw_rumble", sfx_maw_rumble, SFX, 1.60, None),
    ("maw_erupt", sfx_maw_erupt, SFX, 1.40, None),
    ("brood_roar", sfx_brood_roar, SFX, 2.00, None),
    ("brood_spawn", sfx_brood_spawn, SFX, 0.80, None),
    ("creature_hit", sfx_creature_hit, SFX, 0.20, None),
    ("amb_surface_day", amb_surface_day, LOOP, 20.0, None),
    ("amb_surface_night", amb_surface_night, LOOP, 20.0, None),
    ("amb_rain", amb_rain, LOOP, 12.0, None),
    ("amb_mist", amb_mist, LOOP, 12.0, None),
    ("amb_dome", amb_dome, LOOP, 10.0, None),
    ("amb_cave", amb_cave, LOOP, 20.0, None),
    ("amb_deep", amb_deep, LOOP, 20.0, None),
    ("amb_abyss", amb_abyss, LOOP, 20.0, None),
    ("thunder_1", sfx_thunder_1, SFX, 4.00, None),
    ("thunder_2", sfx_thunder_2, SFX, 5.00, None),
    ("music_menu", music_menu, LOOP, 40.0, None),
    ("music_day", music_day, LOOP, 60.0, None),
    ("music_night", music_night, LOOP, 60.0, None),
    ("music_deep", music_deep, LOOP, 60.0, None),
    ("music_ending", music_ending, SFX, 35.0, -3.0),
    ("ship_engine_loop", sfx_ship_engine_loop, LOOP, 3.00, None),
    ("ship_land", sfx_ship_land, SFX, 3.00, None),
    ("fanfare_ready", sfx_fanfare_ready, SFX, 2.00, None),
    ("readiness_all", sfx_readiness_all, SFX, 4.00, None),
    ("day_start", sfx_day_start, SFX, 2.50, None),
    ("night_start", sfx_night_start, SFX, 3.00, None),
]
REG = {s[0]: s for s in SOUNDS}


def finalize(sig, kind, dur, db):
    n = N(dur)
    if kind == SFX:
        sig = hp1(fit(sig, dur), 15.0)
        fade(sig, 0.002, 0.012)
    else:
        if len(sig) != n:
            raise ValueError("loop is %d samples, expected %d" % (len(sig), n))
        m = sum(sig) / n
        sig = [v - m for v in sig]
    peak = max(abs(v) for v in sig) or 1.0
    target = db if db is not None else (SFX_DB if kind == SFX else LOOP_DB)
    g = 10 ** (target / 20.0) / peak
    return [v * g for v in sig]


def write_wav(path, sig):
    a = array.array("h", (max(-32767, min(32767, int(round(v * 32767)))) for v in sig))
    if sys.byteorder == "big":
        a.byteswap()
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(a.tobytes())


def read_wav(path):
    with wave.open(path, "rb") as w:
        if (w.getnchannels(), w.getsampwidth(), w.getframerate()) != (1, 2, SR):
            raise ValueError("not 16-bit mono %d Hz" % SR)
        a = array.array("h")
        a.frombytes(w.readframes(w.getnframes()))
    if sys.byteorder == "big":
        a.byteswap()
    return [v / 32767.0 for v in a]


def doc_lengths():
    """name -> (seconds, is_loop) as promised by the docstring above."""
    out = {}
    for m in re.finditer(r"^  (\w+)\s+([\d.]+) s  (\[LOOP\])?", __doc__, re.M):
        out[m.group(1)] = (float(m.group(2)), bool(m.group(3)))
    return out


def analyze(name):
    """Read a written file back and check it. Returns (row text, problems)."""
    _, _, kind, dur, db = REG[name]
    path = os.path.join(OUT_DIR, name + ".wav")
    if not os.path.exists(path):
        return "%-18s MISSING" % name, 1
    x = read_wav(path)
    n = len(x)
    peak = max(abs(v) for v in x)
    rms = math.sqrt(sum(v * v for v in x) / n)
    clipped = sum(1 for v in x if abs(v) >= 0.9999)
    notes = []
    bad = 0
    if n != N(dur):
        notes.append("LENGTH %d != %d" % (n, N(dur)))
        bad += 1
    doc = doc_lengths().get(name)
    if doc is None or abs(doc[0] - dur) > 1e-6 or doc[1] != (kind == LOOP):
        notes.append("DOCSTRING MISMATCH")
        bad += 1
    if rms < 10 ** (-50 / 20.0):
        notes.append("SILENT?")
        bad += 1
    if clipped:
        notes.append("CLIP x%d" % clipped)
        bad += 1
    target = db if db is not None else (SFX_DB if kind == SFX else LOOP_DB)
    if abs(20 * math.log10(peak or 1e-9) - target) > 0.2:
        notes.append("PEAK OFF TARGET")
        bad += 1
    if kind == LOOP:
        # Seam check: the wrap step x[-1] -> x[0] compared with the largest
        # ordinary step in the 64 samples on either side of the seam.
        w = x[-64:] + x[:64]
        steps = [abs(w[i] - w[i - 1]) for i in range(1, len(w)) if i != 64]
        ratio = abs(x[0] - x[-1]) / (max(steps) or 1e-9)
        notes.append("seam %.2fx" % ratio)
        if ratio > 1.5:
            notes.append("SEAM CLICK")
            bad += 1
    elif abs(x[0]) > 0.01 or abs(x[-1]) > 0.01:
        notes.append("EDGE NOT ZERO")
        bad += 1
    row = "%-18s %-4s %6.2f %7.2f %7.2f  %s" % (
        name, kind, n / SR, 20 * math.log10(peak or 1e-9), 20 * math.log10(rms or 1e-9), ", ".join(notes))
    return row, bad


def generate(name):
    """Render one sound to disk. Runs in a worker process."""
    global rng
    _, fn, kind, dur, db = REG[name]
    t0 = time.time()
    rng = random.Random("planet-terraforming:" + name)
    _NOTE_CACHE.clear()
    sig = finalize(fn(), kind, dur, db)
    write_wav(os.path.join(OUT_DIR, name + ".wav"), sig)
    row, bad = analyze(name)
    return name, time.time() - t0, row, bad


def main(argv):
    jobs = os.cpu_count() or 1
    check_only = False
    wanted = []
    for a in argv:
        if a.startswith("-j"):
            jobs = max(1, int(a[2:] or 1))
        elif a == "--check":
            check_only = True
        else:
            wanted.append(a)
    unknown = [w for w in wanted if w not in REG]
    if unknown:
        print("unknown names: " + ", ".join(unknown))
        return 2
    names = [s[0] for s in SOUNDS if not wanted or s[0] in wanted]
    os.makedirs(OUT_DIR, exist_ok=True)
    start = time.time()
    rows = {}
    problems = 0
    head = "%-18s %-4s %6s %7s %7s  %s" % ("file", "kind", "dur s", "peak dB", "rms dB", "notes")
    if check_only:
        for nm in names:
            rows[nm], bad = analyze(nm)
            problems += bad
    else:
        order = sorted(names, key=lambda nm: -REG[nm][3])     # longest first
        jobs = min(jobs, len(order))
        if jobs > 1:
            import multiprocessing
            with multiprocessing.Pool(jobs) as pool:
                results = pool.imap_unordered(generate, order, 1)
                for nm, dt, row, bad in results:
                    rows[nm] = row
                    problems += bad
                    print("wrote %-18s %6.2f s audio in %5.1f s" % (nm, REG[nm][3], dt), flush=True)
        else:
            for nm in order:
                nm, dt, row, bad = generate(nm)
                rows[nm] = row
                problems += bad
                print("wrote %-18s %6.2f s audio in %5.1f s" % (nm, REG[nm][3], dt), flush=True)
    print()
    print(head)
    for nm in names:
        print(rows[nm])
    missing_doc = sorted(set(REG) ^ set(doc_lengths()))
    if missing_doc:
        print("docstring and registry disagree on: " + ", ".join(missing_doc))
        problems += 1
    total = sum(os.path.getsize(os.path.join(OUT_DIR, nm + ".wav")) for nm in names
                if os.path.exists(os.path.join(OUT_DIR, nm + ".wav")))
    print("\n%d files, %.2f MB, %d problem(s), %.1f s" % (len(names), total / 1e6, problems, time.time() - start))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
