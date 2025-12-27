use super::{VSPreviewer, egui};
use eframe::egui::{RichText, Ui};

pub struct MessageWindowUi {}

impl MessageWindowUi {
    pub fn ui(pv: &mut VSPreviewer, ctx: &egui::Context) {
        let mut vs_messages = None;

        if let Some(script_lock) = pv.script.try_lock()
            && let Some(mut messages) = script_lock.vs_messages.try_lock()
            && !messages.is_empty()
        {
            vs_messages = Some(messages.clone());
            messages.clear();
        }

        if let Some(messages) = &vs_messages {
            // Keep critical errors to avoid rendering image
            let mapped: Vec<String> = messages
                .iter()
                .map(|e| format!("{:?}: {}", &e.message_type, &e.message))
                .collect();

            pv.add_errors("vapoursynth", &mapped);
        }

        if !pv.errors.is_empty() {
            egui::Window::new(RichText::new("Messages").size(20.0))
                .resizable(true)
                .collapsible(false)
                .default_pos((pv.available_size.x / 2.0, pv.available_size.y / 2.0))
                .show(ctx, |ui| {
                    Self::draw_error_section(pv, ui, "vapoursynth", "VapourSynth messages");

                    Self::draw_error_section(
                        pv,
                        ui,
                        "callbacks",
                        "Error fetching frames or reloading",
                    );
                    Self::draw_error_section(
                        pv,
                        ui,
                        "preview",
                        "Error rendering the preview or GUI",
                    );

                    ui.separator();
                    ui.add_space(10.0);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Okay, clear messages").size(22.0))
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

    pub fn draw_error_section(pv: &mut VSPreviewer, ui: &mut Ui, key: &str, header: &str) {
        if let Some(errors) = pv.errors.get(key) {
            let header = RichText::new(header).size(20.0);
            egui::CollapsingHeader::new(header).show(ui, |ui| {
                for (i, e) in errors.iter().enumerate() {
                    Self::draw_error_label(ui, format!("{}. {e}", i + 1));
                }
            });
        }
    }

    pub fn draw_error_label(ui: &mut Ui, value: String) {
        let max_size = value.len().min(75);

        let final_text = if value.len() > 75 {
            let trimmed = value[..max_size].replace('\n', " ");

            format!("{} ...", trimmed)
        } else {
            value.trim().to_string()
        };

        let res = ui.label(RichText::new(final_text).size(18.0));

        if value.len() > 75 {
            res.on_hover_text(RichText::new(value).size(16.0));
        }
    }
}
