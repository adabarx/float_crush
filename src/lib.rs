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
    #[id = "drive"]
    pub drive: FloatParam,

    #[id = "exponent"]
    pub exponent: IntParam,

    #[id = "exponent_bias"]
    pub exponent_bias: FloatParam,

    #[id = "mantissa"]
    pub mantissa: FloatParam,

    #[id = "mantissa_bias"]
    pub mantissa_bias: FloatParam,

    #[id = "dry"]
    pub dry: FloatParam,

    #[id = "wet"]
    pub wet: FloatParam,
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
            drive: FloatParam::new(
                "drive",
                1.,
                FloatRange::Skewed {
                    min: util::db_to_gain(-12.),
                    max: util::db_to_gain(36.),
                    factor: FloatRange::gain_skew_factor(-12., 36.)
                }
            )
            .with_unit(" db")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            
            exponent: IntParam::new(
                "exponent",
                8,
                IntRange::Linear { min: 0, max: 8 }
            )
            .with_unit(" bits"),
            
            exponent_bias: FloatParam::new(
                "exponent_bias",
                1.,
                FloatRange::Linear { min: 0.5, max: 2. }
            ),

            mantissa: FloatParam::new(
                "mantissa",
                8.,
                FloatRange::Linear { min: 0., max: 8. }
            )
            .with_unit(" bits"),

            mantissa_bias: FloatParam::new(
                "mantissa_bias",
                0.,
                FloatRange::Linear { min: -1., max: 1. }
            )
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            dry: FloatParam::new("dry", 0., FloatRange::Skewed {
                min: 0.,
                max: 1.,
                factor: FloatRange::gain_skew_factor(1e-5, 0.)
            })
            .with_unit(" db")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            wet: FloatParam::new("wet", 1., FloatRange::Skewed {
                min: 0.,
                max: 1., 
                factor: FloatRange::gain_skew_factor(1e-5, 0.)
            })
            .with_unit(" db")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

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
            let exponent_bias = self.params.exponent_bias.value();

            let mantissa = 2_f32.powf(self.params.mantissa.value()).round() as i32;
            let mantissa_bias = 100_f32.powf(self.params.mantissa_bias.value());
            let mantissa_bias_invert = 100_f32.powf(self.params.mantissa_bias.value() * -1.);

            let drive = self.params.drive.value();
            let dry_gain = self.params.dry.value();
            let wet_gain = self.params.wet.value();
            
            for sample in channel_samples {
                let polarity = if sample.is_sign_positive() { 1_f32 } else { -1_f32 };
                let s_dry = sample.clone();

                // apply input drive
                *sample *= drive;

                let s_abs = sample.abs();
                let mut s_wet = sample.clone();

                if s_abs >= 1. {
                    let clipped = 1. * polarity;
                    *sample = (s_dry * dry_gain) + (clipped * wet_gain);
                    continue;
                }

                'search_loop: for e in 0..=exponent {
                    let curr_frac = 1_f32 / (exponent_bias * 2.).powi(e);
                    let curr_err  = curr_frac - s_abs;
                    if curr_err.is_sign_negative() {
                        let mut prev_step = curr_frac;
                        let mut prev_err = curr_err;
                        for m in 0..=mantissa {

                            let curr_step = curr_frac + if m == 0 {
                                0.
                            } else if mantissa_bias == 1. {
                                let m_step = curr_frac / mantissa as f32;
                                m_step * m as f32
                            } else {
                                // normalize mantissa to 0.0 - 1.0
                                let m = m as f32 / mantissa as f32;
                                (mantissa_bias.powf(m) - 1.) / (mantissa_bias - 1.)
                            };

                            let curr_err  = curr_step - s_abs;
                            if curr_err.is_sign_positive() {
                                if curr_err.abs() < prev_err.abs() {
                                    s_wet = curr_step * polarity;
                                } else {
                                    s_wet = prev_step * polarity;
                                }
                                break 'search_loop;
                            } else if m == mantissa {
                                s_wet = curr_step * polarity;
                                break 'search_loop;
                            }
                            prev_step = curr_step;
                            prev_err = curr_err;
                        }
                        s_wet = curr_frac * polarity;
                        break 'search_loop;
                    } else if e == exponent {
                        s_wet = 0.;
                        // just gonna shamelessly copy this logic
                        let m_step = curr_frac / mantissa as f32;
                        let mut prev_step = curr_frac;
                        let mut prev_err = curr_err;

                        for m in 0..mantissa {
                            let curr_step = curr_frac - if m == 0 {
                                0.
                            } else if mantissa_bias == 1. {
                                let m_step = curr_frac / mantissa as f32;
                                m_step * m as f32
                            } else {
                                // normalize mantissa to 0.0 - 1.0
                                let m = m as f32 / mantissa as f32;
                                (mantissa_bias_invert.powf(m) - 1.) / (mantissa_bias_invert - 1.)
                            };

                            let curr_err  = curr_step - s_abs;
                            if curr_err.is_sign_negative() {
                                if curr_err.abs() < prev_err.abs() {
                                    s_wet = curr_step * polarity;
                                } else {
                                    s_wet = prev_step * polarity;
                                }
                                break 'search_loop;
                            } else if m == mantissa {
                                s_wet = 0.;
                                break 'search_loop;
                            }
                            prev_step = curr_step;
                            prev_err = curr_err;
                        }
                        break 'search_loop;
                    }
                }

                *sample = (s_dry * dry_gain) + (s_wet * wet_gain)
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
