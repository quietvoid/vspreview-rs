use super::{update_input_key_state, PreviewFilterType, VSPreviewer, MAX_ZOOM, MIN_ZOOM};
use anyhow::Result;
use eframe::{
    egui::{self, RichText},
    epaint, epi,
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
    pub fn ui(
        pv: &mut VSPreviewer,
        ctx: &egui::Context,
        frame: &epi::Frame,
        ui: &mut egui::Ui,
    ) -> Result<()> {
        let cur_output = pv.state.cur_output;
        let has_current_output = !pv.outputs.is_empty() && pv.outputs.contains_key(&cur_output);

        // Draw window on top
        if pv.state.show_gui {
            UiStateWindow::ui(pv, ctx, frame);
        }

        // Centered image painted on
        let canvas_res = UiPreviewImage::ui(pv, ui)?;
        canvas_res.context_menu(|ui| {
            let change_script_text = RichText::new("Open script file")
                .size(18.0)
                .color(STATE_LABEL_COLOR);

            if ui.button(change_script_text).clicked() {
                pv.change_script_file(frame);
                ui.close_menu();
            }
        });

        // Bottom panel
        if pv.state.show_gui && has_current_output {
            UiBottomPanel::ui(pv, ctx)?;
        }

        // Check at the end of frame for reprocessing
        pv.try_rerender(frame)?;

        Ok(())
    }
}
