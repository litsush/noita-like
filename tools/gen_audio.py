#!/usr/bin/env python3
"""Generates every sound and music track in assets/audio as 16-bit mono
WAV, chiptune style: square, triangle and noise voices with simple
envelopes. Pure Python, no dependencies. Rerun after editing; commit the
output."""
import math
import os
import random
import struct
import wave
from array import array

RATE = 22050
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "audio")


def write(name, samples):
    os.makedirs(OUT, exist_ok=True)
    peak = max(1e-6, max(abs(s) for s in samples))
    gain = 0.92 / peak if peak > 0.92 else 1.0
    data = array("h", (int(max(-1.0, min(1.0, s * gain)) * 32767) for s in samples))
    with wave.open(os.path.join(OUT, name + ".wav"), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(data.tobytes())
    print(f"{name}.wav  {len(samples) / RATE:5.2f}s")


def buf(seconds):
    return [0.0] * int(seconds * RATE)


def mix(dst, src, at=0.0, gain=1.0):
    off = int(at * RATE)
    need = off + len(src) - len(dst)
    if need > 0:
        dst.extend([0.0] * need)
    for i, s in enumerate(src):
        dst[off + i] += s * gain


def env(n, a=0.005, d=0.05, s=0.6, r=0.05):
    """ADSR as a list of gains for n samples."""
    a_n, d_n, r_n = int(a * RATE), int(d * RATE), int(r * RATE)
    out = [0.0] * n
    for i in range(n):
        if i < a_n:
            g = i / max(1, a_n)
        elif i < a_n + d_n:
            g = 1.0 - (1.0 - s) * (i - a_n) / max(1, d_n)
        elif i < n - r_n:
            g = s
        else:
            g = s * (n - i) / max(1, r_n)
        out[i] = g
    return out


def tone(freq, dur, wave_="square", duty=0.5, slide=0.0, vib=0.0, vib_hz=6.0, adsr=None, gain=0.5):
    """One note. `slide` is the frequency multiplier reached by the end."""
    n = int(dur * RATE)
    e = env(n, **(adsr or {}))
    out = [0.0] * n
    phase = 0.0
    rnd = random.Random(int(freq * 1000) + n)
    last = 0.0
    for i in range(n):
        t = i / RATE
        f = freq * (slide ** (t / dur) if slide else 1.0)
        if vib:
            f *= 1.0 + vib * math.sin(2 * math.pi * vib_hz * t)
        phase += f / RATE
        p = phase - math.floor(phase)
        if wave_ == "square":
            v = 1.0 if p < duty else -1.0
        elif wave_ == "tri":
            v = 4.0 * abs(p - 0.5) - 1.0
        elif wave_ == "saw":
            v = 2.0 * p - 1.0
        elif wave_ == "noise":
            # Sample-and-hold noise at the given rate for a crunchy tone.
            if int(phase * 2) != int((phase - f / RATE) * 2):
                last = rnd.uniform(-1, 1)
            v = last
        else:
            v = math.sin(2 * math.pi * p)
        out[i] = v * e[i] * gain
    return out


def noise(dur, adsr=None, gain=0.5, lowpass=0.0, seed=1):
    n = int(dur * RATE)
    e = env(n, **(adsr or {}))
    rnd = random.Random(seed)
    out = [0.0] * n
    acc = 0.0
    for i in range(n):
        v = rnd.uniform(-1, 1)
        if lowpass:
            acc += (v - acc) * (1.0 - lowpass)
            v = acc
        out[i] = v * e[i] * gain
    return out


def seq(notes, wave_="square", duty=0.5, gap=0.0, adsr=None, gain=0.4, **kw):
    """notes: list of (freq or 0 for rest, duration)."""
    out = []
    for f, d in notes:
        if f <= 0:
            out.extend([0.0] * int(d * RATE))
        else:
            out.extend(tone(f, d - gap, wave_, duty, adsr=adsr, gain=gain, **kw))
            out.extend([0.0] * int(gap * RATE))
    return out


def note(name):
    """'C4', 'A#3' -> Hz."""
    names = {"C": 0, "C#": 1, "D": 2, "D#": 3, "E": 4, "F": 5, "F#": 6, "G": 7, "G#": 8, "A": 9, "A#": 10, "B": 11}
    key = name[:-1]
    octave = int(name[-1])
    semis = names[key] + (octave - 4) * 12 - 9
    return 440.0 * (2 ** (semis / 12))


# ---------------------------------------------------------------------------
# Effects

SHORT = dict(a=0.002, d=0.03, s=0.4, r=0.04)
PLUCK = dict(a=0.002, d=0.08, s=0.0, r=0.02)
HOLD = dict(a=0.005, d=0.05, s=0.7, r=0.08)


def fx_ui_click():
    return tone(900, 0.06, "square", 0.25, slide=1.5, adsr=PLUCK, gain=0.5)


def fx_ready():
    out = tone(note("E5"), 0.07, "square", 0.5, adsr=PLUCK)
    mix(out, tone(note("B5"), 0.1, "square", 0.5, adsr=PLUCK), at=0.07)
    return out


def fx_join():
    out = []
    for i, n in enumerate(["C5", "E5", "G5"]):
        mix(out, tone(note(n), 0.09, "square", 0.5, adsr=PLUCK), at=i * 0.07)
    return out


def fx_round_start():
    out = []
    for i, n in enumerate(["C4", "G4", "C5", "E5"]):
        mix(out, tone(note(n), 0.12, "square", 0.5, adsr=PLUCK, gain=0.45), at=i * 0.08)
    mix(out, noise(0.25, adsr=dict(a=0.001, d=0.1, s=0.3, r=0.1), gain=0.5, lowpass=0.85), at=0.3)
    mix(out, tone(110, 0.3, "square", 0.5, slide=0.5, adsr=HOLD, gain=0.5), at=0.3)
    return out


def fx_hit():
    out = noise(0.07, adsr=SHORT, gain=0.6, lowpass=0.5, seed=3)
    mix(out, tone(200, 0.08, "square", 0.5, slide=0.4, adsr=SHORT, gain=0.5))
    return out


def fx_hit_heavy():
    out = noise(0.16, adsr=dict(a=0.001, d=0.06, s=0.4, r=0.08), gain=0.7, lowpass=0.7, seed=4)
    mix(out, tone(140, 0.18, "square", 0.5, slide=0.3, adsr=HOLD, gain=0.6))
    mix(out, tone(55, 0.2, "tri", adsr=HOLD, gain=0.5))
    return out


def fx_blocked():
    out = tone(1800, 0.05, "square", 0.5, slide=0.7, adsr=PLUCK, gain=0.45)
    mix(out, tone(2600, 0.08, "square", 0.3, adsr=PLUCK, gain=0.3), at=0.01)
    return out


def fx_clip():
    out = noise(0.12, adsr=SHORT, gain=0.5, lowpass=0.3, seed=5)
    mix(out, tone(400, 0.1, "noise", adsr=SHORT, gain=0.5))
    return out


def fx_whoosh():
    out = noise(0.18, adsr=dict(a=0.06, d=0.04, s=0.5, r=0.08), gain=0.35, lowpass=0.9, seed=6)
    return out


def fx_clash():
    out = tone(1400, 0.12, "square", 0.5, slide=0.8, adsr=PLUCK, gain=0.4)
    mix(out, tone(2100, 0.14, "square", 0.3, slide=0.75, adsr=PLUCK, gain=0.35), at=0.02)
    mix(out, noise(0.1, adsr=SHORT, gain=0.5, seed=7))
    return out


def fx_sever():
    out = noise(0.14, adsr=dict(a=0.001, d=0.05, s=0.3, r=0.08), gain=0.5, lowpass=0.6, seed=8)
    mix(out, tone(300, 0.15, "noise", adsr=SHORT, gain=0.4))
    mix(out, tone(260, 0.2, "square", 0.5, slide=0.4, adsr=HOLD, gain=0.3), at=0.03)
    return out


def fx_down():
    out = tone(330, 0.35, "square", 0.5, slide=0.3, adsr=HOLD, gain=0.5)
    mix(out, noise(0.2, adsr=dict(a=0.001, d=0.08, s=0.3, r=0.1), gain=0.5, lowpass=0.8, seed=9), at=0.2)
    mix(out, tone(60, 0.25, "tri", adsr=HOLD, gain=0.5), at=0.2)
    return out


def fx_finish():
    out = noise(0.5, adsr=dict(a=0.001, d=0.2, s=0.3, r=0.25), gain=0.8, lowpass=0.75, seed=10)
    mix(out, tone(90, 0.6, "square", 0.5, slide=0.25, adsr=dict(a=0.001, d=0.2, s=0.5, r=0.3), gain=0.7))
    mix(out, tone(45, 0.7, "tri", adsr=dict(a=0.001, d=0.3, s=0.5, r=0.3), gain=0.6))
    for i, n in enumerate(["C3", "G3", "C4", "D#4", "G4"]):
        mix(out, tone(note(n), 0.5, "square", 0.5, adsr=dict(a=0.01, d=0.1, s=0.6, r=0.3), gain=0.25), at=0.45 + i * 0.06)
    mix(out, tone(note("C5"), 0.9, "square", 0.5, vib=0.01, adsr=dict(a=0.02, d=0.2, s=0.6, r=0.5), gain=0.25), at=0.8)
    return out


def fx_slam():
    out = tone(70, 0.22, "tri", adsr=HOLD, gain=0.7)
    mix(out, noise(0.12, adsr=SHORT, gain=0.5, lowpass=0.8, seed=11))
    return out


def fx_spit():
    out = noise(0.09, adsr=dict(a=0.001, d=0.03, s=0.4, r=0.05), gain=0.4, lowpass=0.2, seed=12)
    mix(out, tone(600, 0.1, "square", 0.3, slide=2.5, adsr=PLUCK, gain=0.3), at=0.02)
    return out


def fx_grab():
    return tone(220, 0.25, "tri", vib=0.12, vib_hz=18, adsr=HOLD, gain=0.6)


def fx_ink():
    out = []
    for i in range(6):
        mix(out, tone(150 + i * 40, 0.08, "sine", slide=1.6, adsr=PLUCK, gain=0.4), at=i * 0.05)
    mix(out, noise(0.35, adsr=dict(a=0.02, d=0.1, s=0.4, r=0.15), gain=0.3, lowpass=0.92, seed=13))
    return out


def fx_poison():
    return tone(520, 0.4, "tri", slide=0.5, vib=0.08, vib_hz=9, adsr=HOLD, gain=0.5)


def fx_eat():
    out = []
    for i in range(3):
        mix(out, noise(0.05, adsr=SHORT, gain=0.5, lowpass=0.4, seed=20 + i), at=i * 0.09)
        mix(out, tone(900 - i * 150, 0.05, "noise", adsr=SHORT, gain=0.35), at=i * 0.09)
    return out


def fx_eat_meat():
    out = []
    for i in range(2):
        mix(out, noise(0.09, adsr=SHORT, gain=0.5, lowpass=0.85, seed=30 + i), at=i * 0.12)
        mix(out, tone(180, 0.1, "tri", slide=0.6, adsr=SHORT, gain=0.4), at=i * 0.12)
    return out


def fx_starving():
    out = tone(note("E4"), 0.25, "square", 0.5, adsr=HOLD, gain=0.4)
    mix(out, tone(note("C4"), 0.4, "square", 0.5, slide=0.9, adsr=HOLD, gain=0.4), at=0.25)
    return out


def fx_surprise():
    out = []
    for i, n in enumerate(["G5", "C6", "G6"]):
        mix(out, tone(note(n), 0.08, "square", 0.25, adsr=PLUCK, gain=0.45), at=i * 0.05)
    return out


def fx_frenzy():
    out = []
    for i in range(4):
        mix(out, tone(note("A4"), 0.1, "square", 0.5, adsr=PLUCK, gain=0.45), at=i * 0.22)
        mix(out, tone(note("D#5"), 0.1, "square", 0.5, adsr=PLUCK, gain=0.45), at=i * 0.22 + 0.11)
    return out


def jingle(names, step, wave_="square", duty=0.5, gain=0.4, last=0.5):
    out = []
    for i, n in enumerate(names):
        d = last if i == len(names) - 1 else step * 1.1
        mix(out, tone(note(n), d, wave_, duty, adsr=dict(a=0.005, d=0.05, s=0.6, r=0.1), gain=gain), at=i * step)
    return out


def fx_round_won():
    out = jingle(["C5", "E5", "G5", "C6"], 0.11, last=0.6)
    mix(out, jingle(["C3", "E3", "G3", "C4"], 0.11, "tri", gain=0.4, last=0.6))
    return out


def fx_round_lost():
    out = jingle(["E4", "D#4", "D4", "C#4"], 0.18, duty=0.3, last=0.7)
    mix(out, jingle(["E2", "D#2", "D2", "C#2"], 0.18, "tri", gain=0.4, last=0.7))
    return out


def fx_round_draw():
    return jingle(["G4", "G4", "E4"], 0.16, last=0.5)


def fx_evolve():
    out = []
    for i, n in enumerate(["C5", "E5", "G5", "B5", "D6", "G6"]):
        mix(out, tone(note(n), 0.14, "square", 0.25, vib=0.01, adsr=PLUCK, gain=0.35), at=i * 0.06)
    mix(out, tone(note("C6"), 0.5, "tri", adsr=dict(a=0.05, d=0.1, s=0.6, r=0.3), gain=0.3), at=0.3)
    return out


# ---------------------------------------------------------------------------
# Music: a tiny tracker. Patterns are strings of notes per 16th step;
# '-' holds, '.' rests.


def parse(pattern, octave_shift=0):
    steps = pattern.split()
    out = []
    for s in steps:
        if s in ("-", "."):
            out.append(s)
        else:
            out.append(note(s) * (2 ** octave_shift))
    return out


def render_track(steps, step_dur, wave_, duty=0.5, gain=0.3, adsr=None, slide=0.0):
    out = []
    i = 0
    while i < len(steps):
        s = steps[i]
        if s == ".":
            out.extend([0.0] * int(step_dur * RATE))
            i += 1
            continue
        if s == "-":
            out.extend([0.0] * int(step_dur * RATE))
            i += 1
            continue
        # Count holds.
        n = 1
        while i + n < len(steps) and steps[i + n] == "-":
            n += 1
        dur = step_dur * n
        out.extend(tone(s, dur, wave_, duty, slide=slide, adsr=adsr or dict(a=0.003, d=0.05, s=0.7, r=0.03), gain=gain))
        i += n
    return out


def drums(pattern, step_dur):
    """'k' kick, 's' snare, 'h' hat, '.' rest."""
    out = []
    for ch in pattern.split():
        n = int(step_dur * RATE)
        if ch == "k":
            v = tone(120, min(0.12, step_dur), "square", 0.5, slide=0.3, adsr=PLUCK, gain=0.5)
        elif ch == "s":
            v = noise(min(0.1, step_dur), adsr=PLUCK, gain=0.4, lowpass=0.5, seed=41)
        elif ch == "h":
            v = noise(min(0.04, step_dur), adsr=PLUCK, gain=0.18, seed=42)
        else:
            v = []
        v = v + [0.0] * (n - len(v)) if len(v) < n else v[:n]
        out.extend(v)
    return out


def song(bpm, bars, lead, bass, drum, lead2=None, lead_duty=0.5, lead_gain=0.28, sections=None):
    """`sections` is a list of (lead, bass) parts played in turn after the
    first; the counter-melody and drums loop underneath."""
    step = 60.0 / bpm / 4.0
    parts = [(lead, bass)] + list(sections or [])
    total = int(bars * 16 * step * RATE) * len(parts)
    out = [0.0] * total

    def loop_fill(track):
        t = []
        while len(t) < total:
            t.extend(track)
        return t[:total]

    melody, bassline = [], []
    for l, b in parts:
        melody.extend(render_track(parse(l), step, "square", lead_duty, gain=lead_gain))
        bassline.extend(render_track(parse(b), step, "tri", gain=0.4))
    mix(out, loop_fill(melody))
    mix(out, loop_fill(bassline))
    if lead2:
        mix(out, loop_fill(render_track(parse(lead2), step, "square", 0.25, gain=0.16)))
    mix(out, loop_fill(drums(drum, step)))
    return out


def music_menu():
    lead = ("E4 - - - G4 - B4 - E5 - - - D5 - B4 - "
            "C5 - - - B4 - A4 - G4 - - - - - A4 - "
            "B4 - - - D5 - F#5 - B5 - - - A5 - F#5 - "
            "G5 - - - F#5 - E5 - D5 - - - - - - - ")
    lead2 = (". . . . E5 - . . . . . . G5 - . . "
             ". . . . E5 - . . . . . . . . . . "
             ". . . . F#5 - . . . . . . A5 - . . "
             ". . . . B5 - . . . . . . . . . . ")
    bass = ("E2 - - - E2 - - - E2 - - - B1 - - - "
            "C2 - - - C2 - - - C2 - - - G1 - - - "
            "B1 - - - B1 - - - B1 - - - F#1 - - - "
            "G1 - - - G1 - - - D2 - - - D2 - - - ")
    drum = ("k . h . s . h . k . h . s . h h " * 4)
    lead_b = ("G4 - - - B4 - D5 - G5 - - - F#5 - D5 - "
              "E5 - - - D5 - C5 - B4 - - - - - C5 - "
              "D5 - - - F#5 - A5 - D6 - - - C6 - A5 - "
              "B5 - - - A5 - G5 - F#5 - - - E5 - - - ")
    bass_b = ("G1 - - - G1 - - - G1 - - - D2 - - - "
              "C2 - - - C2 - - - C2 - - - G1 - - - "
              "D2 - - - D2 - - - D2 - - - A1 - - - "
              "B1 - - - B1 - - - E2 - - - E2 - - - ")
    return song(96, 4, lead, bass, drum, lead2=lead2, lead_duty=0.5, sections=[(lead_b, bass_b)])


def music_lobby():
    lead = ("A4 - - - - - C5 - E5 - - - D5 - C5 - "
            "B4 - - - - - - - - - - - G4 - A4 - "
            "F4 - - - - - A4 - C5 - - - B4 - A4 - "
            "E5 - - - D5 - C5 - B4 - - - - - - - ")
    lead2 = ("E5 - - - - - - - . . . . . . . . "
             "D5 - - - - - - - . . . . . . . . "
             "C5 - - - - - - - . . . . . . . . "
             "G5 - - - - - - - . . . . . . . . ")
    bass = ("A1 - - - - - - - A1 - - - E2 - - - "
            "G1 - - - - - - - G1 - - - D2 - - - "
            "F1 - - - - - - - F1 - - - C2 - - - "
            "E1 - - - - - - - E1 - - - E2 - - - ")
    drum = ("k . . . h . . . s . . . h . . . " * 4)
    lead_b = ("C5 - - - - - E5 - G5 - - - A5 - G5 - "
              "E5 - - - - - - - - - - - D5 - C5 - "
              "D5 - - - - - F5 - A5 - - - G5 - F5 - "
              "E5 - - - G5 - E5 - D5 - - - - - - - ")
    bass_b = ("C2 - - - - - - - C2 - - - G1 - - - "
              "A1 - - - - - - - A1 - - - E2 - - - "
              "F1 - - - - - - - F1 - - - C2 - - - "
              "G1 - - - - - - - G1 - - - G1 - - - ")
    return song(84, 4, lead, bass, drum, lead2=lead2, lead_duty=0.25, lead_gain=0.22, sections=[(lead_b, bass_b)])


def music_fight():
    lead = ("E5 - E5 - . . E5 - G5 - . . E5 - D5 - "
            "C5 - . . C5 - . . D5 - D#5 - E5 - - - "
            "E5 - E5 - . . E5 - G5 - . . B5 - A5 - "
            "G5 - . . E5 - . . F#5 - F5 - E5 - - - ")
    lead2 = ("B4 - . . B4 - . . B4 - . . B4 - . . "
             "A4 - . . A4 - . . B4 - . . C5 - . . "
             "B4 - . . B4 - . . B4 - . . D5 - . . "
             "C5 - . . B4 - . . A4 - . . G#4 - . . ")
    bass = ("E2 - E2 - E2 - E2 - E2 - E2 - E2 - E2 - "
            "A1 - A1 - A1 - A1 - B1 - B1 - B1 - B1 - "
            "E2 - E2 - E2 - E2 - E2 - E2 - E2 - E2 - "
            "C2 - C2 - C2 - C2 - B1 - B1 - B1 - B1 - ")
    drum = ("k . h . s . h h k . h . s . h . " * 4)
    lead_b = ("G5 - G5 - . . G5 - B5 - . . G5 - F#5 - "
              "E5 - . . E5 - . . F#5 - G5 - A5 - - - "
              "B5 - . . A5 - . . G5 - . . F#5 - E5 - "
              "D5 - . . E5 - . . F5 - F#5 - G5 - - - ")
    bass_b = ("G2 - G2 - G2 - G2 - G2 - G2 - G2 - G2 - "
              "C2 - C2 - C2 - C2 - D2 - D2 - D2 - D2 - "
              "G2 - G2 - G2 - G2 - E2 - E2 - E2 - E2 - "
              "D2 - D2 - D2 - D2 - B1 - B1 - B1 - B1 - ")
    lead_c = ("E5 - - - - - - - D5 - - - - - - - "
              "C5 - - - - - - - B4 - - - - - - - "
              "E5 - - - G5 - - - B5 - - - E6 - - - "
              "D#6 - - - B5 - - - G5 - - - E5 - - - ")
    return song(152, 4, lead, bass, drum, lead2=lead2, lead_duty=0.5, lead_gain=0.3, sections=[(lead_b, bass_b), (lead_c, bass)])


EFFECTS = {
    "ui_click": fx_ui_click,
    "ready": fx_ready,
    "join": fx_join,
    "round_start": fx_round_start,
    "hit": fx_hit,
    "hit_heavy": fx_hit_heavy,
    "blocked": fx_blocked,
    "clip": fx_clip,
    "whoosh": fx_whoosh,
    "clash": fx_clash,
    "sever": fx_sever,
    "down": fx_down,
    "finish": fx_finish,
    "slam": fx_slam,
    "spit": fx_spit,
    "grab": fx_grab,
    "ink": fx_ink,
    "poison": fx_poison,
    "eat": fx_eat,
    "eat_meat": fx_eat_meat,
    "starving": fx_starving,
    "surprise": fx_surprise,
    "frenzy": fx_frenzy,
    "round_won": fx_round_won,
    "round_lost": fx_round_lost,
    "round_draw": fx_round_draw,
    "evolve": fx_evolve,
}

MUSIC = {
    "music_menu": music_menu,
    "music_lobby": music_lobby,
    "music_fight": music_fight,
}

if __name__ == "__main__":
    for name, f in EFFECTS.items():
        write(name, f())
    for name, f in MUSIC.items():
        write(name, f())
