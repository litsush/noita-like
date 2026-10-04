#!/usr/bin/env python3
"""Procedural audio generator for "The Last Apprentice".

Python 3 standard library only. sfxr/jsfxr-style synthesis (band-limited
oscillators, ADSR envelopes, pitch slides, vibrato, state-variable filters,
Schroeder reverb, feedback echo, layering) plus a dark-fantasy kit: formant
"choir" voices (stacked detuned saws through resonant aah/ooh band-pass
banks), noise whispers, detuned string ensembles, church bells, harp, frame
drums, bone rattles and magical shimmer. Fully deterministic: every sound
seeds its own RNG from its name.

Usage (from the repo root):
    python3 tools/gen_audio.py            # regenerate everything
    python3 tools/gen_audio.py jump dash  # regenerate only the named files

Output: assets/audio/<name>.wav -- 16-bit PCM, mono, 22050 Hz.
One-shot SFX peak at about -1 dBFS (the game sets per-sound volume);
continuous SFX loops at -4 dBFS; ambient loops at -6 dBFS; music at -3 dBFS.
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
  climb         short scrape (wall-climbing step)

Dig spell (0.3-0.4 s magical crackles; the game rate-limits them)
  dig_beam_soft     earthy crumble + low crackle
  dig_beam_hard     stony chime + crackle
  dig_beam_crystal  glassy shimmer + tinkles
  dig_beam_metal    ringing clang + electric buzz

Light orb and shards
  cast_light    soft rising chime as an orb of light is conjured (~1.1 s)
  orb_fade      gentle guttering out (~1 s)
  shard_pickup  bright crystal tinkle (0.22 s)
  shard_vein    resonant crystal crack when digging a shard vein (~1 s)

Spells (cast 0.35-0.55 s, impact 0.6-0.9 s)
  cast_pyro,  impact_pyro    roaring whoosh + crackle / fireball burst
  cast_hydro, impact_hydro   watery surge / splash
  cast_terra, impact_terra   grinding stone into a thud / heavy impact + debris
  cast_cryo,  impact_cryo    glassy freeze crack / ice shatter
  cast_aero,  impact_aero    rushing wind swirl / gust burst
  cast_fulm,  impact_fulm    electric zap / thunder snap
  cast_alch,  impact_alch    glass clink + bubbling hiss / flask shatter + fizz
  cast_myco,  impact_myco    soft organic puff + squelch / wet spore splat
  tidecall_loop   [LOOP 1 s] conjured water stream
  levitate_loop   [LOOP 1.5 s] airy hum

Spell moments
  fusion_cast     big layered magical blast (~1.2 s)
  fusion_awaken   dramatic choral sting (~2.5 s)
  attune          solemn chord as the second school is bound (~2.1 s)
  scroll_learned  short magical fanfare: harp, choir, bells (~2.1 s)
  scroll_shatter  paper tearing into a crystal tinkle (~1 s)
  blink           whoosh-pop teleport (~0.55 s)

Lore
  journal_open    paper rustle + faint whisper (~1.5 s)
  remains_search  bone clatter + cloth (~1.2 s)

Creatures
  hollowed_moan        ghostly falling moan (~1.9 s)
  hollowed_cast        twisted bolt: dissonant ring, swell, grunt (~0.7 s)
  wyrm_burrow          [LOOP 2 s] grinding rumble
  wyrm_screech         formant screech over a growl (~1.2 s)
  puppet_squelch       wet squelch (~0.45 s)
  puppet_burst         wet burst + spore puff (~0.9 s)
  wraith_hiss          fiery ghost hiss (~1 s)
  lightseeker_flutter  [LOOP 1 s] papery wings
  lightseeker_shriek   thin high shriek (~0.8 s)
  mimic_bite           wooden snap + wet growl (~0.85 s)
  stalker_step         heavy, muffled footstep (~0.7 s)
  stalker_breath       [LOOP 3 s] slow, wet breathing
  stalker_appear       horror sting: reversed swell into a dissonant stab (~2.6 s)

Horror
  whisper_1     near, bright whisper (~1.8 s)
  whisper_2     far, muffled whisper in a big space (~2.4 s)
  whisper_3     whisper off to one side, comb-coloured with slap echo (~1.4 s)
  whisper_4     two overlapping whisperers, near and far (~2.8 s)
  distant_steps faint footsteps in the dark (~2.2 s)
  sting_1       dissonant string stab (~1.6 s)
  sting_2       reversed swell cut off by a drum hit (~1.5 s)
  sting_3       rising dissonant choir + bone rattle (~1.8 s)

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
  death         thud + choir sliding down, last breath (~1.8 s)
  heartbeat     low-HP lub-dub (1 s; can be repeated back to back)
  gasp          breath recovered after surfacing
  drown_bubble  bubbles (underwater / drowning)

Chests and shrines
  chest_open    wooden creak + dim minor sparkle
  shrine_buy    shard clinks + dark bell chime

UI and stings
  ui_hover      tiny tick
  ui_click      soft click
  ui_toggle     soft rune click (toggles such as the dig mode)
  achievement   arcane bell chime with a choral tail (~2.3 s)
  layer_sting   dark choral swell + low drum hit (~2.7 s)
  victory       ~12 s completed rite: rising choir resolving to A major,
                bells, harp and frame drums
  game_over     somber choir fall (~3.2 s)
  core_hum      [LOOP 4 s] deep heartbeat-like hum with faint choir

Ambient loops [LOOP 18 s each]
  amb_layer0    Whispering Crust: hollow drone, drips, faint whispers
  amb_layer1    Drowned Halls: cold watery drone, echoing drips, muffled choir
  amb_layer2    Fungal Abyss: wet organic pads, dissonant shimmer, creaks
  amb_layer3    Molten Sanctum: heavy drone, slow ritual drum pulse, rumbles
  amb_layer4    Heart of the World: intense choir, heartbeat throb, rising
                Shepard tension

Music
  music_menu    [LOOP 28 s] main menu: A minor choral pad, low strings,
                sparse harp with echo, distant bells
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


# --- Level helpers -----------------------------------------------------------

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


def comb(x, delay_s, g=0.7):
    """Feed-forward comb: a phasey colouring that reads as 'off to one side'."""
    d = max(1, N(delay_s))
    return [x[i] + g * (x[i - d] if i >= d else 0.0) for i in range(len(x))]


# --- Voices: formant filtering, choir, whispers ------------------------------

# (centre Hz, bandwidth Hz, gain) for a low/mid voice
VOWELS = {
    "a": [(730, 90, 1.0), (1090, 110, 0.55), (2440, 160, 0.28)],   # aah
    "o": [(570, 80, 1.0), (840, 100, 0.5), (2410, 160, 0.18)],     # oh
    "u": [(300, 70, 1.0), (870, 100, 0.3), (2240, 160, 0.1)],      # ooh
    "e": [(530, 80, 1.0), (1840, 120, 0.45), (2480, 160, 0.3)],    # eh
    "i": [(270, 70, 1.0), (2290, 130, 0.4), (3010, 200, 0.3)],     # ee
}


def formant(x, vowel, shift=1.0, qs=1.0):
    """Parallel resonant band-pass bank shaping x into a vowel.
    vowel: key of VOWELS, or a (from, to) pair that morphs over the signal."""
    if isinstance(vowel, tuple):
        va, vb = VOWELS[vowel[0]], VOWELS[vowel[1]]
    else:
        va = vb = VOWELS[vowel]
    n = len(x)
    out = [0.0] * n
    for (fa, ba, ga), (fb, bb, gb) in zip(va, vb):
        fc = fa * shift if fa == fb else (fa * shift, fb * shift)
        y = svf(x, fc, qs * (fa + fb) / (ba + bb), "bp")
        g = 0.5 * (ga + gb)
        for i in range(n):
            out[i] += g * y[i]
    return out


def choir(freqs, dur, vowel="a", voices=3, detune=0.007, vib=0.0035, breath=0.05,
          body=0.25, shift=1.0, glide=None, env=None):
    """Stacked detuned saw 'singers' (each with its own vibrato and slow pitch
    drift) through a vowel formant bank, plus breath noise. Peak-normalised.
    glide: optional pitch-multiplier curve spec applied to every voice.
    env:   optional env_pts list."""
    n = N(dur)
    src = [0.0] * n
    s = math.sin
    gl = _curve(glide, n) if glide is not None else None
    for f in freqs:
        for v in range(voices):
            spread = (v - (voices - 1) / 2.0) / max(1.0, (voices - 1) / 2.0)
            det = f * (1.0 + detune * spread + rng.uniform(-0.0015, 0.0015))
            rate = rng.uniform(4.3, 5.7)
            ph = rng.random() * TAU
            drift = slow_random(dur, 0.6, -0.003, 0.003)
            w = TAU * rate / SR
            if gl is None:
                fr = [det * (1.0 + vib * s(w * i + ph) + drift[i]) for i in range(n)]
            else:
                fr = [det * gl[i] * (1.0 + vib * s(w * i + ph) + drift[i]) for i in range(n)]
            sig = osc("saw", fr, dur, phase=rng.random())
            for i in range(n):
                src[i] += sig[i]
    src = lp1(src, 4500)
    nz = scale(white(dur), breath * math.sqrt(len(freqs) * voices))
    out = formant(add(src, nz), vowel, shift)
    if body:
        lo = svf(src, 380, 0.7)
        out = add(norm(out), scale(norm(lo), body))
    out = norm(out)
    if env is not None:
        out = mul(out, env_pts(dur, env))
    return out


def whisper(dur, rate=6.0, bright=1.0, vowels="aeiou", sib=0.5):
    """Unintelligible whisper: noise through formant filters that jump between
    random vowels per 'syllable', with sibilant bursts between syllables."""
    n = N(dur)
    f1, f2, f3 = [500.0] * n, [1500.0] * n, [2500.0] * n
    amp = [0.0] * n
    hiss = [0.0] * n
    t = rng.uniform(0.03, 0.1)
    while t < dur - 0.12:
        sl = rng.uniform(0.6, 1.4) / rate
        v = VOWELS[rng.choice(vowels)]
        sh = rng.uniform(0.9, 1.15)
        i0, i1 = N(t), min(n, N(t + sl))
        g = rng.uniform(0.55, 1.0)
        for i in range(i0, i1):
            f1[i], f2[i], f3[i] = v[0][0] * sh, v[1][0] * sh, v[2][0] * sh
            x = (i - i0) / max(1, i1 - i0)
            amp[i] += g * math.sin(math.pi * x) ** 0.7
        if rng.random() < 0.55:
            cl = N(rng.uniform(0.03, 0.09))
            for k in range(cl):
                j = i0 - cl + k
                if 0 <= j < n:
                    hiss[j] += math.sin(math.pi * k / cl) * rng.uniform(0.7, 1.0)
        t += sl + (rng.uniform(0.06, 0.25) if rng.random() < 0.25 else rng.uniform(0.0, 0.03))
    for i in range(n):
        if f1[i] == 500.0 and i > 0:
            f1[i], f2[i], f3[i] = f1[i - 1], f2[i - 1], f3[i - 1]
    f1, f2, f3 = lp1(f1, 20), lp1(f2, 20), lp1(f3, 20)
    amp, hiss = lp1(amp, 45), lp1(hiss, 60)
    nz = white(dur)
    vo = add(svf(nz, f1, 6.0, "bp"), scale(svf(nz, f2, 8.0, "bp"), 0.75),
             scale(svf(nz, f3, 10.0, "bp"), 0.4))
    vo = mul(vo, amp)
    sb = mul(svf(white(dur), 5200 * bright, 2.5, "bp"), hiss)
    out = add(norm(vo), scale(norm(sb), sib))
    out = svf(out, min(9000.0, 6500 * bright), 0.7)
    return fade(norm(out), 0.01, 0.05)


# --- Strings, bells, harp ----------------------------------------------------

def strings(freqs, dur, env=None, fc=1500, detune=0.005, vib=0.006, vib_rate=4.5, q_=0.8):
    """Detuned saw ensemble through a low-pass (fc may be a curve) with slow
    vibrato. Peak-normalised."""
    n = N(dur)
    src = [0.0] * n
    s = math.sin
    for f in freqs:
        for d in (-1.0, 0.0, 1.0):
            w = TAU * vib_rate * rng.uniform(0.85, 1.15) / SR
            ph = rng.random() * TAU
            base = f * (1.0 + detune * d + rng.uniform(-0.001, 0.001))
            fr = [base * (1.0 + vib * s(w * i + ph)) for i in range(n)]
            sig = osc("saw", fr, dur, phase=rng.random())
            for i in range(n):
                src[i] += sig[i]
    out = norm(svf(src, fc, q_))
    if env is not None:
        out = mul(out, env_pts(dur, env))
    return out


def bell(f, dur, tau=1.5, bright=1.0):
    """Church-bell partials (hum, prime, minor tierce, quint, nominal ...)."""
    ratios = [0.5, 1.0, 1.19, 1.5, 2.0, 2.52, 3.0, 4.07]
    amps = [0.5, 1.0, 0.55, 0.3, 0.45, 0.22 * bright, 0.15 * bright, 0.08 * bright]
    taus = [tau * 1.6, tau, tau * 0.8, tau * 0.6, tau * 0.5, tau * 0.35, tau * 0.25, tau * 0.15]
    keep = [(r, a, t) for r, a, t in zip(ratios, amps, taus) if f * r < SR * 0.42]
    ring = partials(f, [k[0] for k in keep], [k[1] for k in keep], [k[2] for k in keep],
                    dur, detune=0.0015, attack=0.002)
    strike = noise_burst(0.03, 0.004, min(8000.0, f * 3), 1.0, "bp")
    return norm(add(ring, scale(norm(strike), 0.12)))


def harp(f, dur=2.0, tau=0.9, bright=1.0):
    """Plucked, slightly inharmonic string (harp / lyre)."""
    p = partials(f, [1, 2.003, 3.008, 4.015, 5.03],
                 [1, 0.45 * bright, 0.22 * bright, 0.1 * bright, 0.05 * bright],
                 [tau, tau * 0.6, tau * 0.4, tau * 0.28, tau * 0.2], dur, attack=0.002)
    pl = noise_burst(0.02, 0.003, min(8000.0, f * 4), 1.0, "lp")
    return norm(add(norm(p), scale(norm(pl), 0.12)))


# --- Ritual percussion --------------------------------------------------------

def frame_drum(f=65.0, dur=1.0, tau=0.3, skin=0.35):
    """Low frame drum: pitched membrane thump, inharmonic modes, skin slap."""
    body = thump(f * 1.8, f, dur, tau, st=0.018, attack=0.002)
    modes = partials(f, [1.59, 2.14, 2.65], [0.35, 0.22, 0.12],
                     [tau * 0.4, tau * 0.3, tau * 0.2], dur)
    hit = noise_burst(min(dur, 0.12), 0.025, 700, 0.9, "bp")
    return norm(add(body, modes, scale(norm(hit), skin)))


def bone_rattle(dur, count, dist_pow=1.3, amp_tau=None, f_rng=(900, 2600)):
    """Cluster of dry, hollow clicks (bones, teeth, charms)."""
    buf = zeros(N(dur))
    for _ in range(count):
        t = dur * 0.85 * rng.random() ** dist_pow
        fr = rng.uniform(*f_rng)
        click = partials(fr, [1, 2.3], [1, 0.4], [0.006, 0.004], 0.05)
        nz = noise_burst(0.02, 0.003, min(8000.0, fr * 1.5), 2.0, "bp")
        g = rng.uniform(0.3, 1.0) * (math.exp(-t / amp_tau) if amp_tau else 1.0)
        put(buf, add(click, scale(nz, 0.6)), t, g)
    return norm(fit(buf, dur))


# --- Magic --------------------------------------------------------------------

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


def shimmer(freqs, dur, rate=(7.0, 13.0), depth=0.8, vib=0.002):
    """Sustained glitter: sines with fast random amplitude flutter."""
    out = zeros(N(dur))
    for f in freqs:
        am = slow_random(dur, rng.uniform(*rate), 1.0 - depth, 1.0)
        s = osc("sine", f, dur, vib=vib, vib_rate=rng.uniform(4, 6), phase=rng.random())
        out = add(out, mul(s, am))
    return norm(out)


def arcane(freqs, dur, tau=0.4, trem=9.0, attack=0.01):
    """Short resonant magical ring: detuned sine pairs with tremolo, decaying."""
    n = N(dur)
    out = zeros(n)
    for f in freqs:
        for d in (0.997, 1.003):
            out = add(out, osc("sine", f * d, dur, phase=rng.random()))
    w = TAU * trem / SR
    am = [0.65 + 0.35 * math.sin(w * i) for i in range(n)]
    return tail_fade(norm(mul(mul(out, am), decay(dur, tau, attack))))


def swoosh(dur, fc, q_=1.2, env=None):
    """Band-passed noise sweep (fc may be a curve spec)."""
    s = svf(white(dur), fc, q_, "bp")
    s = mul(s, env_pts(dur, env)) if env else s
    return norm(s)


def rev_swell(dur, fc=3000):
    """Reversed-cymbal style noise swell that ends at dur."""
    n = N(dur)
    s = svf(white(dur), fc, 0.7, "hp")
    return norm(mul(s, [(i / n) ** 3 for i in range(n)]))


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


# ---------------------------------------------------------------------------
# Dig spell (short; the game rate-limits and pitch-varies them)
# ---------------------------------------------------------------------------

def beam_crackle(d, density=90, fc=3000, q_=1.4):
    g = grain_cloud(d, max(3, int(density * d)), (0.0012, 0.004))
    return norm(svf(g, fc, q_, "bp"))


def beam_hum(d, f, trem=31.0):
    return arcane([f, f * 1.5], d, tau=d * 0.45, trem=trem, attack=0.01)


def sfx_dig_beam_soft():
    d = 0.3
    buf = zeros(N(d))
    put(buf, thump(110, 55, 0.25, 0.05, st=0.02), 0.0, 0.8)
    cr = grain_cloud(0.26, 24, (0.003, 0.012), dist_pow=1.8, amp_tau=0.1)
    cr = norm(add(svf(cr, 1000, 0.7), scale(svf(cr, 520, 1.5, "bp"), 0.6)))
    put(buf, cr, 0.01, 0.7)
    put(buf, mul(beam_crackle(d, 70, 2200), decay(d, 0.12)), 0.0, 0.3)
    put(buf, beam_hum(d, 392), 0.0, 0.14)
    return svf(fit(buf, d), 5000, 0.7)


def sfx_dig_beam_hard():
    d = 0.32
    buf = zeros(N(d))
    chime = partials(1480, [1, 1.47, 2.09, 2.9], [1, 0.5, 0.3, 0.15],
                     [0.09, 0.06, 0.04, 0.025], d, detune=0.003)
    put(buf, norm(chime), 0.0, 0.55)
    put(buf, noise_burst(0.03, 0.004, 3200, 1.2, "bp"), 0.0, 0.5)
    put(buf, thump(170, 95, 0.2, 0.03), 0.0, 0.5)
    cr = svf(grain_cloud(0.26, 16, (0.003, 0.009), 1.8, 0.09), 2200, 0.7)
    put(buf, norm(cr), 0.03, 0.4)
    put(buf, mul(beam_crackle(d, 90, 3000), decay(d, 0.1)), 0.0, 0.3)
    put(buf, beam_hum(d, 740), 0.0, 0.12)
    return svf(fit(buf, d), 7000, 0.7)


def sfx_dig_beam_crystal():
    d = 0.38
    buf = zeros(N(d))
    glass = partials(2350, [1, 1.83, 2.71, 3.6], [1, 0.4, 0.25, 0.1],
                     [0.16, 0.1, 0.07, 0.04], d, detune=0.003)
    put(buf, norm(glass), 0.0, 0.5)
    put(buf, sparkle(0.34, 7, (3000, 6200), (0.02, 0.06)), 0.02, 0.35)
    put(buf, noise_burst(0.02, 0.003, 4500, 1.0, "bp"), 0.0, 0.3)
    put(buf, mul(beam_crackle(d, 110, 4500, 1.8), decay(d, 0.12)), 0.0, 0.25)
    air = mul(svf(white(d), 6000, 0.8, "bp"), env_pts(d, [(0, 0), (0.05, 1), (d, 0)]))
    put(buf, norm(air), 0.0, 0.08)
    return svf(fit(buf, d), 8500, 0.7)


def sfx_dig_beam_metal():
    d = 0.4
    buf = zeros(N(d))
    ring = partials(520, [1, 2.32, 2.76, 4.07, 5.4], [1, 0.6, 0.45, 0.28, 0.14],
                    [0.2, 0.15, 0.12, 0.08, 0.05], d, detune=0.003)
    put(buf, norm(ring), 0.0, 0.6)
    put(buf, noise_burst(0.03, 0.005, 2200, 1.0, "bp"), 0.0, 0.45)
    put(buf, thump(150, 105, 0.2, 0.05), 0.0, 0.4)
    nseg = N(0.006)
    fl = []
    while len(fl) < N(d):
        fl += [rng.uniform(110, 260)] * nseg
    buzz = mul(svf(osc("saw", fl[:N(d)], d), 1300, 1.5, "bp"), decay(d, 0.08, 0.004))
    put(buf, norm(buzz), 0.0, 0.2)
    put(buf, mul(beam_crackle(d, 80, 3500), decay(d, 0.1)), 0.0, 0.25)
    return svf(fit(buf, d), 7000, 0.7)


# ---------------------------------------------------------------------------
# Light orb and shards
# ---------------------------------------------------------------------------

def sfx_cast_light():
    d = 1.1
    buf = zeros(N(d))
    gl = mul(osc("sine", (440, 880), 0.45), env_pts(0.45, [(0, 0), (0.25, 1), (0.45, 0)]))
    put(buf, gl, 0.0, 0.3)
    for i, nm in enumerate(("E5", "A5", "C#6", "E6")):
        put(buf, fm_bell(nf(nm), 0.8, ratio=2.0, index=0.7, tau=0.3 + 0.05 * i), 0.06 + 0.08 * i, 0.32)
    air = mul(svf(white(0.6), (1500, 6000), 1.0, "bp"), env_pts(0.6, [(0, 0), (0.35, 1), (0.6, 0)]))
    put(buf, norm(air), 0.0, 0.1)
    put(buf, shimmer([nf("A6"), nf("E7")], 0.7), 0.3, 0.05)
    return fade(fit(reverb(buf, 0.84, 0.35, 0.35, tail=0.0), d), 0.002, 0.2)


def sfx_orb_fade():
    d = 1.0
    gate = [min(1.0, max(0.0, 2.2 * v - 0.6)) for v in slow_random(d, 22)]
    flick = mul(svf(white(d), 1100, 0.8), gate)
    flick = mul(flick, env_pts(d, [(0, 0.8), (0.5, 0.5), (0.85, 0.15), (d, 0)]))
    tone = add(osc("sine", (880, 440), d), scale(osc("sine", (1320, 620), d), 0.4))
    tone = mul(mul(tone, gate), env_pts(d, [(0, 0), (0.05, 1), (0.7, 0.3), (d, 0)]))
    buf = add(scale(norm(flick), 0.6), scale(norm(tone), 0.4))
    put(buf, noise_burst(0.12, 0.04, 500, 0.7), 0.78, 0.3)
    return fade(fit(reverb(buf, 0.8, 0.4, 0.3, tail=0.0), d), 0.003, 0.15)


def sfx_shard_pickup():
    d = 0.22
    buf = zeros(N(d))
    put(buf, norm(partials(2637, [1, 2.76], [1, 0.25], [0.05, 0.02], 0.15)), 0.0, 0.7)
    put(buf, norm(partials(3520, [1, 2.76], [1, 0.25], [0.06, 0.02], 0.17)), 0.045, 0.8)
    put(buf, sparkle(0.15, 3, (4500, 7000), (0.01, 0.025)), 0.03, 0.2)
    return fit(reverb(buf, 0.75, 0.3, 0.25, tail=0.0), d)


def sfx_shard_vein():
    d = 0.95
    buf = zeros(N(d))
    put(buf, noise_burst(0.05, 0.008, 1800, 0.7, "hp"), 0.0, 0.7)
    put(buf, thump(140, 70, 0.3, 0.06), 0.0, 0.6)
    res = partials(988, [1, 2.76, 5.4, 8.1], [1, 0.45, 0.2, 0.08], [0.45, 0.25, 0.12, 0.06],
                   0.9, detune=0.002)
    put(buf, norm(res), 0.005, 0.55)
    put(buf, sparkle(0.6, 9, (2500, 5500), (0.03, 0.1), dist_pow=1.5), 0.03, 0.3)
    sh =mul(shimmer([nf("B5"), nf("F#6")], 0.8), env_pts(0.8, [(0, 0), (0.1, 1), (0.8, 0)]))
    put(buf, sh, 0.05, 0.12)
    return fade(fit(reverb(buf, 0.84, 0.35, 0.35, tail=0.0), d), 0.002, 0.2)


# ---------------------------------------------------------------------------
# Spells: one cast and one impact per school
# ---------------------------------------------------------------------------

def sfx_cast_pyro():
    d = 0.5
    buf = zeros(N(d))
    put(buf, swoosh(d, (300, 1800), 0.9, [(0, 0), (0.17, 1), (0.3, 0.6), (d, 0)]), 0.0, 0.7)
    roar = mul(lp1(white(d), 380), env_pts(d, [(0, 0), (0.12, 1), (d, 0)]))
    put(buf, norm(roar), 0.0, 0.8)
    put(buf, thump(55, 95, 0.35, 0.12, st=0.12, attack=0.03), 0.0, 0.5)
    cr = svf(grain_cloud(0.45, 30, (0.0015, 0.004), 0.8), 2600, 1.3, "bp")
    put(buf, norm(cr), 0.05, 0.35)
    put(buf, arcane([nf("E3"), nf("B3"), nf("E4")], d, tau=0.25, trem=11, attack=0.05), 0.0, 0.18)
    return fade(svf(fit(buf, d), 6500, 0.7), 0.003, 0.08)


def sfx_impact_pyro():
    d = 0.8
    buf = zeros(N(d))
    body = svf(white(d), lambda t: 300 + 3200 * math.exp(-t / 0.08), 0.8)
    put(buf, norm(mul(body, decay(d, 0.22, 0.003))), 0.0, 1.0)
    put(buf, thump(120, 40, d, 0.2, st=0.05), 0.0, 0.9)
    cr = svf(grain_cloud(0.75, 40, (0.0015, 0.005), 1.4, 0.3, start=0.03), 2400, 1.2, "bp")
    put(buf, norm(cr), 0.0, 0.35)
    put(buf, arcane([nf("E3"), nf("B3")], 0.6, tau=0.2, trem=13, attack=0.01), 0.0, 0.15)
    buf = drive(fit(buf, d), 1.3)
    return fade(fit(reverb(buf, 0.8, 0.45, 0.25, tail=0.0), d), 0.001, 0.2)


def sfx_cast_hydro():
    d = 0.5
    buf = zeros(N(d))
    wob = slow_random(d, 25, 0.75, 1.25)
    fc = [(400 * (1400 / 400) ** (i / N(d))) * wob[i] for i in range(N(d))]
    surge = mul(svf(white(d), fc, 2.0, "bp"), env_pts(d, [(0, 0), (0.2, 1), (0.35, 0.7), (d, 0)]))
    put(buf, norm(surge), 0.0, 0.7)
    lo = mul(svf(white(d), 500, 0.8), env_pts(d, [(0, 0), (0.15, 1), (d, 0)]))
    put(buf, norm(lo), 0.0, 0.4)
    for k in range(9):
        put(buf, bubble(300 + 90 * k + rng.uniform(-30, 30), rng.uniform(0.03, 0.06), 1.0),
            0.03 + 0.045 * k, rng.uniform(0.15, 0.3))
    sh = arcane([nf("C5"), nf("G5"), nf("D6")], d, tau=0.3, trem=7, attack=0.08)
    put(buf, sh, 0.0, 0.2)
    return fade(svf(fit(buf, d), 6500, 0.7), 0.003, 0.08)


def sfx_impact_hydro():
    d = 0.75
    buf = zeros(N(d))
    imp = mul(svf(white(d), lambda t: 700 + 4300 * math.exp(-t / 0.05), 0.7), decay(d, 0.14, 0.003))
    put(buf, norm(imp), 0.0, 0.9)
    put(buf, thump(100, 50, d, 0.08), 0.0, 0.7)
    for _ in range(12):
        put(buf, bubble(rng.uniform(300, 900), rng.uniform(0.02, 0.06), 0.8),
            rng.uniform(0.04, 0.5), rng.uniform(0.1, 0.3))
    slosh = mul(svf(white(d), [600 * v for v in slow_random(d, 12, 0.7, 1.4)], 2.0, "bp"),
                env_pts(d, [(0, 0), (0.08, 1), (d, 0)]))
    put(buf, norm(slosh), 0.0, 0.35)
    put(buf, arcane([nf("C5"), nf("G5")], 0.6, tau=0.25, trem=7), 0.0, 0.12)
    return fade(fit(reverb(buf, 0.78, 0.4, 0.2, tail=0.0), d), 0.001, 0.2)


def sfx_cast_terra():
    d = 0.5
    buf = zeros(N(d))
    imp = stick_slip(0.4, lambda t: 60 + 200 * t, 0.35)
    grind = norm(add(svf(imp, 300, 4, "bp"), scale(svf(imp, 620, 4, "bp"), 0.6)))
    grind = mul(grind, env_pts(0.4, [(0, 0), (0.3, 1), (0.4, 0.3)]))
    put(buf, grind, 0.0, 0.7)
    rum = mul(lp1(brown(0.45), 150), env_pts(0.45, [(0, 0), (0.35, 1), (0.45, 0)]))
    put(buf, norm(rum), 0.0, 0.6)
    put(buf, thump(95, 45, 0.12, 0.05, st=0.02), 0.37, 0.9)
    put(buf, noise_burst(0.08, 0.02, 700), 0.37, 0.4)
    put(buf, arcane([nf("A2"), nf("E3")], d, tau=0.3, trem=6, attack=0.1), 0.0, 0.2)
    return fade(svf(fit(buf, d), 5000, 0.7), 0.003, 0.03)


def sfx_impact_terra():
    d = 0.85
    buf = zeros(N(d))
    put(buf, thump(80, 35, d, 0.22, st=0.04), 0.0, 1.0)
    put(buf, noise_burst(0.06, 0.012, 1800, 1.0, "bp"), 0.0, 0.5)
    deb = svf(grain_cloud(0.7, 26, (0.003, 0.012), 1.4, 0.25, start=0.04), 2500, 0.7)
    put(buf, norm(deb), 0.0, 0.45)
    rum = mul(lp1(brown(d), 120), env_pts(d, [(0, 0), (0.04, 1), (d, 0)]))
    put(buf, norm(rum), 0.0, 0.6)
    put(buf, arcane([nf("A2"), nf("E3")], 0.6, tau=0.25, trem=6), 0.0, 0.12)
    buf = drive(fit(buf, d), 1.4)
    return fade(fit(reverb(buf, 0.8, 0.5, 0.2, tail=0.0), d), 0.001, 0.2)


def sfx_cast_cryo():
    d = 0.45
    buf = zeros(N(d))
    sh = add(*[osc("sine", (f, f * 1.06), 0.4, phase=rng.random()) for f in (2093, 2637, 3136)])
    am = slow_random(0.4, 12, 0.3, 1.0)
    sh = mul(mul(sh, am), env_pts(0.4, [(0, 0), (0.3, 1), (0.4, 0.2)]))
    put(buf, norm(sh), 0.0, 0.4)
    cr = svf(grain_cloud(0.38, 40, (0.001, 0.003), dist_pow=0.6), 4500, 2.0, "bp")
    put(buf, norm(cr), 0.0, 0.4)
    put(buf, noise_burst(0.04, 0.006, 3000, 0.7, "hp"), 0.35, 0.6)
    put(buf, norm(partials(3951, [1, 1.5], [1, 0.3], [0.04, 0.02], 0.1)), 0.35, 0.35)
    air = mul(svf(white(d), 5000, 0.7, "hp"), env_pts(d, [(0, 0), (0.3, 1), (d, 0)]))
    put(buf, norm(air), 0.0, 0.08)
    return fade(fit(reverb(buf, 0.8, 0.3, 0.25, tail=0.0), d), 0.002, 0.06)


def sfx_impact_cryo():
    d = 0.8
    buf = zeros(N(d))
    put(buf, noise_burst(0.08, 0.012, 2000, 0.7, "hp"), 0.0, 0.8)
    put(buf, thump(300, 150, 0.15, 0.03), 0.0, 0.5)
    put(buf, sparkle(0.7, 25, (2500, 7000), (0.02, 0.08), 1.6, amp_tau=0.25), 0.01, 0.6)
    frost = mul(svf(white(d), 5000, 0.7, "hp"), env_pts(d, [(0, 0), (0.03, 1), (d, 0)]))
    put(buf, norm(frost), 0.0, 0.15)
    put(buf, arcane([nf("E6"), nf("B6")], 0.6, tau=0.2, trem=15), 0.0, 0.12)
    return fade(svf(fit(reverb(buf, 0.84, 0.25, 0.3, tail=0.0), d), 8500, 0.7), 0.001, 0.2)


def sfx_cast_aero():
    d = 0.55
    fc = lambda t: (500 + 1700 * t / d) * (1.0 + 0.35 * math.sin(TAU * 6.0 * t))
    buf = zeros(N(d))
    put(buf, swoosh(d, fc, 2.5, [(0, 0), (0.25, 1), (0.4, 0.8), (d, 0)]), 0.0, 0.7)
    body = mul(svf(white(d), 900), env_pts(d, [(0, 0), (0.2, 1), (d, 0)]))
    put(buf, norm(body), 0.0, 0.35)
    wh = mul(osc("sine", lambda t: 1500 + 400 * math.sin(TAU * 6.0 * t), d, phase=0.0),
             env_pts(d, [(0, 0), (0.25, 1), (d, 0)]))
    put(buf, norm(wh), 0.0, 0.06)
    return fade(fit(buf, d), 0.003, 0.06)


def sfx_impact_aero():
    d = 0.65
    buf = zeros(N(d))
    put(buf, swoosh(d, (2600, 450), 1.2, [(0, 0), (0.03, 1), (0.2, 0.5), (d, 0)]), 0.0, 0.8)
    put(buf, thump(75, 40, 0.3, 0.07, st=0.03, attack=0.006), 0.0, 0.5)
    tail = mul(svf(white(d), 3000, 0.7, "hp"), env_pts(d, [(0, 0), (0.05, 1), (d, 0)]))
    put(buf, norm(tail), 0.0, 0.15)
    put(buf, arcane([nf("D6"), nf("A6")], 0.5, tau=0.15, trem=9), 0.0, 0.06)
    return fade(fit(reverb(buf, 0.8, 0.3, 0.25, tail=0.0), d), 0.002, 0.15)


def _flicker_saw(d, lo, hi, seg=0.008):
    nseg = N(seg)
    fl = []
    while len(fl) < N(d):
        fl += [rng.uniform(lo, hi)] * nseg
    return osc("saw", fl[:N(d)], d)


def sfx_cast_fulm():
    d = 0.35
    buf = zeros(N(d))
    buzz = mul(svf(_flicker_saw(d, 90, 420), 1500, 1.0, "bp"), decay(d, 0.12, 0.004))
    put(buf, norm(buzz), 0.0, 0.5)
    zap = mul(osc("sine", lambda t: 300 + 2400 * math.sin(math.pi * min(1.0, t / 0.12)), 0.12),
              env_pts(0.12, [(0, 0), (0.01, 1), (0.12, 0)]))
    put(buf, norm(zap), 0.0, 0.35)
    gate = []
    while len(gate) < N(d):
        gate += [1.0 if rng.random() < 0.45 else 0.0] * N(0.003)
    cr = mul(mul(svf(white(d), 3000, 0.7, "hp"), lp1(gate[:N(d)], 800)), decay(d, 0.1))
    put(buf, norm(cr), 0.0, 0.35)
    put(buf, arcane([nf("F5"), nf("B5")], d, tau=0.12, trem=37), 0.0, 0.15)
    return fade(svf(fit(buf, d), 6000, 0.7), 0.002, 0.05)


def sfx_impact_fulm():
    d = 0.9
    buf = zeros(N(d))
    put(buf, norm(mul(white(0.006), decay(0.006, 0.0015))), 0.0, 0.8)
    put(buf, noise_burst(0.15, 0.018, 1500, 0.7, "hp"), 0.0, 0.9)
    rum = mul(lp1(brown(d), 200), env_pts(d, [(0, 0), (0.02, 1), (0.25, 0.6), (d, 0)]))
    put(buf, norm(rum), 0.0, 0.8)
    put(buf, thump(110, 45, 0.6, 0.15), 0.0, 0.6)
    buzz = mul(svf(_flicker_saw(0.5, 80, 300), 1200, 1.0, "bp"), decay(0.5, 0.12, 0.002))
    put(buf, norm(buzz), 0.0, 0.25)
    cr = svf(grain_cloud(0.6, 20, (0.001, 0.003), 1.6, 0.2), 3500, 1.2, "bp")
    put(buf, norm(cr), 0.02, 0.25)
    buf = drive(fit(buf, d), 1.3)
    return fade(svf(fit(reverb(buf, 0.85, 0.45, 0.3, tail=0.0), d), 7000, 0.7), 0.001, 0.25)


def sfx_cast_alch():
    d = 0.5
    buf = zeros(N(d))
    put(buf, norm(partials(2900, [1, 2.4, 3.9], [1, 0.4, 0.2], [0.08, 0.05, 0.03], 0.25)), 0.0, 0.5)
    put(buf, noise_burst(0.01, 0.002, 5000, 1.0, "bp"), 0.0, 0.3)
    for k in range(10):
        put(buf, bubble(rng.uniform(600, 1500), rng.uniform(0.015, 0.035), 0.9),
            0.05 + 0.04 * k + rng.uniform(0, 0.02), rng.uniform(0.15, 0.3))
    gr = [v * v for v in slow_random(d, 60)]
    fizz = mul(mul(svf(white(d), 3500, 0.7, "hp"), gr), env_pts(d, [(0, 0), (0.3, 1), (d, 0)]))
    put(buf, norm(fizz), 0.0, 0.3)
    fm = osc("sine", lambda t: nf("F#5") * (1 + 0.02 * math.sin(TAU * 17 * t)), d)
    put(buf, mul(norm(fm), env_pts(d, [(0, 0), (0.15, 1), (d, 0)])), 0.0, 0.08)
    return fade(svf(fit(buf, d), 7500, 0.7), 0.002, 0.06)


def sfx_impact_alch():
    d = 0.85
    buf = zeros(N(d))
    put(buf, noise_burst(0.05, 0.01, 4000, 0.8, "bp"), 0.0, 0.6)
    put(buf, sparkle(0.4, 14, (3000, 7000), (0.015, 0.05), 1.5, amp_tau=0.1), 0.0, 0.55)
    put(buf, thump(160, 80, 0.2, 0.04), 0.0, 0.5)
    gr = [v * v * 2 for v in slow_random(0.75, 70)]
    fizz = mul(mul(svf(white(0.75), 3000, 0.7, "hp"), gr), env_pts(0.75, [(0, 0), (0.1, 1), (0.75, 0)]))
    put(buf, norm(fizz), 0.05, 0.35)
    for _ in range(25):
        put(buf, bubble(rng.uniform(1200, 3500), rng.uniform(0.008, 0.02), 0.6),
            rng.uniform(0.05, 0.7), rng.uniform(0.05, 0.15))
    return fade(svf(fit(reverb(buf, 0.78, 0.35, 0.2, tail=0.0), d), 7500, 0.7), 0.001, 0.2)


def sfx_cast_myco():
    d = 0.4
    buf = zeros(N(d))
    sq = mul(svf(white(0.18), (300, 1200), 6.0, "bp"), env_pts(0.18, [(0, 0), (0.03, 1), (0.18, 0)]))
    put(buf, norm(sq), 0.0, 0.6)
    puff = mul(svf(white(0.35), 800), env_pts(0.35, [(0, 0), (0.05, 1), (0.35, 0)]))
    put(buf, norm(puff), 0.04, 0.6)
    put(buf, norm(mul(osc("sine", (150, 80), 0.1), decay(0.1, 0.03, 0.003))), 0.03, 0.5)
    put(buf, arcane([nf("D4"), nf("A4")], 0.35, tau=0.15, trem=5, attack=0.03), 0.0, 0.12)
    return fade(svf(fit(buf, d), 4000, 0.7), 0.002, 0.05)


def sfx_impact_myco():
    d = 0.6
    buf = zeros(N(d))
    put(buf, thump(90, 50, 0.25, 0.05), 0.0, 0.8)
    sq = mul(svf(white(0.2), (1500, 400), 5.0, "bp"), env_pts(0.2, [(0, 0), (0.01, 1), (0.2, 0)]))
    put(buf, norm(sq), 0.0, 0.6)
    puff = mul(svf(white(0.55), 1500), env_pts(0.55, [(0, 0), (0.06, 1), (0.55, 0)]))
    put(buf, norm(puff), 0.02, 0.45)
    for _ in range(4):
        put(buf, bubble(rng.uniform(150, 350), rng.uniform(0.04, 0.08), 0.6),
            rng.uniform(0.02, 0.3), rng.uniform(0.2, 0.4))
    return fade(svf(fit(reverb(buf, 0.75, 0.5, 0.18, tail=0.0), d), 4000, 0.7), 0.001, 0.15)


# --- Spell loops --------------------------------------------------------------

def sfx_tidecall_loop():
    L, P, X = 1.0, 0.1, 0.3
    T = P + L + 0.1
    sh = add(osc("sine", q(nf("C6"), L), T), scale(osc("sine", q(nf("G6"), L), T), 0.6))
    sh = mul(sh, [0.6 + 0.4 * math.sin(TAU * q(7, L) * i / SR) for i in range(N(T))])
    TN = L + X
    n = white(TN)
    flow = add(svf(n, 900, 1.0, "bp"), scale(svf(n, 2600, 1.2, "bp"), 0.6))
    flow = mul(flow, slow_random(TN, 12, 0.6, 1.0))
    ev = zeros(N(L))
    for k in range(14):
        put(ev, bubble(rng.uniform(350, 1100), rng.uniform(0.02, 0.05), 0.8),
            k * L / 14 + rng.uniform(0, 0.03), rng.uniform(0.3, 0.8))
    return assemble_loop(L, periodic=rms_to(sh, -34), noise=rms_to(flow, -18),
                         events=scale(norm(ev), 0.25), P_s=P, XL_s=0.1, XN_s=X)


def sfx_levitate_loop():
    L, P = 1.5, 0.1
    T = P + L + 0.2
    w = TAU * q(2 / 1.5, L)
    tone = add(osc("sine", q(220, L), T), scale(osc("sine", q(330, L), T), 0.6),
               scale(osc("sine", q(440.67, L), T), 0.35), scale(osc("sine", q(660, L), T), 0.15))
    tone = mul(tone, [0.75 + 0.25 * math.sin(w * i / SR) for i in range(N(T))])
    TN = L + 0.5
    air = mul(svf(white(TN), 1800, 1.5, "bp"), slow_random(TN, 2.0, 0.5, 1.0))
    breath = svf(white(TN), 4000, 0.7, "hp")
    noise = add(rms_to(air, -24), rms_to(breath, -36))
    return assemble_loop(L, periodic=rms_to(tone, -18), noise=noise, P_s=P, XL_s=0.15, XN_s=0.5)


# --- Spell moments -------------------------------------------------------------

def sfx_fusion_cast():
    d = 1.2
    buf = zeros(N(d))
    put(buf, rev_swell(0.25, 2500), 0.0, 0.35)
    h = 0.25
    put(buf, thump(110, 35, 0.9, 0.3, st=0.05), h, 1.0)
    blast = svf(white(0.9), lambda t: 400 + 4500 * math.exp(-t / 0.07), 0.8)
    put(buf, norm(mul(blast, decay(0.9, 0.25, 0.002))), h, 0.8)
    put(buf, choir([nf("A3"), nf("E4"), nf("A4"), nf("C5")], 0.9, "a", voices=2,
                   env=[(0, 0), (0.02, 1), (0.25, 0.5), (0.9, 0)]), h, 0.5)
    put(buf, fm_bell(nf("A5"), 0.9, index=1.5, tau=0.35), h, 0.25)
    put(buf, fm_bell(nf("E6"), 0.9, index=1.2, tau=0.3), h + 0.05, 0.2)
    put(buf, sparkle(0.9, 20, (2500, 6500), (0.03, 0.12), 1.5, amp_tau=0.4), h, 0.3)
    buf = drive(fit(buf, d), 1.2)
    return fade(fit(reverb(buf, 0.86, 0.4, 0.35, tail=0.0), d), 0.002, 0.25)


def sfx_fusion_awaken():
    d = 2.5
    buf = zeros(N(d))
    put(buf, rev_swell(0.6, 2500), 0.0, 0.3)
    sw = choir([nf("D3"), nf("A3"), nf("D4")], 0.6, "o", env=[(0, 0), (0.6, 1)])
    put(buf, sw, 0.0, 0.4)
    h = 0.6
    put(buf, choir([nf("D3"), nf("A3"), nf("D4"), nf("F#4"), nf("A4")], 1.9, "a",
                   env=[(0, 0), (0.03, 1), (0.4, 0.75), (1.9, 0)]), h, 0.9)
    put(buf, strings([nf("D2"), nf("A2")], 1.9, [(0, 0), (0.03, 1), (1.9, 0)], fc=900), h, 0.4)
    put(buf, frame_drum(55, 1.6, 0.45), h, 1.0)
    put(buf, bell(nf("D5"), 1.9, tau=0.8), h, 0.3)
    put(buf, bell(nf("A5"), 1.8, tau=0.7), h + 0.12, 0.2)
    put(buf, sparkle(1.5, 14, (2500, 6000), (0.05, 0.2), 1.6, amp_tau=0.6), h, 0.15)
    return fade(fit(reverb(fit(buf, d), 0.88, 0.4, 0.4, tail=0.0), d), 0.003, 0.4)


def sfx_attune():
    d = 2.1
    buf = zeros(N(d))
    put(buf, choir([nf("D3"), nf("A3"), nf("D4")], 2.0, "o", voices=3,
                   env=[(0, 0), (0.45, 1), (1.2, 0.8), (2.0, 0)]), 0.0, 0.8)
    put(buf, bell(nf("D3"), 2.0, tau=1.0, bright=0.6), 0.0, 0.6)
    put(buf, mul(shimmer([nf("A5"), nf("D6"), nf("E6")], 1.6), env_pts(1.6, [(0, 0), (0.6, 1), (1.6, 0)])),
        0.3, 0.06)
    return fade(fit(reverb(buf, 0.88, 0.4, 0.4, tail=0.0), d), 0.01, 0.35)


def sfx_scroll_learned():
    d = 2.1
    buf = zeros(N(d))
    for i, nm in enumerate(("D4", "F#4", "A4", "D5", "E5", "A5")):
        put(buf, harp(nf(nm), 1.5, 0.7), 0.065 * i, 0.4)
    put(buf, choir([nf("D4"), nf("A4"), nf("F#5")], 1.7, "a", voices=2,
                   env=[(0, 0), (0.25, 1), (0.8, 0.7), (1.7, 0)]), 0.35, 0.55)
    put(buf, bell(nf("D6"), 1.5, tau=0.6), 0.4, 0.25)
    put(buf, bell(nf("A6"), 1.3, tau=0.5), 0.55, 0.15)
    put(buf, sparkle(1.2, 12, (3000, 6500), (0.04, 0.15), 1.3, amp_tau=0.5), 0.35, 0.15)
    return fade(fit(reverb(buf, 0.86, 0.4, 0.35, tail=0.0), d), 0.002, 0.35)


def sfx_scroll_shatter():
    d = 1.0
    buf = zeros(N(d))
    td = 0.32
    imp = stick_slip(td, lambda t: 250 + 900 * t / td, 0.6)
    tear = mul(white(td), [min(1.0, v) for v in lp1([abs(x) * 6 for x in imp], 250)])
    tear = add(svf(tear, 2500, 1.2, "bp"), scale(svf(tear, 5000, 0.7, "hp"), 0.5))
    tear = mul(tear, env_pts(td, [(0, 0), (0.05, 1), (0.26, 1), (td, 0)]))
    put(buf, norm(tear), 0.0, 0.7)
    put(buf, norm(partials(3136, [1, 2.76], [1, 0.3], [0.12, 0.05], 0.3)), 0.28, 0.4)
    put(buf, sparkle(0.65, 18, (2500, 6000), (0.03, 0.12), 1.4, amp_tau=0.25), 0.28, 0.55)
    return fade(fit(reverb(buf, 0.84, 0.3, 0.3, tail=0.0), d), 0.002, 0.2)


def sfx_blink():
    d = 0.55
    buf = zeros(N(d))
    w = 0.2
    sw = mul(svf(white(w), (600, 4000), 1.4, "bp"), [(i / N(w)) ** 2 for i in range(N(w))])
    put(buf, fade(norm(sw), 0.001, 0.005), 0.0, 0.6)
    pop = mul(osc("sine", (300, 1200), 0.05), decay(0.05, 0.015, 0.001))
    put(buf, norm(pop), w, 0.7)
    put(buf, thump(160, 80, 0.12, 0.03), w, 0.4)
    put(buf, arcane([nf("G6"), nf("D7")], 0.3, tau=0.1, trem=19), w, 0.2)
    return fade(fit(reverb(buf, 0.8, 0.3, 0.25, tail=0.0), d), 0.002, 0.1)


# ---------------------------------------------------------------------------
# Lore
# ---------------------------------------------------------------------------

def paper_rustle(d, count=30):
    cr = svf(grain_cloud(d, count, (0.002, 0.008), 1.0), 3200, 1.0, "bp")
    sw = mul(svf(white(d), 1300, 0.9, "bp"), slow_random(d, 9, 0.0, 1.0))
    return norm(add(norm(cr), scale(norm(sw), 0.6)))


def sfx_journal_open():
    d = 1.5
    buf = zeros(N(d))
    put(buf, mul(paper_rustle(0.4, 28), env_pts(0.4, [(0, 0), (0.05, 1), (0.4, 0)])), 0.0, 0.8)
    flip = mul(svf(white(0.18), (900, 2800), 1.0, "bp"), env_pts(0.18, [(0, 0), (0.12, 1), (0.18, 0)]))
    put(buf, norm(flip), 0.25, 0.5)
    w = svf(whisper(0.95, rate=6.5, sib=0.4), 2600, 0.7)
    put(buf, w, 0.45, 0.22)
    return fade(fit(reverb(buf, 0.85, 0.4, 0.35, tail=0.0), d), 0.002, 0.3)


def sfx_remains_search():
    d = 1.2
    buf = zeros(N(d))
    put(buf, bone_rattle(1.0, 16, 1.2, None, (700, 2200)), 0.0, 0.7)
    cloth = mul(svf(white(1.1), 1800), slow_random(1.1, 12, 0.0, 1.0))
    cloth = mul(cloth, env_pts(1.1, [(0, 0), (0.15, 1), (0.8, 0.6), (1.1, 0)]))
    put(buf, norm(cloth), 0.05, 0.4)
    put(buf, norm(partials(410, [1, 2.7], [1, 0.3], [0.04, 0.02], 0.15)), 0.32, 0.5)
    put(buf, norm(partials(360, [1, 2.7], [1, 0.3], [0.035, 0.02], 0.15)), 0.71, 0.4)
    return fade(fit(reverb(buf, 0.8, 0.45, 0.2, tail=0.0), d), 0.002, 0.2)


# ---------------------------------------------------------------------------
# Creatures
# ---------------------------------------------------------------------------

def sfx_hollowed_moan():
    d = 1.9
    wob = lambda t: (1.0 - 0.18 * min(1.0, t / 1.6)) * (1.0 + 0.012 * math.sin(TAU * 2.3 * t))
    v = choir([nf("A2")], d, ("o", "u"), voices=3, detune=0.012, vib=0.01, breath=0.25,
              glide=wob, env=[(0, 0), (0.35, 1), (1.1, 0.7), (d, 0)])
    return fade(fit(reverb(svf(v, 2500, 0.7), 0.85, 0.5, 0.35, tail=0.0), d), 0.01, 0.2)


def sfx_hollowed_cast():
    d = 0.7
    buf = zeros(N(d))
    put(buf, mul(arcane([nf("E5"), nf("A#5")], d, tau=0.35, trem=13, attack=0.15),
                 env_pts(d, [(0, 0.3), (0.3, 1), (d, 1)])), 0.0, 0.35)
    put(buf, rev_swell(0.3, 1800), 0.0, 0.35)
    put(buf, swoosh(0.4, (2200, 500), 1.2, [(0, 1), (0.4, 0)]), 0.3, 0.5)
    put(buf, choir([nf("A2")], 0.3, "a", voices=2, breath=0.3, glide=(1.0, 0.8),
                   env=[(0, 0), (0.04, 1), (0.3, 0)]), 0.28, 0.4)
    buf = drive(fit(buf, d), 1.6)
    return fade(fit(reverb(buf, 0.82, 0.4, 0.3, tail=0.0), d), 0.002, 0.12)


def sfx_wyrm_burrow():
    L, X = 2.0, 0.5
    TN = L + X
    rum = mul(lp1(brown(TN), 180), slow_random(TN, 4, 0.6, 1.0))
    grit = mul(svf(white(TN), 420, 1.5, "bp"), slow_random(TN, 6, 0.3, 1.0))
    P = 0.1
    T = P + L + 0.1
    growl = svf(osc("saw", q(44, L), T), 140, 1.2)
    ev = zeros(N(L))
    for k in range(10):
        t0 = k * L / 10 + rng.uniform(0, 0.08)
        imp = stick_slip(0.18, lambda t: rng.uniform(90, 160), 0.4)
        gr = add(svf(imp, rng.uniform(250, 450), 4, "bp"), scale(svf(imp, rng.uniform(700, 1100), 5, "bp"), 0.5))
        put(ev, mul(gr, env_pts(0.18, [(0, 0), (0.06, 1), (0.18, 0)])), t0, rng.uniform(0.5, 1.0))
        put(ev, svf(grain_cloud(0.15, 6, (0.003, 0.01)), 1500, 0.7), t0, 0.3)
    return assemble_loop(L, periodic=rms_to(growl, -24),
                         noise=add(rms_to(rum, -14), rms_to(grit, -24)),
                         events=scale(norm(ev), 0.35), P_s=P, XL_s=0.1, XN_s=X)


def sfx_wyrm_screech():
    d = 1.2
    gl = env_pts(d, [(0, 0.8), (0.18, 1.12), (0.6, 1.0), (d, 0.72)])
    v = choir([620, 641, 905], d, ("e", "i"), voices=2, detune=0.015, vib=0.02,
              breath=0.2, shift=1.25, glide=gl, env=[(0, 0), (0.05, 1), (0.5, 0.7), (d, 0)])
    growl = mul(svf(osc("saw", (95, 70), d, vib=0.08, vib_rate=27), 400, 1.2),
                env_pts(d, [(0, 0), (0.05, 1), (d, 0)]))
    buf = add(v, scale(norm(growl), 0.5))
    buf = svf(drive(buf, 1.3), 4500, 0.7)
    return fade(fit(reverb(buf, 0.85, 0.45, 0.3, tail=0.0), d), 0.002, 0.2)


def sfx_puppet_squelch():
    d = 0.45
    buf = zeros(N(d))
    fc = env_pts(0.3, [(0, 400), (0.12, 1600), (0.3, 700)])
    sq = mul(svf(white(0.3), fc, 6.0, "bp"), env_pts(0.3, [(0, 0), (0.03, 1), (0.3, 0)]))
    put(buf, norm(sq), 0.0, 0.7)
    put(buf, thump(90, 60, 0.2, 0.05), 0.0, 0.6)
    imp = stick_slip(0.2, lambda t: 45, 0.3)
    put(buf, norm(svf(imp, 520, 5, "bp")), 0.15, 0.25)
    return fade(svf(fit(buf, d), 4000, 0.7), 0.002, 0.06)


def sfx_puppet_burst():
    d = 0.9
    buf = zeros(N(d))
    put(buf, thump(100, 40, 0.5, 0.1), 0.0, 0.9)
    wet = mul(svf(white(d), lambda t: 400 + 2600 * math.exp(-t / 0.06), 0.8), decay(d, 0.15, 0.002))
    put(buf, norm(wet), 0.0, 0.8)
    for _ in range(8):
        put(buf, bubble(rng.uniform(150, 400), rng.uniform(0.04, 0.1), 0.6),
            rng.uniform(0.02, 0.45), rng.uniform(0.2, 0.45))
    puff = mul(svf(white(0.8), 3000), env_pts(0.8, [(0, 0), (0.1, 1), (0.8, 0)]))
    put(buf, norm(puff), 0.05, 0.3)
    return fade(fit(reverb(buf, 0.8, 0.5, 0.25, tail=0.0), d), 0.001, 0.2)


def sfx_wraith_hiss():
    d = 1.0
    nz = white(d)
    v = formant(nz, ("i", "e"), shift=1.3)
    v = mul(svf(v, 800, 0.7, "hp"), env_pts(d, [(0, 0), (0.08, 1), (0.5, 0.8), (d, 0)]))
    hs = mul(svf(white(d), 4500, 1.0, "bp"), env_pts(d, [(0, 0), (0.1, 1), (d, 0)]))
    cr = svf(grain_cloud(d, 30, (0.0015, 0.004)), 2500, 1.2, "bp")
    roar = mul(lp1(white(d), 300), env_pts(d, [(0, 0), (0.15, 1), (d, 0)]))
    buf = add(norm(v), scale(norm(hs), 0.35), scale(norm(cr), 0.25), scale(norm(roar), 0.4))
    return fade(fit(reverb(svf(buf, 7000, 0.7), 0.82, 0.4, 0.3, tail=0.0), d), 0.003, 0.2)


def sfx_lightseeker_flutter():
    L = 1.0
    beats = 13
    ev = zeros(N(L) + N(0.1))
    for k in range(beats):
        t0 = k * L / beats + rng.uniform(-0.004, 0.004) + 0.004
        fd = 0.045
        fl = mul(svf(white(fd), rng.uniform(1800, 3000), 1.2, "bp"), env_pts(fd, [(0, 0), (0.006, 1), (fd, 0)]))
        th = mul(svf(white(fd), 380, 1.5, "bp"), env_pts(fd, [(0, 0), (0.008, 1), (fd, 0)]))
        cr = svf(grain_cloud(fd, 3, (0.001, 0.003)), 4000, 1.0, "bp")
        put(ev, add(norm(fl), scale(norm(th), 0.5), scale(norm(cr), 0.3)), t0, rng.uniform(0.7, 1.0))
    return assemble_loop(L, events=ev)


def sfx_lightseeker_shriek():
    d = 0.8
    fr = lambda t: (1900 + 1200 * math.sin(math.pi * min(1.0, t / 0.5))) if t < 0.5 else 1900 + 700 * (1 - (t - 0.5) / 0.3)
    a = osc("tri", fr, d, vib=0.03, vib_rate=14)
    b = osc("tri", lambda t: fr(t) * 1.015, d, vib=0.03, vib_rate=12.5)
    v = add(a, b)
    v = add(svf(v, 3000, 2.0, "bp"), scale(v, 0.3))
    v = mul(norm(v), env_pts(d, [(0, 0), (0.03, 1), (0.45, 0.7), (d, 0)]))
    hs = mul(svf(white(d), 3500, 1.5, "bp"), env_pts(d, [(0, 0), (0.03, 1), (d, 0)]))
    buf = add(v, scale(norm(hs), 0.2))
    return fade(svf(fit(reverb(buf, 0.85, 0.4, 0.3, tail=0.0), d), 6000, 0.7), 0.002, 0.15)


def sfx_mimic_bite():
    d = 0.85
    buf = zeros(N(d))
    put(buf, norm(partials(620, [1, 2.1, 3.3], [1, 0.5, 0.3], [0.03, 0.02, 0.012], 0.12)), 0.0, 0.8)
    put(buf, noise_burst(0.04, 0.006, 2000, 1.0, "bp"), 0.0, 0.6)
    put(buf, thump(200, 90, 0.15, 0.03), 0.0, 0.6)
    gd = 0.7
    gsrc = add(osc("saw", 75, gd, vib=0.12, vib_rate=31), scale(white(gd), 0.3))
    growl = mul(formant(gsrc, "o", shift=0.8), env_pts(gd, [(0, 0), (0.08, 1), (0.4, 0.7), (gd, 0)]))
    put(buf, norm(growl), 0.08, 0.6)
    for _ in range(3):
        put(buf, bubble(rng.uniform(150, 300), 0.06, 0.5), rng.uniform(0.1, 0.4), 0.2)
    return fade(fit(reverb(drive(fit(buf, d), 1.3), 0.78, 0.5, 0.15, tail=0.0), d), 0.001, 0.15)


def sfx_stalker_step():
    d = 0.7
    buf = zeros(N(d))
    put(buf, thump(70, 32, 0.6, 0.15, st=0.04, attack=0.004), 0.0, 1.0)
    put(buf, noise_burst(0.15, 0.04, 300), 0.0, 0.5)
    scr = mul(svf(white(0.3), 600, 1.0, "bp"), env_pts(0.3, [(0, 0), (0.08, 1), (0.3, 0)]))
    put(buf, norm(scr), 0.1, 0.2)
    buf = svf(fit(buf, d), 500, 0.7)
    return fade(fit(reverb(buf, 0.85, 0.6, 0.3, tail=0.0), d), 0.002, 0.2)


def sfx_stalker_breath():
    L = 3.0
    ev = zeros(N(L + 0.5))
    ind = 1.2
    wet = slow_random(ind, 30, 0.5, 1.0)
    inh = add(svf(white(ind), (500, 1050), 2.5, "bp"), scale(svf(white(ind), 2500, 0.7, "hp"), 0.15))
    inh = mul(mul(inh, wet), env_pts(ind, [(0, 0), (0.8, 1), (1.05, 0.7), (ind, 0)]))
    put(ev, norm(inh), 0.05, 0.6)
    gur = mul(svf(white(ind), 260, 3.0, "bp"), [v ** 3 for v in slow_random(ind, 35, 0.0, 1.0)])
    put(ev, mul(norm(gur), env_pts(ind, [(0, 0), (0.6, 1), (ind, 0)])), 0.05, 0.35)
    exd = 1.4
    exh = mul(svf(white(exd), (700, 350), 2.0, "bp"), env_pts(exd, [(0, 0), (0.12, 1), (0.6, 0.7), (exd, 0)]))
    put(ev, norm(exh), 1.4, 0.75)
    cr = svf(grain_cloud(exd, 25, (0.002, 0.006), 1.3), 900, 1.5, "bp")
    put(ev, mul(norm(cr), env_pts(exd, [(0, 0), (0.1, 1), (exd, 0)])), 1.4, 0.3)
    groan = mul(svf(osc("saw", 55, exd, vib=0.02, vib_rate=3), 220, 1.2),
                env_pts(exd, [(0, 0), (0.3, 1), (exd, 0)]))
    put(ev, norm(groan), 1.4, 0.15)
    return assemble_loop(L, events=svf(ev, 2500, 0.7))


def sfx_stalker_appear():
    d = 2.6
    buf = zeros(N(d))
    hit = strings([nf("C3"), nf("C#3"), nf("F#3"), nf("G4")], 0.8, [(0, 0), (0.01, 1), (0.8, 0)], fc=2500)
    put(buf, reverse(reverb(hit, 0.85, 0.3, 0.6, tail=0.4)), 0.0, 0.55)
    h = 1.2
    put(buf, strings([nf("C3"), nf("C#3"), nf("F#3"), nf("G4"), nf("G#4")], 1.4,
                     [(0, 0), (0.01, 1), (0.25, 0.5), (1.4, 0)], fc=(3500, 700)), h, 0.8)
    put(buf, choir([nf("D4"), nf("D#4"), nf("A4")], 1.3, ("a", "i"), voices=2, breath=0.15,
                   env=[(0, 0), (0.02, 1), (1.3, 0)]), h, 0.5)
    put(buf, frame_drum(48, 1.3, 0.4), h, 1.0)
    put(buf, bone_rattle(0.6, 12, 1.6, 0.3), h, 0.25)
    return fade(fit(reverb(fit(buf, d), 0.88, 0.45, 0.4, tail=0.0), d), 0.003, 0.4)


# ---------------------------------------------------------------------------
# Horror
# ---------------------------------------------------------------------------

def sfx_whisper_1():
    # near: bright and dry
    w = whisper(1.8, rate=6.0, bright=1.1)
    return fade(fit(reverb(w, 0.8, 0.4, 0.2, tail=0.0), 1.8), 0.01, 0.15)


def sfx_whisper_2():
    # far: dark, muffled, in a big space
    w = svf(whisper(2.2, rate=5.0, bright=0.8), 1400, 0.7)
    return fade(fit(reverb(w, 0.9, 0.5, 0.6, tail=0.0), 2.4), 0.01, 0.3)


def sfx_whisper_3():
    # off to one side: comb-coloured, quick, with a short slap echo
    w = comb(whisper(1.3, rate=7.5, bright=1.0, sib=0.7), 0.0007, 0.8)
    w = echo(w, 0.11, 0.25, 0.3, tail=0.0, lp_fc=3000)
    return fade(fit(reverb(w, 0.82, 0.4, 0.3, tail=0.0), 1.4), 0.01, 0.15)


def sfx_whisper_4():
    # two voices overlapping, one near, one far behind
    d = 2.8
    buf = zeros(N(d))
    put(buf, whisper(1.9, rate=6.5, bright=1.0), 0.0, 0.8)
    put(buf, svf(comb(whisper(1.9, rate=5.5, bright=0.9, vowels="aou"), 0.0011, 0.7), 1800, 0.7),
        0.75, 0.55)
    return fade(fit(reverb(buf, 0.88, 0.45, 0.45, tail=0.0), d), 0.01, 0.3)


def sfx_distant_steps():
    d = 2.2
    buf = zeros(N(d))
    for i, t0 in enumerate((0.1, 0.62, 1.15, 1.68)):
        g = (0.55, 0.7, 0.85, 1.0)[i]
        put(buf, thump(90 * rng.uniform(0.95, 1.05), 45, 0.2, 0.05), t0, g)
        gr = svf(grain_cloud(0.1, 7, (0.003, 0.008), 1.5, 0.04), 1200, 0.9, "bp")
        put(buf, norm(gr), t0 + 0.005, 0.35 * g)
    buf = svf(fit(buf, d), 700, 0.7)
    return fade(fit(reverb(buf, 0.9, 0.55, 0.65, tail=0.0), d), 0.002, 0.3)


def sfx_sting_1():
    d = 1.6
    buf = zeros(N(d))
    put(buf, strings([nf("C3"), nf("C#4"), nf("G4"), nf("G#4")], 1.5,
                     [(0, 0), (0.008, 1), (0.2, 0.55), (1.5, 0)], fc=(3200, 800), vib=0.012, vib_rate=6.5),
        0.0, 0.9)
    put(buf, thump(90, 40, 0.6, 0.15), 0.0, 0.5)
    return fade(fit(reverb(buf, 0.86, 0.45, 0.35, tail=0.0), d), 0.002, 0.3)


def sfx_sting_2():
    d = 1.5
    sw = 1.15
    hit = add(scale(bell(nf("F2"), 1.0, tau=0.5, bright=0.7), 0.6),
              strings([nf("F3"), nf("B3"), nf("E4")], 1.0, [(0, 0), (0.01, 1), (1.0, 0)], fc=2000))
    rs = reverse(reverb(fit(hit, 0.6), 0.88, 0.3, 0.8, tail=0.6))
    rs = fit(rs, sw)
    buf = zeros(N(d))
    put(buf, fade(norm(rs), 0.05, 0.008), 0.0, 0.8)
    put(buf, frame_drum(45, 0.35, 0.12), sw, 1.0)
    return fade(fit(reverb(buf, 0.75, 0.5, 0.15, tail=0.0), d), 0.002, 0.15)


def sfx_sting_3():
    d = 1.8
    buf = zeros(N(d))
    put(buf, choir([nf("A4"), nf("A#4"), nf("E5")], 1.6, ("a", "i"), voices=2, breath=0.2,
                   glide=(1.0, 1.25), env=[(0, 0), (0.12, 1), (1.0, 0.7), (1.6, 0)]), 0.0, 0.8)
    put(buf, frame_drum(55, 1.0, 0.3), 0.0, 0.7)
    put(buf, bone_rattle(0.8, 14, 1.5, 0.4), 0.05, 0.3)
    return fade(fit(reverb(buf, 0.86, 0.45, 0.4, tail=0.0), d), 0.002, 0.3)


# ---------------------------------------------------------------------------
# UI and stings
# ---------------------------------------------------------------------------

def sfx_ui_toggle():
    d = 0.14
    buf = zeros(N(d))
    put(buf, noise_burst(0.02, 0.002, 2500, 1.2, "bp"), 0.0, 0.4)
    put(buf, thump(420, 250, 0.06, 0.012), 0.0, 0.5)
    put(buf, norm(partials(1568, [1, 2.0, 2.76], [1, 0.3, 0.15], [0.03, 0.02, 0.012], 0.12)), 0.004, 0.45)
    return fit(buf, d)


def sfx_achievement():
    d = 2.3
    buf = zeros(N(d))
    for i, nm in enumerate(("D5", "A5", "D6", "E6")):
        put(buf, bell(nf(nm), 1.8, tau=0.7 + 0.1 * i), 0.1 * i, 0.4)
    put(buf, choir([nf("D4"), nf("A4"), nf("F#5")], 2.0, "a", voices=2,
                   env=[(0, 0), (0.5, 1), (1.1, 0.6), (2.0, 0)]), 0.2, 0.4)
    return fade(fit(reverb(buf, 0.86, 0.35, 0.4, tail=0.0), d), 0.001, 0.3)


def sfx_layer_sting():
    d = 2.7
    buf = zeros(N(d))
    put(buf, choir([nf("A2"), nf("E3"), nf("A3"), nf("C4")], 1.4, ("o", "a"),
                   env=[(0, 0), (1.25, 1), (1.4, 0.3)]), 0.0, 0.7)
    put(buf, rev_swell(1.25, 2500), 0.0, 0.12)
    h = 1.25
    put(buf, frame_drum(52, 1.4, 0.45), h, 1.0)
    put(buf, choir([nf("A2"), nf("E3"), nf("A3"), nf("C4"), nf("E4")], 1.45, "a",
                   env=[(0, 0), (0.03, 1), (0.4, 0.6), (1.45, 0)]), h, 0.75)
    put(buf, strings([nf("A1"), nf("E2")], 1.45, [(0, 0), (0.02, 1), (1.45, 0)], fc=500), h, 0.5)
    return fade(fit(reverb(fit(buf, d), 0.88, 0.45, 0.4, tail=0.0), d), 0.005, 0.35)


def sfx_game_over():
    d = 3.2
    buf = zeros(N(d))
    gl = lambda t: 1.0 if t < 0.6 else 2 ** (-3.0 * min(1.0, (t - 0.6) / 1.9) / 12)
    put(buf, choir([nf("A3"), nf("C4"), nf("E4"), nf("A4")], 3.0, ("o", "u"), glide=gl,
                   env=[(0, 0), (0.4, 1), (1.6, 0.8), (3.0, 0)]), 0.0, 0.8)
    put(buf, choir([nf("A2")], 3.0, "u", glide=gl, env=[(0, 0), (0.5, 1), (3.0, 0)]), 0.0, 0.4)
    put(buf, bell(nf("A2"), 3.0, tau=1.2, bright=0.5), 0.0, 0.5)
    put(buf, frame_drum(50, 1.5, 0.45), 0.0, 0.6)
    return fade(fit(reverb(buf, 0.88, 0.45, 0.4, tail=0.0), d), 0.005, 0.5)


def sfx_death():
    d = 1.8
    buf = zeros(N(d))
    put(buf, thump(90, 40, 0.5, 0.15), 0.0, 0.8)
    put(buf, noise_burst(0.1, 0.03, 900), 0.0, 0.4)
    put(buf, choir([nf("E3"), nf("A#3")], 1.6, ("a", "u"), voices=3, glide=(1.0, 0.5),
                   env=[(0, 0), (0.05, 1), (0.6, 0.6), (1.6, 0)]), 0.02, 0.7)
    exh = mul(svf(white(0.9), 800, 2.0, "bp"), env_pts(0.9, [(0, 0), (0.1, 1), (0.9, 0)]))
    put(buf, norm(exh), 0.1, 0.25)
    return fade(fit(reverb(buf, 0.86, 0.45, 0.35, tail=0.0), d), 0.002, 0.25)


def sfx_victory():
    """~12 s completed rite: a choir rising through Am - F - G and resolving
    to A major, over frame drums, harp and pealing bells."""
    d = 12.0
    buf = zeros(N(d))
    prog = [(0.0, 2.4, ["A2", "E3", "A3", "C4"], "o"),
            (2.4, 2.4, ["F2", "C3", "F3", "A3", "C4"], "o"),
            (4.8, 1.8, ["G2", "D3", "G3", "B3", "D4"], "a")]
    for t0, ln, ch, vw in prog:
        put(buf, choir([nf(c) for c in ch], ln + 1.0, vw,
                       env=[(0, 0), (0.6, 1), (ln, 0.9), (ln + 1.0, 0)]), t0, 0.55 + 0.1 * t0 / 4.8)
        put(buf, strings([nf(ch[0]) / 2], ln + 1.0, [(0, 0), (0.4, 1), (ln, 0.9), (ln + 1.0, 0)], fc=350),
            t0, 0.35)
    # sus-to-major lift into the arrival
    put(buf, choir([nf("A3"), nf("D4"), nf("E4")], 0.9, "a", env=[(0, 0), (0.6, 1), (0.9, 0)]), 6.0, 0.3)
    arr = 6.6
    put(buf, choir([nf(c) for c in ("A2", "E3", "A3", "C#4", "E4", "A4")], 5.4, "a",
                   env=[(0, 0), (0.08, 1), (2.5, 0.85), (5.4, 0)]), arr, 0.9)
    put(buf, strings([nf("A1"), nf("E2")], 5.4, [(0, 0), (0.05, 1), (5.4, 0)], fc=600), arr, 0.45)
    # drums: a slow pulse that quickens into the arrival
    for t0, g in ((0.0, 0.5), (1.2, 0.35), (2.4, 0.55), (3.6, 0.4), (4.8, 0.6), (5.4, 0.45),
                  (6.0, 0.55), (6.3, 0.6)):
        put(buf, frame_drum(62, 1.0, 0.3), t0, g)
    put(buf, frame_drum(50, 2.0, 0.6), arr, 1.0)
    put(buf, bone_rattle(1.2, 18, 1.5, 0.5), arr, 0.15)
    for k, t0 in enumerate((8.4, 9.6)):
        put(buf, frame_drum(55, 1.2, 0.35), t0, 0.4 - 0.1 * k)
    # harp: rising minor arpeggios, then A major
    for b, ch in enumerate((["A3", "C4", "E4", "A4"], ["F3", "A3", "C4", "F4"], ["G3", "B3", "D4", "G4"])):
        for k, nm in enumerate(ch + ch[1:3]):
            put(buf, harp(nf(nm), 1.4, 0.6), [0.0, 2.4, 4.8][b] + 0.3 * k, 0.16)
    for k, nm in enumerate(("A4", "C#5", "E5", "A5", "C#6", "E6")):
        put(buf, harp(nf(nm), 2.0, 0.9), arr + 0.12 + 0.11 * k, 0.2)
    # bells peal on the arrival
    for k, (nm, dt) in enumerate((("A5", 0.0), ("E5", 0.35), ("C#6", 0.7), ("A5", 1.05),
                                  ("E6", 1.4), ("A4", 2.0), ("E5", 2.6), ("A5", 3.2))):
        put(buf, bell(nf(nm), 2.5, tau=0.9), arr + dt, 0.28 - 0.015 * k)
    put(buf, sparkle(3.0, 20, (3000, 6500), (0.08, 0.3), 1.3, amp_tau=1.2), arr, 0.1)
    buf = reverb(fit(buf, d), 0.88, 0.4, 0.35, tail=0.0)
    return fade(fit(buf, d), 0.005, 1.2)


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
    for i, nm in enumerate(("E6", "G6", "B6", "D#7")):
        put(sp, fm_bell(nf(nm), 0.5, index=0.8, tau=0.18), 0.07 * i, 0.3)
    sp = add(sp, scale(mul(svf(white(0.9), 6000, 0.7, "hp"), env_pts(0.9, [(0, 0), (0.1, 1), (0.9, 0)])), 0.04))
    put(buf, reverb(sp, 0.8, 0.3, 0.4, tail=0.3), 0.5)
    return fit(buf, d)


def sfx_shrine_buy():
    d = 1.4
    buf = zeros(N(d))
    for t in (0.0, 0.05, 0.11, 0.16, 0.24):
        put(buf, norm(partials(rng.uniform(2600, 3600), [1, 2.76], [1, 0.3], [0.05, 0.02], 0.15)),
            t, rng.uniform(0.3, 0.5))
    put(buf, bell(nf("A4"), 1.1, tau=0.5), 0.3, 0.55)
    put(buf, bell(nf("E5"), 1.0, tau=0.45), 0.44, 0.4)
    put(buf, mul(shimmer([nf("A6"), nf("E7")], 0.8), env_pts(0.8, [(0, 0), (0.15, 1), (0.8, 0)])), 0.3, 0.06)
    return fade(fit(reverb(buf, 0.84, 0.35, 0.35, tail=0.0), d), 0.002, 0.25)


def lubdub(rate, base=0.25):
    """Heartbeat amplitude shape at `rate` beats/s (soft ~12 ms attacks that
    start with zero slope, so no envelope kinks)."""
    def f(t):
        ph = (t * rate) % 1.0
        p2 = ph - 0.25
        lub = (1.0 - math.exp(-(ph / 0.012) ** 2)) * math.exp(-ph / 0.08)
        dub = (1.0 - math.exp(-(p2 / 0.012) ** 2)) * math.exp(-p2 / 0.07) if p2 > 0 else 0.0
        return base + 1.6 * lub + 1.0 * dub
    return f


def sfx_core_hum():
    L, P = 4.0, 1.5  # preroll puts the seam between beats
    T = P + L + 0.2
    hum = add(osc("sine", 55, T), scale(osc("sine", 55.25, T), 0.6),
              scale(osc("sine", 110, T), 0.35), scale(osc("sine", 165, T), 0.12))
    saw = svf(osc("saw", 55, T), lambda t: 250 + 150 * math.sin(TAU * 0.5 * t), 1.5)
    s = add(hum, scale(saw, 0.5))
    s = drive(mul(s, _curve(lubdub(1.0, 0.35), N(T))), 1.5)
    X = 1.0
    ch = choir([nf("A2"), nf("E3"), nf("A3")], L + X, "u", voices=2, breath=0.1)
    ch = svf(ch, 900, 0.7)
    return assemble_loop(L, periodic=rms_to(s, -12), noise=rms_to(ch, -30), P_s=P, XL_s=0.1, XN_s=X)


# ---------------------------------------------------------------------------
# Ambient loops (L = 18 s). Built from three parts: an exactly periodic drone
# (frequencies quantised to the loop), crossfaded noise beds, and events
# (drips, whispers, choir swells, drums) whose tails fold round the seam.
# ---------------------------------------------------------------------------

AL = 18.0


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


def _swells(L, chords, vowel, lp, length=8.0, att=3.0, rel=4.0, voices=3, shift=1.0):
    """Overlapping choir chord swells placed through a loop (fold afterwards)."""
    buf = zeros(N(L + length))
    for t0, ch in chords:
        c = choir([nf(x) for x in ch], length, vowel, voices=voices, breath=0.08, shift=shift,
                  env=[(0, 0), (att, 1), (length - rel, 0.85), (length, 0)])
        put(buf, svf(c, lp, 0.7), t0)
    return buf


def amb_layer0():
    """The Whispering Crust: hollow drone, drips, faint whispers."""
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    tone = add(osc("sine", q(55, L), T), scale(osc("sine", q(55.33, L), T), 0.7),
               scale(osc("sine", q(82.5, L), T), 0.3), scale(osc("sine", q(110.2, L), T), 0.12),
               scale(osc("sine", q(164.9, L), T), 0.05))
    tone = mul(tone, _curve(lambda t: 0.75 + 0.25 * math.sin(TAU * q(1 / 9, L) * t), N(T)))
    TN = L + XN
    hol_fc = [240 + 260 * v for v in slow_random(TN, 0.3)]
    hollow = mul(svf(white(TN), hol_fc, 7.0, "bp"), slow_random(TN, 0.25, 0.25, 1.0))
    whistle = mul(svf(white(TN), 196, 16.0, "bp"), slow_random(TN, 0.2, 0.0, 1.0))
    noise = add(rms_to(hollow, -27), rms_to(whistle, -33))
    ev = _drips(L, (0.9, 2.8), (850, 1600), 1.0)
    ev = echo(ev, 0.31, 0.3, 0.3, tail=1.0, lp_fc=2200)
    ev = scale(norm(ev), 0.35)
    for t0, dd, rt in ((4.2, 2.0, 5.5), (12.6, 1.6, 6.5)):
        w = svf(whisper(dd, rate=rt, bright=0.9), 1600, 0.7)
        put(ev, w, t0, 0.13)
    ev = reverb(ev, 0.9, 0.35, 0.8, tail=3.0)
    return assemble_loop(L, periodic=rms_to(tone, -20), noise=noise, events=ev,
                         P_s=P, XL_s=XL, XN_s=XN)


def amb_layer1():
    """The Drowned Halls: cold watery drone, distant dripping echoes, muffled choir."""
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    drone = add(osc("sine", q(nf("B1"), L), T), scale(osc("sine", q(nf("B1") + 0.17, L), T), 0.8),
                scale(osc("tri", q(nf("F#2"), L), T), 0.25), scale(osc("sine", q(nf("B2"), L), T), 0.2),
                scale(osc("sine", q(nf("D3") + 0.11, L), T), 0.06))
    drone = mul(drone, _curve(lambda t: 0.8 + 0.2 * math.sin(TAU * q(1 / 6, L) * t + 1.0), N(T)))
    TN = L + XN
    under = mul(lp1(brown(TN), 260), slow_random(TN, 0.5, 0.5, 1.0))
    gurgle = mul(svf(white(TN), [500 * v for v in slow_random(TN, 3, 0.7, 1.4)], 3.0, "bp"),
                 [v ** 2 for v in slow_random(TN, 1.5, 0.0, 1.0)])
    noise = add(rms_to(under, -24), rms_to(gurgle, -34))
    ev = _drips(L, (0.6, 2.0), (700, 1400), 1.0)
    for _ in range(5):
        put(ev, bubble(rng.uniform(120, 220), 0.12, 0.6), rng.uniform(0.5, L - 0.5), 0.5)
    ev = echo(ev, 0.42, 0.45, 0.4, tail=2.0, lp_fc=1800)
    ev = scale(norm(ev), 0.4)
    sw = _swells(L, [(0.5, ["B2", "F#3", "D4"]), (6.5, ["G2", "D3", "B3"]), (12.5, ["E2", "B2", "G3"])],
                 "u", 700, length=9.0, att=3.5, rel=4.0)
    ev = add(ev, scale(norm(sw), 0.3))
    ev = reverb(ev, 0.9, 0.45, 0.7, tail=3.0)
    return assemble_loop(L, periodic=rms_to(drone, -21), noise=noise, events=ev,
                         P_s=P, XL_s=XL, XN_s=XN)


def amb_layer2():
    """The Fungal Abyss: wet organic pads, dissonant shimmer, creaks."""
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    n = N(T)
    pad = zeros(n)
    for nm in ("C2", "G2", "C#3", "G#3"):
        f = q(nf(nm), L)
        pad = add(pad, osc("saw", f, T, phase=rng.random()), osc("saw", f + 2.0 / L, T, phase=rng.random()))
    wah = lambda t: 420 + 300 * math.sin(TAU * q(1 / 6, L) * t) + 120 * math.sin(TAU * q(0.5, L) * t)
    pad = add(svf(pad, wah, 3.0, "bp"), scale(svf(pad, 260, 0.7), 0.6))
    sh = add(osc("sine", q(nf("F6"), L), T), osc("sine", q(nf("F#6"), L), T),
             scale(osc("sine", q(nf("C7"), L), T), 0.5))
    sh = mul(sh, _curve(lambda t: (0.5 + 0.5 * math.sin(TAU * q(1 / 4.5, L) * t)) ** 2, n))
    periodic = add(rms_to(pad, -22), rms_to(sh, -40))
    TN = L + XN
    wet = mul(svf(white(TN), 700, 3.0, "bp"), [v ** 3 for v in slow_random(TN, 9, 0.0, 1.0)])
    bed = lp1(brown(TN), 200)
    noise = add(rms_to(wet, -33), rms_to(bed, -27))
    ev = zeros(N(L + 2.0))
    for t0 in (1.7, 6.9, 11.2, 15.8):
        cd = rng.uniform(0.5, 0.9)
        imp = stick_slip(cd, lambda t, cd=cd: 25 + 30 * math.sin(math.pi * t / cd), 0.25)
        f0 = rng.uniform(280, 520)
        cr = add(svf(imp, f0, 5, "bp"), scale(svf(imp, f0 * 2.1, 6, "bp"), 0.5))
        put(ev, mul(norm(cr), adsr(cd, 0.05, 0.1, 0.8, 0.2)), t0, rng.uniform(0.4, 0.7))
    for _ in range(9):
        put(ev, bubble(rng.uniform(140, 320), rng.uniform(0.06, 0.12), 0.7), rng.uniform(0, L), 0.35)
    for t0 in (4.3, 13.4):
        puff = mul(svf(white(0.8), 1800), env_pts(0.8, [(0, 0), (0.08, 1), (0.8, 0)]))
        put(ev, norm(puff), t0, 0.2)
    sw = _swells(L, [(3.0, ["C#4", "D4", "G#4"]), (11.5, ["C4", "C#4", "G4"])], "o", 1100,
                 length=8.0, att=3.0, rel=3.5, voices=2)
    ev = add(scale(norm(ev), 0.5), scale(norm(sw), 0.18))
    ev = reverb(ev, 0.9, 0.5, 0.6, tail=3.0)
    return assemble_loop(L, periodic=periodic, noise=noise, events=ev, P_s=P, XL_s=XL, XN_s=XN)


def amb_layer3():
    """The Molten Sanctum: deep heavy drone, slow ritual drum pulse, rumbles."""
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    base = 41.2
    drone = zeros(N(T))
    for r, a in zip([1, 2.0, 2.76, 4.07, 5.4, 6.8], [1, 0.5, 0.35, 0.25, 0.15, 0.08]):
        f = q(base * r, L)
        drone = add(drone, scale(osc("sine", f, T), a), scale(osc("sine", f + 1 / L, T), a * 0.7))
    drone = add(drone, scale(svf(osc("saw", q(base, L), T), 180, 1.0), 0.8))
    drone = drive(norm(drone), 1.4)
    TN = L + XN
    bed = mul(lp1(brown(TN), 140), slow_random(TN, 0.6, 0.5, 1.0))
    grind = mul(svf(white(TN), 350, 2.0, "bp"), slow_random(TN, 1.0, 0.0, 1.0))
    noise = add(rms_to(bed, -22), rms_to(grind, -36))
    ev = zeros(N(L + 3.0))
    # ritual drum: a slow two-stroke pulse every 2.25 s, the downbeat heavier
    for k in range(8):
        t0 = k * 2.25
        put(ev, frame_drum(52, 1.4, 0.4), t0, 1.0 if k % 4 == 0 else 0.75)
        put(ev, frame_drum(60, 1.0, 0.3), t0 + 0.45, 0.45)
    ev = svf(ev, 420, 0.7)
    for t0 in (3.5, 12.0):
        r = mul(lp1(brown(4.0), 90), env_pts(4.0, [(0, 0), (1.5, 1), (4.0, 0)]))
        put(ev, norm(r), t0, 0.6)
    for t0 in (6.8, 15.1):
        put(ev, bone_rattle(0.7, 10, 1.4, 0.3, (600, 1800)), t0, 0.12)
    t = 0.3
    while t < L:
        for k in range(rng.randint(2, 6)):
            put(ev, noise_burst(0.006, 0.0015, 3500, 0.7, "hp"), t + rng.uniform(0, 0.03),
                rng.uniform(0.03, 0.08))
        t += rng.expovariate(1.5)
    ev = reverb(ev, 0.88, 0.5, 0.45, tail=3.0)
    return assemble_loop(L, periodic=rms_to(drone, -19), noise=noise, events=scale(norm(ev), 0.75),
                         P_s=P, XL_s=XL, XN_s=XN)


def shepard(dur, L, fmin=55.0, octaves=6, center=220.0, width=1.0):
    n = N(dur)
    out = [0.0] * n
    phases = [0.0] * octaves
    c = math.log2(center / fmin)
    s = math.sin
    e = math.exp
    prev = 0.0
    for i in range(n):
        pos = ((i / SR) / L) % 1.0
        if pos < prev:
            # each oscillator takes over the octave (and phase) of the one
            # below it, so the wrap is continuous
            phases = [phases[-1]] + phases[:-1]
        prev = pos
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
    """The Heart of the World: intense choir, heartbeat throb, rising tension."""
    L, P, XL, XN = AL, 2.0, 1.0, 2.0
    T = P + L + XL
    n = N(T)
    throb = lubdub(q(1.25, L))
    body = add(osc("sine", q(40, L), T), scale(osc("sine", q(80, L), T), 0.5),
               scale(osc("sine", q(120, L), T), 0.2))
    body = drive(mul(body, _curve(throb, n)), 1.8)
    tens = add(osc("sine", q(nf("D#4"), L), T), osc("sine", q(nf("E4"), L), T))
    tens = mul(tens, _curve(lambda t: 0.5 + 0.5 * math.sin(TAU * q(3 / 18, L) * t), n))
    periodic = add(rms_to(body, -16), rms_to(tens, -40))
    TN = L + XN
    shep = shepard(TN, L, 55.0, 6, 260.0, 0.9)
    rum = mul(lp1(brown(TN), 120), _curve(lambda t: 0.4 + 0.3 * throb(t), N(TN)))
    noise = add(rms_to(shep, -30), rms_to(rum, -26))
    sw = _swells(L, [(0.0, ["D3", "A3", "D#4", "A4"]), (6.0, ["C#3", "G#3", "D4", "G4"]),
                     (12.0, ["D3", "G#3", "D#4", "A#4"])], "a", 2200, length=9.0, att=4.0, rel=3.0)
    sw = add(sw, _swells(L, [(3.0, ["D5", "D#5"]), (12.5, ["C#5", "D5"])], "i", 3000,
                         length=6.0, att=3.0, rel=2.5, voices=2))
    ev = reverb(norm(sw), 0.9, 0.4, 0.5, tail=3.0)
    return assemble_loop(L, periodic=periodic, noise=noise, events=scale(norm(ev), 0.32),
                         P_s=P, XL_s=XL, XN_s=XN)


# ---------------------------------------------------------------------------
# Menu music (28 s loop: eight 3.5 s chords in A minor)
# ---------------------------------------------------------------------------

def music_menu():
    L = 28.0
    cl = 3.5
    tail = 7.0
    buf = zeros(N(L + tail))
    chords = [["A2", "E3", "A3", "C4"], ["F2", "C3", "A3", "C4"], ["D3", "A3", "D4", "F4"],
              ["E2", "B2", "G#3", "E4"], ["A2", "E3", "A3", "C4"], ["G2", "D3", "G3", "B3"],
              ["F2", "C3", "F3", "A3"], ["E2", "B2", "E3", "G#3"]]
    roots = ["A1", "F1", "D2", "E1", "A1", "G1", "F1", "E1"]
    harp_lines = [
        [(0, "A4"), (2, "E5"), (3, "C5"), (6, "B4")],
        [(0, "C5"), (3, "A4"), (5, "F5")],
        [(1, "D5"), (2, "F5"), (4, "A5"), (7, "E5")],
        [(0, "G#4"), (3, "B4"), (6, "E5")],
        [(0, "A4"), (2, "C5"), (4, "E5"), (5, "D5")],
        [(1, "B4"), (3, "D5"), (6, "G4")],
        [(0, "A4"), (2, "C5"), (3, "F5"), (5, "E5")],
        [(0, "B4"), (4, "G#4"), (6, "E4")],
    ]
    pads = zeros(N(L + tail))
    low = zeros(N(L + tail))
    harps = zeros(N(L + tail))
    for b in range(8):
        t0 = b * cl
        ln = cl + 2.4
        c = choir([nf(x) for x in chords[b]], ln, "o" if b % 2 == 0 else "a", voices=3, breath=0.06,
                  env=[(0, 0), (1.2, 1), (cl, 0.8), (ln, 0)])
        put(pads, svf(c, 1800, 0.7), t0)
        put(low, strings([nf(roots[b])], ln, [(0, 0), (1.0, 1), (cl, 0.9), (ln, 0)], fc=300, vib=0.003), t0)
        for k, nm in harp_lines[b]:
            put(harps, harp(nf(nm), 2.2, 0.9, bright=0.8), t0 + k * cl / 8, rng.uniform(0.7, 1.0))
    harps = echo(harps, cl / 8 * 3, fb=0.3, mix=0.3, tail=0.0, lp_fc=2200)
    bells = zeros(N(L + tail))
    for t0, nm in ((0.2, "A3"), (14.2, "E3"), (21.2, "C4")):
        put(bells, bell(nf(nm), 5.0, tau=1.6, bright=0.5), t0)
    bells = svf(bells, 1500, 0.7)
    buf = add(rms_to(pads, -17), rms_to(low, -23), scale(norm(harps), 0.32), scale(norm(bells), 0.22))
    buf = reverb(buf, 0.88, 0.45, 0.4, tail=0.0)
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
    ("dig_beam_soft", sfx_dig_beam_soft, SFX),
    ("dig_beam_hard", sfx_dig_beam_hard, SFX),
    ("dig_beam_crystal", sfx_dig_beam_crystal, SFX),
    ("dig_beam_metal", sfx_dig_beam_metal, SFX),
    ("cast_light", sfx_cast_light, SFX),
    ("orb_fade", sfx_orb_fade, SFX),
    ("shard_pickup", sfx_shard_pickup, SFX),
    ("shard_vein", sfx_shard_vein, SFX),
    ("cast_pyro", sfx_cast_pyro, SFX),
    ("impact_pyro", sfx_impact_pyro, SFX),
    ("cast_hydro", sfx_cast_hydro, SFX),
    ("impact_hydro", sfx_impact_hydro, SFX),
    ("cast_terra", sfx_cast_terra, SFX),
    ("impact_terra", sfx_impact_terra, SFX),
    ("cast_cryo", sfx_cast_cryo, SFX),
    ("impact_cryo", sfx_impact_cryo, SFX),
    ("cast_aero", sfx_cast_aero, SFX),
    ("impact_aero", sfx_impact_aero, SFX),
    ("cast_fulm", sfx_cast_fulm, SFX),
    ("impact_fulm", sfx_impact_fulm, SFX),
    ("cast_alch", sfx_cast_alch, SFX),
    ("impact_alch", sfx_impact_alch, SFX),
    ("cast_myco", sfx_cast_myco, SFX),
    ("impact_myco", sfx_impact_myco, SFX),
    ("tidecall_loop", sfx_tidecall_loop, LOOP),
    ("levitate_loop", sfx_levitate_loop, LOOP),
    ("fusion_cast", sfx_fusion_cast, SFX),
    ("fusion_awaken", sfx_fusion_awaken, SFX),
    ("attune", sfx_attune, SFX),
    ("scroll_learned", sfx_scroll_learned, SFX),
    ("scroll_shatter", sfx_scroll_shatter, SFX),
    ("blink", sfx_blink, SFX),
    ("journal_open", sfx_journal_open, SFX),
    ("remains_search", sfx_remains_search, SFX),
    ("hollowed_moan", sfx_hollowed_moan, SFX),
    ("hollowed_cast", sfx_hollowed_cast, SFX),
    ("wyrm_burrow", sfx_wyrm_burrow, LOOP),
    ("wyrm_screech", sfx_wyrm_screech, SFX),
    ("puppet_squelch", sfx_puppet_squelch, SFX),
    ("puppet_burst", sfx_puppet_burst, SFX),
    ("wraith_hiss", sfx_wraith_hiss, SFX),
    ("lightseeker_flutter", sfx_lightseeker_flutter, LOOP),
    ("lightseeker_shriek", sfx_lightseeker_shriek, SFX),
    ("mimic_bite", sfx_mimic_bite, SFX),
    ("stalker_step", sfx_stalker_step, SFX),
    ("stalker_breath", sfx_stalker_breath, LOOP),
    ("stalker_appear", sfx_stalker_appear, SFX),
    ("whisper_1", sfx_whisper_1, SFX),
    ("whisper_2", sfx_whisper_2, SFX),
    ("whisper_3", sfx_whisper_3, SFX),
    ("whisper_4", sfx_whisper_4, SFX),
    ("distant_steps", sfx_distant_steps, SFX),
    ("sting_1", sfx_sting_1, SFX),
    ("sting_2", sfx_sting_2, SFX),
    ("sting_3", sfx_sting_3, SFX),
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
    ("chest_open", sfx_chest_open, SFX),
    ("shrine_buy", sfx_shrine_buy, SFX),
    ("ui_hover", sfx_ui_hover, SFX),
    ("ui_click", sfx_ui_click, SFX),
    ("ui_toggle", sfx_ui_toggle, SFX),
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
