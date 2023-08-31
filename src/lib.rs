#![allow(unused_variables)]
use nih_plug::prelude::*;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct FloatCrush {
    params: Arc<FloatCrushParams>,
}

#[derive(Params)]
struct FloatCrushParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "exponent"]
    pub exponent: IntParam,

    #[id = "mantissa"]
    pub mantissa: IntParam,
}

impl Default for FloatCrush {
    fn default() -> Self {
        Self {
            params: Arc::new(FloatCrushParams::default()),
        }
    }
}

impl Default for FloatCrushParams {
    fn default() -> Self {
        Self {
            exponent: IntParam::new("exponent", 8, IntRange::Linear { min: 1, max: 8 }),
            mantissa: IntParam::new("mantissa", 8, IntRange::Linear { min: 1, max: 8 }),
        }
    }
}

impl Plugin for FloatCrush {
    const NAME: &'static str = "Float Crush";
    const VENDOR: &'static str = "Katlyn Thomas";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "katlyn.c.thomas@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];


    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let exponent = self.params.exponent.value();
            let mantissa = self.params.mantissa.value();
            
            for sample in channel_samples {
                let polarity = if sample.is_sign_positive() { 1_f32 } else { -1_f32 };
                let s_abs = sample.abs();

                if s_abs >= 1. {
                    *sample = 1. * polarity;
                    continue;
                }

                'search_loop: for e in 0..=exponent {
                    let curr_frac = 1_f32 / 2_f32.powi(e);
                    let curr_err  = curr_frac - s_abs;
                    if curr_err.is_sign_negative() {
                        let m_step = curr_frac / mantissa as f32;
                        let mut prev_step = curr_frac;
                        let mut prev_err = curr_err;
                        for m in 0..=mantissa {
                            let curr_step = curr_frac + (m_step * m as f32);
                            let curr_err  = curr_step - s_abs;
                            if curr_err.is_sign_positive() {
                                if curr_err.abs() < prev_err.abs() {
                                    *sample = curr_step * polarity;
                                } else {
                                    *sample = prev_step * polarity;
                                }
                                break 'search_loop;
                            } else if m == mantissa {
                                *sample = curr_step * polarity;
                                break 'search_loop;
                            }
                            prev_step = curr_step;
                            prev_err = curr_err;
                        }
                        *sample = curr_frac * polarity;
                        break 'search_loop;
                    } else if e == exponent {
                        let threshold = curr_frac / 2.;
                        if s_abs > threshold {
                            *sample = curr_frac;
                        } else {
                            *sample = 0.;
                        }
                        break 'search_loop;
                    }
                }
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for FloatCrush {
    const CLAP_ID: &'static str = "com.your-domain.float-crush";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("low bit floating point quantization?");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for FloatCrush {
    const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(FloatCrush);
nih_export_vst3!(FloatCrush);
