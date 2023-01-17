use eframe::egui::style::Margin;
use eframe::egui::{self, Frame};
use eframe::epaint::{Color32, Stroke};

use super::*;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SavedState {
    preview_state: PreviewState,
    transforms: PreviewTransforms,
}

impl VSPreviewer {
    pub fn with_cc(mut self, cc: &eframe::CreationContext) -> Self {
        // Load existing or default state
        if let Some(storage) = cc.storage {
            let saved_state: SavedState =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            self.state = saved_state.preview_state;
            self.transforms = Arc::new(Mutex::new(saved_state.transforms));
        }

        // Set the global theme, default to dark mode
        let mut global_visuals = egui::style::Visuals::dark();
        global_visuals.window_shadow = egui::epaint::Shadow::small_light();
        cc.egui_ctx.set_visuals(global_visuals);

        // Fix invalid state options
        if self.state.scroll_multiplier <= 0.0 {
            self.state.scroll_multiplier = 1.0;
        }

        // Limit to 2.0 multiplier every zoom, should be plenty
        self.state.zoom_multiplier = self.state.zoom_multiplier.clamp(1.0, 2.0);

        self.init_transforms();

        // Request initial outputs
        self.reload(cc.egui_ctx.clone());

        self
    }
}

impl eframe::App for VSPreviewer {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let promise_res = self.check_promise_callbacks(ctx);
        self.add_error("callbacks", &promise_res);

        let panel_frame = Frame::default()
            .fill(Color32::from_gray(51))
            .inner_margin(Margin::same(self.state.canvas_margin))
            .stroke(Stroke::NONE);

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                // Check for quit, GUI toggle, reload, etc.
                self.check_misc_keyboard_inputs(ctx, frame, ui);

                // React on canvas resolution change
                if self.available_size != ui.available_size() {
                    self.available_size = ui.available_size();

                    // If the win size changed and we were already translated
                    let translate_changed = self.state.translate.length() > 0.0;

                    self.reprocess_outputs(true, translate_changed);
                }

                let preview_res = PreviewerMainUi::ui(self, ctx, ui);
                self.add_error("preview", &preview_res);

                // Display errors if any
                if self.state.show_gui {
                    MessageWindowUi::ui(self, ctx);
                }
            });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let saved_state = SavedState {
            preview_state: self.state,
            transforms: self.transforms.lock().clone(),
        };

        eframe::set_value(storage, eframe::APP_KEY, &saved_state);
    }
}
