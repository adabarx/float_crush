#![allow(unused)]
use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;

use nih_plug_vizia::widgets::util;
use nih_plug_vizia::widgets::{
    param_base::ParamWidgetBase,
    util::ModifiersExt,
};

const GRANULAR_DRAG_MULTIPLIER: f32 = 0.1;

#[derive(Lens)]
pub struct ParamKnob {
    param_base: ParamWidgetBase,

    text_input_active: bool,

    drag_active: bool,

    granular_drag_status: Option<GranularDragStatus>,

    use_scroll_wheel: bool,

    scrolled_lines: f32,

    style: ParamKnobStyle,

    label_override: Option<String>,
}

enum ParamKnobEvent {
    CancelTextInput,
    TextInput(String),
}

// where the value should be displayed relative to the knob
#[derive(Debug, Clone, Copy, PartialEq, Eq, Data)]
pub enum ParamKnobStyle {
    Centered,
    FromLeft,
    CurrentStep { even: bool },
    CurrentStepLabeled { even: bool },
}

#[derive(Clone, Copy)]
pub struct GranularDragStatus {
    pub starting_x_coordinate: f32,
    pub starting_value: f32,
}

impl ParamKnob {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        style: ParamKnobStyle,
        params: L,
        params_to_param: FMap,
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params.clone(), params_to_param),

            text_input_active: false,

            drag_active: false,

            granular_drag_status: None,

            use_scroll_wheel: true,

            scrolled_lines: 0.,

            style,

            label_override: None,
        }
        .build(
            cx,
            ParamWidgetBase::build_view(params, params_to_param, move |cx, param_data| {
                Binding::new(cx, ParamKnob::style, move |cx, style| {
                    let style = style.get(cx);

                    // Needs to be moved into the below closures, and it can't be `Copy`
                    let param_data = param_data.clone();

                    // Can't use `.to_string()` here as that would include the modulation.
                    let unmodulated_normalized_value_lens =
                        param_data.make_lens(|param| param.unmodulated_normalized_value());
                    let display_value_lens = param_data.make_lens(|param| {
                        param.normalized_value_to_string(param.unmodulated_normalized_value(), true)
                    });

                    // The resulting tuple `(start_t, delta)` corresponds to the start and the
                    // signed width of the bar. `start_t` is in `[0, 1]`, and `delta` is in
                    // `[-1, 1]`.
                    let fill_start_delta_lens = {
                        let param_data = param_data.clone();
                        unmodulated_normalized_value_lens.map(move |current_value| {
                            Self::compute_fill_start_delta(
                                style,
                                param_data.param(),
                                *current_value,
                            )
                        })
                    };

                    // If the parameter is being modulated by the host (this only works for CLAP
                    // plugins with hosts that support this), then this is the difference
                    // between the 'true' value and the current value after modulation has been
                    // applied. This follows the same format as `fill_start_delta_lens`.
                    let modulation_start_delta_lens = param_data.make_lens(move |param| {
                        Self::compute_modulation_fill_start_delta(style, param)
                    });

                    // This is used to draw labels for `CurrentStepLabeled`
                    let make_preview_value_lens = {
                        let param_data = param_data.clone();
                        move |normalized_value| {
                            param_data.make_lens(move |param| {
                                param.normalized_value_to_string(normalized_value, true)
                            })
                        }
                    };

                    // Only draw the text input widget when it gets focussed. Otherwise, overlay the
                    // label with the slider. Creating the textbox based on
                    // `ParamKnobInternal::text_input_active` lets us focus the textbox when it gets
                    // created.
                    Binding::new(
                        cx,
                        ParamKnob::text_input_active,
                        move |cx, text_input_active| {
                            if text_input_active.get(cx) {
                                Self::text_input_view(cx, display_value_lens.clone());
                            } else {
                                // All of this data needs to be moved into the `ZStack` closure, and
                                // the `Map` lens combinator isn't `Copy`
                                let param_data = param_data.clone();
                                let fill_start_delta_lens = fill_start_delta_lens.clone();
                                let modulation_start_delta_lens =
                                    modulation_start_delta_lens.clone();
                                let display_value_lens = display_value_lens.clone();
                                let make_preview_value_lens = make_preview_value_lens.clone();

                                ZStack::new(cx, move |cx| {
                                    Self::slider_fill_view(
                                        cx,
                                        fill_start_delta_lens,
                                        modulation_start_delta_lens,
                                    );
                                    Self::slider_label_view(
                                        cx,
                                        param_data.param(),
                                        style,
                                        display_value_lens,
                                        make_preview_value_lens,
                                        ParamKnob::label_override,
                                    );
                                })
                                .hoverable(false);
                            }
                        },
                    );
                });
            })
        )
    }

    fn text_input_view(cx: &mut Context, display_value_lens: impl Lens<Target = String>) {
        Textbox::new(cx, display_value_lens)
            .class("value-entry")
            .on_submit(|cx, string, success| {
                if success {
                    cx.emit(ParamKnobEvent::TextInput(string))
                } else {
                    cx.emit(ParamKnobEvent::CancelTextInput);
                }
            })
            .on_build(|cx| {
                cx.emit(TextEvent::StartEdit);
                cx.emit(TextEvent::SelectAll);
            })
            // `.child_space(Stretch(1.0))` no longer works
            .class("align_center")
            .child_top(Stretch(1.0))
            .child_bottom(Stretch(1.0))
            .height(Stretch(1.0))
            .width(Stretch(1.0));
    }

    fn slider_fill_view(
        cx: &mut Context,
        fill_start_delta_lens: impl Lens<Target = (f32, f32)>,
        modulation_start_delta_lens: impl Lens<Target = (f32, f32)>,
    ) {
        // The filled bar portion. This can be visualized in a couple different ways depending on
        // the current style property. See [`ParamKnobStyle`].
        Element::new(cx)
            .class("fill")
            .height(Stretch(1.0))
            .left(
                fill_start_delta_lens
                    .clone()
                    .map(|(start_t, _)| Percentage(start_t * 100.0)),
            )
            .width(fill_start_delta_lens.map(|(_, delta)| Percentage(delta * 100.0)))
            // Hovering is handled on the param slider as a whole, this
            // should not affect that
            .hoverable(false);

        // If the parameter is being modulated, then we'll display another
        // filled bar showing the current modulation delta
        // VIZIA's bindings make this a bit, uh, difficult to read
        Element::new(cx)
            .class("fill")
            .class("fill--modulation")
            .height(Stretch(1.0))
            .visibility(
                modulation_start_delta_lens
                    .clone()
                    .map(|(_, delta)| *delta != 0.0),
            )
            // Widths cannot be negative, so we need to compensate the start
            // position if the width does happen to be negative
            .width(
                modulation_start_delta_lens
                    .clone()
                    .map(|(_, delta)| Percentage(delta.abs() * 100.0)),
            )
            .left(modulation_start_delta_lens.map(|(start_t, delta)| {
                if *delta < 0.0 {
                    Percentage((start_t + delta) * 100.0)
                } else {
                    Percentage(start_t * 100.0)
                }
            }))
            .hoverable(false);
    }

    fn slider_label_view<P: Param, L: Lens<Target = String>>(
        cx: &mut Context,
        param: &P,
        style: ParamKnobStyle,
        display_value_lens: impl Lens<Target = String>,
        make_preview_value_lens: impl Fn(f32) -> L,
        label_override_lens: impl Lens<Target = Option<String>>,
    ) {
        let step_count = param.step_count();

        // Either display the current value, or display all values over the
        // parameter's steps
        // TODO: Do the same thing as in the iced widget where we draw the
        //       text overlapping the fill area slightly differently. We can
        //       set the cip region directly in vizia.
        match (style, step_count) {
            (ParamKnobStyle::CurrentStepLabeled { .. }, Some(step_count)) => {
                HStack::new(cx, |cx| {
                    // There are step_count + 1 possible values for a
                    // discrete parameter
                    for value in 0..step_count + 1 {
                        let normalized_value = value as f32 / step_count as f32;
                        let preview_lens = make_preview_value_lens(normalized_value);

                        Label::new(cx, preview_lens)
                            .class("value")
                            .class("value--multiple")
                            .child_space(Stretch(1.0))
                            .height(Stretch(1.0))
                            .width(Stretch(1.0))
                            .hoverable(false);
                    }
                })
                .height(Stretch(1.0))
                .width(Stretch(1.0))
                .hoverable(false);
            }
            _ => {
                Binding::new(cx, label_override_lens, move |cx, label_override_lens| {
                    // If the label override is set then we'll use that. If not, the parameter's
                    // current display value (before modulation) is used.
                    match label_override_lens.get(cx) {
                        Some(label_override) => Label::new(cx, &label_override),
                        None => Label::new(cx, display_value_lens.clone()),
                    }
                    .class("value")
                    .class("value--single")
                    .child_space(Stretch(1.0))
                    .height(Stretch(1.0))
                    .width(Stretch(1.0))
                    .hoverable(false);
                });
            }
        };
    }

    /// Calculate the start position and width of the slider's fill region based on the selected
    /// style, the parameter's current value, and the parameter's step sizes. The resulting tuple
    /// `(start_t, delta)` corresponds to the start and the signed width of the bar. `start_t` is in
    /// `[0, 1]`, and `delta` is in `[-1, 1]`.
    fn compute_fill_start_delta<P: Param>(
        style: ParamKnobStyle,
        param: &P,
        current_value: f32,
    ) -> (f32, f32) {
        let default_value = param.default_normalized_value();
        let step_count = param.step_count();
        let draw_fill_from_default = matches!(style, ParamKnobStyle::Centered)
            && step_count.is_none()
            && (0.45..=0.55).contains(&default_value);

        match style {
            ParamKnobStyle::Centered if draw_fill_from_default => {
                let delta = (default_value - current_value).abs();

                // Don't draw the filled portion at all if it could have been a
                // rounding error since those slivers just look weird
                (
                    default_value.min(current_value),
                    if delta >= 1e-3 { delta } else { 0.0 },
                )
            }
            ParamKnobStyle::Centered | ParamKnobStyle::FromLeft => (0.0, current_value),
            ParamKnobStyle::CurrentStep { even: true }
            | ParamKnobStyle::CurrentStepLabeled { even: true }
                if step_count.is_some() =>
            {
                // Assume the normalized value is distributed evenly
                // across the range.
                let step_count = step_count.unwrap() as f32;
                let discrete_values = step_count + 1.0;
                let previous_step = (current_value * step_count) / discrete_values;

                (previous_step, discrete_values.recip())
            }
            ParamKnobStyle::CurrentStep { .. } | ParamKnobStyle::CurrentStepLabeled { .. } => {
                let previous_step = param.previous_normalized_step(current_value, false);
                let next_step = param.next_normalized_step(current_value, false);

                (
                    (previous_step + current_value) / 2.0,
                    ((next_step - current_value) + (current_value - previous_step)) / 2.0,
                )
            }
        }
    }

    /// The same as `compute_fill_start_delta`, but just showing the modulation offset.
    fn compute_modulation_fill_start_delta<P: Param>(
        style: ParamKnobStyle,
        param: &P,
    ) -> (f32, f32) {
        match style {
            // Don't show modulation for stepped parameters since it wouldn't
            // make a lot of sense visually
            ParamKnobStyle::CurrentStep { .. } | ParamKnobStyle::CurrentStepLabeled { .. } => {
                (0.0, 0.0)
            }
            ParamKnobStyle::Centered | ParamKnobStyle::FromLeft => {
                let modulation_start = param.unmodulated_normalized_value();

                (
                    modulation_start,
                    param.modulated_normalized_value() - modulation_start,
                )
            }
        }
    }

    /// `self.param_base.set_normalized_value()`, but resulting from a mouse drag. When using the
    /// 'even' stepped slider styles from [`ParamKnobStyle`] this will remap the normalized range
    /// to match up with the fill value display. This still needs to be wrapped in a parameter
    /// automation gesture.
    fn set_normalized_value_drag(&self, cx: &mut EventContext, normalized_value: f32) {
        let normalized_value = match (self.style, self.param_base.step_count()) {
            (
                ParamKnobStyle::CurrentStep { even: true }
                | ParamKnobStyle::CurrentStepLabeled { even: true },
                Some(step_count),
            ) => {
                // We'll remap the value range to be the same as the displayed range, e.g. with each
                // value occupying an equal area on the slider instead of the centers of those
                // ranges being distributed over the entire `[0, 1]` range.
                let discrete_values = step_count as f32 + 1.0;
                let rounded_value = ((normalized_value * discrete_values) - 0.5).round();
                rounded_value / step_count as f32
            }
            _ => normalized_value,
        };

        self.param_base.set_normalized_value(cx, normalized_value);
    }
}

impl View for ParamKnob {
    fn element(&self) -> Option<&'static str> {
        Some("param-slider")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|param_slider_event, meta| match param_slider_event {
            ParamKnobEvent::CancelTextInput => {
                self.text_input_active = false;
                cx.set_active(false);

                meta.consume();
            }
            ParamKnobEvent::TextInput(string) => {
                if let Some(normalized_value) = self.param_base.string_to_normalized_value(string) {
                    self.param_base.begin_set_parameter(cx);
                    self.param_base.set_normalized_value(cx, normalized_value);
                    self.param_base.end_set_parameter(cx);
                }

                self.text_input_active = false;

                meta.consume();
            }
        });

        event.map(|window_event, meta| match window_event {
            // Vizia always captures the third mouse click as a triple click. Treating that triple
            // click as a regular mouse button makes double click followed by another drag work as
            // expected, instead of requiring a delay or an additional click. Double double click
            // still won't work.
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                if cx.modifiers.alt() {
                    // ALt+Click brings up a text entry dialog
                    self.text_input_active = true;
                    cx.set_active(true);
                } else if cx.modifiers.command() {
                    // Ctrl+Click, double click, and right clicks should reset the parameter instead
                    // of initiating a drag operation
                    self.param_base.begin_set_parameter(cx);
                    self.param_base
                        .set_normalized_value(cx, self.param_base.default_normalized_value());
                    self.param_base.end_set_parameter(cx);
                } else {
                    self.drag_active = true;
                    cx.capture();
                    // NOTE: Otherwise we don't get key up events
                    cx.focus();
                    cx.set_active(true);

                    // When holding down shift while clicking on a parameter we want to granuarly
                    // edit the parameter without jumping to a new value
                    self.param_base.begin_set_parameter(cx);
                    if cx.modifiers.shift() {
                        self.granular_drag_status = Some(GranularDragStatus {
                            starting_x_coordinate: cx.mouse.cursorx,
                            starting_value: self.param_base.unmodulated_normalized_value(),
                        });
                    } else {
                        self.granular_drag_status = None;
                        self.set_normalized_value_drag(
                            cx,
                            util::remap_current_entity_x_coordinate(cx, cx.mouse.cursorx),
                        );
                    }
                }

                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseDown(MouseButton::Right)
            | WindowEvent::MouseDoubleClick(MouseButton::Right)
            | WindowEvent::MouseTripleClick(MouseButton::Right) => {
                // Ctrl+Click, double click, and right clicks should reset the parameter instead of
                // initiating a drag operation
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, self.param_base.default_normalized_value());
                self.param_base.end_set_parameter(cx);

                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    cx.release();
                    cx.set_active(false);

                    self.param_base.end_set_parameter(cx);

                    meta.consume();
                }
            }
            WindowEvent::MouseMove(x, _y) => {
                if self.drag_active {
                    // If shift is being held then the drag should be more granular instead of
                    // absolute
                    if cx.modifiers.shift() {
                        let granular_drag_status =
                            *self
                                .granular_drag_status
                                .get_or_insert_with(|| GranularDragStatus {
                                    starting_x_coordinate: *x,
                                    starting_value: self.param_base.unmodulated_normalized_value(),
                                });

                        // These positions should be compensated for the DPI scale so it remains
                        // consistent
                        let start_x =
                            util::remap_current_entity_x_t(cx, granular_drag_status.starting_value);
                        let delta_x = ((*x - granular_drag_status.starting_x_coordinate)
                            * GRANULAR_DRAG_MULTIPLIER)
                            * cx.style.dpi_factor as f32;

                        self.set_normalized_value_drag(
                            cx,
                            util::remap_current_entity_x_coordinate(cx, start_x + delta_x),
                        );
                    } else {
                        self.granular_drag_status = None;

                        self.set_normalized_value_drag(
                            cx,
                            util::remap_current_entity_x_coordinate(cx, *x),
                        );
                    }
                }
            }
            WindowEvent::KeyUp(_, Some(Key::Shift)) => {
                // If this happens while dragging, snap back to reality uh I mean the current screen
                // position
                if self.drag_active && self.granular_drag_status.is_some() {
                    self.granular_drag_status = None;
                    self.param_base.set_normalized_value(
                        cx,
                        util::remap_current_entity_x_coordinate(cx, cx.mouse.cursorx),
                    );
                }
            }
            WindowEvent::MouseScroll(_scroll_x, scroll_y) if self.use_scroll_wheel => {
                // With a regular scroll wheel `scroll_y` will only ever be -1 or 1, but with smooth
                // scrolling trackpads being a thing `scroll_y` could be anything.
                self.scrolled_lines += scroll_y;

                if self.scrolled_lines.abs() >= 1.0 {
                    let use_finer_steps = cx.modifiers.shift();

                    // Scrolling while dragging needs to be taken into account here
                    if !self.drag_active {
                        self.param_base.begin_set_parameter(cx);
                    }

                    let mut current_value = self.param_base.unmodulated_normalized_value();

                    while self.scrolled_lines >= 1.0 {
                        current_value = self
                            .param_base
                            .next_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines -= 1.0;
                    }

                    while self.scrolled_lines <= -1.0 {
                        current_value = self
                            .param_base
                            .previous_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines += 1.0;
                    }

                    if !self.drag_active {
                        self.param_base.end_set_parameter(cx);
                    }
                }

                meta.consume();
            }
            _ => {}
        });
    }
}

/// Extension methods for [`ParamKnob`] handles.
pub trait ParamKnobExt {
    /// Don't respond to scroll wheel events. Useful when this slider is used as part of a scrolling
    /// view.
    fn disable_scroll_wheel(self) -> Self;

    /// Change how the [`ParamKnob`] visualizes the current value.
    fn set_style(self, style: ParamKnobStyle) -> Self;

    /// Manually set a fixed label for the slider instead of displaying the current value. This is
    /// currently not reactive.
    fn with_label(self, value: impl Into<String>) -> Self;
}

impl ParamKnobExt for Handle<'_, ParamKnob> {
    fn disable_scroll_wheel(self) -> Self {
        self.modify(|param_slider: &mut ParamKnob| param_slider.use_scroll_wheel = false)
    }

    fn set_style(self, style: ParamKnobStyle) -> Self {
        self.modify(|param_slider: &mut ParamKnob| param_slider.style = style)
    }

    fn with_label(self, value: impl Into<String>) -> Self {
        self.modify(|param_slider: &mut ParamKnob| {
            param_slider.label_override = Some(value.into())
        })
    }
}

