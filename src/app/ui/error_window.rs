use super::{egui, VSPreviewer};
use eframe::egui::RichText;

pub struct ErrorWindowUi {}

impl ErrorWindowUi {
    pub fn ui(pv: &mut VSPreviewer, ctx: &egui::Context) {
        let mut vs_errors = None;
        if let Some(mut script_lock) = pv.script.try_lock() {
            if let Some(errors) = script_lock.vs_error.as_mut() {
                if !errors.is_empty() {
                    vs_errors = Some(errors.clone());
                    errors.clear();
                }
            }
        }

        if let Some(errors) = &vs_errors {
            pv.add_errors("vapoursynth", errors);
        }

        if !pv.errors.is_empty() {
            egui::Window::new(RichText::new("Some errors occurred!").size(20.0))
                .collapsible(false)
                .resizable(true)
                .auto_sized()
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, -300.0))
                .show(ctx, |ui| {
                    if let Some(errors) = pv.errors.get("vapoursynth") {
                        let header = RichText::new("VapourSynth errors").size(20.0);
                        egui::CollapsingHeader::new(header).show(ui, |ui| {
                            for (i, e) in errors.iter().enumerate() {
                                let value = format!("{}. {e}", i + 1);
                                ui.label(RichText::new(value).size(18.0));
                            }
                        });
                    }

                    if let Some(errors) = pv.errors.get("callbacks") {
                        let header = RichText::new("Error fetching frames or reloading").size(20.0);
                        egui::CollapsingHeader::new(header).show(ui, |ui| {
                            for (i, e) in errors.iter().enumerate() {
                                let value = format!("{}. {e}", i + 1);
                                ui.label(RichText::new(value).size(18.0));
                            }
                        });
                    }

                    if let Some(errors) = pv.errors.get("preview") {
                        let header = RichText::new("Error rendering the preview or GUI").size(20.0);
                        egui::CollapsingHeader::new(header).show(ui, |ui| {
                            for (i, e) in errors.iter().enumerate() {
                                let value = format!("{}. {e}", i + 1);
                                ui.label(RichText::new(value).size(18.0));
                            }
                        });
                    }

                    ui.separator();
                    ui.add_space(10.0);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Okay, clear errors").size(22.0))
                            .on_hover_text(
                                RichText::new("The previewer may not end up in a useable state!")
                                    .size(18.0),
                            )
                            .clicked()
                        {
                            pv.errors.clear();
                        }
                    });
                });
        }
    }
}
