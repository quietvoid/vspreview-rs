use super::Previewer;

use conrod_core::{UiCell, widget, color, Colorable, Positionable, Sizeable, Widget, Borderable};
use conrod_core::widget::text_box;

pub struct PreviewUi {
    text_box_value: String,
    scaled_ui: f64,
}

impl PreviewUi {
    pub fn new(text_box_value: String, scaled_ui: f64) -> Self {
        Self {
            text_box_value,
            scaled_ui,
        }
    }

    pub fn gui(&mut self, ui: &mut UiCell, ids: &Ids, previewer: &mut Previewer, script_info: &str, window_width: f64) {
        widget::Canvas::new()
            .mid_bottom()
            .w(window_width)
            .h(self.scaled_ui)
            .color(conrod_core::color::TRANSPARENT)
            .border(0.0)
            .set(ids.canvas, ui);
    
        let current_frame = previewer.get_current_no();
    
        let max = previewer.get_clip_length();
        let slider_width = window_width / 1.5;
        let pointer_width = -50.0 + (current_frame as f64 / max as f64) * slider_width;
    
        if let Some(val) = widget::Slider::new(current_frame as f32, 0.0, max as f32)
            .mid_bottom_with_margin(55.0)
            .w_h(slider_width, 20.0)
            .rgba(0.75, 0.75, 0.75, 1.00)
            .set(ids.slider, ui)
        {
            previewer.seek_to(val.into());
        }
    
        widget::Text::new(&current_frame.to_string())
            .bottom_left_with_margins_on(ids.slider, 20.0, pointer_width)
            .rgba(0.75, 0.75, 0.75, 1.00)
            .font_size(32)
            .set(ids.min_label, ui);
    
        widget::Text::new(script_info)
            .bottom_left_with_margins_on(ids.canvas, 15.0, 10.0)
            .rgba(0.75, 0.75, 0.75, 1.00)
            .font_size(26)
            .set(ids.frame_info, ui);
    
        for event in widget::TextBox::new(&self.text_box_value)
            .bottom_left_with_margins_on(ids.canvas, 40.0, 10.0)
            .rgba(0.25, 0.25, 0.25, 0.5)
            .w(70.0)
            .font_size(20)
            .text_color(color::LIGHT_GREY)
            .right_justify()
            .set(ids.frame_no_box, ui)
        {
            match event {
                text_box::Event::Update(s) => self.text_box_value = s,
                text_box::Event::Enter => {
                    // Only allow seeking to numeric strings
                    if let Ok(frame_no) = self.text_box_value.parse::<u32>() {
                        if frame_no > max {
                            self.text_box_value = max.to_string();
                        } else {
                            previewer.seek_to(frame_no as f64);
                            self.text_box_value = frame_no.to_string();
                        }
                    }
                }
            };
        }
    }

    pub fn update_frame(&mut self, frame_no: String) {
        self.text_box_value = frame_no;
    }
}

widget_ids!(pub struct Ids { canvas, slider, min_label, frame_info, frame_no_box });
