// The MIT License (MIT)

// Copyright (c) 2014 PistonDevelopers

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A widget for displaying and mutating a one-line field of text.
//! From https://github.com/PistonDevelopers/conrod/blob/master/conrod_core/src/widget/text_box.rs
//! Renamed and modified.

use conrod_core::event;
use conrod_core::input;
use conrod_core::position::{Range, Rect, Scalar};
use conrod_core::text;
use conrod_core::widget;
use conrod_core::{Borderable, Color, Colorable, FontSize, Positionable, Sizeable, Widget};

/// A widget for displaying and mutating a small, one-line field of text, given by the user in the
/// form of a `String`.
///
/// It's reaction is triggered upon pressing of the `Enter`/`Return` key.
#[derive(WidgetCommon)]
pub struct NumberBox<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    number: &'a str,
    style: Style,
}

/// Unique graphical styling for the NumberBox.
#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {
    /// The length of the gap between the bounding rectangle's border and the edge of the text.
    #[conrod(default = "5.0")]
    pub text_padding: Option<Scalar>,
    /// Color of the rectangle behind the text.
    ///
    /// If you don't want to see the rectangle, either set the color with a zeroed alpha or use
    /// the `TextEdit` widget directly.
    #[conrod(default = "theme.shape_color")]
    pub color: Option<Color>,
    /// The width of the bounding `BorderedRectangle` border.
    #[conrod(default = "theme.border_width")]
    pub border: Option<Scalar>,
    /// The color of the `BorderedRecangle`'s border.
    #[conrod(default = "theme.border_color")]
    pub border_color: Option<Color>,
    /// The color of the `TextEdit` widget.
    #[conrod(default = "theme.label_color")]
    pub text_color: Option<Color>,
    /// The font size for the text.
    #[conrod(default = "theme.font_size_medium")]
    pub font_size: Option<FontSize>,
    /// The typographic alignment of the text.
    #[conrod(default = "text::Justify::Left")]
    pub justify: Option<text::Justify>,
    /// The font used for the `Text`.
    #[conrod(default = "theme.font_id")]
    pub font_id: Option<Option<text::font::Id>>,
}

widget_ids! {
    struct Ids {
        text_edit,
        rectangle,
    }
}

/// The `State` of the `NumberBox` widget that will be cached within the `Ui`.
pub struct State {
    ids: Ids,
}

impl<'a> NumberBox<'a> {
    /// Construct a NumberBox widget.
    pub fn new(number: &'a str) -> Self {
        NumberBox {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            number,
        }
    }

    /// Align the text to the right of its bounding **Rect**'s *x* axis range.
    pub fn right_justify(self) -> Self {
        self.justify(text::Justify::Right)
    }

    builder_methods! {
        pub text_color { style.text_color = Some(Color) }
        pub font_size { style.font_size = Some(FontSize) }
        pub justify { style.justify = Some(text::Justify) }
        pub pad_text { style.text_padding = Some(Scalar) }
    }
}

/// Events produced by the `NumberBox`.
#[derive(Clone, Debug)]
pub enum Event {
    /// The `String` was updated.
    Update(String),
    /// The `Return` or `Enter` key was pressed.
    Enter,
    Click,
    Unfocus,
}

impl<'a> Widget for NumberBox<'a> {
    type State = State;
    type Style = Style;
    type Event = Vec<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    /// Update the state of the TextEdit.
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            style,
            ui,
            ..
        } = args;
        let NumberBox { number, .. } = self;

        let font_size = style.font_size(ui.theme());
        let border = style.border(ui.theme());
        let text_padding = style.text_padding(ui.theme());
        let justify = style.justify(ui.theme());

        let text_rect = {
            let w = rect.x.pad(border + text_padding).len();
            let h = font_size as Scalar + 1.0;
            let x = Range::new(0.0, w).align_middle_of(rect.x);
            let y = Range::new(0.0, h).align_middle_of(rect.y);
            Rect { x: x, y: y }
        };

        let color = style.color(ui.theme());
        let border_color = style.border_color(ui.theme());
        widget::BorderedRectangle::new(rect.dim())
            .xy(rect.xy())
            .graphics_for(id)
            .parent(id)
            .border(border)
            .color(color)
            .border_color(border_color)
            .set(state.ids.rectangle, ui);

        let mut events = Vec::new();

        let text_color = style.text_color(ui.theme());
        let font_id = style.font_id(&ui.theme).or(ui.fonts.ids().next());
        if let Some(new_string) = widget::TextEdit::new(&number.to_string())
            .and_then(font_id, widget::TextEdit::font_id)
            .wh(text_rect.dim())
            .xy(text_rect.xy())
            .font_size(font_size)
            .color(text_color)
            .justify(justify)
            .parent(id)
            .set(state.ids.text_edit, ui)
        {
            if let Ok(_) = new_string.parse::<u32>() {
                events.push(Event::Update(new_string));
            } else if new_string.is_empty() {
                events.push(Event::Update(new_string));
            }
        }

        // Produce an event for any `Enter`/`Return` presses.
        //
        // TODO: We should probably be doing this via the `TextEdit` widget.
        for widget_event in ui.widget_input(state.ids.text_edit).events() {
            match widget_event {
                event::Widget::Press(press) => match press.button {
                    event::Button::Keyboard(key) => match key {
                        input::Key::Return => events.push(Event::Enter),
                        _ => (),
                    },
                    _ => (),
                },
                event::Widget::Click(c) => match c.button {
                    input::MouseButton::Left => events.push(Event::Click),
                    _ => (),
                },
                event::Widget::UncapturesInputSource(s) => match s {
                    input::Source::Keyboard => events.push(Event::Unfocus),
                    _ => (),
                },
                _ => (),
            }
        }

        events
    }
}

impl<'a> Borderable for NumberBox<'a> {
    builder_methods! {
        border { style.border = Some(Scalar) }
        border_color { style.border_color = Some(Color) }
    }
}

impl<'a> Colorable for NumberBox<'a> {
    builder_method!(color { style.color = Some(Color) });
}
