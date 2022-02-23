use eframe::egui::{Key, Ui};
use eframe::epaint::ColorImage;
use eframe::{egui, epi};
use fast_image_resize as fir;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::*;

#[derive(Default)]
pub struct Previewer {
    pub script: Arc<Mutex<PreviewedScript>>,
    pub reload_data: Option<Promise<(HashMap<i32, VSOutput>, APreviewFrame)>>,
    pub state: PreviewState,

    pub initialized: bool,

    pub outputs: HashMap<i32, PreviewOutput>,
    pub last_output_key: i32,

    pub rerender: bool,
    pub reprocess: bool,
    pub replace_frame_promise: Option<FramePromise>,

    pub available_size: Vec2,

    pub inputs_focused: HashMap<&'static str, bool>,
}

impl Previewer {
    pub fn process_image(
        orig: &ColorImage,
        state: &PreviewState,
        win_size: &eframe::epaint::Vec2,
    ) -> ColorImage {
        let src_size = Vec2::from([orig.size[0] as f32, orig.size[1] as f32]);
        let (src_w, src_h) = (src_size.x, src_size.y);

        let mut img = image_from_colorimage(orig);

        let zoom_factor = state.zoom_factor;

        // Rounded up
        let win_size = win_size.round();
        let (mut w, mut h) = (src_w as f32, src_h as f32);

        // Unzoom first and foremost
        if zoom_factor < 1.0 && !state.upscale_to_window {
            w *= zoom_factor;
            h *= zoom_factor;

            img = resize_fast(
                img,
                w.round() as u32,
                h.round() as u32,
                fir::FilterType::Box,
            );
        }

        if w > win_size.x || h > win_size.y || zoom_factor > 1.0 {
            // Factors for translations relative to the image resolution
            // -1 means no translation, 1 means translated to the bound
            let coeffs = translate_norm_coeffs(&src_size, &win_size, zoom_factor);

            let (tx_norm, ty_norm) = (state.translate_norm.x, state.translate_norm.y);

            // Scale [-1, 1] coords back to pixels
            let tx = (tx_norm.abs() * coeffs.x).round();
            let ty = (ty_norm.abs() * coeffs.y).round();

            // Positive = crop right part
            let x = if tx_norm.is_sign_negative() { 0.0 } else { tx };
            let y = if ty_norm.is_sign_negative() { 0.0 } else { ty };

            if (tx > 0.0 || ty > 0.0) && zoom_factor <= 1.0 {
                w -= tx;
                h -= ty;
            }

            // Limit to window size
            w = w.min(win_size.x);
            h = h.min(win_size.y);

            img = img.crop_imm(x as u32, y as u32, w as u32, h as u32);
        }

        // Zoom after translate
        if zoom_factor > 1.0 {
            // Cropped size of the zoomed zone
            let cw = (w / zoom_factor).round();
            let ch = (h / zoom_factor).round();

            // Crop for performance, we only want the visible zoomed part
            img = img.crop_imm(0, 0, cw as u32, ch as u32);

            // Size for nearest resize, same as current image size
            // But since we cropped, it creates the zoom effect.
            let new_size = Vec2::new(w, h).round();

            let target_size = if state.upscale_to_window {
                // Resize up to max size of window
                dimensions_for_window(win_size, new_size).round()
            } else {
                new_size
            };

            img = resize_fast(
                img,
                target_size.x.round() as u32,
                target_size.y.round() as u32,
                fir::FilterType::Box,
            );
        }

        // Upscale small images
        if state.upscale_to_window {
            // Image size after crop
            let orig_size = Vec2::new(img.width() as f32, img.height() as f32);

            // Scaled size to window bounds
            let target_size = dimensions_for_window(win_size, orig_size).round();

            if orig_size != target_size {
                let fr_filter = fir::FilterType::from(&state.upsample_filter);
                img = resize_fast(img, target_size.x as u32, target_size.y as u32, fr_filter);
            }
        }

        let new_size = [img.width() as usize, img.height() as usize];
        let processed = ColorImage::from_rgba_unmultiplied(new_size, img.as_bytes());

        processed
    }

    pub fn reload(&mut self, ctx: egui::Context, frame: epi::Frame, force_reload: bool) {
        let state = self.state.clone();
        let cur_output = state.cur_output;
        let cur_frame_no = state.cur_frame_no;

        let script = self.script.clone();
        let win_size = self.available_size;

        self.reload_data = Some(poll_promise::Promise::spawn_thread(
            "initialization/reload",
            move || {
                // This is OK because we didn't have an initial texture
                let mut mutex = script.lock().unwrap();

                if force_reload || !mutex.is_initialized() {
                    mutex.reload();
                }

                let outputs = mutex.get_outputs();
                assert!(!outputs.is_empty());

                let output = if !outputs.contains_key(&cur_output) {
                    // Fallback to first output in order
                    let mut keys: Vec<&i32> = outputs.keys().collect();
                    keys.sort();

                    **keys.first().unwrap()
                } else {
                    cur_output
                };

                let vsframe = mutex
                    .get_frame(output, cur_frame_no, &state.frame_transform_opts)
                    .unwrap();

                // Return unprocess while we don't have a proper window size
                let processed_image = if win_size.min_elem() > 0.0 {
                    Previewer::process_image(&vsframe.frame_image, &state, &win_size)
                } else {
                    vsframe.frame_image.clone()
                };

                let pf = PreviewFrame {
                    vsframe,
                    texture: ctx.load_texture("initial_frame", processed_image),
                };

                frame.request_repaint();

                (outputs, Arc::new(RwLock::new(pf)))
            },
        ));
    }

    pub fn check_reload_finish(&mut self) {
        if let Some(promise) = &self.reload_data {
            if let Some(data) = promise.ready() {
                self.outputs = data
                    .0
                    .iter()
                    .map(|(key, o)| {
                        let new = PreviewOutput {
                            vsoutput: o.clone(),
                            ..Default::default()
                        };

                        (*key, new)
                    })
                    .collect();

                if !data.0.contains_key(&self.state.cur_output) {
                    // Fallback to first output in order
                    let mut keys: Vec<&i32> = data.0.keys().collect();
                    keys.sort();

                    self.state.cur_output = **keys.first().unwrap();
                }

                let output = self.outputs.get_mut(&self.state.cur_output).unwrap();
                let node_info = &output.vsoutput.node_info;

                let (sender, promise) = Promise::new();
                sender.send(data.1.clone());

                output.frame_promise = Some(promise);

                self.reload_data = None;
                self.last_output_key = self.state.cur_output;

                if self.state.cur_frame_no >= node_info.num_frames {
                    self.state.cur_frame_no = node_info.num_frames - 1;
                }

                // First reload
                if !self.initialized {
                    self.initialized = true;

                    // Force rerender once we have the initial window size
                    if self.state.upscale_to_window {
                        self.rerender = true;
                    }
                }
            }
        }
    }

    pub fn check_rerender(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        if !self.outputs.is_empty() {
            let output = self.outputs.get_mut(&self.state.cur_output).unwrap();

            if output.force_reprocess {
                self.rerender = true;

                // Reprocess only if the output is already the correct frame
                self.reprocess = output.last_frame_no == self.state.cur_frame_no;

                output.force_reprocess = false;
            }
        }

        if self.rerender && self.replace_frame_promise.is_none() {
            self.rerender = false;

            let reprocess = self.reprocess;
            self.reprocess = false;

            let script = self.script.clone();
            let win_size = self.available_size;

            let pf = if reprocess {
                self.get_current_frame()
            } else {
                None
            };

            let state = self.state.clone();

            let ctx = ctx.clone();
            let frame = frame.clone();

            self.replace_frame_promise = Some(poll_promise::Promise::spawn_thread(
                "fetch_frame",
                move || Self::get_preview_image(ctx, frame, script, state, pf, reprocess, win_size),
            ));
        }
    }

    pub fn get_current_frame(&self) -> Option<APreviewFrame> {
        if !self.outputs.is_empty() {
            let output = self.outputs.get(&self.state.cur_output).unwrap();

            // Already have a frame
            if let Some(p) = &output.frame_promise {
                p.ready().cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_preview_image(
        ctx: egui::Context,
        frame: epi::Frame,
        script: Arc<Mutex<PreviewedScript>>,
        state: PreviewState,
        pf: Option<APreviewFrame>,
        reprocess: bool,
        win_size: Vec2,
    ) -> APreviewFrame {
        // This is fine because only one promise may be executing at a time
        let mut mutex = script.lock().unwrap();

        let have_existing_frame = pf.is_some();

        // Reuse existing image, process and recreate texture
        let pf = if reprocess && have_existing_frame {
            let pf = pf.unwrap();

            if let Ok(mut pf) = pf.write() {
                // Reprocess and update texture
                let processed_image =
                    Previewer::process_image(&pf.vsframe.frame_image, &state, &win_size);
                pf.texture = ctx.load_texture("frame", processed_image);
            };

            pf
        } else {
            // println!("Requesting new frame");

            // Request new frame, process and recreate texture
            let vsframe = mutex
                .get_frame(
                    state.cur_output,
                    state.cur_frame_no,
                    &state.frame_transform_opts,
                )
                .unwrap();
            let processed_image = Previewer::process_image(&vsframe.frame_image, &state, &win_size);

            let pf = RwLock::new(PreviewFrame {
                vsframe,
                texture: ctx.load_texture("frame", processed_image),
            });

            Arc::new(pf)
        };

        // Once frame is ready
        frame.request_repaint();

        pf
    }

    pub fn handle_keypresses(&mut self, ui: &mut Ui) {
        let mut rerender = self.check_update_seek(ui);
        rerender |= self.check_update_output(ui);

        if ui.input().key_pressed(Key::S) {
            self.save_screenshot();
        }

        self.rerender = rerender;
    }

    /// Returns whether to rerender
    pub fn check_update_seek(&mut self, ui: &mut Ui) -> bool {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return false;
        }

        let output = self.outputs.get_mut(&self.state.cur_output).unwrap();
        let node_info = &output.vsoutput.node_info;

        let current = self.state.cur_frame_no;

        let res = if ui.input().key_pressed(Key::ArrowLeft) || ui.input().key_pressed(Key::H) {
            if current > 0 {
                self.state.cur_frame_no -= 1;
                true
            } else {
                false
            }
        } else if ui.input().key_pressed(Key::ArrowRight) || ui.input().key_pressed(Key::L) {
            if current < node_info.num_frames - 1 {
                self.state.cur_frame_no += 1;
                true
            } else {
                false
            }
        } else if ui.input().key_pressed(Key::ArrowUp) | ui.input().key_pressed(Key::K) {
            if current >= node_info.framerate {
                self.state.cur_frame_no -= node_info.framerate;
                true
            } else if current < node_info.framerate {
                self.state.cur_frame_no = 0;
                true
            } else {
                false
            }
        } else if ui.input().key_pressed(Key::ArrowDown) | ui.input().key_pressed(Key::J) {
            self.state.cur_frame_no += node_info.framerate;

            self.state.cur_frame_no < node_info.num_frames - 1
        } else {
            false
        };

        // Update frame once it's loaded
        output.last_frame_no = current;

        self.state.cur_frame_no = self.state.cur_frame_no.clamp(0, node_info.num_frames - 1);

        res
    }

    pub fn check_update_output(&mut self, ui: &mut Ui) -> bool {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return false;
        }

        let old_output = self.state.cur_output;

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

        if new_output >= 0 && self.outputs.contains_key(&new_output) {
            self.state.cur_output = new_output;

            // Changed output
            self.output_needs_rerender(old_output)
        } else {
            false
        }
    }

    /// Size of the image to scroll/zoom, not the final texture
    pub fn handle_move_inputs(
        &mut self,
        ui: &mut Ui,
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

        let win_size = self.available_size;

        // Calculate zoom factor
        let res_zoom = if delta != 1.0 {
            // Zoom
            let mut new_factor = self.state.zoom_factor;
            let zoom_modifier = if small_step { 0.1 } else { 1.0 };

            // Ignore 1.0 delta, means no zoom done
            if delta < 1.0 {
                // Smaller unzooming when below 1.0
                if new_factor <= 1.0 {
                    new_factor -= 0.125;
                } else if !small_step && self.state.zoom_multiplier > 1.0 {
                    new_factor /= self.state.zoom_multiplier;
                } else {
                    new_factor -= zoom_modifier;
                }
            } else if delta > 1.0 {
                if new_factor < 1.0 {
                    // Zoom back from a unzoomed state
                    // Go back to no zoom
                    new_factor += 0.125;
                } else if !small_step && self.state.zoom_multiplier > 1.0 {
                    new_factor *= self.state.zoom_multiplier;
                } else {
                    new_factor += zoom_modifier;
                }
            }

            let min = if self.state.upscale_to_window {
                1.0
            } else {
                MIN_ZOOM
            };

            new_factor = new_factor.clamp(min, MAX_ZOOM);

            if new_factor != self.state.zoom_factor {
                let trunc_factor = if new_factor < 1.0 { 1000.0 } else { 10.0 };
                self.state.zoom_factor = (new_factor * trunc_factor).round() / trunc_factor;

                true
            } else {
                false
            }
        } else {
            false
        };

        let old_translate = self.state.translate;

        // Calculate new translates
        let res_scroll = if scroll_delta.length() > 0.0 {
            let res_multiplier = *size / win_size;
            let final_delta = scroll_delta * res_multiplier * self.state.scroll_multiplier;

            self.state.translate -= final_delta;

            true
        } else {
            false
        };

        // NOTE: We are outside the scroll_delta condition
        // Because we want to modify the translations on zoom as well
        self.correct_translation_bounds(size);

        let res = res_zoom || (res_scroll && old_translate != self.state.translate);

        // Set other outputs to reprocess if we're modifying the image
        if res {
            self.reprocess_outputs();
        }

        self.rerender |= res;
    }

    pub fn save_screenshot(&self) {
        if let Ok(script) = self.script.try_lock() {
            let mut save_path = script.get_script_dir();

            let screen_file = format!(
                "vspreview-rs_out{}_{}.png",
                self.state.cur_output, self.state.cur_frame_no
            );
            save_path.push(screen_file);

            let output = self.outputs.get(&self.state.cur_output).unwrap();
            if let Some(promise) = &output.frame_promise {
                if let Some(pf) = promise.ready() {
                    if let Ok(pf) = &pf.read() {
                        let img = crate::utils::image_from_colorimage(&pf.vsframe.frame_image);
                        img.save_with_format(&save_path, image::ImageFormat::Png)
                            .unwrap();
                    } else {
                        println!("Apparently the frame is being written to");
                    }
                } else {
                    println!("Apparently the frame is not ready yet");
                }
            } else {
                println!("Apparently there are no frames for the current output");
            }

            println!("Screenshot saved to {}", &save_path.to_str().unwrap());
        } else {
            println!("Apparently the script is busy rendering a frame, try again later");
        }
    }

    pub fn correct_translation_bounds(&mut self, size: &Vec2) {
        let win_size = self.available_size;

        // Updated zoom factor
        // We need the new zoom factor to be able to correct invalid translations
        // Reduce (unzoom) or increase max translate (zooming)
        let zoom_factor = self.state.zoom_factor;

        let coeffs = translate_norm_coeffs(size, &win_size, zoom_factor);

        // Clamp to valid translates
        // Min has to be negative to be able to detect when there's no translate
        self.state.translate.x = if coeffs.x.is_sign_positive() {
            self.state.translate.x.clamp(-0.01, coeffs.x)
        } else {
            // Negative means the image isn't clipped by the window rect
            self.state.translate.x.clamp(0.0, 0.0)
        };

        self.state.translate.y = if coeffs.y.is_sign_positive() {
            self.state.translate.y.clamp(-0.01, coeffs.y)
        } else {
            // Negative means the image isn't clipped by the window rect
            self.state.translate.y.clamp(0.0, 0.0)
        };

        // Normalize to [-1, 1]
        self.state.translate_norm.x = self.state.translate.x / coeffs.x;
        self.state.translate_norm.y = self.state.translate.y / coeffs.y;
    }

    // Update zoom/translate for the new output
    // Returns if we need to rerender
    pub fn output_needs_rerender(&mut self, old_output: i32) -> bool {
        let old = self.outputs.get(&old_output).unwrap();
        let new = self.outputs.get(&self.state.cur_output).unwrap();

        let old_node = &old.vsoutput.node_info;
        let old_size = Vec2::from([old_node.width as f32, old_node.height as f32]);

        // Update translate values
        let new_node = &new.vsoutput.node_info;
        let new_size = Vec2::from([new_node.width as f32, new_node.height as f32]);

        if self.state.zoom_factor > 1.0 && self.state.translate_norm.length() > 0.0 {
            if old_size.length() > new_size.length() {
                self.state.zoom_factor += 1.5;
            } else if old_size.length() < new_size.length() {
                self.state.zoom_factor -= 1.5;
            }
        }

        let coeffs = translate_norm_coeffs(&new_size, &self.available_size, self.state.zoom_factor);
        let (tx_norm, ty_norm) = (self.state.translate_norm.x, self.state.translate_norm.y);

        // Scale [-1, 1] coords back to pixels
        self.state.translate.x = (tx_norm.abs() * coeffs.x).round();
        self.state.translate.y = (ty_norm.abs() * coeffs.y).round();

        old.last_frame_no != new.last_frame_no
    }

    pub fn correct_translate_for_current_output(&mut self) {
        let info = {
            let output = self.outputs.get(&self.state.cur_output).unwrap();
            output.vsoutput.node_info.clone()
        };

        self.correct_translation_bounds(&Vec2::from([info.width as f32, info.height as f32]));

        self.reprocess_outputs();
    }

    pub fn update_pixels_translation_for_current_output(&mut self) {
        let info = {
            let output = self.outputs.get(&self.state.cur_output).unwrap();
            output.vsoutput.node_info.clone()
        };

        self.state.translate = crate::utils::translate_norm_to_pixels(
            &self.state.translate_norm,
            &Vec2::from([info.width as f32, info.height as f32]),
            &self.available_size,
            self.state.zoom_factor,
        );

        self.correct_translate_for_current_output();
    }

    pub fn any_input_focused(&self) -> bool {
        self.inputs_focused.values().any(|e| *e)
    }

    pub fn reprocess_outputs(&mut self) {
        self.outputs
            .values_mut()
            .for_each(|o| o.force_reprocess = true);
    }
}
