use super::{egui, UiControls, UiFrameProps, UiPreferences, VSPreviewer};

pub struct UiStateWindow {}

impl UiStateWindow {
    pub fn ui(pv: &mut VSPreviewer, ctx: &egui::Context) {
        let has_current_output =
            !pv.outputs.is_empty() && pv.outputs.contains_key(&pv.state.cur_output);

        egui::Window::new("State")
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                UiControls::ui(pv, ui);
                ui.separator();

                if has_current_output {
                    let res = UiFrameProps::ui(pv, ctx, ui);
                    pv.add_error("preview", &res);
                }

                UiPreferences::ui(pv, ctx, ui);
            });
    }
}
