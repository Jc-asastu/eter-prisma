# ÉTER PRISMA

**Spectral dispersion with an exact group-delay curve.** The first ÉTER device.

PRISMA splits sound in time the way a prism splits light: every frequency
arrives with its own delay, following a group-delay curve you shape directly.
No feedback networks, no allpass chains approximating the effect — the curve
D(f) you draw is the curve you get, rendered as an exact FIR kernel and
convolved in real time.

<!-- TODO: GIF demo (pendiente: captura de la GUI en uso) -->

## What it does

- **Spread** (0–2 s): how far the spectrum is smeared in time.
- **Tilt** (−1…+1): +1 = *arco* (highs arrive late, classic rainbow riser),
  −1 = *caída* (lows arrive late, alien drop). Fully morphable in between.
- **Shape** (0.3–2.5): curvature of the log-frequency mapping — from soft
  liquid sweeps to laser-like chirps.
- **F Min / F Max**: the frequency band the dispersion curve spans.
- **Mix / Output / Quality** (Eco 0.5 s · Normal 1 s · Max 2 s kernel cap).

The GUI is an *optical bench*: live waterfall, phosphor oscilloscope, and a
prism whose fan is the actual D(f) curve of your current parameters.

## Formats

- **VST3** and **CLAP**, Windows x64.
- Latency: 512 samples (reported to the host).
- Kernel changes are click-free: dual convolvers with a 2048-sample crossfade.

## Install

**Installer** (recommended): grab `eter-prisma-x.y.z-win64-setup.exe` from
[Releases](../../releases) and run it. It installs to the standard system
folders (`Common Files\VST3`, `Common Files\CLAP`).

**Manual**: copy `eter_prisma.vst3` (the whole folder) to
`C:\Program Files\Common Files\VST3\` and/or `eter_prisma.clap` to
`C:\Program Files\Common Files\CLAP\`.

## Build from source

Rust stable + [nih-plug](https://github.com/robbert-vdh/nih-plug)'s xtask:

```
cargo xtask bundle eter_prisma --release --features webview
```

Bundles land in `target\bundled\`. Omit `--features webview` for a headless
build (no GUI, same DSP). Run the test suite with `cargo test --release`
(golden tests reproduce the Python reference to below −140 dB; a full
sample-rate × block-size matrix and an automation torture test guard the
real-time engine).

## Quality assurance

- pluginval strictness 10: **SUCCESS** (VST3, with GUI, host running).
- Golden tests vs. the reference implementation: diff < −140 dB.
- SR matrix 44.1–192 kHz × block 32–4096: no NaN, bounded peaks.
- Automation torture (kernel swept 0→2 s while processing): click-free.

## License

[GPL-3.0-or-later](LICENSE). The bundled `vendor/nih-plug-webview` fork
(ported to current baseview/raw-window-handle, Windows-first) is part of this
repository under the same terms.

---

Juan Cruz Maisú ♥
