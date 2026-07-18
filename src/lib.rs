//! PRISMA — dispersión espectral con curva de group-delay exacta.
//! The first ÉTER device. F0: esqueleto gain para validar la toolchain.

use nih_plug::prelude::*;
#[cfg(feature = "webview")]
use nih_plug_webview::{HTMLSource, WebViewEditor};
#[cfg(feature = "webview")]
use serde_json::json;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct EterPrisma {
    params: Arc<EterPrismaParams>,
}

#[derive(Params)]
pub struct EterPrismaParams {
    /// F0: un solo parámetro para validar automatización/smoothing/estado.
    #[id = "gain"]
    pub gain: FloatParam,

    /// Señal GUI: el host/automación movió el parámetro → refrescar webview.
    pub gain_changed: Arc<AtomicBool>,
}

impl Default for EterPrisma {
    fn default() -> Self {
        Self {
            params: Arc::new(EterPrismaParams::default()),
        }
    }
}

impl Default for EterPrismaParams {
    fn default() -> Self {
        let gain_changed = Arc::new(AtomicBool::new(false));
        let flag = gain_changed.clone();
        Self {
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db())
            .with_callback(Arc::new(move |_| flag.store(true, Ordering::Relaxed))),
            gain_changed,
        }
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
        let params = self.params.clone();
        let changed = self.params.gain_changed.clone();
        let editor = WebViewEditor::new(HTMLSource::String(include_str!("gui.html")), (440, 280))
            .with_background_color((5, 6, 10, 255))
            .with_event_loop(move |ctx, setter, _window| {
                while let Ok(value) = ctx.next_event() {
                    let tipo = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match tipo {
                        "SetGain" => {
                            if let Some(v) = value.get("value").and_then(|v| v.as_f64()) {
                                setter.begin_set_parameter(&params.gain);
                                setter.set_parameter_normalized(&params.gain, v as f32);
                                setter.end_set_parameter(&params.gain);
                            }
                        }
                        "Init" => {
                            ctx.send_json(json!({
                                "type": "param_change",
                                "value": params.gain.unmodulated_normalized_value(),
                                "text": params.gain.to_string(),
                            }))
                            .ok();
                        }
                        _ => {}
                    }
                }
                // host/automación movió el parámetro → reflejar en la GUI
                if changed.swap(false, Ordering::Relaxed) {
                    ctx.send_json(json!({
                        "type": "param_change",
                        "value": params.gain.unmodulated_normalized_value(),
                        "text": params.gain.to_string(),
                    }))
                    .ok();
                }
            });
        Some(Box::new(editor))
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();
            for sample in channel_samples {
                *sample *= gain;
            }
        }
        ProcessStatus::Normal
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
