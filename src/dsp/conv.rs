//! Convolución particionada uniforme (overlap-save en dominio frecuencia).
//! Latencia estructural: 0 dentro de esta clase (el buffering al bloque interno
//! lo maneja el shell del plugin). Sin allocs después de `new`.

use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub struct PartitionedConv {
    block: usize,
    max_parts: usize,
    parts: usize,
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
    h_spec: Vec<Vec<Complex<f32>>>,
    fdl: Vec<Vec<Complex<f32>>>,
    fdl_pos: usize,
    prev_in: Vec<f32>,
    time_buf: Vec<f32>,
    spec_buf: Vec<Complex<f32>>,
    acc: Vec<Complex<f32>>,
    out_time: Vec<f32>,
    norm: f32,
}

impl PartitionedConv {
    pub fn new(block: usize, max_kernel: usize) -> Self {
        assert!(block.is_power_of_two());
        let max_parts = max_kernel.div_ceil(block).max(1);
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(2 * block);
        let ifft = planner.plan_fft_inverse(2 * block);
        let bins = block + 1;
        Self {
            block,
            max_parts,
            parts: 0,
            h_spec: vec![vec![Complex::default(); bins]; max_parts],
            fdl: vec![vec![Complex::default(); bins]; max_parts],
            fdl_pos: 0,
            prev_in: vec![0.0; block],
            time_buf: vec![0.0; 2 * block],
            spec_buf: vec![Complex::default(); bins],
            acc: vec![Complex::default(); bins],
            out_time: vec![0.0; 2 * block],
            norm: 1.0 / (2 * block) as f32,
            fft,
            ifft,
        }
    }

    pub fn block(&self) -> usize {
        self.block
    }

    /// Carga un kernel nuevo (llamar FUERA del audio thread).
    pub fn set_kernel(&mut self, h: &[f32]) {
        let parts = h.len().div_ceil(self.block).min(self.max_parts);
        for p in 0..parts {
            let a = p * self.block;
            let b = (a + self.block).min(h.len());
            self.time_buf[..b - a].copy_from_slice(&h[a..b]);
            self.time_buf[b - a..].fill(0.0);
            self.fft
                .process(&mut self.time_buf, &mut self.h_spec[p])
                .expect("fft partición");
        }
        self.parts = parts;
    }

    pub fn reset(&mut self) {
        for v in &mut self.fdl {
            v.fill(Complex::default());
        }
        self.prev_in.fill(0.0);
        self.fdl_pos = 0;
    }

    /// Procesa exactamente `block` samples in-place.
    pub fn process_block(&mut self, io: &mut [f32]) {
        debug_assert_eq!(io.len(), self.block);
        let b = self.block;
        // ventana overlap-save [bloque anterior | bloque actual]
        self.time_buf[..b].copy_from_slice(&self.prev_in);
        self.time_buf[b..].copy_from_slice(io);
        self.prev_in.copy_from_slice(io);
        self.fft
            .process(&mut self.time_buf, &mut self.spec_buf)
            .expect("fft bloque");
        self.fdl[self.fdl_pos].copy_from_slice(&self.spec_buf);
        // acumular Σ_p X_{t-p} · H_p
        self.acc.fill(Complex::default());
        for p in 0..self.parts {
            let idx = (self.fdl_pos + self.max_parts - p) % self.max_parts;
            let x = &self.fdl[idx];
            let hh = &self.h_spec[p];
            for (a, (xi, hi)) in self.acc.iter_mut().zip(x.iter().zip(hh.iter())) {
                *a += xi * hi;
            }
        }
        self.fdl_pos = (self.fdl_pos + 1) % self.max_parts;
        self.ifft
            .process(&mut self.acc, &mut self.out_time)
            .expect("ifft bloque");
        for (o, v) in io.iter_mut().zip(&self.out_time[b..]) {
            *o = v * self.norm;
        }
    }
}
