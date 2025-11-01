use super::{MAX_ZOOM, MIN_ZOOM, PreviewFilterType, VSPreviewer, update_input_key_state};
use anyhow::Result;
use eframe::{
    egui::{self, Layout, RichText},
    emath::{Align, Align2, Vec2},
    epaint,
};

mod bottom_panel;
mod controls;
mod frame_props;
mod message_window;
mod preferences;
mod preview_image;
mod state_window;

mod custom_widgets;

pub use bottom_panel::UiBottomPanel;
use controls::UiControls;
use frame_props::UiFrameProps;
pub use message_window::MessageWindowUi;
use preferences::UiPreferences;
pub use preview_image::UiPreviewImage;
pub use state_window::UiStateWindow;

const STATE_LABEL_COLOR: epaint::Color32 = epaint::Color32::from_gray(160);

pub struct PreviewerMainUi {}

impl PreviewerMainUi {
    pub fn ui(pv: &mut VSPreviewer, ctx: &egui::Context, ui: &mut egui::Ui) -> Result<()> {
        let cur_output = pv.state.cur_output;
        let has_current_output = !pv.outputs.is_empty() && pv.outputs.contains_key(&cur_output);

        // Draw window on top
        if pv.state.show_gui {
            UiStateWindow::ui(pv, ctx);
        }

        // Centered image painted on
        let canvas_res = UiPreviewImage::ui(pv, ui)?;
        canvas_res.context_menu(|ui| {
            let change_script_text = RichText::new("Open script file")
                .size(18.0)
                .color(STATE_LABEL_COLOR);

            let about_text = RichText::new("About").size(18.0).color(STATE_LABEL_COLOR);

            if ui.button(change_script_text).clicked() {
                pv.change_script_file(ctx);
                ui.close();
            }

            if ui.button(about_text).clicked() {
                pv.about_window_open = true;
                ui.close();
            }
        });

        // Bottom panel
        if pv.state.show_gui && has_current_output {
            UiBottomPanel::ui(pv, ctx)?;
        }

        // About window
        egui::Window::new("About")
            .open(&mut pv.about_window_open)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.heading("vspreview-rs");
                    ui.label("Minimal and functional VapourSynth script previewer");

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;

                        ui.label("Built on top of ");
                        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                        ui.label(" and ");
                        ui.hyperlink_to(
                            "vapoursynth-rs",
                            "https://github.com/YaLTeR/vapoursynth-rs",
                        );
                    });
                });
            });

        // Check at the end of frame for reprocessing
        pv.try_rerender(ctx)?;

        Ok(())
    }
}
