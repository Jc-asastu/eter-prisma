//! Diseño del kernel de dispersión — puerto EXACTO de forja/prisma.py.
//! La curva de group-delay D(f) se especifica y la fase all-pass se integra:
//! φ(f) = −2π·∫₀^f D(ν)dν  (suma de Riemann idéntica a la referencia python).

use realfft::num_complex::Complex;
use realfft::RealFftPlanner;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KernelParams {
    /// Apertura del arcoíris en segundos (0..2).
    pub spread: f64,
    /// −1 = caída (graves tarde) … +1 = arco (agudos tarde). Morfable.
    pub tilt: f64,
    /// Curvatura del mapeo log (0.3..2.5).
    pub shape: f64,
    pub fmin: f64,
    pub fmax: f64,
}

impl Default for KernelParams {
    fn default() -> Self {
        Self { spread: 0.3, tilt: 1.0, shape: 1.0, fmin: 30.0, fmax: 18000.0 }
    }
}

/// D(f) en segundos. Réplica del orden de operaciones de python:
/// t = clamp(log-map)^shape ; tilt "low" = 1 − t  (¡no (1−t)^shape!).
pub fn curve_d(f: f64, p: &KernelParams) -> f64 {
    let fc = f.clamp(p.fmin, p.fmax);
    let t = ((fc / p.fmin).ln() / (p.fmax / p.fmin).ln()).clamp(0.0, 1.0);
    let up = t.powf(p.shape);
    let down = 1.0 - up;
    let w = (p.tilt + 1.0) * 0.5;
    p.spread * (w * up + (1.0 - w) * down)
}

/// Espectro all-pass e^{jφ(f)} para una FFT de tamaño `n` (cualquier n).
/// Idéntico a python: acc += D(k·df)·df ANTES de aplicar φ al bin k.
pub fn dispersion_spectrum(n: usize, sr: f64, p: &KernelParams) -> Vec<Complex<f64>> {
    let bins = n / 2 + 1;
    let df = sr / n as f64;
    let mut out = Vec::with_capacity(bins);
    let mut acc = 0.0f64;
    for k in 0..bins {
        acc += curve_d(k as f64 * df, p) * df;
        let phi = -2.0 * std::f64::consts::PI * acc;
        out.push(Complex::new(phi.cos(), phi.sin()));
    }
    out
}

/// Kernel FIR causal para el motor en tiempo real.
/// `max_len` debe ser potencia de 2 (cap de calidad/CPU).
pub fn design_kernel(p: &KernelParams, sr: f64, max_len: usize) -> Vec<f32> {
    debug_assert!(max_len.is_power_of_two());
    let need = (p.spread * sr).ceil() as usize + 2048;
    let mut n = 1usize;
    while n < need {
        n <<= 1;
    }
    n = n.min(max_len);

    let mut spec = dispersion_spectrum(n, sr, p);
    // realfft exige DC y Nyquist reales (numpy los ignora: equivalente).
    spec[0].im = 0.0;
    let last = spec.len() - 1;
    spec[last].im = 0.0;

    let mut planner = RealFftPlanner::<f64>::new();
    let c2r = planner.plan_fft_inverse(n);
    let mut out = c2r.make_output_vec();
    c2r.process(&mut spec, &mut out).expect("irfft kernel");
    let scale = 1.0 / n as f64;
    out.into_iter().map(|v| (v * scale) as f32).collect()
}
