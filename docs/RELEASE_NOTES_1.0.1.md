## ÉTER PRISMA v1.0.1

Polish release — same exact physics, friendlier laboratory.

### New
- **"How this works" overlay**: a discreet button in the header opens an in-plugin
  walkthrough of the actual physics — the dispersion curve D(f), the pure all-pass
  phase response, the partitioned-convolution engine, and what each knob does in
  physical terms. No marketing physics; the real formulas, with a wink.
- The overlay links to this repository (opened safely via the host system browser).

### Improved
- **Typography contrast**: all labels, readouts and knob values switched from the
  old low-contrast gray to a bone/gold palette with proper weight — legible at a
  glance on any screen.
- **Full English UI**: every label and readout is now in English
  (spectrometer, refraction, arc/fall tilt readout).

### Unchanged
- DSP is bit-identical to v1.0.0. Parameter IDs and plugin ID unchanged —
  existing projects load exactly as saved.
- QA: full test suite green, pluginval strictness 10 SUCCESS.

### Install
Run `eter-prisma-1.0.1-win64-setup.exe` (VST3 + CLAP), or unzip
`eter-prisma-1.0.1-win64.zip` into your plugin folders manually.
