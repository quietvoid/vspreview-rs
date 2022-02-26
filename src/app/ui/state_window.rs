use super::{egui, epi, UiControls, UiFrameProps, UiPreferences, VSPreviewer};

pub struct UiStateWindow {}

impl UiStateWindow {
    pub fn ui(pv: &mut VSPreviewer, ctx: &egui::Context, frame: &epi::Frame) {
        let has_current_output =
            !pv.outputs.is_empty() && pv.outputs.contains_key(&pv.state.cur_output);

        egui::Window::new("State")
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                UiControls::ui(pv, ui);
                ui.separator();

                if has_current_output {
                    UiFrameProps::ui(pv, frame, ui);
                }

                UiPreferences::ui(pv, ui);
            });
    }
}
