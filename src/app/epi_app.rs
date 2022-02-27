use eframe::egui::style::Margin;
use eframe::epaint::{Color32, Stroke};
use eframe::{
    egui::{self, Frame},
    epi,
};

use super::*;

impl epi::App for VSPreviewer {
    fn name(&self) -> &str {
        "vspreview-rs"
    }

    fn setup(
        &mut self,
        ctx: &egui::Context,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load existing or default state
        if let Some(storage) = _storage {
            self.state = epi::get_value(storage, epi::APP_KEY).unwrap_or(PreviewState {
                zoom_factor: 1.0,
                zoom_multiplier: 1.0,
                scroll_multiplier: 1.0,
                canvas_margin: 0.0,
                ..Default::default()
            });
        }

        // Set the global theme, default to dark mode
        let mut global_visuals = egui::style::Visuals::dark();
        global_visuals.window_shadow = egui::epaint::Shadow::small_light();
        ctx.set_visuals(global_visuals);

        // Fix invalid state options
        if self.state.scroll_multiplier <= 0.0 {
            self.state.scroll_multiplier = 1.0;
        }

        // Limit to 2.0 multiplier every zoom, should be plenty
        if self.state.zoom_multiplier < 1.0 {
            self.state.zoom_multiplier = 1.0;
        } else if self.state.zoom_multiplier > 2.0 {
            self.state.zoom_multiplier = 2.0;
        }

        // Request initial outputs
        self.reload(frame.clone());
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let promise_res = self.check_promise_callbacks(ctx, frame);
        self.add_error("callbacks", promise_res);

        let panel_frame = Frame::default()
            .fill(Color32::from_gray(51))
            .margin(Margin::same(self.state.canvas_margin))
            .stroke(Stroke::none());

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                // Check for quit, GUI toggle, etc.
                self.check_misc_keyboard_inputs(frame, ui);

                // React on canvas resolution change
                if self.available_size != ui.available_size() {
                    self.available_size = ui.available_size();
                    self.reprocess_outputs();
                }

                let preview_res = PreviewerMainUi::ui(self, ctx, frame, ui);
                self.add_error("preview", preview_res);

                // Display errors if any
                ErrorWindowUi::ui(self, ctx);
            });
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, &self.state);
    }
}
