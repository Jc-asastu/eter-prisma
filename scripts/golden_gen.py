# -*- coding: utf-8 -*-
"""Genera renders de referencia con forja/prisma.py para los golden tests.
Correr: python scripts/golden_gen.py  (desde la raiz del repo)"""
import sys
from pathlib import Path

import numpy as np
from scipy.io import wavfile

sys.path.insert(0, r"C:\Users\Juan\Desktop\Darkpsy-engine\forja")
from prisma import prisma  # noqa: E402

SR = 48000
OUT = Path(__file__).resolve().parent.parent / "golden"
OUT.mkdir(exist_ok=True)

rng = np.random.default_rng(7)
x = np.zeros(SR)
x[1000] = 1.0
x += rng.standard_normal(SR) * 0.3 * np.exp(-np.arange(SR) / SR * 12.0)
x -= x.mean()
x = np.convolve(x, [0.25, 0.5, 0.25], "same")   # sin DC ni Nyquist
x = x.astype(np.float64)

wavfile.write(OUT / "in.wav", SR, x.astype(np.float32))

CASOS = {
    "arco":  dict(spread=0.30, tilt="high", shape=1.0),
    "caida": dict(spread=0.25, tilt="low",  shape=1.0),
    "laser": dict(spread=0.35, tilt="high", shape=1.6),
}
for nombre, kw in CASOS.items():
    y = prisma(x[:, None], SR, **kw)[:, 0]
    wavfile.write(OUT / f"out_{nombre}.wav", SR, y.astype(np.float32))
    print(f"golden out_{nombre}.wav  len={len(y)}  {kw}")
print("ok ->", OUT)
