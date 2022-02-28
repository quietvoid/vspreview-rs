use super::{
    custom_widgets::CustomImage, egui, egui::Key, epaint::Vec2, VSPreviewer, MAX_ZOOM, MIN_ZOOM,
};
use anyhow::{anyhow, Result};

pub struct UiPreviewImage {}

impl UiPreviewImage {
    pub fn ui(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> Result<()> {
        let cur_output = pv.state.cur_output;
        let has_current_output = !pv.outputs.is_empty() && pv.outputs.contains_key(&cur_output);

        // If the outputs differ in frame index, we should wait for the render
        // instead of rendering the old frame
        let output_diff_frame = if has_current_output {
            let cur_output = pv
                .outputs
                .get(&cur_output)
                .ok_or(anyhow!("UiPreviewImage::ui: Invalid current output key"))?;
            let last_output = pv
                .outputs
                .get(&pv.last_output_key)
                .ok_or(anyhow!("UiPreviewImage::ui: Invalid last output key"))?;

            last_output.last_frame_no != cur_output.last_frame_no
        } else {
            false
        };

        let zoom_delta = ui.input().zoom_delta();
        let scroll_delta = ui.input().scroll_delta;

        // Acquire frame texture to render now
        let preview_frame = if has_current_output {
            let output = pv.outputs.get(&cur_output).ok_or(anyhow!(
                "UiPreviewImage::ui preview_frame: Invalid current output key"
            ))?;

            if output_diff_frame {
                None
            } else {
                output.rendered_frame.as_ref().map(Clone::clone)
            }
        } else {
            None
        };

        let mut painted_image = false;

        let mut image_size = Vec2::ZERO;

        // We want the image size for alignment
        if let Some(pf) = &preview_frame {
            let pf = pf.read();

            let image = &pf.vsframe.image;
            image_size = Vec2::from([image.width() as f32, image.height() as f32]);
        }

        let cross_align = if (image_size.x * pv.state.zoom_factor.min(1.0)) > pv.available_size.x {
            egui::Align::Min
        } else {
            egui::Align::Center
        };

        let canvas_layout = egui::Layout::centered_and_justified(egui::Direction::TopDown)
            .with_cross_align(cross_align);

        ui.with_layout(canvas_layout, |ui| {
            if let Some(pf) = preview_frame {
                let pf = pf.read();
                if let Some(tex_mutex) = pf.texture.try_lock() {
                    if let Some(tex) = &*tex_mutex {
                        painted_image = true;

                        let tex_size = tex.size_vec2();
                        let custom_image = CustomImage::new(tex.id(), tex_size);

                        ui.add(custom_image);

                        if !pv.any_input_focused() && !pv.frame_promise.is_locked() {
                            let mut res = Self::handle_move_inputs(
                                pv,
                                ui,
                                &image_size,
                                zoom_delta,
                                scroll_delta,
                            );
                            pv.add_error("preview", res);

                            res = Self::handle_keypresses(pv, ui);
                            pv.add_error("preview", res);
                        }
                    }
                };
            }

            if !painted_image && pv.errors.is_empty() {
                ui.add(egui::Spinner::new().size(200.0));
            }
        });

        Ok(())
    }

    pub fn handle_keypresses(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> Result<()> {
        let mut rerender = Self::check_update_seek(pv, ui)?;
        rerender |= Self::check_update_output(pv, ui)?;

        if ui.input().key_pressed(Key::S) {
            pv.save_screenshot()?;
        }

        pv.rerender |= rerender;

        Ok(())
    }

    /// Returns whether to rerender
    pub fn check_update_seek(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> Result<bool> {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return Ok(false);
        }

        let output = pv
            .outputs
            .get_mut(&pv.state.cur_output)
            .ok_or(anyhow!("check_update_seek: Invalid current output key"))?;
        let node_info = &output.vsoutput.node_info;

        let current = pv.state.cur_frame_no;

        let res = if ui.input().key_pressed(Key::ArrowLeft) || ui.input().key_pressed(Key::H) {
            if current > 0 {
                pv.state.cur_frame_no -= 1;
                true
            } else {
                false
            }
        } else if ui.input().key_pressed(Key::ArrowRight) || ui.input().key_pressed(Key::L) {
            if current < node_info.num_frames - 1 {
                pv.state.cur_frame_no += 1;
                true
            } else {
                false
            }
        } else if ui.input().key_pressed(Key::ArrowUp) | ui.input().key_pressed(Key::K) {
            if current >= node_info.framerate {
                pv.state.cur_frame_no -= node_info.framerate;
                true
            } else if current < node_info.framerate {
                pv.state.cur_frame_no = 0;
                true
            } else {
                false
            }
        } else if ui.input().key_pressed(Key::ArrowDown) | ui.input().key_pressed(Key::J) {
            pv.state.cur_frame_no += node_info.framerate;

            pv.state.cur_frame_no < node_info.num_frames - 1
        } else {
            false
        };

        // Update frame once it's loaded
        output.last_frame_no = current;

        pv.state.cur_frame_no = pv.state.cur_frame_no.clamp(0, node_info.num_frames - 1);

        Ok(res)
    }

    pub fn check_update_output(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> Result<bool> {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return Ok(false);
        }

        let old_output = pv.state.cur_output;

        let new_output: i32 = if ui.input().key_pressed(Key::Num1) {
            0
        } else if ui.input().key_pressed(Key::Num2) {
            1
        } else if ui.input().key_pressed(Key::Num3) {
            2
        } else if ui.input().key_pressed(Key::Num4) {
            3
        } else if ui.input().key_pressed(Key::Num5) {
            4
        } else if ui.input().key_pressed(Key::Num6) {
            5
        } else if ui.input().key_pressed(Key::Num7) {
            6
        } else if ui.input().key_pressed(Key::Num8) {
            7
        } else if ui.input().key_pressed(Key::Num9) {
            8
        } else if ui.input().key_pressed(Key::Num0) {
            9
        } else {
            -1
        };

        if new_output >= 0 && pv.outputs.contains_key(&new_output) {
            pv.state.cur_output = new_output;

            // Changed output
            pv.output_needs_rerender(old_output)
        } else {
            Ok(false)
        }
    }

    /// Size of the image to scroll/zoom, not the final texture
    pub fn handle_move_inputs(
        pv: &mut VSPreviewer,
        ui: &mut egui::Ui,
        size: &Vec2,
        zoom_delta: f32,
        scroll_delta: Vec2,
    ) -> Result<()> {
        // Update zoom delta to take into consideration small step keyboard input
        let mut delta = zoom_delta;
        let small_step = delta == 1.0
            && ui.input().modifiers.ctrl
            && (ui.input().key_pressed(Key::ArrowDown) || ui.input().key_pressed(Key::ArrowUp));

        if small_step {
            if ui.input().key_pressed(Key::ArrowDown) {
                delta = 0.0;
            } else {
                delta = 2.0;
            }
        }

        let mut scroll_delta = scroll_delta;

        // Keyboard based scrolling
        if ui.input().key_pressed(Key::End) {
            scroll_delta.x = -50.0;
        } else if ui.input().key_pressed(Key::Home) {
            scroll_delta.x = 50.0;
        } else if ui.input().key_pressed(Key::PageDown) {
            scroll_delta.y = -50.0;
        } else if ui.input().key_pressed(Key::PageUp) {
            scroll_delta.y = 50.0;
        }

        let win_size = pv.available_size;

        // Calculate zoom factor
        let res_zoom = if delta != 1.0 {
            // Zoom
            let mut new_factor = pv.state.zoom_factor;
            let zoom_modifier = if small_step { 0.1 } else { 1.0 };

            // Ignore 1.0 delta, means no zoom done
            if delta < 1.0 {
                // Smaller unzooming when below 1.0
                if new_factor <= 1.0 {
                    new_factor -= 0.125;
                } else if !small_step && pv.state.zoom_multiplier > 1.0 {
                    new_factor /= pv.state.zoom_multiplier;
                } else {
                    new_factor -= zoom_modifier;
                }
            } else if delta > 1.0 {
                if new_factor < 1.0 {
                    // Zoom back from a unzoomed state
                    // Go back to no zoom
                    new_factor += 0.125;
                } else if !small_step && pv.state.zoom_multiplier > 1.0 {
                    new_factor *= pv.state.zoom_multiplier;
                } else {
                    new_factor += zoom_modifier;
                }
            }

            let min = if pv.state.upscale_to_window {
                1.0
            } else {
                MIN_ZOOM
            };

            new_factor = new_factor.clamp(min, MAX_ZOOM);

            if new_factor != pv.state.zoom_factor {
                let trunc_factor = if new_factor < 1.0 { 1000.0 } else { 10.0 };
                pv.state.zoom_factor = (new_factor * trunc_factor).round() / trunc_factor;

                true
            } else {
                false
            }
        } else {
            false
        };

        let mut new_translate = pv.state.translate;

        // Calculate new translates
        let res_scroll = if scroll_delta.length() > 0.0 {
            let res_multiplier = *size / win_size;
            let final_delta = scroll_delta * res_multiplier * pv.state.scroll_multiplier;

            new_translate -= final_delta;

            true
        } else {
            false
        };

        // NOTE: We are outside the scroll_delta condition
        // Because we want to modify the translations on zoom as well
        let reprocess_translate = pv.correct_translate_for_current_output(new_translate, false)?;

        let res = res_zoom || (res_scroll && reprocess_translate);

        // Set other outputs to reprocess if we're modifying the image
        if res {
            pv.reprocess_outputs(reprocess_translate);
        }

        pv.rerender |= res;

        Ok(())
    }
}
