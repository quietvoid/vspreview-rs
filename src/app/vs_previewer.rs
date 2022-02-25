use eframe::epaint::ColorImage;
use eframe::{egui, epi};
use fast_image_resize as fir;
use image::DynamicImage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::utils::image_to_colorimage;

use super::*;

#[derive(Default)]
pub struct VSPreviewer {
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

impl VSPreviewer {
    pub fn process_image(
        orig: &DynamicImage,
        state: &PreviewState,
        win_size: &eframe::epaint::Vec2,
    ) -> ColorImage {
        // Rounded up
        let win_size = win_size.round();

        let needs_processing = if state.zoom_factor != 1.0
            || state.translate_norm.length() > 0.0
            || state.upscale_to_window
        {
            let orig_size = Vec2::new(orig.width() as f32, orig.height() as f32);

            // Scaled size to window bounds
            let target_size = dimensions_for_window(win_size, orig_size).round();

            orig_size != target_size
        } else {
            false
        };

        if !needs_processing {
            return image_to_colorimage(orig);
        }

        let src_size = Vec2::from([orig.width() as f32, orig.height() as f32]);
        let (src_w, src_h) = (src_size.x, src_size.y);

        let mut img = orig.clone();

        let zoom_factor = state.zoom_factor;
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
                let fr_filter = fir::FilterType::from(&state.upsampling_filter);
                img = resize_fast(img, target_size.x as u32, target_size.y as u32, fr_filter);
            }
        }

        image_to_colorimage(&img)
    }

    pub fn reload(&mut self, ctx: egui::Context, frame: epi::Frame, force_reload: bool) {
        let state = self.state;
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

                let output_no = if !outputs.contains_key(&cur_output) {
                    // Fallback to first output in order
                    let mut keys: Vec<&i32> = outputs.keys().collect();
                    keys.sort();

                    **keys.first().unwrap()
                } else {
                    cur_output
                };

                // Adjust frame number to max of node
                let cur_frame_no = if let Some(output) = outputs.get(&output_no) {
                    let max_frame = output.node_info.num_frames - 1;
                    cur_frame_no.min(max_frame)
                } else {
                    cur_frame_no
                };

                let vsframe = mutex
                    .get_frame(output_no, cur_frame_no, &state.frame_transform_opts)
                    .unwrap();

                // Return unprocess while we don't have a proper window size
                let processed_image = if win_size.min_elem() > 0.0 {
                    VSPreviewer::process_image(&vsframe.frame_image, &state, &win_size)
                } else {
                    image_to_colorimage(&vsframe.frame_image)
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

            if self.rerender && !self.reprocess {
                // Remove original frame props when a VS render is requested
                output.original_props_promise = None;
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

            let state = self.state;

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
                    Self::process_image(&pf.vsframe.frame_image, &state, &win_size);
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
            let processed_image = Self::process_image(&vsframe.frame_image, &state, &win_size);

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
                        pf.vsframe
                            .frame_image
                            .save_with_format(&save_path, image::ImageFormat::Png)
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

    // Only called when the output changes
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

        // Only adjust the zoom if we're not scaling up
        if !self.state.upscale_to_window
            && self.state.zoom_factor > 1.0
            && self.state.translate_norm.length() > 0.0
        {
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

    // Can only be called when an output is selected
    pub fn fetch_original_props(&mut self, frame: &epi::Frame) {
        let cur_output = self.state.cur_output;
        let cur_frame_no = self.state.cur_frame_no;

        let frame = frame.clone();

        let script = self.script.clone();

        let promise = poll_promise::Promise::spawn_thread("fetch_original_props", move || {
            if let Ok(mut mutex) = script.try_lock() {
                let props = mutex.get_original_props(cur_output, cur_frame_no);
                frame.request_repaint();

                props
            } else {
                None
            }
        });

        let output = self.outputs.get_mut(&cur_output).unwrap();
        output.original_props_promise = Some(promise);
    }
}
