use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

use crate::FloatCrushParams;

mod knob;
use knob::{ParamKnob, ParamKnobStyle};

#[derive(Lens)]
struct Data {
    params: Arc<FloatCrushParams>
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (200, 450))
}

pub(crate) fn create(
    params: Arc<FloatCrushParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data { params: params.clone() }
            .build(cx);

        ResizeHandle::new(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "float_crush")
                .font_family(vec![FamilyOwned::Name(String::from(
                    assets::NOTO_SANS_THIN,
                ))])
                .font_size(30.0)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));

            Label::new(cx, "input_gain");
            ParamKnob::new(cx, ParamKnobStyle::FromLeft, Data::params, |params| &params.input_gain);

            Label::new(cx, "round");
            ParamKnob::new(cx, ParamKnobStyle::Centered, Data::params, |params| &params.round);

            Label::new(cx, "exponent");
            ParamKnob::new(cx, ParamKnobStyle::FromLeft, Data::params, |params| &params.exponent);

            Label::new(cx, "exponent_base");
            ParamKnob::new(cx, ParamKnobStyle::Centered, Data::params, |params| &params.exponent_bias);

            Label::new(cx, "mantissa");
            ParamKnob::new(cx, ParamKnobStyle::FromLeft, Data::params, |params| &params.mantissa);

            Label::new(cx, "mantissa_bias");
            ParamKnob::new(cx, ParamKnobStyle::Centered, Data::params, |params| &params.mantissa_bias);

            Label::new(cx, "dry_gain");
            ParamKnob::new(cx, ParamKnobStyle::FromLeft, Data::params, |params| &params.dry);

            Label::new(cx, "wet_gain");
            ParamKnob::new(cx, ParamKnobStyle::FromLeft, Data::params, |params| &params.wet);
        })
        .row_between(Pixels(0.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    })
}
