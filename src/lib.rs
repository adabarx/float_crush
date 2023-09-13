#![allow(unused_variables, dead_code)]
use crush::{Quantizator, mix_dry_wet, Polarity, search_mantissa, SampleRange, SearchRange, SearchType, CullResult};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

use std::sync::Arc;

mod crush;

struct FloatCrush {
    params: Arc<FloatCrushParams>,
}

#[derive(Params)]
struct FloatCrushParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[id = "input gain"]
    pub input_gain: FloatParam,

    #[id = "round"]
    pub round: IntParam,

    #[id = "exponent"]
    pub exponent: FloatParam,

    #[id = "exponent bias"]
    pub exponent_bias: FloatParam,

    #[id = "mantissa"]
    pub mantissa: FloatParam,

    #[id = "mantissa bias"]
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
            editor_state: ViziaState::new(|| (800, 600)),

            input_gain: FloatParam::new(
                "input",
                1.,
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.),
                    max: util::db_to_gain(30.),
                    factor: FloatRange::gain_skew_factor(-30., 30.)
                }
            )
            .with_unit(" db")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            round: IntParam::new("round", 0, IntRange::Linear { min: -1, max: 1 }),
            
            exponent: FloatParam::new(
                "exponent",
                8.,
                FloatRange::Skewed { min: 0., max: 12., factor: 1.6666667 }
            )
            .with_unit(" bits")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            
            exponent_bias: FloatParam::new(
                "exponent base",
                2.,
                FloatRange::Skewed {
                    min: 1.,
                    max: 8.,
                    factor: 0.35
                }
            ),

            mantissa: FloatParam::new(
                "mantissa",
                12.,
                FloatRange::Skewed { min: 0., max: 12., factor: 1.6666667 }
            )
            .with_unit(" bits")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            mantissa_bias: FloatParam::new(
                "mantissa_bias",
                0.,
                FloatRange::Linear { min: -1., max: 1. }
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

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

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];


    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let e_param = self.params.exponent.value();
            let m_param = self.params.mantissa.value();

            let exponent = 2_f32.powf(e_param).floor() as u32;
            let e_bias = self.params.exponent_bias.value();

            let mantissa = 2_f32.powf(m_param).floor() as u32;
            let m_bias = 50000_f32.powf(self.params.mantissa_bias.value());

            let input_gain = self.params.input_gain.value();
            let dry_gain = self.params.dry.value();
            let wet_gain = self.params.wet.value();

            let quantizer = Quantizator::from_i32(self.params.round.value());
            
            for sample in channel_samples {
                let sample_dry = sample.clone();
                let sample_wet = sample.clone();

                // apply input gain
                let sample_wet = sample_wet * input_gain;

                if sample_wet.abs() >= 1. {
                    // clip sample, mix w/ dry according to dry/wet settings
                    *sample = mix_dry_wet(
                        sample_dry,
                        dry_gain,
                        sample_wet.polarity(),
                        wet_gain
                    );
                    continue;
                }

                if e_param == 0. && m_param == 0. {
                    // no search necessary, quantize to zero or one
                    let sample_wet = quantizer.quantize_abs(1., 0., sample_wet.abs())
                        * sample_wet.polarity();

                    *sample = mix_dry_wet(sample_dry, dry_gain, sample_wet, wet_gain);
                    continue;
                }

                if e_param == 0. {
                    *sample = search_mantissa(
                        mantissa,
                        m_bias,
                        SampleRange::new(1., 0.),
                        sample_wet,
                        quantizer
                    );
                    continue;
                }

                let sample_wet = {
                    let sample_abs = sample_wet.abs();
                    let polarity = sample_wet.polarity();
                    let mut search_range = SearchRange {
                        start: 0,
                        length: exponent,
                        search_type: SearchType::Exponent(exponent, e_bias),
                        range: SampleRange::new(1., 0.),
                        sample: sample_abs
                    };

                    loop {
                        match search_range.cull() {
                            CullResult::ExactMatch(sample_abs) => break sample_abs * polarity,
                            CullResult::TwoLeft(upper, lower, sample_abs) => {
                                if lower > sample_abs {
                                    break search_mantissa(
                                        mantissa,
                                        m_bias,
                                        SampleRange::new(lower, 0.),
                                        sample_wet,
                                        quantizer
                                    );
                                }
                                break search_mantissa(
                                    mantissa,
                                    m_bias,
                                    SampleRange::new(upper, lower),
                                    sample_wet,
                                    quantizer
                                );
                            },
                            CullResult::CutHalf => (),
                        }
                    }
                };
                *sample = (sample_dry * dry_gain) + (sample_wet * wet_gain)
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for FloatCrush {
    const CLAP_ID: &'static str = "com.your-domain.float-crush";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("floating bit crusher");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::AudioEffect, ClapFeature::Distortion, ClapFeature::Glitch];
}

impl Vst3Plugin for FloatCrush {
    const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(FloatCrush);
nih_export_vst3!(FloatCrush);

