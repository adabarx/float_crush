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
            input_gain: FloatParam::new(
                "drive",
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
                FloatRange::Skewed { min: 0., max: 8., factor: 1.5 }
            )
            .with_unit(" bits")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            
            exponent_bias: FloatParam::new(
                "exponent_bias",
                1.,
                FloatRange::Skewed {
                    min: 0.500000059604644775390625,
                    max: 8.,
                    factor: 0.25
                }
            ),

            mantissa: FloatParam::new(
                "mantissa",
                12.,
                FloatRange::Skewed { min: 0., max: 12., factor: 1.25 }
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
            let exponent = 2_f32.powf(self.params.exponent.value()).round() as i32;
            let e_bias = self.params.exponent_bias.value();

            let mantissa = 2_f32.powf(self.params.mantissa.value()).round() as i32;
            let m_bias = 50000_f32.powf(self.params.mantissa_bias.value());

            let input_gain = self.params.input_gain.value();
            let dry_gain = self.params.dry.value();
            let wet_gain = self.params.wet.value();

            let quantizer = Quantizator::from_i32(self.params.round.value());
            
            for sample in channel_samples {
                let sample_dry = sample.clone();
                let sample_wet = sample.clone();

                // apply input drive
                let sample_wet = sample_wet * input_gain;

                if sample_wet.abs() >= 1. {
                    // clip sample, mix w/ dry according to dry/wet settings
                    *sample = mix_dry_wet(
                        sample_dry,
                        dry_gain,
                        sample_wet / sample_wet.abs(),
                        wet_gain
                    );
                    continue;
                }

                if exponent < 1 && mantissa < 1 {
                    // no search necessary, quantize to zero or one
                    let polarity = if sample_wet.is_sign_positive() { 1_f32 } else { -1_f32 };
                    let sample_wet = quantizer.quantize_abs(1., 0., sample_wet.abs()) * polarity;
                    *sample = mix_dry_wet(sample_dry, dry_gain, sample_wet, wet_gain);
                    continue;
                }
                
                let sample_wet = {
                    // if exponent == 0 {
                        search_mantissa(
                            mantissa,
                            m_bias,
                            (1., 0.),
                            sample_wet,
                            quantizer
                        )
                    // } else {
                    //     let sample_abs = sample_wet.abs();
                    //     let mut position = Position {
                    //         sample: 1.,
                    //         index: 0,
                    //         range: 0.5,
                    //         error: 1. - sample_abs,
                    //     };
                    //
                    //     loop {
                    //         let other_index = position.index + (exponent as f32 * position.range).round() as i32;
                    //         let other_sample = 1. / (e_bias * 2.).powi(other_index);
                    //         let other_err = other_sample - sample_abs;
                    //
                    //         // if a sample is already in a quantized position, return it
                    //         if other_sample == sample_abs { break sample_wet; }
                    //
                    //         if position.index - other_index <= 1 {
                    //             break search_mantissa(
                    //                 mantissa,
                    //                 m_bias,
                    //                 (position.sample, other_sample),
                    //                 sample_wet,
                    //                 quantizer
                    //             )
                    //         }
                    //
                    //         if other_err.is_sign_negative() {
                    //             position.range *= 0.5;
                    //         } else {
                    //             position.index = other_index;
                    //             position.sample = other_sample;
                    //             position.error = other_err;
                    //         }
                    //     }
                    // }
                };
                *sample = (sample_dry * dry_gain) + (sample_wet * wet_gain)
            }
        }

        ProcessStatus::Normal
    }
}

fn mix_dry_wet(dry: f32, dry_gain: f32, wet: f32, wet_gain: f32) -> f32 {
    (dry * dry_gain) + (wet * wet_gain)
}

#[derive(Clone, Copy)]
 enum Quantizator {
    RoundUp,
    Nearest,
    RoundDown,
}

impl Quantizator {
    pub fn from_i32(int: i32) -> Self {
        if int == 0 {
            Self::Nearest
        } else if int > 0 {
            Self::RoundUp
        } else {
            Self::RoundDown
        }
    }

    pub fn quantize_abs(&self, upper_bound: f32, lower_bound: f32, sample: f32) -> f32 {
        match self {
            Self::Nearest => {
                let midpoint = (upper_bound + lower_bound) / 2.;

                if sample > midpoint { upper_bound }
                else { lower_bound }
            },
            Self::RoundUp => upper_bound,
            Self::RoundDown => lower_bound,
        }
    }
}

struct IndexRange {
    start: i32,
    length: i32,
}

impl IndexRange {
    pub fn center(&self) -> i32 {
        let rv = self.start as f32 + (self.length as f32/ 2.);
        rv.floor() as i32
    }

    pub fn cull(&mut self, curr_index: i32, lower: bool) {
        if lower {
            self.length = self.length - curr_index;
        } else {
            self.start = curr_index;
            self.length = self.length - curr_index;
        }
    }
}


fn search_mantissa(mantissa: i32, m_bias: f32, range: (f32, f32), sample: f32, quantizer: Quantizator) -> f32 {
    let polarity = if sample.is_sign_positive() { 1_f32 } else { -1_f32 };
    let sample_abs = sample.abs();

    let high_end = if range.0 > range.1 { range.0 } else { range.1 };
    let low_end  = if range.0 < range.1 { range.0 } else { range.1 };

    if sample_abs < low_end {
        return quantizer.quantize_abs(low_end, 0., sample_abs) * polarity;
    }

    if mantissa == 0 {
        return quantizer
            .quantize_abs(high_end, low_end, sample.abs())
            * polarity;
    }

    let sample_range = high_end - low_end;
    let mut index_range = IndexRange {
        start: 0,
        length: mantissa,
    };


    loop {
        // found the two closest values
        if index_range.length <= 2 {
            let index_one = index_range.start;
            let index_two = index_range.start + 1;

            let sample_one = find_m_sample(high_end, sample_range, mantissa, index_one, m_bias);
            let sample_two = find_m_sample(high_end, sample_range, mantissa, index_two, m_bias);

            break quantizer.quantize_abs(sample_one, sample_two, sample_abs) * polarity;
        }

        let curr_index = index_range.center();

        let curr_sample = find_m_sample(high_end, sample_range, mantissa, curr_index, m_bias);

        // found the value
        if curr_sample == sample_abs { break sample; }

        let curr_err = curr_sample - sample_abs;

        index_range.cull(curr_index, curr_sample.is_sign_positive());
    }

}

fn find_m_sample(high_end: f32, sample_range: f32, mantissa: i32, index: i32, m_bias: f32) -> f32 {
    high_end - if m_bias == 1. {
        let step_size = sample_range / mantissa as f32;
        step_size * index as f32
    } else {
        // normalize mantissa to 0.0 - 1.0
        let m = index as f32 / mantissa as f32;
        let position =
            (m_bias.powf(m) - 1.) / (m_bias - 1.);
        position * sample_range
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_mantissa_zero() {
        let quantizer = Quantizator::from_i32(0);
        let sample_one = search_mantissa(0, 1., (1., 0.), 0.6, quantizer);
        let sample_two = search_mantissa(0, 1., (1., 0.), -0.6, quantizer);

        assert_eq!(1., sample_one);
        assert_eq!(-1., sample_two);
    }

    #[test]
    fn linear_mantissa_one() {
        let quantizer = Quantizator::from_i32(0);
        let sample_one = search_mantissa(1, 1., (1., 0.), 0.6, quantizer);
        let sample_two = search_mantissa(1, 1., (1., 0.), -0.6, quantizer);

        assert_eq!(1., sample_one);
        assert_eq!(-1., sample_two);
    }

    #[test]
    fn linear_mantissa_two() {
        let quantizer = Quantizator::from_i32(0);
        let sample_one = search_mantissa(2, 1., (1., 0.), 0.6, quantizer);
        let sample_two = search_mantissa(2, 1., (1., 0.), -0.6, quantizer);

        assert_eq!(0.5, sample_one);
        assert_eq!(-0.5, sample_two);
    }

    #[test]
    fn linear_mantissa_four() {
        let quantizer = Quantizator::from_i32(0);
        let sample_one = search_mantissa(4, 1., (1., 0.), 0.6, quantizer);
        let sample_two = search_mantissa(4, 1., (1., 0.), -0.6, quantizer);

        assert_eq!(0.5, sample_one);
        assert_eq!(-0.5, sample_two);
    }

    #[test]
    fn linear_mantissa_eight() {
        let quantizer = Quantizator::from_i32(0);
        let sample_one = search_mantissa(8, 1., (1., 0.), 0.6, quantizer);
        let sample_two = search_mantissa(8, 1., (1., 0.), -0.6, quantizer);

        assert_eq!(0.5, sample_one);
        assert_eq!(-0.5, sample_two);
    }

    #[test]
    fn linear_mantissa_sixteen() {
        let quantizer = Quantizator::from_i32(0);
        let sample_one = search_mantissa(16, 1., (1., 0.), 0.6, quantizer);
        let sample_two = search_mantissa(16, 1., (1., 0.), -0.6, quantizer);

        assert_eq!(0.5, sample_one);
        assert_eq!(-0.5, sample_two);
    }
}
