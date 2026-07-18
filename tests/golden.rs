//! Golden tests: el motor Rust debe reproducir la referencia python
//! (forja/prisma.py) y la convolución particionada debe igualar a la directa.
//! Prerequisito: `python scripts/golden_gen.py` (genera golden/*.wav).

use eter_prisma::dsp::conv::PartitionedConv;
use eter_prisma::dsp::kernel::{design_kernel, dispersion_spectrum, KernelParams};
use realfft::RealFftPlanner;
use std::path::PathBuf;

fn golden(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden").join(p)
}

fn read_wav(path: &PathBuf) -> (Vec<f64>, f64) {
    let mut r = hound::WavReader::open(path)
        .unwrap_or_else(|e| panic!("falta {path:?} — correr scripts/golden_gen.py ({e})"));
    let sr = r.spec().sample_rate as f64;
    let xs: Vec<f64> = r.samples::<f32>().map(|s| s.unwrap() as f64).collect();
    (xs, sr)
}

fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}

/// Réplica del pipeline python: X = rfft(x, n) · H(n) → irfft → pico igualado.
fn render_espectral(x: &[f64], n: usize, sr: f64, p: &KernelParams) -> Vec<f64> {
    let mut planner = RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(n);
    let c2r = planner.plan_fft_inverse(n);
    let mut input = vec![0.0; n];
    input[..x.len()].copy_from_slice(x);
    let mut spec = r2c.make_output_vec();
    r2c.process(&mut input, &mut spec).unwrap();
    let h = dispersion_spectrum(n, sr, p);
    for (s, hh) in spec.iter_mut().zip(h.iter()) {
        *s *= hh;
    }
    spec[0].im = 0.0;
    let last = spec.len() - 1;
    spec[last].im = 0.0;
    let mut y = c2r.make_output_vec();
    c2r.process(&mut spec, &mut y).unwrap();
    let scale = 1.0 / n as f64;
    for v in &mut y {
        *v *= scale;
    }
    // normalización de pico (misma regla que python)
    let pico_in = x.iter().fold(0.0f64, |m, v| m.max(v.abs())) + 1e-12;
    let pico_out = y.iter().fold(0.0f64, |m, v| m.max(v.abs())) + 1e-12;
    let g = pico_in / pico_out;
    for v in &mut y {
        *v *= g;
    }
    y
}

#[test]
fn golden_vs_python() {
    let (x, sr) = read_wav(&golden("in.wav"));
    let casos = [
        ("arco", KernelParams { spread: 0.30, tilt: 1.0, shape: 1.0, ..Default::default() }),
        ("caida", KernelParams { spread: 0.25, tilt: -1.0, shape: 1.0, ..Default::default() }),
        ("laser", KernelParams { spread: 0.35, tilt: 1.0, shape: 1.6, ..Default::default() }),
    ];
    for (nombre, p) in casos {
        let (y_py, _) = read_wav(&golden(&format!("out_{nombre}.wav")));
        let n = y_py.len(); // el nfft exacto que usó python
        let y = render_espectral(&x, n, sr, &p);
        let m = n.min(y.len());
        let diff: Vec<f64> = (0..m).map(|i| y[i] - y_py[i]).collect();
        let db = 20.0 * (rms(&diff) / rms(&y_py)).log10();
        println!("golden {nombre}: diff = {db:.1} dB");
        assert!(db < -60.0, "{nombre}: diff {db:.1} dB >= -60 dB");
    }
}

#[test]
fn partitioned_equals_direct() {
    // señal y kernel deterministas (xorshift)
    let mut estado = 0x2545_F491_4F6C_DD1Du64;
    let mut rnd = || {
        estado ^= estado << 13;
        estado ^= estado >> 7;
        estado ^= estado << 17;
        (estado >> 11) as f64 / (1u64 << 53) as f64 - 0.5
    };
    let x: Vec<f64> = (0..8192).map(|_| rnd()).collect();
    let h: Vec<f64> = (0..3000)
        .map(|i| rnd() * (-(i as f64) / 900.0).exp())
        .collect();

    // directa O(n·m)
    let mut directa = vec![0.0f64; x.len()];
    for (i, d) in directa.iter_mut().enumerate() {
        let kmax = h.len().min(i + 1);
        let mut s = 0.0;
        for k in 0..kmax {
            s += x[i - k] * h[k];
        }
        *d = s;
    }

    // particionada B=256
    let b = 256;
    let mut conv = PartitionedConv::new(b, h.len().next_power_of_two());
    let h32: Vec<f32> = h.iter().map(|v| *v as f32).collect();
    conv.set_kernel(&h32);
    let mut y = Vec::with_capacity(x.len());
    for chunk in x.chunks(b) {
        let mut blk = vec![0.0f32; b];
        for (d, s) in blk.iter_mut().zip(chunk.iter()) {
            *d = *s as f32;
        }
        conv.process_block(&mut blk);
        y.extend(blk.iter().take(chunk.len()).map(|v| *v as f64));
    }

    let diff: Vec<f64> = y.iter().zip(&directa).map(|(a, b)| a - b).collect();
    let rel = rms(&diff) / rms(&directa);
    println!("particionada vs directa: rel = {rel:.2e}");
    assert!(rel < 1e-5, "rel {rel:.2e} >= 1e-5");
}

#[test]
#[ignore] // bench manual: cargo test --release -- --ignored --nocapture
fn bench_simple() {
    let sr = 48000.0;
    let p = KernelParams { spread: 2.0, ..Default::default() };
    let h = design_kernel(&p, sr, 1 << 18);
    let b = 512;
    let mut conv = PartitionedConv::new(b, 1 << 18);
    conv.set_kernel(&h);
    let mut blk = vec![0.1f32; b];
    let t0 = std::time::Instant::now();
    let iters = 2000;
    for _ in 0..iters {
        conv.process_block(&mut blk);
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / iters as f64;
    let budget = b as f64 / sr * 1000.0;
    println!(
        "kernel {} taps: {ms:.3} ms/bloque (presupuesto {budget:.2} ms) = {:.1}% CPU",
        h.len(),
        ms / budget * 100.0
    );
}
