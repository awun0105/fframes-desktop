"""Deterministic synthesized soundtrack and sound effects; no downloaded or licensed recordings.

music.wav is 156 s at 120 BPM (a beat is 0.5 s) on a Dm - Bb - F - C loop, arranged to the
film: a kick on every cut of the intro montage (0.5-4.5), a roll into the hit at 6.0 and the
pad under the logo; pulse and arpeggio for who VinUni is (10), the full beat from its
standards (26), a drop into a darker pumping beat for the college tour (34), a wide
half-time for how you learn and the world (74), a warm groove for life on campus and the
graduates (102), a build into a final hit for the invitation (138-146), then the pad alone
under the piano for the finale. A fill and a swell lead into every chapter's wipe. The
effects are separate files placed by `Video::audio`.
Run: py tools/soundtrack.py
"""
import math
import pathlib
import random
import struct
import wave

RATE = 48000
BEAT = 0.5
BAR = 2.0
SECONDS = 156.0
# the intro montage's cuts, its roll and the hit
MONTAGE = (0.5, 4.5)
HIT = 6.0
# the chapters' first seconds, and the last bars of the build into the coda
CHAPTERS = [10.0, 34.0, 74.0, 84.0, 102.0, 124.0]
BUILD = (138.0, 146.0)
CODA = 146.0
ROOT = pathlib.Path(__file__).resolve().parents[1]
TAU = 2 * math.pi

# Dm - Bb - F - C, one chord per bar
CHORDS = [(146.83, 174.61, 220.00), (116.54, 146.83, 174.61),
          (174.61, 220.00, 261.63), (130.81, 164.81, 196.00)]
ROOTS = [73.42, 58.27, 87.31, 65.41]


def write(name, samples):
    data = bytearray()
    for s in samples:
        data.extend(struct.pack('<h', int(max(-1.0, min(1.0, s)) * 32767)))
    with wave.open(str(ROOT / 'media' / name), 'wb') as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(data)


def env(t, attack, decay):
    return (1 - math.exp(-t * attack)) * math.exp(-t * decay)


def saw(f, t, partials=6):
    return sum(math.sin(TAU * f * k * t) / k for k in range(1, partials + 1)) * 0.6


def kick_at(d, decay=8.0):
    return math.sin(TAU * (48 + 130 * math.exp(-d * 28)) * d) * math.exp(-d * decay)


def music():
    rng = random.Random(3)
    out = []
    lp = 0.0
    for i in range(int(RATE * SECONDS)):
        t = i / RATE
        bar = int(t / BAR) % 4
        chord = CHORDS[bar]
        root = ROOTS[bar]
        in_beat = t % BEAT
        beat_index = int(t / BEAT)
        off = (t + BEAT / 2) % BEAT
        montage = MONTAGE[0] <= t < MONTAGE[1]
        roll = MONTAGE[1] <= t < HIT
        logo = HIT <= t < CHAPTERS[0]
        origin = CHAPTERS[0] <= t < 26.0
        proof = 26.0 <= t < CHAPTERS[1]
        research = CHAPTERS[1] <= t < CHAPTERS[2]
        world = CHAPTERS[2] <= t < CHAPTERS[4]
        people = CHAPTERS[4] <= t < BUILD[0]
        build = BUILD[0] <= t < BUILD[1]
        coda = t >= CODA
        # pad: an octave down in the research, string-like in the world and the build
        octave = 0.5 if research else 1.0
        pad = sum(math.sin(TAU * f * octave * t) + 0.45 * math.sin(TAU * f * octave * 1.004 * t) for f in chord) / 4.5
        if world or build:
            pad = 0.6 * pad + 0.5 * sum(saw(f * 2, t, 5) for f in chord) / 3 * (0.8 + 0.2 * math.sin(TAU * 5 * t))
        pad_level = 0.22 if logo or coda else 0.08 if montage or roll else 0.12
        if coda:
            pad_level *= max(0.0, 1 - (t - CODA) / 9.0)
        # 16th arpeggio two octaves up; muffled (no overtone) in the research
        step = int(t / (BEAT / 4))
        note = chord[step % 3] * (4 if (step // 3) % 2 else 2)
        st = t % (BEAT / 4)
        bright = 0.0 if research else 0.25
        arp = (math.sin(TAU * note * t) + bright * math.sin(TAU * note * 3 * t)) * env(st, 500, 16)
        if logo:
            arp_level = 0.06 * progress(t, HIT + 1.0, 2.0)
        elif montage or origin or proof or people or build:
            arp_level = 0.09
        elif research:
            arp_level = 0.07
        elif world:
            arp_level = 0.05
        else:
            arp_level = 0.0
        # kick: every beat (every cut of the montage), half time in the world, eighths in the
        # last two bars of the build
        kick = 0.0
        if montage or origin or proof or research or people or build or (world and beat_index % 2 == 0):
            kick = kick_at(in_beat)
        if BUILD[1] - 2.0 <= t < BUILD[1]:
            kick = kick_at(t % (BEAT / 2), 10.0)
        # the hits under the logo and on the downbeat of the coda
        for at in (HIT, CODA):
            if t >= at:
                d = t - at
                kick += math.sin(TAU * (40 + 90 * math.exp(-d * 10)) * d) * math.exp(-d * 2.2) * 1.3
        noise = rng.uniform(-1, 1)
        lp += 0.3 * (noise - lp)
        # snare on 2 and 4 (on 4 only in the half-time world)
        snare = 0.0
        if (montage or proof or people) and beat_index % 2 == 1 or world and beat_index % 4 == 3:
            snare = ((noise - lp) * 0.8 + math.sin(TAU * 190 * in_beat) * 0.5) * math.exp(-in_beat * 18)
        # a 16th fill and a swell into every chapter and the proof, and rolls that speed up
        # into the intro's hit and through the build
        swell = 0.0
        for start in CHAPTERS + [26.0]:
            if start - 0.5 <= t < start:
                snare += (noise - lp) * math.exp(-(t % (BEAT / 4)) * 40) * 0.8
            if start - 2.0 <= t < start:
                k = (t - (start - 2.0)) / 2.0
                swell = (noise - lp) * 0.1 * k * k
        for a, b in (MONTAGE[1], HIT), BUILD:
            if a <= t < b:
                k = (t - a) / (b - a)
                div = BEAT / (1 if k < 0.25 else 2 if k < 0.5 else 4 if k < 0.75 else 8)
                snare += (noise - lp) * math.exp(-(t % div) * 45) * (0.2 + 0.8 * k)
                swell = (noise - lp) * 0.15 * k * k
        hat = (noise - lp) * math.exp(-off * 70) if montage or 14.0 <= t < BUILD[0] else 0.0
        # bass: off-beat plucks, a pumping saw for the research and the build
        if research or build:
            bass = saw(root, t, 8) * min(in_beat / 0.18, 1.0) * 0.85
        elif montage or origin or proof or world or people:
            bass = math.sin(TAU * root * t) * env(off, 300, 6)
        else:
            bass = 0.0
        fade = min(t / 0.03, 1) * min((SECONDS - t) / 1.2, 1)
        s = (pad_level * pad + arp_level * arp + 0.45 * kick + 0.20 * snare + 0.05 * hat + 0.22 * bass + swell) * fade
        out.append(s * 0.85)
    return out


def progress(t, start, duration):
    return max(0.0, min(1.0, (t - start) / duration))


def sub():
    # the sub drop under the first frame
    return [math.sin(TAU * (30 + 25 * math.exp(-t * 3)) * t) * env(t, 200, 1.8) * 0.9
            for t in (i / RATE for i in range(int(RATE * 2.5)))]


def riser():
    # 1.5 s of brightening noise and a rising tone into a boom at 1.5 s
    rng = random.Random(11)
    out, lp, phase = [], 0.0, 0.0
    for i in range(int(RATE * 2.6)):
        t = i / RATE
        noise = rng.uniform(-1, 1)
        if t < 1.5:
            k = t / 1.5
            lp += (0.02 + 0.5 * k * k) * (noise - lp)
            phase += TAU * (200 + 900 * k * k) / RATE
            s = (lp * 0.8 + 0.25 * math.sin(phase)) * k * k
        else:
            d = t - 1.5
            lp += 0.05 * (noise - lp)
            s = 0.8 * math.sin(TAU * (40 + 60 * math.exp(-d * 12)) * d) * math.exp(-d * 3) + lp * math.exp(-d * 6) * 0.5
        out.append(s * min((2.6 - t) / 0.2, 1))
    return out


def swoosh():
    # a short whoosh peaking at 0.6 s, for the cuts
    rng = random.Random(5)
    out, lp = [], 0.0
    for i in range(int(RATE * 1.2)):
        t = i / RATE
        k = math.exp(-((t - 0.6) / 0.22) ** 2)
        lp += (0.03 + 0.4 * k) * (rng.uniform(-1, 1) - lp)
        out.append(lp * k * 1.4)
    return out


def chime():
    base = 1567.98  # G6
    return [(math.sin(TAU * base * t) + 0.5 * math.sin(TAU * base * 2.76 * t) * math.exp(-t * 4)
             + 0.3 * math.sin(TAU * base * 5.4 * t) * math.exp(-t * 8)) * env(t, 900, 2.4) * 0.45 * min((2.0 - t) / 0.1, 1)
            for t in (i / RATE for i in range(int(RATE * 2.0)))]


def pop():
    return [math.sin(TAU * (900 * math.exp(-t * 40) + 220) * t) * env(t, 3000, 45) * 0.8
            for t in (i / RATE for i in range(int(RATE * 0.15)))]


def blip():
    return [(1 if math.sin(TAU * (1320 if t < 0.05 else 1760) * t) > 0 else -1) * env(t, 2000, 22) * 0.25
            for t in (i / RATE for i in range(int(RATE * 0.22)))]


def servo():
    out, phase = [], 0.0
    for i in range(int(RATE * 0.8)):
        t = i / RATE
        phase += TAU * (420 + 380 * math.sin(math.pi * t / 0.8)) / RATE
        out.append((math.sin(phase) + 0.3 * math.sin(3 * phase)) * math.sin(math.pi * t / 0.8) * 0.18)
    return out


def ticks():
    # a counter rolling: ticks that speed up over 1.4 s
    out = [0.0] * int(RATE * 1.5)
    t = 0.0
    while t < 1.4:
        start = int(t * RATE)
        for j in range(int(RATE * 0.012)):
            if start + j < len(out):
                d = j / RATE
                out[start + j] += math.sin(TAU * 3200 * d) * math.exp(-d * 500) * 0.5
        t += 0.09 * (1 - t / 1.6) + 0.018
    return out


def piano():
    # a low D with a long, soft tail that ends by the last frame
    notes = [(73.42, 1.0), (146.83, 0.6), (220.0, 0.35), (293.66, 0.25)]
    out = []
    for i in range(int(RATE * 3.0)):
        t = i / RATE
        s = 0.0
        for f, a in notes:
            s += a * (math.sin(TAU * f * t) + 0.3 * math.sin(TAU * f * 2 * t) * math.exp(-t * 3)) * env(t, 600, 1.1 + f / 400)
        out.append(s * 0.35 * min((3.0 - t) / 0.4, 1))
    return out


if __name__ == '__main__':
    (ROOT / 'media').mkdir(exist_ok=True)
    for name, make in [('sub.wav', sub), ('riser.wav', riser), ('swoosh.wav', swoosh), ('chime.wav', chime),
                       ('pop.wav', pop), ('blip.wav', blip), ('servo.wav', servo), ('ticks.wav', ticks),
                       ('piano.wav', piano), ('music.wav', music)]:
        write(name, make())
