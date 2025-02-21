use super::{VSPreviewer, egui, epaint::Color32, update_input_key_state};
use anyhow::{Result, anyhow};
use eframe::epaint::Marginf;

pub struct UiBottomPanel {}

impl UiBottomPanel {
    pub fn ui(pv: &mut VSPreviewer, ctx: &egui::Context) -> Result<()> {
        let output = pv
            .outputs
            .get_mut(&pv.state.cur_output)
            .ok_or_else(|| anyhow!("UiBottomPanel::ui: Invalid current output key"))?;
        let node_info = &output.vsoutput.node_info;

        let transparent_frame = egui::Frame::default()
            .fill(Color32::from_black_alpha(96))
            .inner_margin(Marginf {
                left: 20.0,
                right: 20.0,
                top: 10.0,
                bottom: 10.0,
            });

        egui::TopBottomPanel::bottom("BottomInfo")
            .frame(transparent_frame)
            .show(ctx, |ui| {
                // Add slider
                ui.spacing_mut().slider_width = 600.0;

                let mut slider_frame_no = pv.state.cur_frame_no;

                // We want a bit more precision to within ~50 frames
                let frames_slider =
                    egui::Slider::new(&mut slider_frame_no, 0..=(node_info.num_frames - 1))
                        .smart_aim(false)
                        .integer();

                let slider_res = ui.add(frames_slider);
                let in_use = slider_res.has_focus() || slider_res.drag_started();
                let lost_focus = update_input_key_state(
                    &mut pv.inputs_focused,
                    "frame_slider",
                    in_use,
                    &slider_res,
                );

                // Released/changed value
                if lost_focus {
                    output.last_frame_no = pv.state.cur_frame_no;
                    pv.state.cur_frame_no = slider_frame_no;

                    pv.rerender = true;
                } else if slider_frame_no != pv.state.cur_frame_no {
                    pv.state.cur_frame_no = slider_frame_no;
                }

                let output_info = format!("Output {} - {}", output.vsoutput.index, node_info);

                let node_info_label = egui::RichText::new(output_info)
                    .color(Color32::from_gray(200))
                    .size(20.0);
                ui.label(node_info_label);
            });

        Ok(())
    }
}
