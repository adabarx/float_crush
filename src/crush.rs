

pub fn search_mantissa(mantissa: u32, m_bias: f32, range: SampleRange, sample: f32, quantizer: Quantizator) -> f32 {
    let sample_abs = sample.abs();
    let polarity = sample.polarity();

    if sample_abs < range.low {
        return quantizer.quantize_abs(range.low, 0., sample_abs) * polarity;
    }

    if mantissa == 0 {
        return quantizer.quantize_abs(range.high, range.low, sample_abs) * polarity;
    }

    let search_type = SearchType::Mantissa(mantissa, m_bias);

    let mut search_range = SearchRange::new(search_type, range, sample_abs).unwrap();


    loop {
        match search_range.cull() {
            CullResult::ExactMatch(sample_abs) =>
                break sample_abs * polarity,
            CullResult::TwoLeft(upper, lower, sample_abs) =>
                break quantizer.quantize_abs(upper, lower, sample_abs) * polarity,
            CullResult::CutHalf => (),
        }
    }

}

pub trait Polarity {
    fn polarity(&self) -> f32;
}

impl Polarity for f32 {
    fn polarity(&self) -> f32 {
        if self.is_sign_positive() { 1_f32 } else { -1_f32 }
    }
}

pub fn mix_dry_wet(dry: f32, dry_gain: f32, wet: f32, wet_gain: f32) -> f32 {
    (dry * dry_gain) + (wet * wet_gain)
}

#[derive(Clone, Copy)]
 pub enum Quantizator {
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

pub enum SearchType {
    // u32 is the mantissa/exponent length
    // f32 is the bias
    Mantissa(u32, f32),
    Exponent(u32, f32),
}

impl SearchType {
    pub fn length(&self) -> u32 {
        match self {
            &Self::Mantissa(length, _) => length,
            &Self::Exponent(length, _) => length,
        }
    }

    pub fn get_sample(&self, index: u32, range: SampleRange) -> f32 {
        match self {
            &Self::Mantissa(length, bias) => {
                range.high - if bias == 1. {
                    let step_size = range.distance() / length as f32;
                    step_size * index as f32
                } else {
                    // normalize mantissa to 0.0 - 1.0
                    let m = index as f32 / length as f32;
                    let position = (bias.powf(m) - 1.) / (bias - 1.);
                    position * range.distance()
                }
            },
            &Self::Exponent(_, bias) => {
                bias.powi(index as i32 * -1)
            },
        }
    }
}

pub struct SearchRange {
    pub start: u32,
    pub length: u32,
    pub search_type: SearchType,
    pub range: SampleRange,
    pub sample: f32,
}

impl SearchRange {
    pub fn new(
        search_type: SearchType,
        range: SampleRange,
        sample_abs: f32
    ) -> anyhow::Result<Self> {
        if !range.in_range(sample_abs) { anyhow::bail!("not in range") }
        else {
            Ok(Self {
                start: 0,
                length: search_type.length(),
                search_type,
                range,
                sample: sample_abs
            })
        }
    }

    pub fn center(&self) -> u32 {
        self.start + self.half_length()
    }

    fn half_length(&self) -> u32 {
        (self.length as f32 / 2.).floor() as u32
    }

    pub fn cull(&mut self) -> CullResult {
        let c_sample = self.center_sample();
        if self.length == 1 {
            let start = self.search_type.get_sample(self.start, self.range);
            let end = self.search_type.get_sample(self.start + 1, self.range);
            CullResult::TwoLeft(start, end, self.sample)
        } else if c_sample == self.sample {
            CullResult::ExactMatch(self.sample)
        } else if c_sample > self.sample {
            self.start = self.center();
            self.length = self.length - self.half_length();
            CullResult::CutHalf
        } else {
            self.length = self.length - self.half_length();
            CullResult::CutHalf
        }
    }

    pub fn center_sample(&self) -> f32 {
        self.search_type.get_sample(self.center(), self.range)
    }
}


pub enum CullResult {
    CutHalf,
    TwoLeft(f32, f32, f32),
    ExactMatch(f32)
}

#[derive(Clone, Copy)]
pub struct SampleRange {
    high: f32,
    low: f32
}

impl SampleRange {
    pub fn new(a: f32, b: f32) -> SampleRange {
        if a >= b { SampleRange { high: a, low: b } }
        else { SampleRange { high: b, low: a } }
    }

    pub fn distance (&self) -> f32 {
        self.high - self.low
    }

    pub fn in_range(&self, sample: f32) -> bool {
        sample >= self.low && sample <= self.high
    }
}


fn find_m_sample(high_end: f32, sample_range: f32, mantissa: u32, index: u32, m_bias: f32) -> f32 {
    high_end - if m_bias == 1. {
        let step_size = sample_range / mantissa as f32;
        step_size * index as f32
    } else {
        // normalize mantissa to 0.0 - 1.0
        let m = index as f32 / mantissa as f32;
        let position = (m_bias.powf(m) - 1.) / (m_bias - 1.);
        position * sample_range
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! linear_mantissa {
        ($bits:expr, $sample:expr, $expected:expr) => {
            paste::item! {
                #[test]
                fn [ < linear_mantissa_ $bits > ] () {
                    let sample_one = search_mantissa(
                        $bits,
                        1.,
                        SampleRange::new(1., 0.),
                        $sample,
                        Quantizator::Nearest,
                    );

                    assert_eq!($expected, sample_one);
                }
            }
        };
    }

    linear_mantissa!(0, 0.6, 1.);
    linear_mantissa!(1, 0.6, 1.);
    linear_mantissa!(2, 0.6, 0.5);
    linear_mantissa!(4, 0.6, 0.5);
    linear_mantissa!(8, 0.6, 0.625);
    linear_mantissa!(16, 0.6, 0.625);
    linear_mantissa!(32, 0.6, 0.59375);
    linear_mantissa!(64, 0.6, 0.59375);

    #[test]
    fn test_find_m_sample() {
        let s = find_m_sample(1., 1., 10, 0, 1.);
        assert_eq!(s, 1.);
        let s = find_m_sample(1., 1., 10, 1, 1.);
        assert_eq!(s, 0.9);
        let s = find_m_sample(1., 1., 10, 2, 1.);
        assert_eq!(s, 0.8);
        let s = find_m_sample(1., 1., 4, 3, 1.);
        assert_eq!(s, 0.25);
    }

    #[test]
    fn sample_range() {
        let t = SampleRange::new(4., 5.);

        assert_eq!(t.high, 5.);
        assert_eq!(t.low, 4.);
        assert_eq!(t.distance(), 1.);
    }
}
