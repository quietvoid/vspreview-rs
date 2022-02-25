use super::{egui, egui::Key, epaint::Vec2, epi, VSPreviewer, MAX_ZOOM, MIN_ZOOM};

pub struct UiPreviewImage {}

impl UiPreviewImage {
    pub fn ui(pv: &mut VSPreviewer, ctx: &egui::Context, frame: &epi::Frame, ui: &mut egui::Ui) {
        let cur_output = pv.state.cur_output;
        let has_current_output = !pv.outputs.is_empty() && pv.outputs.contains_key(&cur_output);

        // If the outputs differ in frame index, we should wait for the render
        // instead of rendering the old frame
        let output_diff_frame = if has_current_output {
            let cur_output = pv.outputs.get(&cur_output).unwrap();
            let last_output = pv.outputs.get(&pv.last_output_key).unwrap();

            last_output.last_frame_no != cur_output.last_frame_no
        } else {
            false
        };

        let zoom_delta = ui.input().zoom_delta();
        let scroll_delta = ui.input().scroll_delta;

        ui.centered_and_justified(|ui| {
            // Acquire frame texture to render now
            let frame_promise = if has_current_output {
                let output = pv.outputs.get(&cur_output).unwrap();

                if output_diff_frame {
                    None
                } else {
                    output.frame_promise.as_ref()
                }
            } else {
                None
            };

            if pv.reload_data.is_some() || frame_promise.is_none() {
                ui.add(egui::Spinner::new().size(200.0));
            } else if let Some(promise) = frame_promise {
                if let Some(pf) = promise.ready() {
                    let mut image_size: Option<[f32; 2]> = None;

                    if let Ok(pf) = &pf.read() {
                        image_size = Some(pf.vsframe.frame_image.size.map(|i| i as f32));

                        let tex_size = pf.texture.size_vec2();
                        ui.image(&pf.texture, tex_size);
                    }

                    if !pv.any_input_focused() {
                        // We could read the image rendered
                        if let Some(image_size) = image_size {
                            if !pv.rerender && pv.replace_frame_promise.is_none() {
                                let size = Vec2::from(image_size);

                                Self::handle_move_inputs(pv, ui, &size, zoom_delta, scroll_delta);
                                Self::handle_keypresses(pv, ctx, frame, ui);
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn handle_keypresses(
        pv: &mut VSPreviewer,
        ctx: &egui::Context,
        frame: &epi::Frame,
        ui: &mut egui::Ui,
    ) {
        let mut rerender = Self::check_update_seek(pv, ui);
        rerender |= Self::check_update_output(pv, ui);

        if ui.input().key_pressed(Key::S) {
            pv.save_screenshot();
        }

        pv.rerender = rerender;

        if ui.input().key_pressed(Key::R) {
            pv.reload(ctx.clone(), frame.clone(), true)
        }
    }

    /// Returns whether to rerender
    pub fn check_update_seek(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> bool {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return false;
        }

        let output = pv.outputs.get_mut(&pv.state.cur_output).unwrap();
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

        res
    }

    pub fn check_update_output(pv: &mut VSPreviewer, ui: &mut egui::Ui) -> bool {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return false;
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
            false
        }
    }

    /// Size of the image to scroll/zoom, not the final texture
    pub fn handle_move_inputs(
        pv: &mut VSPreviewer,
        ui: &mut egui::Ui,
        size: &Vec2,
        zoom_delta: f32,
        scroll_delta: Vec2,
    ) {
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

        let old_translate = pv.state.translate;

        // Calculate new translates
        let res_scroll = if scroll_delta.length() > 0.0 {
            let res_multiplier = *size / win_size;
            let final_delta = scroll_delta * res_multiplier * pv.state.scroll_multiplier;

            pv.state.translate -= final_delta;

            true
        } else {
            false
        };

        // NOTE: We are outside the scroll_delta condition
        // Because we want to modify the translations on zoom as well
        pv.correct_translation_bounds(size);

        let res = res_zoom || (res_scroll && old_translate != pv.state.translate);

        // Set other outputs to reprocess if we're modifying the image
        if res {
            pv.reprocess_outputs();
        }

        pv.rerender |= res;
    }
}
