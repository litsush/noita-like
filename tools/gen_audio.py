#!/usr/bin/env python3
"""Procedural audio generator for "Descent to the Core".

Python 3 standard library only. sfxr/jsfxr-style synthesis (band-limited
oscillators, ADSR envelopes, pitch slides, vibrato, state-variable filters,
bit crush, Schroeder reverb, feedback echo, layering). Fully deterministic:
every sound seeds its own RNG from its name.

Usage (from the repo root):
    python3 tools/gen_audio.py            # regenerate everything
    python3 tools/gen_audio.py jump dash  # regenerate only the named files

Output: assets/audio/<name>.wav -- 16-bit PCM, mono, 22050 Hz.
One-shot SFX peak at about -1 dBFS; continuous SFX loops (fire_loop,
water_spray, core_hum) at -4 dBFS; ambient loops at -6 dBFS; music at -3 dBFS.
Files marked [LOOP] are seamless and meant to be played with looping on.

Footsteps (two variants each; pick randomly per step)
  step_soft_1, step_soft_2     dirt / grass / fungus: muffled thud
  step_hard_1, step_hard_2     stone / obsidian: click / tap
  step_gravel_1, step_gravel_2 sand / gravel: crunchy noise burst
  step_metal_1, step_metal_2   metal: ringing clank

Movement
  jump          light upward whoosh + blip
  land          soft thump on landing
  dash          fast whoosh
  climb         short scrape (wall / rope climbing step)

Digging (short; game rate-limits and pitch-varies them)
  dig_soft      dirt crumble
  dig_hard      pick on stone: sharp chink + crumble
  dig_crystal   glassy ping with shimmer
  dig_metal     metallic clang

Tools / actions
  place_rope    rope whip + thunk
  throw         whoosh (torches, flasks)
  torch_ignite  match-strike + ignition whoosh
  pickup        bright two-note blip (ore pickup)
  item_get      ~2.6 s triumphant fanfare arpeggio
  chest_open    wooden creak + sparkle
  shrine_buy    coins + chime
  water_spray   [LOOP ~1 s] spraying water hiss
  spark         electric zap
  freeze        icy crackle
  plant         soft squelch / pop

Hazards
  fire_loop     [LOOP 3 s] crackling fire
  sizzle        water hitting lava: steamy hiss (~1 s)
  splash        water splash
  acid_hiss     corrosive fizz (~1 s)
  explosion     big boomy blast with low rumble tail (~1.9 s)
  explosion_small  smaller blast (~0.8 s)
  rumble        ~1.6 s ominous low rumble / creak (cave-in warning)
  collapse      falling rubble
  hurt          short pained hit / grunt
  burn          short sizzle-hurt
  death         descending sad tone (~1.6 s)
  heartbeat     low-HP lub-dub (1 s; can be repeated back to back)
  gasp          breath recovered after surfacing
  drown_bubble  bubbles (underwater / drowning)

UI
  ui_hover      tiny tick
  ui_click      soft click
  achievement   pleasant chime (~1.6 s)
  layer_sting   ~2.8 s dramatic sting on entering a new layer
  victory       ~13 s victory theme (melody, bass, arpeggio, drums)
  game_over     ~3.4 s somber sting
  core_hum      [LOOP 4 s] deep pulsing hum near the core

Ambient loops [LOOP 16 s each]
  amb_layer0    Crust: hollow cave tone, water drips, faint wind
  amb_layer1    Upper Mantle: warm low drone, distant rumbles, steam hiss
  amb_layer2    Deep Mantle: eerie crystalline pads, dissonance, sparse chimes
  amb_layer3    Outer Core: heavy metallic drone, grinding pulses, crackle
  amb_layer4    Core chamber: deep pulsing throb, rising Shepard tension

Music
  music_menu    [LOOP 24 s] main menu: A minor, 80 BPM, pad + bass + soft
                arpeggiated lead with echo
"""

import array
import math
import os
import random
import sys
import wave

SR = 22050
TAU = 2.0 * math.pi
OUT_DIR = os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "audio"))

rng = random.Random(0)


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
            r = (b / a) ** (1.0 / (n - 1))
            out = [0.0] * n
            v = float(a)
            for i in range(n):
                out[i] = v
                v *= r
            return out
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
    n = len(x)
    m = min(n, len(y))
    out = [x[i] * y[i] for i in range(m)]
    if m < n:
        out += [0.0] * (n - m)
    return out


def add(*sigs):
    n = max(len(s) for s in sigs)
    out = [0.0] * n
    for s in sigs:
        for i, v in enumerate(s):
            out[i] += v
    return out


def put(buf, sig, t, g=1.0):
    """Mix sig into buf at time t (seconds), extending buf if needed."""
    o = N(t)
    end = o + len(sig)
    if end > len(buf):
        buf.extend([0.0] * (end - len(buf)))
    for i, v in enumerate(sig):
        buf[o + i] += v * g
    return buf


def fit(x, dur):
    n = N(dur)
    if len(x) >= n:
        return x[:n]
    return x + [0.0] * (n - len(x))


def drive(x, amt):
    k = math.tanh(amt)
    th = math.tanh
    return [th(amt * v) / k for v in x]


def crush(x, bits=8, hold=1):
    q = 2 ** (bits - 1)
    out = [0.0] * len(x)
    last = 0.0
    for i, v in enumerate(x):
        if i % hold == 0:
            last = round(v * q) / q
        out[i] = last
    return out


# ---------------------------------------------------------------------------
# Oscillators / noise
# ---------------------------------------------------------------------------

def osc(wave_, freq, dur, duty=0.5, vib=0.0, vib_rate=5.0, vib_delay=0.0, phase=0.0):
    n = N(dur)
    f = _curve(freq, n)
    if vib:
        s = math.sin
        ramp = max(vib_delay, 1e-6)
        for i in range(n):
            t = i / SR
            d = vib * min(1.0, t / ramp) if vib_delay else vib
            f[i] *= 1.0 + d * s(TAU * vib_rate * t)
    out = [0.0] * n
    ph = phase % 1.0
    inv = 1.0 / SR
    if wave_ == "sine":
        s = math.sin
        for i in range(n):
            out[i] = s(TAU * ph)
            ph += f[i] * inv
            ph -= int(ph)
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
    for i in range(n):
        x = i / SR * rate
        k = int(x)
        fr = x - k
        w = 0.5 - 0.5 * math.cos(math.pi * fr)
        out[i] = lo + (hi - lo) * (pts[k] * (1 - w) + pts[k + 1] * w)
    return out


# ---------------------------------------------------------------------------
# Envelopes
# ---------------------------------------------------------------------------

def env_pts(dur, pts):
    """Piecewise-linear envelope from [(t, v), ...]."""
    n = N(dur)
    out = [0.0] * n
    j = 0
    for i in range(n):
        t = i / SR
        while j < len(pts) - 2 and t >= pts[j + 1][0]:
            j += 1
        t0, v0 = pts[j]
        t1, v1 = pts[min(j + 1, len(pts) - 1)]
        if t1 <= t0 or t <= t0:
            out[i] = v0 if t <= t0 else v1
        elif t >= t1:
            out[i] = v1
        else:
            out[i] = v0 + (v1 - v0) * (t - t0) / (t1 - t0)
    return out


def adsr(dur, a=0.005, d=0.05, s=0.7, r=0.05):
    hold = max(a + d, dur - r)
    return env_pts(dur, [(0, 0.0), (a, 1.0), (a + d, s), (hold, s), (dur, 0.0)])


def decay(dur, tau, attack=0.001):
    n = N(dur)
    na = max(1, N(attack))
    e = math.exp
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        v = e(-t / tau)
        if i < na:
            v *= i / na
        out[i] = v
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
    return [x[i] - lo[i] for i in range(len(x))]


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

    a1, a2, a3 = coefs(fc if const else fcs[0])
    ic1 = ic2 = 0.0
    out = [0.0] * n
    m = {"lp": 0, "bp": 1, "hp": 2}[mode]
    for i in range(n):
        if not const and (i & 7) == 0:
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
    for L in (557, 593, 641, 677, 709, 743):
        buf = [0.0] * L
        idx = 0
        filt = 0.0
        d1 = 1.0 - damp
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
            o = -v + b
            buf[idx] = v + b * 0.5
            idx += 1
            if idx == L:
                idx = 0
            acc[i] = o
    ed = sum(v * v for v in x) or 1e-12
    ew = sum(v * v for v in acc) or 1e-12
    g = wet * math.sqrt(ed / ew)
    return [dry * src[i] + g * acc[i] for i in range(n)]


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
        y[i] = src[i] + fb * st
        out[i] = src[i] + mix * st
    return out


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
    out = [0.0] * L
    for i, v in enumerate(sig):
        out[i % L] += v
    return out


def assemble_loop(L_s, periodic=None, noise=None, events=None, P_s=0.0, XL_s=0.5, XN_s=1.0):
    """periodic: list of length >= P+L+XL (window after preroll P, linear xfade)
    noise:    list of length >= L+XN (equal-power xfade)
    events:   list of any length (tails folded around)"""
    L = N(L_s)
    out = [0.0] * L
    if periodic is not None:
        P = N(P_s)
        seg = xfade_loop(periodic[P:], L, N(XL_s), "lin")
        out = add(out, seg)
    if noise is not None:
        out = add(out, xfade_loop(noise, L, N(XN_s), "power"))
    if events is not None:
        out = add(out, fold(events, L))
    return out[:L]


def q(f, L):
    """Round a frequency so it has an integer number of cycles in L seconds."""
    return max(1.0, round(f * L)) / L


# ---------------------------------------------------------------------------
# Building blocks (each ends with a short fade so truncated decays never click)
# ---------------------------------------------------------------------------

def tail_fade(x, sec=0.02):
    n = len(x)
    m = min(N(sec), max(1, int(n * 0.3)))
    for i in range(m):
        x[n - 1 - i] *= i / m
    return x


def thump(f0, f1, dur, tau, st=0.03, attack=0.002):
    s = osc("sine", lambda t: f1 + (f0 - f1) * math.exp(-t / st), dur)
    return tail_fade(mul(s, decay(dur, tau, attack)))


def noise_burst(dur, tau, fc, q_=0.7, mode="lp", attack=0.001):
    return tail_fade(mul(svf(white(dur), fc, q_, mode), decay(dur, tau, attack)))


def partials(base, ratios, amps, taus, dur, detune=0.0, attack=0.001):
    out = zeros(N(dur))
    for r, a, tau in zip(ratios, amps, taus):
        s = mul(osc("sine", base * r, dur, phase=rng.random()), decay(dur, tau, attack))
        out = add(out, scale(s, a))
        if detune:
            s2 = mul(osc("sine", base * r * (1 + detune), dur, phase=rng.random()),
                     decay(dur, tau * 0.9, attack))
            out = add(out, scale(s2, a * 0.6))
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
    """Impulse train with irregular spacing (creaks, scrapes)."""
    buf = zeros(N(dur))
    t = 0.0
    while t < dur:
        i = N(t)
        if i < len(buf):
            buf[i] += rng.uniform(0.6, 1.0)
        r = max(5.0, rate_fn(t))
        t += (1.0 / r) * (1.0 + rng.uniform(-jitter, jitter))
    return buf


# --- Instruments ------------------------------------------------------------

def inst_lead(f, dur, vib=0.007, cutoff=2600, duty=0.25, rel=0.08, sq=0.4):
    total = dur + rel
    a = osc("square", f, total, duty=duty, vib=vib, vib_rate=5.5, vib_delay=0.15)
    b = osc("tri", f, total, vib=vib, vib_rate=5.5, vib_delay=0.15)
    s = svf(add(scale(a, sq), b), cutoff, 0.7)
    return mul(s, adsr(total, 0.006, 0.1, 0.7, rel))


def inst_pluck(f, dur, tau=0.15):
    total = dur + 0.05
    s = add(osc("tri", f, total), scale(osc("sine", f * 2, total), 0.25))
    return mul(s, mul(decay(total, tau, 0.003), adsr(total, 0.001, 0.0, 1.0, 0.05)))


def inst_bass(f, dur, cutoff=450):
    total = dur + 0.06
    s = add(scale(svf(osc("saw", f, total), cutoff, 0.8), 0.6),
            scale(osc("sine", f, total), 0.8),
            scale(osc("sine", f / 2, total), 0.25))
    return mul(s, adsr(total, 0.006, 0.15, 0.7, 0.06))


def inst_pad(freqs, dur, cutoff=900, att=0.6, rel=1.0, detune=0.003):
    total = dur + rel
    out = zeros(N(total))
    for f in freqs:
        for d in (1 - detune, 1 + detune):
            out = add(out, osc("saw", f * d, total, phase=rng.random()))
    out = svf(out, cutoff, 0.6)
    return mul(scale(out, 1.0 / len(freqs)), adsr(total, att, 0.4, 0.8, rel))


def brass(freqs, dur, env, fc_lo=200, fc_hi=1800):
    """Detuned saws through a lowpass that tracks the amplitude envelope."""
    total = dur
    e = env_pts(total, env)
    out = zeros(N(total))
    for f in freqs:
        for d in (0.996, 1.0, 1.004):
            out = add(out, osc("saw", f * d, total, phase=rng.random()))
    out = svf(out, [fc_lo + (fc_hi - fc_lo) * v for v in e], 0.9)
    return scale(mul(out, e), 1.0 / len(freqs))


def kick(g=1.0):
    d = 0.35
    return scale(add(thump(160, 48, d, 0.12, st=0.025),
                     scale(noise_burst(0.02, 0.004, 2500), 0.3)), g)


def snare(g=1.0):
    d = 0.22
    return scale(add(scale(noise_burst(d, 0.07, 1900, 0.6, "bp"), 0.9),
                     scale(thump(240, 180, d, 0.04, st=0.02), 0.5)), g)


def hat(g=1.0):
    return scale(noise_burst(0.06, 0.015, 7500, 0.8, "bp"), g)


def crash(g=1.0):
    return scale(svf(noise_burst(1.6, 0.5, 4500, 0.5, "hp", attack=0.002), 7000), g)


# ---------------------------------------------------------------------------
# SFX definitions
# ---------------------------------------------------------------------------

def step_soft(v):
    d = 0.12
    body = thump(130 * v, 60 * v, d, 0.035, st=0.02, attack=0.003)
    nz = noise_burst(d, 0.025, 800 * v, 0.7, "lp", 0.002)
    return svf(add(body, scale(nz, 0.7)), 1400, 0.7)


def step_hard(v):
    d = 0.09
    click = noise_burst(d, 0.010, 2300 * v, 1.8, "bp", 0.0005)
    tap = thump(400 * v, 260 * v, d, 0.022, st=0.015)
    tick = mul(osc("tri", 1100 * v, d), decay(d, 0.007))
    return svf(add(click, scale(tap, 0.7), scale(tick, 0.25)), 6000, 0.7)


def step_gravel(v):
    d = 0.14
    g = grain_cloud(0.11, 11, (0.003, 0.009), dist_pow=1.6, amp_tau=0.05)
    g = svf(g, 1700 * v, 0.9, "bp")
    thud = noise_burst(d, 0.02, 400, 0.7)
    return add(fit(g, d), scale(thud, 0.45))


def step_metal(v):
    d = 0.16
    base = 470 * v
    ring = partials(base, [1, 2.41, 3.93, 5.62], [1, 0.5, 0.3, 0.15],
                    [0.055, 0.04, 0.028, 0.018], d, detune=0.004)
    click = noise_burst(d, 0.004, 3000, 1.0, "bp")
    body = thump(170, 120, d, 0.02)
    return add(scale(ring, 0.6), scale(click, 0.5), scale(body, 0.5))


def sfx_jump():
    d = 0.17
    blip = osc("square", (190, 560), 0.14, duty=0.25)
    blip = mul(svf(blip, 2200, 0.7), adsr(0.14, 0.004, 0.04, 0.6, 0.07))
    wh = svf(white(d), (500, 2200), 1.2, "bp")
    wh = mul(wh, env_pts(d, [(0, 0), (0.05, 1), (d, 0)]))
    return add(scale(blip, 0.6), scale(wh, 0.35))


def sfx_land():
    d = 0.22
    body = thump(120, 45, d, 0.06, st=0.025)
    nz = noise_burst(d, 0.035, 450)
    grit = svf(grain_cloud(0.12, 6, amp_tau=0.04), 1500, 0.7)
    return add(body, scale(nz, 0.7), scale(fit(grit, d), 0.25))


def sfx_dash():
    d = 0.28
    fc = lambda t: 500 + 2100 * math.sin(math.pi * min(1.0, t / d)) ** 1.5
    wh = mul(svf(white(d), fc, 1.4, "bp"), env_pts(d, [(0, 0), (0.06, 1), (0.14, 0.7), (d, 0)]))
    body = mul(svf(white(d), 700), env_pts(d, [(0, 0), (0.05, 1), (d, 0)]))
    return add(wh, scale(body, 0.4))


def sfx_climb():
    d = 0.15
    imp = stick_slip(d, lambda t: 70 + 30 * t / d, 0.3)
    nz = mul(white(d), [min(1.0, v) for v in lp1([abs(x) * 6 for x in imp], 120)])
    s = add(svf(nz, 1600, 1.1, "bp"), scale(svf(white(d), 900, 0.8, "bp"), 0.15))
    return mul(s, adsr(d, 0.01, 0.05, 0.7, 0.06))


def sfx_dig_soft():
    d = 0.3
    thud = thump(90, 50, d, 0.05, st=0.02)
    cr = grain_cloud(0.26, 26, (0.003, 0.012), dist_pow=2.0, amp_tau=0.1)
    cr = add(svf(cr, 1300, 0.7), scale(svf(cr, 700, 1.2, "bp"), 0.5))
    return add(scale(thud, 0.8), fit(cr, d))


def sfx_dig_hard():
    d = 0.4
    chink = partials(1650, [1, 1.51, 2.08], [1, 0.45, 0.2], [0.05, 0.035, 0.02], d)
    click = noise_burst(d, 0.005, 3500, 1.0, "bp")
    thud = thump(170, 90, d, 0.03)
    cr = svf(grain_cloud(0.3, 18, (0.003, 0.01), 1.8, 0.1, start=0.03), 2200, 0.7)
    return add(scale(chink, 0.55), scale(click, 0.6), scale(thud, 0.6), scale(fit(cr, d), 0.45))


def sfx_dig_crystal():
    d = 0.95
    f = 1175
    ping = partials(f, [1, 2.01, 2.76, 5.4], [1, 0.35, 0.25, 0.08], [0.45, 0.28, 0.18, 0.07],
                    0.9, detune=0.0025)
    sp = zeros(N(0.9))
    for t, fr in ((0.05, 2349), (0.11, 2794), (0.18, 3136), (0.26, 2637)):
        put(sp, mul(osc("sine", fr, 0.3), decay(0.3, 0.07)), t, 0.18)
    click = noise_burst(0.02, 0.003, 4000, 1.0, "bp")
    s = add(scale(ping, 0.6), sp, scale(click, 0.25))
    return fit(reverb(s, 0.82, 0.3, 0.35, tail=0.3), d)


def sfx_dig_metal():
    d = 0.7
    ring = partials(260, [1, 2.32, 2.76, 4.07, 5.4], [1, 0.6, 0.45, 0.28, 0.14],
                    [0.32, 0.24, 0.2, 0.12, 0.07], d, detune=0.003)
    hit = noise_burst(d, 0.006, 2000, 1.0, "bp")
    body = thump(140, 100, d, 0.07)
    s = add(scale(ring, 0.5), scale(hit, 0.6), scale(body, 0.5))
    return fit(reverb(s, 0.78, 0.4, 0.2, tail=0.2), d)


def sfx_place_rope():
    d = 0.4
    wh = svf(white(0.12), (3500, 700), 2.0, "bp")
    wh = mul(wh, env_pts(0.12, [(0, 0), (0.06, 0.35), (0.085, 1.0), (0.12, 0)]))
    buf = zeros(N(d))
    put(buf, wh, 0.0, 0.9)
    put(buf, thump(150, 70, 0.25, 0.05), 0.14, 0.9)
    put(buf, noise_burst(0.1, 0.03, 600), 0.14, 0.5)
    put(buf, mul(osc("sine", 420, 0.1), decay(0.1, 0.02)), 0.14, 0.3)
    return fit(buf, d)


def sfx_throw():
    d = 0.32
    wh = svf(white(d), (420, 1700), 1.4, "bp")
    e = env_pts(d, [(0, 0), (0.13, 1), (d, 0)])
    lo = svf(white(d), 600)
    return mul(add(wh, scale(lo, 0.3)), e)


def sfx_torch_ignite():
    d = 0.8
    buf = zeros(N(d))
    strike_d = 0.08
    imp = stick_slip(strike_d, lambda t: 220, 0.5)
    sc = mul(white(strike_d), [min(1.0, v) for v in lp1([abs(x) * 8 for x in imp], 300)])
    sc = mul(svf(sc, 2800, 1.4, "bp"), adsr(strike_d, 0.005, 0.02, 0.7, 0.03))
    put(buf, sc, 0.0, 0.8)
    ig_d = 0.7
    fc = lambda t: 400 + 2600 * math.exp(-((t - 0.12) / 0.12) ** 2) + 900 * (t > 0.12)
    ig = svf(white(ig_d), fc, 0.8)
    ig = mul(ig, env_pts(ig_d, [(0, 0), (0.1, 1), (0.25, 0.6), (ig_d, 0)]))
    put(buf, ig, 0.06, 0.9)
    put(buf, thump(70, 110, 0.3, 0.1, st=0.1), 0.06, 0.4)
    cr = svf(grain_cloud(0.6, 14, (0.002, 0.005), 1.3, 0.3), 2800, 1.2, "bp")
    put(buf, cr, 0.15, 0.5)
    return fit(buf, d)


def sfx_pickup():
    d = 0.22
    buf = zeros(N(d))
    for t, f, nd in ((0.0, 988, 0.06), (0.055, 1319, 0.15)):
        s = add(scale(osc("square", f, nd), 0.35), osc("tri", f, nd))
        s = mul(svf(s, 3500, 0.7), adsr(nd, 0.002, 0.03, 0.6, nd * 0.6))
        put(buf, s, t)
    return fit(crush(buf, 10), d)


def sfx_item_get():
    d = 2.6
    buf = zeros(N(d))
    for i, nm in enumerate(("G4", "C5", "E5", "G5")):
        put(buf, inst_lead(nf(nm), 0.1, vib=0), 0.1 * i, 0.8)
    put(buf, inst_lead(nf("A5"), 0.13, vib=0), 0.42, 0.8)
    put(buf, inst_lead(nf("B5"), 0.13, vib=0), 0.57, 0.8)
    put(buf, inst_lead(nf("C6"), 1.25, vib=0.008, rel=0.5), 0.72, 0.9)
    put(buf, inst_pad([nf("C4"), nf("E4"), nf("G4")], 1.2, cutoff=1400, att=0.05, rel=0.6), 0.72, 0.6)
    put(buf, inst_bass(nf("C3"), 1.2), 0.72, 0.6)
    put(buf, inst_bass(nf("G2"), 0.35), 0.0, 0.4)
    put(buf, fm_bell(nf("C6") * 2, 1.2, index=1.2, tau=0.4), 0.72, 0.15)
    return fit(reverb(buf, 0.82, 0.4, 0.25, tail=0.4), d)


def sfx_chest_open():
    d = 1.4
    buf = zeros(N(d))
    cd = 0.55
    imp = stick_slip(cd, lambda t: 35 + 40 * math.sin(math.pi * t / cd), 0.2)
    creak = add(svf(imp, 650, 5, "bp"), scale(svf(imp, 1250, 6, "bp"), 0.6),
                scale(svf(imp, 300, 3, "bp"), 0.5))
    creak = mul(creak, adsr(cd, 0.05, 0.1, 0.8, 0.15))
    put(buf, creak, 0.0, 1.0)
    put(buf, thump(190, 110, 0.2, 0.04), 0.52, 0.5)
    sp = zeros(N(0.9))
    for i, nm in enumerate(("E6", "G#6", "B6", "E7")):
        put(sp, fm_bell(nf(nm), 0.5, index=0.8, tau=0.18), 0.07 * i, 0.3)
    sp = add(sp, scale(mul(svf(white(0.9), 6000, 0.7, "hp"), env_pts(0.9, [(0, 0), (0.1, 1), (0.9, 0)])), 0.04))
    put(buf, reverb(sp, 0.8, 0.3, 0.4, tail=0.3), 0.5)
    return fit(buf, d)


def sfx_shrine_buy():
    d = 1.3
    buf = zeros(N(d))
    for t in (0.0, 0.06, 0.1, 0.17, 0.23, 0.3):
        b = rng.uniform(2000, 2600)
        c = partials(b, [1, 1.38, 2.05], [1, 0.5, 0.25], [0.06, 0.04, 0.025], 0.2)
        put(buf, c, t, rng.uniform(0.35, 0.55))
    put(buf, fm_bell(nf("C6"), 0.9, index=1.5, tau=0.35), 0.36, 0.6)
    put(buf, fm_bell(nf("G6"), 0.9, index=1.2, tau=0.4), 0.5, 0.5)
    return fit(reverb(buf, 0.8, 0.35, 0.3, tail=0.3), d)


def sfx_water_spray():
    L, X = 1.0, 0.3
    n = white(L + X)
    s = add(svf(n, 1300, 0.7, "hp"), scale(svf(n, 3800, 0.8, "bp"), 0.8))
    am = [0.8 + 0.2 * v for v in slow_random(L + X, 25)]
    s = svf(mul(s, am), 5500, 0.7)
    return assemble_loop(L, noise=s, XN_s=X)


def sfx_spark():
    d = 0.3
    nseg = N(0.008)
    fl = []
    while len(fl) < N(d):
        fl += [rng.uniform(80, 420)] * nseg
    buzz = mul(osc("saw", fl[:N(d)], d), decay(d, 0.09, 0.001))
    zap = mul(osc("sine", (2200, 250), 0.06), decay(0.06, 0.02))
    gate = []
    while len(gate) < N(d):
        gate += [1.0 if rng.random() < 0.5 else 0.0] * N(0.003)
    cr = mul(mul(svf(white(d), 3000, 0.7, "hp"), gate[:N(d)]), decay(d, 0.1))
    s = add(scale(buzz, 0.6), fit(scale(zap, 0.5), d), scale(cr, 0.4))
    return svf(crush(s, 6, 2), 5000, 0.7)


def sfx_freeze():
    d = 0.85
    cr = grain_cloud(0.7, 45, (0.001, 0.003), dist_pow=0.7)
    cr = svf(cr, 4500, 2.0, "bp")
    sh = zeros(N(0.8))
    for f, rate in ((2093, 7), (2637, 9), (3136, 11)):
        s = mul(osc("sine", f, 0.8), [0.6 + 0.4 * math.sin(TAU * rate * i / SR) for i in range(N(0.8))])
        sh = add(sh, scale(s, 0.12))
    sh = mul(sh, env_pts(0.8, [(0, 0), (0.15, 1), (0.8, 0)]))
    wh = mul(svf(white(0.8), 4000, 0.7, "hp"), env_pts(0.8, [(0, 0), (0.2, 1), (0.8, 0)]))
    s = add(fit(cr, 0.8), sh, scale(wh, 0.12))
    return fit(reverb(s, 0.8, 0.3, 0.3, tail=0.2), d)


def sfx_plant():
    d = 0.25
    pop = mul(osc("sine", (180, 650), 0.05), decay(0.05, 0.02, 0.002))
    sq = mul(svf(white(0.14), (300, 1400), 6.0, "bp"), env_pts(0.14, [(0, 0), (0.02, 1), (0.14, 0)]))
    buf = zeros(N(d))
    put(buf, sq, 0.0, 0.7)
    put(buf, pop, 0.06, 0.8)
    return svf(fit(buf, d), 3000, 0.7)


def sfx_fire_loop():
    L, X = 3.0, 0.5
    T = L + X
    roar = mul(lp1(white(T), 300), [0.8 + 0.2 * v for v in slow_random(T, 3)])
    mid = mul(svf(white(T), 900, 0.8, "bp"), slow_random(T, 6, 0.2, 1.0))
    bed = add(scale(roar, 1.6), scale(mid, 0.35))
    ev = zeros(N(L))
    t = 0.0
    while t < L:
        g = min(1.0, rng.expovariate(3.0))
        L_ = rng.uniform(0.001, 0.004)
        c = svf(mul(white(L_ + 0.01), decay(L_ + 0.01, L_ * 0.5, 0.0003)),
                rng.uniform(1500, 4000), 1.5, "bp")
        put(ev, c, t, g)
        if rng.random() < 0.12:
            put(ev, noise_burst(0.03, 0.008, 600, 1.2, "bp"), t, 0.8)
        t += rng.expovariate(14.0)
    return assemble_loop(L, noise=bed, events=ev, XN_s=X)


def sfx_sizzle():
    d = 1.0
    hiss = add(svf(white(d), 2500, 0.7, "hp"), scale(svf(white(d), 6000, 0.8, "bp"), 0.6))
    hiss = mul(svf(hiss, 6000, 0.7), env_pts(d, [(0, 0), (0.02, 1), (0.25, 0.7), (d, 0)]))
    steam = mul(svf(white(d), 800, 0.7), env_pts(d, [(0, 0), (0.15, 1), (d, 0)]))
    cr = svf(grain_cloud(0.8, 40, (0.001, 0.004), 1.8, 0.3), 3500, 1.2, "bp")
    buf = add(scale(hiss, 0.7), scale(steam, 0.4), scale(fit(cr, d), 0.5))
    for _ in range(3):
        put(buf, bubble(rng.uniform(250, 450), 0.06, 1.0), rng.uniform(0.0, 0.2), 0.35)
    return fit(buf, d)


def sfx_splash():
    d = 0.75
    imp = mul(svf(white(d), lambda t: 700 + 4500 * math.exp(-t / 0.05), 0.7),
              decay(d, 0.15, 0.003))
    body = thump(110, 60, d, 0.06)
    buf = add(imp, scale(body, 0.5))
    for _ in range(7):
        put(buf, bubble(rng.uniform(400, 900), rng.uniform(0.02, 0.05), 0.8),
            rng.uniform(0.05, 0.45), rng.uniform(0.1, 0.25))
    for _ in range(5):
        put(buf, bubble(rng.uniform(1500, 2500), 0.015, 0.5), rng.uniform(0.1, 0.5), 0.08)
    return fit(reverb(buf, 0.75, 0.4, 0.15, tail=0.2), d)


def sfx_acid_hiss():
    d = 1.0
    gr = [v * v * 3.0 for v in slow_random(d, 80)]
    fizz = mul(svf(white(d), 3000, 0.7, "hp"), gr)
    buf = scale(svf(fizz, 5500, 0.7), 0.6)
    for _ in range(60):
        put(buf, bubble(rng.uniform(1500, 4000), rng.uniform(0.008, 0.02), 0.6),
            rng.uniform(0.0, 0.9), rng.uniform(0.05, 0.18))
    buf = mul(fit(buf, d), env_pts(d, [(0, 0), (0.04, 1), (0.6, 0.8), (d, 0)]))
    return buf


def make_explosion(d, s):
    """s = size (1 = big, ~0.5 = small)."""
    buf = zeros(N(d))
    put(buf, mul(white(0.04), decay(0.04, 0.008)), 0.0, 0.8)
    body = svf(white(d), lambda t: 150 / s + 4000 * math.exp(-t / (0.09 * s)), 0.8)
    put(buf, mul(body, decay(d, 0.35 * s, 0.002)), 0.0, 1.0)
    put(buf, thump(110 / s ** 0.5, 32 / s ** 0.5, d, 0.5 * s, st=0.12 * s), 0.0, 1.2 * s)
    rum = mul(lp1(brown(d), 140), env_pts(d, [(0, 0), (0.06, 1), (d * 0.4, 0.45), (d, 0)]))
    put(buf, rum, 0.0, 0.9 * s)
    deb = svf(grain_cloud(d * 0.6, int(30 * s), (0.003, 0.012), 1.5, 0.3 * s, start=0.1 * s),
              2500, 0.7)
    put(buf, deb, 0.0, 0.35)
    buf = drive(fit(buf, d), 1.6)
    buf = fit(reverb(buf, 0.85, 0.5, 0.25, tail=0.0), d)
    return fade(buf, 0.001, 0.3 * s)


def sfx_rumble():
    d = 1.6
    e = env_pts(d, [(0, 0), (0.5, 0.6), (1.1, 1.0), (d, 0)])
    rb = mul(mul(lp1(brown(d), 100), e), slow_random(d, 8, 0.5, 1.0))
    sub = mul(osc("sine", lambda t: 42 + 3 * math.sin(TAU * 0.7 * t), d), e)
    imp = stick_slip(0.9, lambda t: 25 + 15 * t / 0.9, 0.25)
    creak = mul(add(svf(imp, 220, 4, "bp"), scale(svf(imp, 480, 5, "bp"), 0.6)),
                env_pts(0.9, [(0, 0), (0.3, 1), (0.9, 0)]))
    buf = add(scale(rb, 2.0), scale(sub, 0.5))
    put(buf, creak, 0.35, 0.8)
    for _ in range(6):
        put(buf, noise_burst(0.02, 0.004, 2000, 1.0, "bp"), rng.uniform(0.3, 1.4), rng.uniform(0.1, 0.25))
    return fit(drive(buf, 1.3), d)


def sfx_collapse():
    d = 1.7
    buf = zeros(N(d))
    for _ in range(24):
        t = 1.25 * rng.random() ** 1.5
        g = rng.uniform(0.4, 1.0) * math.exp(-t / 0.9)
        put(buf, thump(rng.uniform(80, 220), 50, 0.2, rng.uniform(0.03, 0.06)), t, g * 0.8)
        put(buf, noise_burst(0.08, 0.02, rng.uniform(800, 2500)), t, g * 0.6)
    bed = mul(lp1(brown(d), 200), env_pts(d, [(0, 0), (0.05, 1), (d, 0)]))
    buf = add(fit(buf, d), scale(bed, 1.2))
    return fade(fit(reverb(buf, 0.8, 0.5, 0.2, tail=0.0), d), 0.001, 0.3)


def sfx_hurt():
    d = 0.26
    saw = osc("saw", (220, 130), 0.22, vib=0.06, vib_rate=32)
    vox = add(svf(saw, 650, 4, "bp"), scale(svf(saw, 1100, 5, "bp"), 0.6), scale(svf(saw, 400), 0.4))
    vox = mul(vox, env_pts(0.22, [(0, 0), (0.01, 1), (0.08, 0.7), (0.22, 0)]))
    buf = zeros(N(d))
    put(buf, vox, 0.0, 1.0)
    put(buf, thump(150, 80, 0.12, 0.04), 0.0, 0.6)
    put(buf, noise_burst(0.03, 0.01, 2000), 0.0, 0.4)
    return fit(buf, d)


def sfx_burn():
    d = 0.35
    hiss = mul(svf(white(d), 3000, 0.7, "hp"), decay(d, 0.12, 0.003))
    saw = osc("saw", (300, 200), 0.25, vib=0.05, vib_rate=28)
    vox = mul(add(svf(saw, 800, 4, "bp"), scale(svf(saw, 1300, 5, "bp"), 0.5)),
              env_pts(0.25, [(0, 0), (0.015, 1), (0.25, 0)]))
    cr = svf(grain_cloud(0.3, 15, (0.001, 0.003)), 3500, 1.2, "bp")
    return add(scale(svf(hiss, 5000), 0.5), fit(scale(vox, 0.8), d), scale(fit(cr, d), 0.4))


def sfx_death():
    d = 1.6
    buf = zeros(N(d))
    for i, nm in enumerate(("E4", "D#4", "D4")):
        put(buf, inst_lead(nf(nm), 0.2, vib=0, cutoff=1600, duty=0.5, sq=0.3), 0.25 * i, 0.8)
    f0 = nf("C#4")
    last = lambda t: f0 * 2 ** (-max(0.0, t - 0.2) / 0.8)
    s = add(scale(osc("square", last, 0.85, duty=0.5, vib=0.02, vib_rate=5, vib_delay=0.3), 0.3),
            osc("tri", last, 0.85, vib=0.02, vib_rate=5, vib_delay=0.3),
            scale(osc("sine", lambda t: last(t) / 2, 0.85), 0.5))
    s = mul(svf(s, 1600, 0.7), adsr(0.85, 0.01, 0.1, 0.7, 0.4))
    put(buf, s, 0.75, 0.8)
    return fade(fit(reverb(buf, 0.82, 0.4, 0.3, tail=0.0), d), 0.002, 0.15)


def sfx_heartbeat():
    d = 1.0
    buf = zeros(N(d))
    put(buf, thump(95, 55, 0.25, 0.07, st=0.03, attack=0.004), 0.0, 1.0)
    put(buf, noise_burst(0.06, 0.02, 200), 0.0, 0.4)
    put(buf, thump(85, 50, 0.25, 0.06, st=0.03, attack=0.004), 0.28, 0.7)
    buf = drive(fit(buf, d), 2.0)
    return svf(buf, 450, 0.7)


def sfx_gasp():
    d = 0.7
    buf = zeros(N(d))
    inh_d = 0.32
    n = white(inh_d)
    inh = add(svf(n, (700, 1800), 3.0, "bp"), scale(svf(n, 2600, 4.0, "bp"), 0.4))
    inh = mul(inh, env_pts(inh_d, [(0, 0), (0.22, 1), (0.28, 0.8), (inh_d, 0)]))
    put(buf, inh, 0.0, 1.0)
    exh = mul(svf(white(0.3), 900, 2.0, "bp"), env_pts(0.3, [(0, 0), (0.03, 1), (0.3, 0)]))
    put(buf, exh, 0.38, 0.5)
    return svf(fit(buf, d), 300, 0.7, "hp")


def sfx_drown_bubble():
    d = 0.8
    buf = zeros(N(d))
    for t in sorted(rng.uniform(0.0, 0.55) for _ in range(9)):
        put(buf, bubble(rng.uniform(250, 700), rng.uniform(0.04, 0.09), rng.uniform(0.6, 1.2)),
            t, rng.uniform(0.4, 1.0))
    gur = mul(svf(white(d), 400, 2.0, "bp"), env_pts(d, [(0, 0), (0.1, 1), (d, 0)]))
    buf = add(fit(buf, d), scale(gur, 0.1))
    return fit(reverb(buf, 0.7, 0.5, 0.15, tail=0.0), d)


def sfx_ui_hover():
    d = 0.035
    return svf(mul(osc("tri", 1800, d), decay(d, 0.006, 0.0005)), 4000, 0.7)


def sfx_ui_click():
    d = 0.07
    a = mul(osc("sine", (1000, 550), d), decay(d, 0.015, 0.0008))
    b = noise_burst(d, 0.003, 3000, 1.0, "bp")
    c = mul(osc("sine", 300, d), decay(d, 0.01, 0.001))
    return add(a, scale(b, 0.3), scale(c, 0.4))


def sfx_achievement():
    d = 1.6
    buf = zeros(N(d))
    for i, nm in enumerate(("E5", "G#5", "B5", "E6")):
        put(buf, fm_bell(nf(nm), 1.2, index=2.0, tau=0.5 + 0.1 * i), 0.09 * i, 0.4)
    pad = mul(add(osc("tri", nf("E5"), 1.3), osc("tri", nf("B5"), 1.3)),
              env_pts(1.3, [(0, 0), (0.4, 1), (1.3, 0)]))
    put(buf, pad, 0.27, 0.08)
    return fade(fit(reverb(buf, 0.84, 0.35, 0.35, tail=0.0), d), 0.001, 0.2)


def sfx_layer_sting():
    d = 2.8
    buf = zeros(N(d))
    swell = brass([nf("A1"), nf("A2"), nf("E3"), nf("A3"), nf("C4")], 1.35,
                  [(0, 0), (1.2, 1.0), (1.3, 0.4), (1.35, 0)], 150, 1600)
    put(buf, swell, 0.0, 0.8)
    rev = mul(svf(white(1.3), 3000, 0.7, "hp"), [(i / N(1.3)) ** 3 for i in range(N(1.3))])
    put(buf, rev, 0.0, 0.15)
    hit_t = 1.3
    put(buf, thump(95, 52, 1.4, 0.4, st=0.06), hit_t, 1.1)
    put(buf, noise_burst(0.3, 0.06, 1500), hit_t, 0.6)
    stab = brass([nf("A2"), nf("E3"), nf("A3"), nf("C4"), nf("E4")], 1.5,
                 [(0, 0), (0.02, 1.0), (0.3, 0.55), (1.5, 0)], 300, 2200)
    put(buf, stab, hit_t, 0.9)
    put(buf, mul(osc("sine", nf("A1"), 1.5), decay(1.5, 0.6)), hit_t, 0.5)
    buf = reverb(fit(buf, d), 0.88, 0.45, 0.35, tail=0.0)
    return fade(fit(buf, d), 0.005, 0.3)


def sfx_game_over():
    d = 3.4
    buf = zeros(N(d))
    chords = [(0.0, ["A2", "C3", "E3"], 1.5), (1.5, ["G#2", "B2", "E3"], 0.55),
              (2.0, ["A2", "C3", "E3", "A3"], 0.9)]
    for t, ch, cd in chords:
        put(buf, inst_pad([nf(c) for c in ch], cd, cutoff=700, att=0.3, rel=0.7), t, 0.6)
        put(buf, inst_bass(nf(ch[0]) / 2, cd, 300), t, 0.5)
    for t, nm, nd in ((0.0, "E4", 0.45), (0.5, "D4", 0.45), (1.0, "C4", 0.45),
                      (1.5, "B3", 0.45), (2.0, "A3", 1.0)):
        put(buf, inst_lead(nf(nm), nd, vib=0.01, cutoff=1500, duty=0.5, sq=0.2, rel=0.3), t, 0.7)
    buf = reverb(fit(buf, d), 0.86, 0.4, 0.35, tail=0.0)
    return fade(fit(buf, d), 0.005, 0.5)


def sfx_core_hum():
    L, P = 4.0, 1.0
    T = P + L + 0.2
    hum = add(osc("sine", 55, T), scale(osc("sine", 55.25, T), 0.6),
              scale(osc("sine", 110, T), 0.35), scale(osc("sine", 165, T), 0.12))
    saw = svf(osc("saw", 55, T), lambda t: 250 + 150 * math.sin(TAU * 0.5 * t), 1.5)
    s = add(hum, scale(saw, 0.5))
    am = [0.55 + 0.45 * (0.5 + 0.5 * math.cos(TAU * 1.5 * i / SR)) ** 3 for i in range(N(T))]
    s = drive(mul(s, am), 1.5)
    return assemble_loop(L, periodic=s, P_s=P, XL_s=0.1)


def sfx_victory():
    beat = 0.5
    bar = 4 * beat
    d = 13.2
    buf = zeros(N(d))
    melody = [
        (0, 0, "C5", .5), (0, .5, "E5", .5), (0, 1, "G5", 1), (0, 2, "E5", 1), (0, 3, "G5", 1),
        (1, 0, "D5", 1), (1, 1, "G5", 1), (1, 2, "B5", 1.5), (1, 3.5, "A5", .5),
        (2, 0, "C6", 1), (2, 1, "B5", .5), (2, 1.5, "A5", .5), (2, 2, "E5", 2),
        (3, 0, "F5", 1), (3, 1, "A5", 1), (3, 2, "C6", 1), (3, 3, "A5", 1),
        (4, 0, "B5", 1), (4, 1, "D6", 1), (4, 2, "G5", .5), (4, 2.5, "A5", .5),
        (4, 3, "B5", .5), (4, 3.5, "D6", .5),
        (5, 0, "C6", 4),
    ]
    for b, bt, nm, ln in melody:
        rel = 0.6 if ln >= 4 else 0.06
        put(buf, inst_lead(nf(nm), ln * beat * 0.92, cutoff=2500, sq=0.35, rel=rel),
            b * bar + bt * beat, 0.5)
    chords = [["C4", "E4", "G4", "C5"], ["G3", "B3", "D4", "G4"], ["A3", "C4", "E4", "A4"],
              ["F3", "A3", "C4", "F4"], ["G3", "B3", "D4", "G4"]]
    roots = ["C3", "G2", "A2", "F2", "G2", "C3"]
    for b, ch in enumerate(chords):
        pat = [0, 1, 2, 3, 2, 1, 2, 3] * 2
        for k in range(16):
            put(buf, inst_pluck(nf(ch[pat[k]]), 0.12, 0.08), b * bar + k * beat / 4, 0.16)
    for b, r in enumerate(roots):
        f = nf(r)
        if b < 5:
            for k in range(8):
                put(buf, inst_bass(f if k % 2 == 0 else f * 2, beat / 2 * 0.85), b * bar + k * beat / 2, 0.35)
        else:
            put(buf, inst_bass(f, 2.0), b * bar, 0.45)
            put(buf, inst_pad([nf("C4"), nf("E4"), nf("G4"), nf("C5")], 2.0, cutoff=1500, att=0.05, rel=1.0),
                b * bar, 0.3)
    for b in range(5):
        t0 = b * bar
        for k in range(8):
            put(buf, hat(0.12 if k % 2 else 0.18), t0 + k * beat / 2)
        put(buf, kick(0.6), t0)
        put(buf, kick(0.6), t0 + 2 * beat)
        if b % 2 == 1:
            put(buf, kick(0.4), t0 + 2.5 * beat)
        put(buf, snare(0.4), t0 + beat)
        put(buf, snare(0.4), t0 + 3 * beat)
    for k in range(4):
        put(buf, snare(0.15 + 0.07 * k), 4 * bar + 3 * beat + k * beat / 4)
    put(buf, kick(0.7), 5 * bar)
    put(buf, crash(0.25), 5 * bar)
    buf = reverb(fit(buf, d), 0.8, 0.45, 0.2, tail=0.0)
    return fade(fit(buf, d), 0.005, 0.8)


# ---------------------------------------------------------------------------
# Ambient loops (L = 16 s)
# ---------------------------------------------------------------------------

AL = 16.0


def _drips(L, count_rng, f_rng, gain):
    ev = zeros(N(L))
    t = rng.uniform(0.2, 1.0)
    while t < L:
        f0 = rng.uniform(*f_rng)
        dd = 0.035
        s = mul(osc("sine", lambda tt, f0=f0: f0 * (1 + 1.4 * tt / dd), dd), decay(dd, 0.012, 0.001))
        put(ev, s, t, rng.uniform(0.5, 1.0) * gain)
        t += rng.uniform(*count_rng)
    return ev


def amb_layer0():
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    am = lambda t: 0.7 + 0.3 * math.sin(TAU * q(0.125, L) * t)
    tone = add(osc("sine", q(110, L), T), scale(osc("sine", q(110.4, L), T), 0.6),
               scale(osc("sine", q(165, L), T), 0.25), scale(osc("sine", q(220.2, L), T), 0.08))
    tone = mul(tone, _curve(am, N(T)))
    TN = L + XN
    wind_fc = [350 + 400 * v for v in slow_random(TN, 0.4)]
    wind = mul(svf(white(TN), wind_fc, 0.9, "bp"), slow_random(TN, 0.3, 0.2, 1.0))
    whistle = mul(svf(white(TN), 220, 12.0, "bp"), slow_random(TN, 0.25, 0.0, 1.0))
    noise = add(scale(wind, 0.9), scale(whistle, 0.6))
    ev = _drips(L, (0.7, 2.4), (900, 1700), 0.45)
    put(ev, bubble(520, 0.06, 0.9), 7.3, 0.35)
    ev = reverb(ev, 0.9, 0.35, 0.8, tail=3.0)
    return assemble_loop(L, periodic=scale(tone, 0.22), noise=noise, events=ev,
                         P_s=P, XL_s=XL, XN_s=XN)


def amb_layer1():
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    fc = lambda t: 220 + 80 * math.sin(TAU * q(1 / 16, L) * t)
    drone = add(osc("saw", q(55, L), T), osc("saw", q(55.19, L), T))
    drone = svf(drone, fc, 0.8)
    drone = add(drone, scale(osc("sine", q(82.5, L), T), 0.35), scale(osc("sine", q(110, L), T), 0.2))
    drone = mul(drone, _curve(lambda t: 0.8 + 0.2 * math.sin(TAU * q(0.125, L) * t + 1.0), N(T)))
    TN = L + XN
    bed = scale(lp1(brown(TN), 160), 0.5)
    ev = zeros(N(L))
    for t0 in (2.5, 10.0):
        r = mul(lp1(brown(3.5), 90), env_pts(3.5, [(0, 0), (1.2, 1), (3.5, 0)]))
        put(ev, r, t0, 1.6)
    for t0 in (6.3, 13.6):
        h = svf(add(svf(white(1.6), 2500, 0.7, "hp"), scale(svf(white(1.6), 5000, 0.8, "bp"), 0.6)), 8000)
        put(ev, mul(h, env_pts(1.6, [(0, 0), (0.35, 1), (1.6, 0)])), t0, 0.12)
    for t0 in (4.1, 8.7, 15.2):
        put(ev, bubble(rng.uniform(90, 140), 0.12, 0.6), t0, 0.25)
    ev = reverb(ev, 0.85, 0.5, 0.4, tail=2.0)
    return assemble_loop(L, periodic=scale(drone, 0.3), noise=bed, events=ev,
                         P_s=P, XL_s=XL, XN_s=XN)


def amb_layer2():
    L, P, XL, XN = AL, 4.0, 1.0, 2.0
    T = P + L + XL
    n = N(T)
    pad = zeros(n)
    voices = [("B3", 3, 0.0), ("F4", 5, 1.3), ("C5", 7, 2.1), ("E5", 4, 0.7)]
    for nm, rate, ph in voices:
        f = q(nf(nm), L)
        trem = _curve(lambda t, r=rate, p=ph: 0.5 + 0.5 * math.sin(TAU * q(r / 16, L) * t + p), n)
        v = add(osc("sine", f, T, vib=0.003, vib_rate=q(0.25 * rate, L)),
                scale(osc("sine", q(f * 2, L), T), 0.25), scale(osc("sine", q(f * 3, L), T), 0.08))
        pad = add(pad, mul(v, trem))
    ghost = osc("sine", lambda t: nf("B4") * 2 ** (0.5 / 12 * (1 + math.sin(TAU * t / L))), T)
    ghost = mul(ghost, _curve(lambda t: 0.5 + 0.5 * math.sin(TAU * 2 * t / L + 2.0), n))
    pad = add(scale(pad, 0.25), scale(ghost, 0.08))
    pad = reverb(pad, 0.86, 0.4, 0.5, tail=0.0)
    TN = L + XN
    shimmer = mul(svf(white(TN), 5000, 2.0, "bp"), slow_random(TN, 0.5, 0.0, 1.0))
    ev = zeros(N(L))
    for t0, nm in ((1.5, "B5"), (5.8, "F6"), (9.2, "C6"), (13.0, "E6"), (14.1, "F#6")):
        put(ev, fm_bell(nf(nm), 3.0, ratio=3.5, index=1.5, tau=1.0), t0, 0.22)
    ev = reverb(ev, 0.9, 0.3, 0.7, tail=3.0)
    return assemble_loop(L, periodic=pad, noise=scale(shimmer, 0.04), events=ev,
                         P_s=P, XL_s=XL, XN_s=XN)


def amb_layer3():
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    base = 41.2
    drone = zeros(N(T))
    for r, a in zip([1, 2.0, 2.76, 4.07, 5.4, 6.8], [1, 0.5, 0.35, 0.25, 0.15, 0.08]):
        f = q(base * r, L)
        drone = add(drone, scale(osc("sine", f, T), a), scale(osc("sine", f + 0.125, T), a * 0.7))
    drone = add(drone, scale(svf(osc("saw", q(base, L), T), 180, 1.0), 0.8))
    drone = drive(scale(drone, 0.35), 1.4)
    TN = L + XN
    pulse_rate = q(0.75, L)
    pulse = [(0.5 + 0.5 * math.sin(TAU * pulse_rate * i / SR)) ** 4 for i in range(N(TN))]
    grind = mul(mul(svf(white(TN), 350, 2.0, "bp"), pulse), slow_random(TN, 2.0, 0.4, 1.0))
    grind = drive(scale(grind, 2.0), 1.5)
    ev = zeros(N(L))
    t = 0.3
    while t < L:
        for k in range(rng.randint(3, 8)):
            put(ev, noise_burst(0.006, 0.0015, 3500, 0.7, "hp"), t + rng.uniform(0, 0.03),
                rng.uniform(0.1, 0.3))
        t += rng.expovariate(2.5)
    for t0 in (4.0, 11.5):
        g = svf(osc("saw", (62, 52), 3.0), 300, 1.5)
        put(ev, mul(g, env_pts(3.0, [(0, 0), (1.5, 1), (3.0, 0)])), t0, 0.25)
    ev = reverb(ev, 0.85, 0.4, 0.3, tail=2.0)
    return assemble_loop(L, periodic=drone, noise=scale(grind, 0.35), events=ev,
                         P_s=P, XL_s=XL, XN_s=XN)


def shepard(dur, L, fmin=55.0, octaves=6, center=220.0, width=1.0):
    n = N(dur)
    out = [0.0] * n
    phases = [0.0] * octaves
    c = math.log2(center / fmin)
    s = math.sin
    e = math.exp
    for i in range(n):
        pos = ((i / SR) / L) % 1.0
        acc = 0.0
        for k in range(octaves):
            o = (k + pos) % octaves
            f = fmin * 2.0 ** o
            phases[k] += f / SR
            phases[k] -= int(phases[k])
            acc += e(-((o - c) ** 2) / (2 * width * width)) * s(TAU * phases[k])
        out[i] = acc
    return out


def amb_layer4():
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    n = N(T)
    rate = q(1.25, L)

    def throb(t):
        # lub-dub pulse with ~12 ms soft attacks (no amplitude steps)
        ph = (t * rate) % 1.0
        p2 = ph - 0.25
        lub = (1.0 - math.exp(-ph / 0.012)) * math.exp(-ph / 0.08)
        dub = (1.0 - math.exp(-p2 / 0.012)) * math.exp(-p2 / 0.07) if p2 > 0 else 0.0
        return 0.25 + 1.6 * lub + 1.0 * dub

    body = add(osc("sine", 40, T), scale(osc("sine", 80, T), 0.5), scale(osc("sine", 120, T), 0.2))
    body = drive(mul(body, _curve(throb, n)), 1.8)
    tens = add(osc("sine", q(nf("D#4"), L), T), osc("sine", q(nf("E4"), L), T))
    tens = mul(tens, _curve(lambda t: 0.5 + 0.5 * math.sin(TAU * q(3 / 16, L) * t), n))
    periodic = add(scale(body, 0.45), scale(tens, 0.04))
    TN = L + XN
    shep = scale(shepard(TN, L, 55.0, 6, 260.0, 0.9), 0.1)
    rum = mul(lp1(brown(TN), 120), _curve(lambda t: 0.4 + 0.3 * throb(t), N(TN)))
    hiss = mul(svf(white(TN), 4000, 0.7, "hp"), slow_random(TN, 0.6, 0.0, 1.0))
    noise = add(shep, scale(rum, 0.6), scale(hiss, 0.03))
    return assemble_loop(L, periodic=periodic, noise=noise, P_s=P, XL_s=XL, XN_s=XN)


# ---------------------------------------------------------------------------
# Menu music (24 s loop: 8 bars of 4/4 at 80 BPM)
# ---------------------------------------------------------------------------

def music_menu():
    beat = 0.75
    bar = 4 * beat
    L = 8 * bar
    buf = zeros(N(L + 5.0))
    pads = [["A2", "E3", "A3", "C4"], ["F2", "C3", "F3", "A3"], ["C3", "G3", "C4", "E4"],
            ["G2", "D3", "G3", "B3"], ["A2", "E3", "A3", "C4"], ["F2", "C3", "F3", "A3"],
            ["D3", "A3", "D4", "F4"], ["E2", "B2", "E3", "G#3"]]
    arps = [["A4", "C5", "E5", "A5"], ["F4", "A4", "C5", "F5"], ["G4", "C5", "E5", "G5"],
            ["G4", "B4", "D5", "G5"], ["A4", "C5", "E5", "A5"], ["F4", "A4", "C5", "F5"],
            ["F4", "A4", "D5", "F5"], ["E4", "G#4", "B4", "E5"]]
    roots = ["A2", "F2", "C3", "G2", "A2", "F2", "D3", "E2"]
    pats = [[0, 1, 2, 3, 1, 2, 3, 2], [3, 2, 1, 2, 0, 1, 2, 1]]
    lead = zeros(N(L + 5.0))
    for b in range(8):
        t0 = b * bar
        put(buf, inst_pad([nf(x) for x in pads[b]], bar, cutoff=800, att=0.9, rel=1.2), t0, 0.5)
        f = nf(roots[b])
        for st, ln in ((0.0, 1.35), (1.5, 0.4), (2.0, 1.9)):
            put(buf, inst_bass(f, ln * beat, 380), t0 + st * beat, 0.45)
        pat = pats[b % 2]
        for k in range(8):
            if b % 4 == 3 and k >= 6:
                continue
            nm = arps[b][pat[k]]
            vel = 0.9 if k % 2 == 0 else 0.7
            s = inst_lead(nf(nm), 0.3, vib=0, cutoff=1700, duty=0.3, sq=0.3, rel=0.25)
            put(lead, mul(s, decay(len(s) / SR, 0.25, 0.004)), t0 + k * beat / 2, vel)
    lead = echo(lead, 1.5 * beat / 2 * 1.0, fb=0.35, mix=0.35, tail=0.0, lp_fc=2500)
    buf = add(buf, scale(lead, 0.35))
    buf = reverb(buf, 0.86, 0.45, 0.3, tail=0.0)
    return fold(buf, N(L))


# ---------------------------------------------------------------------------
# Registry / output
# ---------------------------------------------------------------------------

SFX, LOOP, AMB, MUSIC = "sfx", "loop", "amb", "music"
TARGET_DB = {SFX: -1.0, LOOP: -4.0, AMB: -6.0, MUSIC: -3.0}

SOUNDS = [
    ("step_soft_1", lambda: step_soft(1.0), SFX),
    ("step_soft_2", lambda: step_soft(0.85), SFX),
    ("step_hard_1", lambda: step_hard(1.0), SFX),
    ("step_hard_2", lambda: step_hard(1.18), SFX),
    ("step_gravel_1", lambda: step_gravel(1.0), SFX),
    ("step_gravel_2", lambda: step_gravel(1.25), SFX),
    ("step_metal_1", lambda: step_metal(1.0), SFX),
    ("step_metal_2", lambda: step_metal(1.12), SFX),
    ("jump", sfx_jump, SFX),
    ("land", sfx_land, SFX),
    ("dash", sfx_dash, SFX),
    ("climb", sfx_climb, SFX),
    ("dig_soft", sfx_dig_soft, SFX),
    ("dig_hard", sfx_dig_hard, SFX),
    ("dig_crystal", sfx_dig_crystal, SFX),
    ("dig_metal", sfx_dig_metal, SFX),
    ("place_rope", sfx_place_rope, SFX),
    ("throw", sfx_throw, SFX),
    ("torch_ignite", sfx_torch_ignite, SFX),
    ("pickup", sfx_pickup, SFX),
    ("item_get", sfx_item_get, SFX),
    ("chest_open", sfx_chest_open, SFX),
    ("shrine_buy", sfx_shrine_buy, SFX),
    ("water_spray", sfx_water_spray, LOOP),
    ("spark", sfx_spark, SFX),
    ("freeze", sfx_freeze, SFX),
    ("plant", sfx_plant, SFX),
    ("fire_loop", sfx_fire_loop, LOOP),
    ("sizzle", sfx_sizzle, SFX),
    ("splash", sfx_splash, SFX),
    ("acid_hiss", sfx_acid_hiss, SFX),
    ("explosion", lambda: make_explosion(1.9, 1.0), SFX),
    ("explosion_small", lambda: make_explosion(0.8, 0.5), SFX),
    ("rumble", sfx_rumble, SFX),
    ("collapse", sfx_collapse, SFX),
    ("hurt", sfx_hurt, SFX),
    ("burn", sfx_burn, SFX),
    ("death", sfx_death, SFX),
    ("heartbeat", sfx_heartbeat, SFX),
    ("gasp", sfx_gasp, SFX),
    ("drown_bubble", sfx_drown_bubble, SFX),
    ("ui_hover", sfx_ui_hover, SFX),
    ("ui_click", sfx_ui_click, SFX),
    ("achievement", sfx_achievement, SFX),
    ("layer_sting", sfx_layer_sting, SFX),
    ("victory", sfx_victory, SFX),
    ("game_over", sfx_game_over, SFX),
    ("core_hum", sfx_core_hum, LOOP),
    ("amb_layer0", amb_layer0, AMB),
    ("amb_layer1", amb_layer1, AMB),
    ("amb_layer2", amb_layer2, AMB),
    ("amb_layer3", amb_layer3, AMB),
    ("amb_layer4", amb_layer4, AMB),
    ("music_menu", music_menu, MUSIC),
]


def finalize(sig, kind):
    if kind == SFX:
        sig = hp1(sig, 15.0)
        fade(sig, 0.002, 0.012)
    else:
        m = sum(sig) / len(sig)
        sig = [v - m for v in sig]
    peak = max(abs(v) for v in sig) or 1.0
    g = 10 ** (TARGET_DB[kind] / 20.0) / peak
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
        a = array.array("h")
        a.frombytes(w.readframes(w.getnframes()))
    if sys.byteorder == "big":
        a.byteswap()
    return [v / 32767.0 for v in a]


def analyze(names):
    print()
    print("%-16s %5s %7s %8s %8s %7s  %s" % ("file", "kind", "dur s", "peak dB", "rms dB", "zcr/s", "notes"))
    total = 0
    problems = 0
    kinds = {nm: k for nm, _, k in SOUNDS}
    for nm in names:
        path = os.path.join(OUT_DIR, nm + ".wav")
        total += os.path.getsize(path)
        x = read_wav(path)
        n = len(x)
        peak = max(abs(v) for v in x)
        rms = math.sqrt(sum(v * v for v in x) / n)
        zc = sum(1 for i in range(1, n) if (x[i - 1] < 0) != (x[i] < 0)) / (n / SR)
        clipped = sum(1 for v in x if abs(v) >= 0.9999)
        notes = []
        if rms < 1e-3:
            notes.append("SILENT?")
            problems += 1
        if clipped:
            notes.append("CLIP x%d" % clipped)
            problems += 1
        if kinds[nm] != SFX:
            # Seam check: the wrap step x[-1] -> x[0] compared with the largest
            # ordinary step in the 64 samples on either side of the seam, and
            # the 2nd difference across the seam vs. the local 2nd differences.
            w = x[-64:] + x[:64]
            steps = [abs(w[i] - w[i - 1]) for i in range(1, len(w)) if i != 64]
            jump = abs(x[0] - x[-1])
            ratio = jump / (max(steps) or 1e-9)
            d2 = [abs(w[i + 1] - 2 * w[i] + w[i - 1]) for i in range(1, len(w) - 1)]
            seam_d2 = max(d2[62], d2[63])
            loc_d2 = sorted(d2)[int(len(d2) * 0.95)] or 1e-9
            notes.append("seam step %.2fx local max, curvature %.2fx p95%s" % (
                ratio, seam_d2 / loc_d2, "  BAD" if ratio > 1.5 else ""))
            if ratio > 1.5:
                problems += 1
        else:
            if abs(x[0]) > 0.01 or abs(x[-1]) > 0.01:
                notes.append("edge not zero")
                problems += 1
        print("%-16s %5s %7.3f %8.2f %8.2f %7.0f  %s" % (
            nm, kinds[nm], n / SR, 20 * math.log10(peak or 1e-9),
            20 * math.log10(rms or 1e-9), zc, ", ".join(notes)))
    print("\n%d files, total %.2f MB, %d problem(s)" % (len(names), total / 1e6, problems))


def main(argv):
    global rng
    os.makedirs(OUT_DIR, exist_ok=True)
    wanted = set(argv)
    names = []
    for name, fn, kind in SOUNDS:
        if wanted and name not in wanted:
            continue
        rng = random.Random("descent:" + name)
        sig = finalize(fn(), kind)
        write_wav(os.path.join(OUT_DIR, name + ".wav"), sig)
        names.append(name)
        print("wrote %-16s %6.2f s" % (name, len(sig) / SR), flush=True)
    unknown = wanted - {n for n, _, _ in SOUNDS}
    if unknown:
        print("unknown names:", ", ".join(sorted(unknown)))
    analyze(names)


if __name__ == "__main__":
    main(sys.argv[1:])
