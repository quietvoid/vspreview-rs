use eframe::egui::style::Margin;
use eframe::egui::{Key, Visuals};
use eframe::epaint::{self, Color32, Stroke};
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
        if let Some(storage) = _storage {
            self.state = epi::get_value(storage, epi::APP_KEY).unwrap_or(PreviewState {
                zoom_factor: 1.0,
                zoom_multiplier: 1.0,
                scroll_multiplier: 1.0,
                canvas_margin: 0.0,
                ..Default::default()
            })
        }

        if self.state.scroll_multiplier <= 0.0 {
            self.state.scroll_multiplier = 1.0;
        }

        // Limit to 2.0 multiplier every zoom, should be plenty
        if self.state.zoom_multiplier < 1.0 {
            self.state.zoom_multiplier = 1.0;
        } else if self.state.zoom_multiplier > 2.0 {
            self.state.zoom_multiplier = 2.0;
        }

        self.reload(ctx.clone(), frame.clone(), true);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let cur_output = self.state.cur_output;

        // Initial callback
        self.check_reload_finish();

        // We want a new frame
        // Previously rendering frames must have completed
        self.check_rerender(ctx, frame);

        // Poll new requested frame, replace old if ready
        if let Some(promise) = self.replace_frame_promise.as_ref() {
            if promise.poll().is_ready() {
                let output = self.outputs.get_mut(&cur_output).unwrap();
                output.frame_promise = Some(self.replace_frame_promise.take().unwrap());

                // Update last output once the new frame is rendered
                self.last_output_key = cur_output;
            }
        }

        let has_current_output = !self.outputs.is_empty() && self.outputs.contains_key(&cur_output);
        let new_frame = Frame::default()
            .fill(Color32::from_gray(51))
            .margin(Margin::same(self.state.canvas_margin))
            .stroke(Stroke::none());

        let mut global_visuals = Visuals::dark();
        global_visuals.window_shadow = epaint::Shadow::small_light();

        egui::CentralPanel::default()
            .frame(new_frame)
            .show(ctx, |ui| {
                ui.ctx().set_visuals(global_visuals);

                // Don't allow quit when inputs are still focused
                if !self.any_input_focused() {
                    if ui.input().key_pressed(Key::Q) || ui.input().key_pressed(Key::Escape) {
                        frame.quit();
                    } else if ui.input().key_pressed(Key::I) {
                        self.state.show_gui = !self.state.show_gui;

                        // Clear if the GUI is hidden
                        if !self.state.show_gui {
                            self.inputs_focused.clear();
                        }
                    } else if ui.input().modifiers.ctrl
                        && ui.input().modifiers.shift
                        && ui.input().key_pressed(Key::C)
                    {
                        ui.output().copied_text = self.state.cur_frame_no.to_string();
                    }
                }

                // React on canvas resolution change
                if self.available_size != ui.available_size() {
                    self.available_size = ui.available_size();

                    self.reprocess_outputs();
                }

                // Draw window on top
                if self.state.show_gui {
                    UiStateWindow::ui(self, ctx, frame);
                }

                // Centered image painted on
                UiPreviewImage::ui(self, ctx, frame, ui);

                // Bottom panel
                if self.state.show_gui && has_current_output {
                    UiBottomPanel::ui(self, ctx);
                }

                // Check at the end of frame for reprocessing
                self.check_rerender(ctx, frame);
            });
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, &self.state);
    }
}
