use super::{
    egui, egui::RichText, update_input_key_state, VSPreviewer, MAX_ZOOM, MIN_ZOOM,
    STATE_LABEL_COLOR,
};
use anyhow::{anyhow, Result};
use itertools::Itertools;

pub struct UiControls {}

impl UiControls {
    pub fn ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) {
        egui::Grid::new("controls_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                let mut res = Self::output_select_ui(pv, ui);
                pv.add_error("preview", &res);
                ui.end_row();

                res = Self::zoom_slider_ui(pv, ui);
                pv.add_error("preview", &res);
                ui.end_row();

                res = Self::translate_drag_ui(pv, ui);
                pv.add_error("preview", &res);
                ui.end_row();
            });
    }

    pub fn output_select_ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> Result<()> {
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

            let out = pv
                .outputs
                .get_mut(&old_output)
                .ok_or_else(|| anyhow!("output_select_ui: Invalid old output key"))?;
            out.original_props = None;

            if pv.output_needs_rerender(old_output)? {
                pv.rerender = true;
            }
        }

        Ok(())
    }

    pub fn zoom_slider_ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> Result<()> {
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

            pv.correct_translate_for_current_output(pv.state.translate, false)?;
        }

        Ok(())
    }

    pub fn translate_drag_ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> Result<()> {
        let old_translate = pv.state.translate_norm;
        let mut new_translate = old_translate;

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

        if old_translate != new_translate {
            // Fix and update state
            pv.correct_translate_for_current_output(new_translate, true)?;
        }

        Ok(())
    }
}
