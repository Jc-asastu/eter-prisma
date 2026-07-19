# ÉTER PRISMA — diseño ejecutable (fuente de verdad)

Plugin de dispersión espectral (curva de group-delay EXACTA). GPL-3.0, marca ÉTER.
Referencia matemática validada: `Darkpsy-engine/forja/prisma.py` (NO tocar ese repo).
Estado: F0 ✅ (gain plugin carga en Bitwig; webview bit-rot → F3 con timebox).

## Reglas de ejecución (para sesiones baratas)
- Modelo: Sonnet alcanza para TODO lo de este doc. Fable solo si el diseño falla.
- SIN screenshots de DAW: verificar por `cargo test` + `pluginval` + log de Bitwig
  (`%LOCALAPPDATA%\Bitwig Studio\BitwigStudio.log`, grep "eter_prisma").
  La verificación de oído la hace Juan a mano y reporta por texto.
- Commits SIN Co-Authored-By (regla dura). Push: NUNCA sin pedido de Juan.
- No renombrar params ni IDs una vez shippeados (rompe proyectos de usuarios).

## F1 — Núcleo DSP

### Archivos
- `src/dsp/mod.rs` → `pub mod kernel; pub mod conv;`
- `src/dsp/kernel.rs`, `src/dsp/conv.rs`
- `tests/golden.rs`, `scripts/golden_gen.py`
- lib.rs: agregar `pub mod dsp;`
- Cargo.toml: dep `realfft = "3"`; dev-dep `hound = "3"`

### kernel.rs — puerto EXACTO de prisma.py
```rust
pub struct KernelParams { pub spread: f64, pub tilt: f64, pub shape: f64,
                          pub fmin: f64, pub fmax: f64 }  // tilt -1..+1
```
Curva D(f) — OJO al orden de operaciones (igual a python):
```
t = clamp( ln(clamp(f,fmin,fmax)/fmin) / ln(fmax/fmin), 0, 1 )
up = t^shape            // python tilt="high"
down = 1.0 - up         // python tilt="low"  (¡NO es (1-t)^shape!)
w = (tilt+1)/2
D(f) = spread * ( w*up + (1-w)*down )     // ±1 reproduce python exacto
```
Espectro de dispersión (cualquier n, no solo pow2):
```rust
pub fn dispersion_spectrum(n: usize, sr: f64, p: &KernelParams) -> Vec<Complex<f64>>
// bins = n/2+1; df = sr/n; acc=0;
// for k in 0..bins { acc += D(k*df)*df; phi = -2π*acc; out[k] = (cosφ, sinφ) }
// (python suma D del bin ANTES de aplicar φ a ese bin — replicar)
```
Kernel FIR (runtime):
```rust
pub fn design_kernel(p, sr, max_len_pow2) -> Vec<f32>
// n = next_pow2(spread*sr + 2048).min(max_len); spec = dispersion_spectrum(n,..)
// spec[0].im = 0; spec[n/2].im = 0;  // realfft exige DC/Nyquist reales
// irfft (realfft inverse, escalar 1/n) → f32
```

### conv.rs — convolución particionada uniforme (overlap-save)
```rust
pub struct PartitionedConv { /* block B pow2, FFT tamaño 2B, bins B+1 */ }
impl PartitionedConv {
  pub fn new(block: usize, max_kernel: usize) -> Self  // prealoca max_parts
  pub fn set_kernel(&mut self, h: &[f32])   // FFT de particiones (fuera de audio thread)
  pub fn reset(&mut self)
  pub fn process_block(&mut self, io: &mut [f32])  // len == block, in-place
}
```
Algoritmo por bloque: time=[prev_in, io] → R2C → guardar en FDL (ring de
max_parts espectros) → acc = Σ_p FDL[(pos+max−p)%max] ⊙ H_p → C2R → salida =
últimos B samples × 1/(2B). `prev_in = io` antes de procesar.
Nota: DC/Nyquist quedan reales solos (señal y kernel reales) — no zeroear en runtime.

### golden_gen.py (correr con Python312: `%LOCALAPPDATA%\Programs\Python\Python312`)
```python
import sys, numpy as np; from scipy.io import wavfile
sys.path.insert(0, r"C:\Users\Juan\Desktop\Darkpsy-engine\forja")
from prisma import prisma
sr=48000; rng=np.random.default_rng(7)
x=np.zeros(sr); x[1000]=1.0
x+=rng.standard_normal(sr)*0.3*np.exp(-np.arange(sr)/sr*12.0)
x-=x.mean(); x=np.convolve(x,[.25,.5,.25],'same')   # sin DC ni Nyquist
# guardar golden/in.wav float32 y por caso golden/out_<n>.wav = prisma(x[:,None],sr,**kw)[:,0]
CASOS={"arco":dict(spread=.3,tilt="high",shape=1.),
       "caida":dict(spread=.25,tilt="low",shape=1.),
       "laser":dict(spread=.35,tilt="high",shape=1.6)}
```
(prisma.py usa fmin=30 fmax=18000 → los KernelParams del test usan ESOS valores.)

### tests/golden.rs
1. `golden_vs_python`: por caso — n = len(out_py); X=rfft(x pad n) en f64;
   X ⊙ dispersion_spectrum(n,sr,p) → irfft/n → normalizar pico al pico de x
   (misma regla que python) → RMS(diff) vs out_py **< −60 dB** (esperado ~−80).
   tilt: "high"→+1.0, "low"→−1.0.
2. `partitioned_equals_direct`: x=xorshift 8192 samples, kernel 3000 taps
   determinista, B=256; alimentar bloques (x + ceil pad de ceros) y comparar
   contra convolución directa O(n·m) en f64: RMS rel **< 1e-5**.
3. `#[ignore] bench_simple`: medir ms por bloque con kernel de 2 s @48k, B=512;
   imprimir. Objetivo < 0.5 ms/bloque. (criterion NO — ahorro.)
Los wav de golden/ van gitignoreados; el script los regenera.

## F2 — Shell del plugin (resumen ejecutable)
- Params nih-plug (IDs CONGELADOS): `spread` s 0–2 skew log · `tilt` −1..+1 ·
  `shape` 0.3–2.5 · `fmin` 20–500 log · `fmax` 2k–20k log · `mix` 0–100% ·
  `out` −24..+12 dB · `quality` enum {Eco=kernel≤0.5s, Normal≤1s, Max≤2s}.
- Motor en process(): 2× PartitionedConv (L/R), B = 512 fijo interno con
  re-buffering del host block (FIFO in/out) → `latency()` = 512 + host-align.
- Recomputo de kernel: thread worker (std::thread + canal crossbeam ya viene
  con nih-plug deps) → double-buffer de convolvers, crossfade lineal 2048
  samples al swap. Cap de recomputo: 1 cada 50 ms (coalescing).
- Mix dry/wet: dry retrasado 512 (delay line) para alinear con wet. Bypass del
  host = param bypass propio con el mismo crossfade.
- Presets fábrica (spread/tilt/shape): arco_corto .12/+1/1 · arco_extremo
  .6/+1/1.2 · caida_alien .25/−1/1 · laser .35/+1/1.6 · liquido .18/−1/0.7 ·
  sutil .06/+1/1.
- pluginval (descargar release win) strictness 8 primero, 10 después.

## F3 GUI (decisión pendiente, timebox 2 h)
1º intento: portar `vendor/nih-plug-webview` a baseview/rwh actuales (9 errores
conocidos, ver commit 14ba61d). Si no sale en 2 h → `nih_plug_vizia`.
El bridge JSON y `src/gui.html` ya existen feature-gated (`webview`).

## F4 — QA de release (resultados 2026-07-16)

### Checklist
- [x] `cargo test --release` — golden_vs_python + partitioned_equals_direct verdes.
- [x] Matriz SR/block por test Rust (`tests/matrix.rs::matriz_sr_block`):
      SR {44100, 48000, 88200, 96000, 192000} × block {32, 64, 128, 512, 1024,
      4096}, 2 s de ruido xorshift por celda, spread=2.0 (peor caso). Verifica
      sin NaN/inf, pico acotado, salida no silenciosa. 30/30 celdas verdes.
- [x] Automation torture (`tests/matrix.rs::automation_torture_crossfade`):
      spread 0→2 s en 50 pasos, kernel recomputado e instalado con el patrón
      crossfade dual-convolver de lib.rs mientras corre audio. Sin NaN;
      diff sample-a-sample máx = 2.16 (límite 3.0, ver nota abajo).
- [x] Sanity (`tests/matrix.rs::overshoot_particionada_vs_directa`):
      PartitionedConv == convolución directa en pico (1.3637 vs 1.3637).
- [x] pluginval strictness 10: **verde SIN webview** (SUCCESS, todos los tests
      incl. Audio processing 44.1/48/96k × 64–1024, Automation, Plugin state,
      Fuzz parameters). **ROJO CON webview** — ver bug abajo.
- [x] State round-trip: verificado por inspección — TODO el estado persistente
      son los 8 params de `PrismaParams` con IDs congelados (spread, tilt,
      shape, fmin, fmax, mix, out, quality). No hay campos `#[persist]` ni
      estado serializado fuera de params; el resto (convolvers, buffers, tap,
      fade) es runtime reconstruible en initialize()/reset(). nih-plug maneja
      el save/restore solo → no hace falta test de host.

### Hallazgos técnicos (esperados, NO bugs)
- Overshoot de pico: un all-pass con group delay grande sobre ruido blanco de
  banda completa produce hasta +6.94 dB de overshoot por interferencia
  constructiva (Gibbs). Confirmado idéntico en convolución directa → es
  matemática del filtro, no implementación. Límite del test: +9 dB.
- Diff sample-a-sample durante crossfade: mezclar dos señales de ruido
  filtradas por kernels distintos da saltos de hasta ~2.2 con input ±1
  (cada rama puede overshootear ±1.8 con signos opuestos). Baseline sin
  crossfade sobre la misma señal: diff ≈ 0.002. El mecanismo de fade en sí
  es suave; límite del test: 3.0.

### BUG ABIERTO (bloqueante para release con GUI)
- pluginval strictness 10 con feature `webview` ABORTA en el test "Editor":
  panic no-unwind en `vendor/nih-plug-webview/src/lib.rs:293` —
  WebView2 HRESULT 0x8007139F (recurso en estado incorrecto). Determinístico
  (2/2 runs). Causa probable: `WebContext::new(Some(std::env::temp_dir()))`
  en lib.rs:259 usa el temp dir global como user-data-folder de WebView2;
  colisiona con el lock exclusivo de otro environment WebView2 activo
  (Bitwig con el plugin cargado corría durante la validación). Fix candidato:
  user-data-folder propio por proceso (p. ej. temp_dir()/eter-prisma-{pid})
  y/o reemplazar el panic por editor degradado sin abortar el host.
  Re-validar también sin Bitwig corriendo para aislar la condición.

### Pendiente manual (no headless)
- Matriz DAW en vivo: Bitwig + Reaper (Juan a oído, reporta texto). Ableton después.
- Sesión de uso real (automation con mouse, presets, bypass del host).
- Re-run de pluginval webview sin ningún WebView2 activo en la máquina.

## F5-F6 (recorte ahorro)
- Packaging: Inno Setup + README con GIF. Página /prisma y video DESPUÉS del
  release binario (no bloquean v1.0.0).
- Release: gh repo create (público) + tag v1.0.0 + binarios. KVR listing manual.
