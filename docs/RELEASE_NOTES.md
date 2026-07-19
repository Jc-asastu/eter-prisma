# ÉTER PRISMA v1.0.0 — Release Notes (borrador)

**Spectral dispersion with an exact group-delay curve. The first ÉTER device.**

PRISMA smears sound across time the way a prism splits light: every frequency
arrives with its own delay, following a group-delay curve D(f) you shape
directly. The curve is rendered as an exact FIR kernel and convolved in real
time — no allpass approximations.

## Features

- **Exact dispersion**: the D(f) curve you set is the curve you get, verified
  against the reference implementation to below −140 dB.
- **Spread** 0–2 s · **Tilt** −1…+1 (arco/caída, fully morphable) ·
  **Shape** 0.3–2.5 · **F Min/Max** band limits · **Mix** · **Output** ·
  **Quality** (Eco 0.5 s / Normal 1 s / Max 2 s kernel cap).
- **Click-free automation**: kernel recomputes on a worker thread and swaps in
  via dual convolvers with a 2048-sample crossfade. Zero allocations on the
  audio thread.
- **Optical-bench GUI**: living prism whose fan IS your Δt(f) curve, photons
  that travel with their real per-frequency delay, particle spectrometer
  (color = wavelength), phosphor oscilloscope, spectral-gradient dials.
- **Formats**: VST3 + CLAP, Windows x64. Reported latency: 512 samples.
- **6 factory presets** (arco_corto, arco_extremo, caida_alien, laser,
  liquido, sutil).

## Quality assurance (v1.0.0)

- pluginval strictness 10: **SUCCESS** (VST3, GUI enabled, host running).
- Golden tests vs. Python reference: diff < −140 dB (three curve cases).
- Partitioned convolution == direct convolution: rel RMS < 1e-5, identical
  peaks (1.3637 vs 1.3637 on the 2 s worst case).
- Sample-rate matrix 44.1/48/88.2/96/192 kHz × block 32–4096: 30/30 green —
  no NaN/inf, bounded peaks, no silent output.
- Automation torture (spread swept 0→2 s in 50 steps while processing):
  no NaN, no discontinuities beyond the expected noise-mixing bound.

## Install

Run `eter-prisma-1.0.0-win64-setup.exe` (installs VST3 + CLAP to the standard
`Common Files` folders), or copy the bundles manually — see README.

## Known limitations

- Windows x64 only in this release (macOS/Linux: planned, the DSP core is
  portable Rust).
- Fixed internal block of 512 samples → 512 samples of latency (reported to
  the host for compensation).
- The GUI requires the WebView2 runtime (preinstalled on Windows 10/11).
- Spread is capped at 80% of the Quality kernel budget (e.g. Normal caps the
  effective spread at 0.8 s) to guarantee the tail fits.

## License

GPL-3.0-or-later. Includes a vendored fork of nih-plug-webview (ported to
current baseview/raw-window-handle) under the same terms.
Built with [nih-plug](https://github.com/robbert-vdh/nih-plug).
