use super::{
    egui, egui::RichText, update_input_key_state, VSPreviewer, MAX_ZOOM, MIN_ZOOM,
    STATE_LABEL_COLOR,
};
use itertools::Itertools;

pub struct UiControls {}

impl UiControls {
    pub fn ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) {
        egui::Grid::new("controls_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                Self::output_select_ui(pv, ui);
                ui.end_row();

                Self::zoom_slider_ui(pv, ui);
                ui.end_row();

                Self::translate_drag_ui(pv, ui);
                ui.end_row();
            });
    }

    pub fn output_select_ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) {
        let old_output = pv.state.cur_output;
        let mut new_output = old_output;

        ui.label(RichText::new("Output").color(STATE_LABEL_COLOR));

        egui::ComboBox::from_id_source(egui::Id::new("output_select"))
            .selected_text(format!("Output {}", new_output))
            .show_ui(ui, |ui| {
                for i in pv.outputs.keys().sorted() {
                    ui.selectable_value(&mut new_output, *i, format!("Output {}", i));
                }
            });

        // Changed output
        if new_output != old_output {
            pv.state.cur_output = new_output;

            let out = pv.outputs.get_mut(&old_output).unwrap();
            out.original_props = None;

            if pv.output_needs_rerender(old_output) {
                pv.rerender = true;
            }
        }
    }

    pub fn zoom_slider_ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) {
        let old_zoom = pv.state.zoom_factor;
        let mut new_zoom = old_zoom;

        let zoom_range = MIN_ZOOM..=MAX_ZOOM;
        let frames_slider = egui::Slider::new(&mut new_zoom, zoom_range).max_decimals(3);

        ui.label(RichText::new("Zoom factor").color(STATE_LABEL_COLOR));
        let res = ui.add(frames_slider);

        let in_use = res.has_focus() || res.drag_started();
        update_input_key_state(&mut pv.inputs_focused, "zoom_factor_dragval", in_use, &res);

        if new_zoom != old_zoom {
            pv.state.zoom_factor = new_zoom;
            pv.rerender = true;

            pv.correct_translate_for_current_output(pv.state.translate, false);
        }
    }

    pub fn translate_drag_ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) {
        let mut new_translate = pv.state.translate_norm;

        ui.label(RichText::new("Translate").color(STATE_LABEL_COLOR));
        ui.horizontal(|ui| {
            let x_drag = egui::DragValue::new(&mut new_translate.x)
                .speed(0.01)
                .clamp_range(0.0..=1.0)
                .max_decimals(3);

            let y_drag = egui::DragValue::new(&mut new_translate.y)
                .speed(0.01)
                .clamp_range(0.0..=1.0)
                .max_decimals(3);

            ui.label(RichText::new("x").color(STATE_LABEL_COLOR));
            let res = ui.add(x_drag);

            let in_use = res.has_focus() || res.drag_started();
            update_input_key_state(&mut pv.inputs_focused, "translate_x_dragval", in_use, &res);

            ui.label(RichText::new("y").color(STATE_LABEL_COLOR));
            let res = ui.add(y_drag);

            let in_use = res.has_focus() || res.drag_started();
            update_input_key_state(&mut pv.inputs_focused, "translate_y_dragval", in_use, &res);
        });

        // Fix and update state
        pv.correct_translate_for_current_output(new_translate, true);
    }
}
