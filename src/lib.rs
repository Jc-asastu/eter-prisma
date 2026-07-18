//! PRISMA — dispersión espectral con curva de group-delay exacta.
//! The first ÉTER device. F2: shell completo del plugin.

pub mod dsp;

use crossbeam_channel::{bounded, Receiver, Sender};
use dsp::conv::{PartitionedConv, Spectra, SpectraBuilder};
use dsp::kernel::{design_kernel, KernelParams};
use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

/// Bloque interno del motor (= latencia reportada).
const B: usize = 512;
/// Largo del crossfade al cambiar kernel (samples).
const FADE: usize = 2048;
/// Cap absoluto de kernel (segundos) — define la memoria preasignada.
const MAX_SECS: f64 = 2.0;

#[derive(Enum, PartialEq, Clone, Copy)]
pub enum Calidad {
    #[name = "Eco (0.5 s)"]
    Eco,
    #[name = "Normal (1 s)"]
    Normal,
    #[name = "Max (2 s)"]
    Max,
}

impl Calidad {
    fn secs(self) -> f64 {
        match self {
            Calidad::Eco => 0.5,
            Calidad::Normal => 1.0,
            Calidad::Max => 2.0,
        }
    }
}

#[derive(Params)]
pub struct PrismaParams {
    /// IDs CONGELADOS — no renombrar jamás (rompe proyectos de usuarios).
    #[id = "spread"]
    pub spread: FloatParam,
    #[id = "tilt"]
    pub tilt: FloatParam,
    #[id = "shape"]
    pub shape: FloatParam,
    #[id = "fmin"]
    pub fmin: FloatParam,
    #[id = "fmax"]
    pub fmax: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "out"]
    pub out: FloatParam,
    #[id = "quality"]
    pub quality: EnumParam<Calidad>,
}

impl Default for PrismaParams {
    fn default() -> Self {
        Self {
            spread: FloatParam::new(
                "Spread",
                0.30,
                FloatRange::Skewed { min: 0.0, max: 2.0, factor: 0.5 },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(3)),
            tilt: FloatParam::new("Tilt", 1.0, FloatRange::Linear { min: -1.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| {
                    if v >= 0.0 {
                        format!("arco {:.2}", v)
                    } else {
                        format!("caída {:.2}", -v)
                    }
                })),
            shape: FloatParam::new(
                "Shape",
                1.0,
                FloatRange::Skewed { min: 0.3, max: 2.5, factor: 0.8 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            fmin: FloatParam::new(
                "F Min",
                30.0,
                FloatRange::Skewed { min: 20.0, max: 500.0, factor: 0.35 },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
            fmax: FloatParam::new(
                "F Max",
                16000.0,
                FloatRange::Skewed { min: 2000.0, max: 20000.0, factor: 0.5 },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(20.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            out: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 12.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            quality: EnumParam::new("Quality", Calidad::Normal),
        }
    }
}

/// Mensaje audio→worker con todo lo necesario para diseñar el kernel.
#[derive(Clone, Copy, PartialEq)]
struct KernelMsg {
    kp: KernelParams,
    cap_len: usize,
}

struct ChannelDsp {
    conv_a: PartitionedConv,
    conv_b: PartitionedConv,
    in_buf: Vec<f32>,
    out_a: Vec<f32>,
    out_b: Vec<f32>,
    dry_buf: Vec<f32>,
}

impl ChannelDsp {
    fn new(max_kernel: usize) -> Self {
        Self {
            conv_a: PartitionedConv::new(B, max_kernel),
            conv_b: PartitionedConv::new(B, max_kernel),
            in_buf: vec![0.0; B],
            out_a: vec![0.0; B],
            out_b: vec![0.0; B],
            dry_buf: vec![0.0; B],
        }
    }

    fn reset(&mut self) {
        self.conv_a.reset();
        self.conv_b.reset();
        self.in_buf.fill(0.0);
        self.out_a.fill(0.0);
        self.out_b.fill(0.0);
        self.dry_buf.fill(0.0);
    }
}

pub struct EterPrisma {
    params: Arc<PrismaParams>,
    sr: f64,
    ch: Vec<ChannelDsp>,
    pos: usize,
    active_b: bool,
    fading: bool,
    fade_pos: usize,
    tail: u32,
    last_msg: Option<KernelMsg>,
    tx_params: Option<Sender<KernelMsg>>,
    rx_spectra: Option<Receiver<(Spectra, Spectra)>>,
    tx_back: Option<Sender<(Spectra, Spectra)>>,
}

impl Default for EterPrisma {
    fn default() -> Self {
        Self {
            params: Arc::new(PrismaParams::default()),
            sr: 48000.0,
            ch: Vec::new(),
            pos: 0,
            active_b: false,
            fading: false,
            fade_pos: 0,
            tail: 0,
            last_msg: None,
            tx_params: None,
            rx_spectra: None,
            tx_back: None,
        }
    }
}

impl EterPrisma {
    fn kernel_msg(&self) -> KernelMsg {
        let q = self.params.quality.value().secs();
        let spread = (self.params.spread.value() as f64).min(q * 0.8);
        let fmin = self.params.fmin.value() as f64;
        let fmax = (self.params.fmax.value() as f64).max(fmin * 2.0);
        let cap = (q * self.sr) as usize + 2048;
        let mut cap_len = 1usize;
        while cap_len < cap {
            cap_len <<= 1;
        }
        KernelMsg {
            kp: KernelParams {
                spread,
                tilt: self.params.tilt.value() as f64,
                shape: self.params.shape.value() as f64,
                fmin,
                fmax,
            },
            cap_len,
        }
    }

    fn max_kernel(&self) -> usize {
        let need = (MAX_SECS * self.sr) as usize + 2048;
        let mut n = 1usize;
        while n < need {
            n <<= 1;
        }
        n
    }
}

impl Plugin for EterPrisma {
    const NAME: &'static str = "ETER PRISMA (dev)";
    const VENDOR: &'static str = "Juan Cruz Maisu";
    const URL: &'static str = "https://jcmaisu.tech";
    const EMAIL: &'static str = "juancmaisu@outlook.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    #[cfg(feature = "webview")]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        use nih_plug_webview::{HTMLSource, WebViewEditor};
        use serde_json::json;
        let params = self.params.clone();
        let editor = WebViewEditor::new(HTMLSource::String(include_str!("gui.html")), (560, 380))
            .with_background_color((5, 6, 10, 255))
            .with_event_loop(move |ctx, setter| {
                while let Ok(v) = ctx.next_event() {
                    match v.get("type").and_then(|x| x.as_str()).unwrap_or("") {
                        "set" => {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let val =
                                v.get("value").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                            macro_rules! setp {
                                ($p:expr) => {{
                                    setter.begin_set_parameter($p);
                                    setter.set_parameter_normalized($p, val);
                                    setter.end_set_parameter($p);
                                }};
                            }
                            match id {
                                "spread" => setp!(&params.spread),
                                "tilt" => setp!(&params.tilt),
                                "shape" => setp!(&params.shape),
                                "mix" => setp!(&params.mix),
                                _ => {}
                            }
                        }
                        "init" => {
                            ctx.send_json(json!({
                                "type": "state",
                                "spread": params.spread.unmodulated_normalized_value(),
                                "spread_text": params.spread.to_string(),
                                "tilt": params.tilt.unmodulated_normalized_value(),
                                "tilt_text": params.tilt.to_string(),
                                "shape": params.shape.unmodulated_normalized_value(),
                                "shape_text": params.shape.to_string(),
                                "mix": params.mix.unmodulated_normalized_value(),
                                "mix_text": params.mix.to_string(),
                            }));
                        }
                        _ => {}
                    }
                }
            });
        Some(Box::new(editor))
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sr = buffer_config.sample_rate as f64;
        let max_kernel = self.max_kernel();
        self.ch = (0..2).map(|_| ChannelDsp::new(max_kernel)).collect();
        self.pos = 0;
        self.active_b = false;
        self.fading = false;

        // kernel inicial SINCRÓNICO (acá los allocs están permitidos)
        let msg = self.kernel_msg();
        let h0 = design_kernel(&msg.kp, self.sr, msg.cap_len.min(max_kernel));
        self.tail = (h0.len() + B) as u32;
        for d in &mut self.ch {
            d.conv_a.set_kernel(&h0);
        }
        self.last_msg = Some(msg);

        // worker de recomputo de kernel
        let (tx_p, rx_p) = bounded::<KernelMsg>(16);
        let (tx_s, rx_s) = bounded::<(Spectra, Spectra)>(2);
        let (tx_b, rx_b) = bounded::<(Spectra, Spectra)>(4);
        let sr = self.sr;
        std::thread::spawn(move || {
            let mut builder = SpectraBuilder::new(B, max_kernel);
            let mut pool: Vec<(Spectra, Spectra)> = (0..2)
                .map(|_| (builder.nuevo_contenedor(), builder.nuevo_contenedor()))
                .collect();
            while let Ok(first) = rx_p.recv() {
                let mut msg = first;
                loop {
                    match rx_p.recv_timeout(Duration::from_millis(40)) {
                        Ok(m) => msg = m,
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => break,
                        Err(_) => return,
                    }
                }
                while let Ok(pair) = rx_b.try_recv() {
                    if pool.len() < 4 {
                        pool.push(pair);
                    }
                }
                let (mut c1, mut c2) = pool.pop().unwrap_or_else(|| {
                    (builder.nuevo_contenedor(), builder.nuevo_contenedor())
                });
                let h = design_kernel(&msg.kp, sr, msg.cap_len.min(max_kernel));
                builder.build(&h, &mut c1);
                c2.parts = c1.parts;
                for p in 0..c1.parts {
                    c2.data[p].copy_from_slice(&c1.data[p]);
                }
                if tx_s.send((c1, c2)).is_err() {
                    return;
                }
            }
        });
        self.tx_params = Some(tx_p);
        self.rx_spectra = Some(rx_s);
        self.tx_back = Some(tx_b);

        context.set_latency_samples(B as u32);
        true
    }

    fn reset(&mut self) {
        for d in &mut self.ch {
            d.reset();
        }
        self.pos = 0;
        self.fading = false;
        self.fade_pos = 0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let ns = buffer.samples();
        let raw = buffer.as_slice();
        let nch = raw.len().min(2);

        for i in 0..ns {
            let mix = self.params.mix.smoothed.next();
            let og = self.params.out.smoothed.next();
            let t_fade = if self.fading {
                self.fade_pos as f32 / FADE as f32
            } else {
                0.0
            };

            for (ch, chan) in raw.iter_mut().enumerate().take(nch) {
                let d = &mut self.ch[ch];
                let x = chan[i];
                let dry = d.dry_buf[self.pos];
                d.dry_buf[self.pos] = x;
                d.in_buf[self.pos] = x;
                let (act, oth) = if self.active_b {
                    (&d.out_b, &d.out_a)
                } else {
                    (&d.out_a, &d.out_b)
                };
                let wet = if self.fading {
                    act[self.pos] * (1.0 - t_fade) + oth[self.pos] * t_fade
                } else {
                    act[self.pos]
                };
                chan[i] = (dry * (1.0 - mix) + wet * mix) * og;
            }

            if self.fading {
                self.fade_pos += 1;
                if self.fade_pos >= FADE {
                    self.fading = false;
                    self.active_b = !self.active_b;
                }
            }

            self.pos += 1;
            if self.pos == B {
                self.pos = 0;
                for d in self.ch.iter_mut().take(nch) {
                    d.out_a.copy_from_slice(&d.in_buf);
                    d.conv_a.process_block(&mut d.out_a);
                    d.out_b.copy_from_slice(&d.in_buf);
                    d.conv_b.process_block(&mut d.out_b);
                }

                // ¿cambiaron los parámetros del kernel? → worker (coalesced)
                let msg = self.kernel_msg();
                if self.last_msg != Some(msg) {
                    if let Some(tx) = &self.tx_params {
                        if tx.try_send(msg).is_ok() {
                            self.last_msg = Some(msg);
                        }
                    }
                }

                // ¿llegó un kernel nuevo? → instalar en el conv inactivo + fade
                if !self.fading {
                    if let Some(rx) = &self.rx_spectra {
                        if let Ok((mut c1, mut c2)) = rx.try_recv() {
                            self.tail = ((msg.kp.spread * self.sr) as usize + 2 * B) as u32;
                            {
                                let d = &mut self.ch[0];
                                let conv = if self.active_b { &mut d.conv_a } else { &mut d.conv_b };
                                conv.install(&mut c1);
                            }
                            if nch > 1 {
                                let d = &mut self.ch[1];
                                let conv = if self.active_b { &mut d.conv_a } else { &mut d.conv_b };
                                conv.install(&mut c2);
                            }
                            self.fading = true;
                            self.fade_pos = 0;
                            if let Some(tx) = &self.tx_back {
                                if let Err(e) = tx.try_send((c1, c2)) {
                                    // jamás deallocar en el audio thread
                                    std::mem::forget(e.into_inner());
                                }
                            }
                        }
                    }
                }
            }
        }

        ProcessStatus::Tail(self.tail)
    }
}

impl ClapPlugin for EterPrisma {
    const CLAP_ID: &'static str = "tech.jcmaisu.eter-prisma";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Dispersion espectral con curva exacta - the first ETER device");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for EterPrisma {
    const VST3_CLASS_ID: [u8; 16] = *b"ETERPrismaJCM001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

nih_export_clap!(EterPrisma);
nih_export_vst3!(EterPrisma);
