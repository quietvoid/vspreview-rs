use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use eframe::egui::Key;
use eframe::egui::{self, TextureOptions};
use fast_image_resize as fir;
use image::DynamicImage;
use parking_lot::{Mutex, RwLock};

use crate::utils::image_to_colorimage;

use super::*;

#[derive(Default)]
pub struct VSPreviewer {
    pub script: Arc<Mutex<PreviewedScript>>,
    pub state: PreviewState,
    pub errors: HashMap<&'static str, Vec<String>>,
    pub about_window_open: bool,

    /// Promise returning the newly reloaded outputs
    pub reload_data: Option<ReloadPromise>,
    /// Outputs available from the script
    pub outputs: HashMap<i32, PreviewOutput>,
    /// Last output used
    pub last_output_key: i32,

    /// Canvas drawing available size
    pub available_size: Vec2,
    /// Map of the currently active inputs
    pub inputs_focused: HashMap<&'static str, bool>,

    /// Force rerender/reprocess
    pub rerender: bool,
    /// Override to only reprocess without requesting a new VS frame
    pub reprocess: bool,

    /// Promise returning a new requested frame
    pub frame_promise: Arc<Mutex<Option<FramePromise>>>,
    /// Promise returning the original props of the current frame
    pub original_props_promise: Arc<Mutex<Option<PropsPromise>>>,

    /// Promise returning a bool, whether to rerender or not
    pub misc_promise: Arc<Mutex<Option<Promise<ReloadType>>>>,

    pub transforms: Arc<Mutex<PreviewTransforms>>,
}

impl VSPreviewer {
    pub fn process_image(
        orig: &DynamicImage,
        state: &PreviewState,
        win_size: &eframe::epaint::Vec2,
    ) -> Result<DynamicImage> {
        // Rounded up
        let win_size = win_size.round();
        let src_size = Vec2::from([orig.width() as f32, orig.height() as f32]);

        let (src_w, src_h) = (src_size.x, src_size.y);

        let mut img = orig.clone();

        let zoom_factor = state.zoom_factor;
        let (mut w, mut h) = (src_w, src_h);

        // Unzoom first and foremost
        if zoom_factor < 1.0 && !state.upscale_to_window {
            w *= zoom_factor;
            h *= zoom_factor;

            img = resize_fast(
                img,
                w.round() as u32,
                h.round() as u32,
                fir::FilterType::Box,
            )?;
        }

        if w > win_size.x || h > win_size.y || zoom_factor > 1.0 {
            // Factors for translations relative to the image resolution
            // -1 means no translation, 1 means translated to the bound
            let (tx_norm, ty_norm) = (state.translate_norm.x, state.translate_norm.y);

            let translate_pixel = crate::utils::translate_norm_to_pixels(
                &state.translate_norm,
                &src_size,
                &win_size,
                zoom_factor,
            );

            // Scale [-1, 1] coords back to pixels
            let (tx, ty) = (translate_pixel.x, translate_pixel.y);

            // Positive = crop right part
            let x = if tx_norm.is_sign_negative() { 0.0 } else { tx };
            let y = if ty_norm.is_sign_negative() { 0.0 } else { ty };

            if (tx > 0.0 || ty > 0.0) && zoom_factor <= 1.0 {
                w -= tx;
                h -= ty;
            }

            // Limit to window size if not scaling down
            // Also when zooming in
            if state.zoom_factor > 1.0 || !state.fit_to_window {
                w = w.min(win_size.x);
                h = h.min(win_size.y);
            }

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
                dimensions_for_window(&win_size, &new_size).round()
            } else {
                new_size
            };

            img = resize_fast(
                img,
                target_size.x.round() as u32,
                target_size.y.round() as u32,
                fir::FilterType::Box,
            )?;
        }

        // Upscale small images
        if state.upscale_to_window && state.upsampling_filter != PreviewFilterType::Gpu {
            // Image size after crop
            let orig_size = Vec2::new(img.width() as f32, img.height() as f32);

            // Scaled size to window bounds
            let target_size = dimensions_for_window(&win_size, &orig_size).round();

            if orig_size != target_size {
                let fr_filter = fir::FilterType::from(&state.upsampling_filter);
                img = resize_fast(img, target_size.x as u32, target_size.y as u32, fr_filter)?;
            }
        }

        Ok(img)
    }

    // Always reloads the script
    pub fn reload(&mut self, ctx: egui::Context) {
        if self.reload_data.is_some() {
            return;
        }

        if !self.errors.is_empty() {
            self.errors.clear();
        }

        let script = self.script.clone();

        self.reload_data = Some(poll_promise::Promise::spawn_thread(
            "initialization/reload",
            move || {
                let mut script_mutex = script.lock();
                let res = script_mutex.reload();
                script_mutex.add_vs_error(&res);

                if res.is_ok() {
                    let outputs_res = script_mutex.get_outputs();
                    script_mutex.add_vs_error(&outputs_res);

                    let outputs = if let Ok(outputs) = outputs_res {
                        // No output case handled by vapoursynth-rs
                        assert!(!outputs.is_empty());
                        Some(outputs)
                    } else {
                        None
                    };

                    // Not ready but we need to get the checker going
                    ctx.request_repaint();

                    outputs
                } else {
                    None
                }
            },
        ));
    }

    pub fn check_reload_finish(&mut self) -> Result<()> {
        if let Some(promise) = &self.reload_data {
            if let Some(promise_res) = promise.ready() {
                self.outputs.clear();

                if let Some(outputs) = promise_res {
                    self.outputs = outputs
                        .iter()
                        .map(|(key, o)| {
                            let new = PreviewOutput {
                                vsoutput: o.clone(),
                                ..Default::default()
                            };

                            (*key, new)
                        })
                        .collect();

                    if !self.outputs.contains_key(&self.state.cur_output) {
                        // Fallback to first output in order
                        let mut keys: Vec<&i32> = self.outputs.keys().collect();
                        keys.sort();

                        self.state.cur_output = **keys
                            .first()
                            .ok_or_else(|| anyhow!("No outputs available"))?;
                    }

                    let output = self
                        .outputs
                        .get_mut(&self.state.cur_output)
                        .ok_or_else(|| anyhow!("outputs reload: Invalid current output key"))?;
                    let node_info = &output.vsoutput.node_info;

                    self.reload_data = None;
                    self.last_output_key = self.state.cur_output;

                    if self.state.cur_frame_no >= node_info.num_frames {
                        self.state.cur_frame_no = node_info.num_frames - 1;
                    }

                    // Fetch a frame for new current output
                    self.rerender = true;
                }

                // Done reloading, remove promise
                if let Some(mut mutex) = self.misc_promise.try_lock() {
                    if (*mutex).is_some() {
                        *mutex = None;
                    };
                }

                // Reset reload data even if errored
                self.reload_data = None;
            }
        }

        Ok(())
    }

    pub fn try_rerender(&mut self, ctx: &egui::Context) -> Result<()> {
        if let Some(mut promise) = self.frame_promise.try_lock() {
            let misc_in_progress = if let Some(p) = self.misc_promise.try_lock() {
                p.is_some()
            } else {
                true
            };

            // Still rendering, reloading or changing
            if promise.is_some() || misc_in_progress || self.reload_data.is_some() {
                return Ok(());
            }

            if !self.outputs.is_empty() {
                let output = self
                    .outputs
                    .get_mut(&self.state.cur_output)
                    .ok_or_else(|| anyhow!("rerender: Invalid current output key"))?;

                if output.force_reprocess {
                    self.rerender = true;

                    // Reprocess only if the output is already the correct frame
                    self.reprocess = output.last_frame_no == self.state.cur_frame_no;

                    output.force_reprocess = false;
                }

                if self.rerender && !self.reprocess {
                    // Remove original frame props when a VS render is requested
                    output.original_props = None;
                }
            }

            if self.rerender {
                self.rerender = false;

                let mut reprocess = self.reprocess;
                self.reprocess = false;

                // Still reloading, can't reprocess
                if self.reload_data.is_some() && reprocess {
                    reprocess = false;
                }

                // Get current state at the moment the frame is requested
                let state = self.state;

                // Reset current changed flag
                self.state.translate_changed = false;

                let script = self.script.clone();
                let win_size = self.available_size;

                let pf = self.get_current_frame()?;

                let ctx = ctx.clone();
                let frame_mutex = self.frame_promise.clone();

                let fetch_image_state = FetchImageState {
                    ctx,
                    frame_mutex,
                    script,
                    state,
                    pf,
                    reprocess,
                    win_size,
                };

                *promise = Some(poll_promise::Promise::spawn_thread(
                    "fetch_frame",
                    move || {
                        match Self::get_preview_image(fetch_image_state) {
                            Ok(preview_frame) => preview_frame,
                            Err(e) => {
                                // Errors here are not recoverable
                                panic!("{}", e)
                            }
                        }
                    },
                ));
            }
        }

        Ok(())
    }

    pub fn check_rerender_finish(&mut self, ctx: &egui::Context) -> Result<()> {
        if let Some(mut promise_mutex) = self.frame_promise.try_lock() {
            let mut updated_tex = false;

            if let Some(promise) = &*promise_mutex {
                if let Some(Some(rendered_frame)) = promise.ready() {
                    // Block as it's supposed to be ready
                    let pf = rendered_frame.read();

                    if let Some(mut tex_mutex) = pf.texture.try_lock() {
                        let output =
                            self.outputs
                                .get_mut(&self.state.cur_output)
                                .ok_or_else(|| {
                                    anyhow!("current_output_mut: Invalid current output key")
                                })?;

                        // Set PreviewFrame from what the promise returned
                        output.rendered_frame = Some(rendered_frame.clone());

                        // Processed if available, otherwise original
                        let final_image = if let Some(image) = &pf.processed_image {
                            image
                        } else {
                            &pf.vsframe.image
                        };

                        let transforms = self.transforms.lock();

                        // Convert to ColorImage on texture change
                        let colorimage = image_to_colorimage(final_image, &self.state, &transforms);

                        let tex_filter = egui::TextureFilter::from(&self.state.texture_filter);
                        let tex_opts = TextureOptions {
                            magnification: tex_filter,
                            minification: tex_filter,
                        };
                        // Update texture on render done
                        if let Some(ref mut tex) = *tex_mutex {
                            tex.set(colorimage, tex_opts);
                        } else {
                            *tex_mutex = Some(ctx.load_texture("frame", colorimage, tex_opts));
                        }

                        // Update last output once the new frame is rendered
                        self.last_output_key = output.vsoutput.index;

                        updated_tex = true;
                    };
                }
            }

            if updated_tex {
                *promise_mutex = None;
            }
        }

        Ok(())
    }

    pub fn get_current_frame(&self) -> Result<Option<VSPreviewFrame>> {
        if !self.outputs.is_empty() {
            let output = self
                .outputs
                .get(&self.state.cur_output)
                .ok_or_else(|| anyhow!("get_current_Frame: Invalid current output key"))?;
            Ok(output.rendered_frame.clone())
        } else {
            Ok(None)
        }
    }

    pub fn get_preview_image(fetch_image_state: FetchImageState) -> Result<Option<VSPreviewFrame>> {
        let FetchImageState {
            ctx,
            frame_mutex,
            script,
            state,
            pf,
            reprocess,
            win_size,
        } = fetch_image_state;

        // This is fine because only one promise may be executing at a time
        let mut script_mutex = script.lock();

        let have_existing_frame = pf.is_some();

        let _lock = frame_mutex.lock();

        // Reuse existing image, process and recreate texture
        let pf = if reprocess && have_existing_frame {
            // Verified above, cannot panic
            let pf = pf.unwrap();

            // Force blocking as we need to reprocess the image
            let mut existing_frame = pf.write();
            let image = &existing_frame.vsframe.image;
            let image_size = Vec2::from([image.width() as f32, image.height() as f32]);

            if Self::state_needs_processing(&state, &image_size, &win_size) {
                // Reprocess and update image for painting
                existing_frame.processed_image =
                    Some(Self::process_image(image, &state, &win_size)?);
            } else {
                existing_frame.processed_image = None;
            }

            Some(pf.clone())
        } else {
            // Request new frame, process and recreate image for painting
            let vsframe_res = script_mutex.get_frame(
                state.cur_output,
                state.cur_frame_no,
                &state.frame_transform_opts,
            );
            script_mutex.add_vs_error(&vsframe_res);

            if let Ok(vsframe) = vsframe_res {
                let image_size =
                    Vec2::from([vsframe.image.width() as f32, vsframe.image.height() as f32]);

                let processed_image =
                    if Self::state_needs_processing(&state, &image_size, &win_size) {
                        Some(Self::process_image(&vsframe.image, &state, &win_size)?)
                    } else {
                        None
                    };

                let new_pf = if let Some(existing_frame) = pf {
                    let mut pf = existing_frame.write();
                    pf.vsframe = vsframe;
                    pf.processed_image = processed_image;

                    existing_frame.clone()
                } else {
                    Arc::new(RwLock::new(PreviewFrame {
                        vsframe,
                        processed_image,
                        texture: Mutex::new(None),
                    }))
                };

                Some(new_pf)
            } else {
                pf
            }
        };

        // Once frame is ready
        ctx.request_repaint();

        Ok(pf)
    }

    pub fn save_screenshot(&self) -> Result<()> {
        if let Some(script) = self.script.try_lock() {
            let mut save_path = script.get_script_dir();

            let screen_file = format!(
                "vspreview-rs_out{}_{}.png",
                self.state.cur_output, self.state.cur_frame_no
            );
            save_path.push(screen_file);

            let output = self
                .outputs
                .get(&self.state.cur_output)
                .ok_or_else(|| anyhow!("save_screenshot: Invalid current output key"))?;
            if let Some(pf) = &output.rendered_frame {
                let pf = pf.read();

                // Shouldn't fail at this point
                pf.vsframe
                    .image
                    .save_with_format(&save_path, image::ImageFormat::Png)?;
            } else {
                bail!("There is no rendered frame for the current output");
            }

            let path_str = save_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid UTF-8 save path"))?;

            script.send_debug_message(format!("Screenshot saved to {}", path_str))?;
        } else {
            bail!("The script is busy rendering a frame, try again later");
        }

        Ok(())
    }

    // Returns fixed pixel based and normalized translation vectors
    pub fn fix_translation_bounds(&self, image_size: &Vec2, new_translate: &Vec2) -> (Vec2, Vec2) {
        let win_size = self.available_size;

        // Updated zoom factor
        // We need the new zoom factor to be able to correct invalid translations
        // Reduce (unzoom) or increase max translate (zooming)
        let zoom_factor = self.state.zoom_factor;

        let mut fixed_translate = *new_translate;

        let coeffs = translate_norm_coeffs(image_size, &win_size, zoom_factor);

        // Clamp to valid translates
        // Min has to be negative to be able to detect when there's no translate
        fixed_translate.x = if coeffs.x.is_sign_positive() {
            new_translate.x.clamp(0.0, coeffs.x)
        } else {
            // Negative means the image isn't clipped by the window rect
            new_translate.x.clamp(0.0, 0.0)
        };

        fixed_translate.y = if coeffs.y.is_sign_positive() {
            new_translate.y.clamp(0.0, coeffs.y)
        } else {
            // Negative means the image isn't clipped by the window rect
            new_translate.y.clamp(0.0, 0.0)
        };

        // Normalize to [0, 1]
        let normalized_translate = Vec2::new(
            (fixed_translate.x / coeffs.x).clamp(0.0, 1.0),
            (fixed_translate.y / coeffs.y).clamp(0.0, 1.0),
        );

        (fixed_translate, normalized_translate)
    }

    // Only called when the output changes
    // Update zoom/translate for the new output
    // Returns if we need to rerender
    pub fn output_needs_rerender(&mut self, old_output: i32) -> Result<bool> {
        let old = self
            .outputs
            .get(&old_output)
            .ok_or_else(|| anyhow!("output_needs_rerender: Invalid new output key"))?;
        let new = self
            .outputs
            .get(&self.state.cur_output)
            .ok_or_else(|| anyhow!("output_needs_rerender: Invalid current output key"))?;

        // Update translate values
        let new_node = &new.vsoutput.node_info;
        let new_size = Vec2::from([new_node.width as f32, new_node.height as f32]);

        // Scale normalized coords back to pixels
        self.state.translate_changed = true;
        self.state.translate = crate::utils::translate_norm_to_pixels(
            &self.state.translate_norm,
            &new_size,
            &self.available_size,
            self.state.zoom_factor,
        );

        // Different frame or output not rendered yet
        Ok(old.last_frame_no != new.last_frame_no || new.rendered_frame.is_none())
    }

    // Returns if the translate changed and we need to reprocess
    pub fn correct_translate_for_current_output(
        &mut self,
        new_translate: Vec2,
        normalized: bool,
    ) -> Result<bool> {
        if self.outputs.is_empty() {
            return Ok(false);
        }

        let info = {
            let output = self.outputs.get(&self.state.cur_output).ok_or_else(|| {
                anyhow!("correct_translate_for_current_output: Invalid current output key")
            })?;
            output.vsoutput.node_info.clone()
        };

        let image_size = Vec2::from([info.width as f32, info.height as f32]);
        let old_translate = self.state.translate;

        let new_translate = if normalized {
            crate::utils::translate_norm_to_pixels(
                &new_translate,
                &image_size,
                &self.available_size,
                self.state.zoom_factor,
            )
        } else {
            new_translate
        };

        let (fix_pixel, fix_norm) = self.fix_translation_bounds(&image_size, &new_translate);

        let useless_translate = self.state.fit_to_window && self.state.zoom_factor <= 1.0;

        // Update if necessary
        if fix_pixel != old_translate {
            if !useless_translate {
                self.state.translate = fix_pixel;
                self.state.translate_norm = fix_norm;
            } else {
                self.state.translate = Vec2::ZERO;
                self.state.translate_norm = Vec2::ZERO;
            }

            self.reprocess_outputs(true, true);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn any_input_focused(&self) -> bool {
        self.inputs_focused.values().any(|e| *e)
    }

    pub fn reprocess_outputs(&mut self, flag: bool, translate_changed: bool) {
        if translate_changed {
            self.state.translate_changed |= translate_changed;
        }

        self.outputs.values_mut().for_each(|o| {
            o.force_reprocess = flag;
        });
    }

    // Can only be called when an output is selected
    pub fn fetch_original_props(&mut self, ctx: &egui::Context) {
        let cur_output = self.state.cur_output;
        let cur_frame_no = self.state.cur_frame_no;

        let ctx = ctx.clone();
        let script = self.script.clone();

        if let Some(mut promise_mutex) = self.original_props_promise.try_lock() {
            // Block frame requests
            let frame_mutex = self.frame_promise.clone();

            let promise = poll_promise::Promise::spawn_thread("fetch_original_props", move || {
                if let Some(mut script_mutex) = script.try_lock() {
                    let _lock = frame_mutex.lock();

                    let props_res = script_mutex.get_original_props(cur_output, cur_frame_no);
                    script_mutex.add_vs_error(&props_res);

                    if let Ok(props) = props_res {
                        ctx.request_repaint();

                        Some(props)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

            *promise_mutex = Some(promise);
        }
    }

    pub fn check_original_props_finish(&mut self) -> Result<()> {
        if let Some(mut mutex) = self.original_props_promise.try_lock() {
            if let Some(promise) = &*mutex {
                if let Some(props) = promise.ready() {
                    let output = self
                        .outputs
                        .get_mut(&self.state.cur_output)
                        .ok_or_else(|| {
                            anyhow!("check_original_props_finish: Invalid current output key")
                        })?;
                    output.original_props = *props;

                    *mutex = None;
                }
            };
        }

        Ok(())
    }

    pub fn check_misc_keyboard_inputs(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        ui: &mut egui::Ui,
    ) {
        // Don't allow quit when inputs are still focused
        if !self.any_input_focused() {
            if ui.input().key_pressed(Key::Q) || ui.input().key_pressed(Key::Escape) {
                frame.close();
            } else if ui.input().key_pressed(Key::I) {
                self.state.show_gui = !self.state.show_gui;

                // Clear if the GUI is hidden
                if !self.state.show_gui {
                    self.inputs_focused.clear();
                }
            } else if ui.input().key_pressed(Key::R) {
                self.reload(ctx.clone())
            } else if ui.input().modifiers.ctrl
                && ui.input().modifiers.shift
                && ui.input().key_pressed(Key::C)
            {
                ui.output().copied_text = self.state.cur_frame_no.to_string();
            }
        }
    }

    pub fn state_needs_processing(
        state: &PreviewState,
        image_size: &Vec2,
        win_size: &Vec2,
    ) -> bool {
        if state.upscale_to_window
            && state.zoom_factor == 1.0
            && !state.translate_changed
            && state.translate_norm.length() <= 0.0
        {
            // Pure upscale
            // Scaled size to window bounds
            let target_size = dimensions_for_window(win_size, image_size).round();

            // Needs to be scaled up
            // Only go into processing if not using egui to scale the texture
            image_size.length() < target_size.length()
                && state.upsampling_filter != PreviewFilterType::Gpu
        } else if state.fit_to_window {
            // Downscaling image

            // Allow upscaling if enabled
            state.upscale_to_window || state.zoom_factor != 1.0
        } else {
            // Any other processing needed
            state.zoom_factor != 1.0
                || state.translate_norm.length() > 0.0
                || state.translate_changed
        }
    }

    pub fn check_promise_callbacks(&mut self, ctx: &egui::Context) -> Result<()> {
        // Initial callback
        self.check_reload_finish()?;

        // Poll new requested frame, replace old if ready
        self.check_rerender_finish(ctx)?;

        // Check for original props if requested
        self.check_original_props_finish()?;

        self.check_misc_finish(ctx);

        // We want a new frame
        // Previously rendering frames must have completed to request a new one
        self.try_rerender(ctx)?;

        Ok(())
    }

    pub fn add_error<T>(&mut self, key: &'static str, res: &Result<T>) {
        if let Err(e) = res {
            if let Some(list) = self.errors.get_mut(key) {
                list.push(format!("{:?}", e));
            } else {
                self.errors.insert(key, vec![format!("{:?}", e)]);
            }
        }
    }

    pub fn add_errors(&mut self, key: &'static str, errors: &[String]) {
        if !errors.is_empty() {
            if let Some(list) = self.errors.get_mut(key) {
                list.extend(errors.iter().cloned());
            } else {
                self.errors.insert(key, errors.to_owned());
            }
        }
    }

    pub fn change_script_file(&mut self, ctx: &egui::Context) {
        let script = self.script.clone();
        let ctx = ctx.clone();

        if let Some(mut promise_mutex) = self.misc_promise.try_lock() {
            let promise = Promise::spawn_thread("change_script", move || {
                let path = std::env::current_dir().unwrap();

                let new_file = rfd::FileDialog::new()
                    .set_title("Select a VapourSynth script file")
                    .add_filter("VapourSynth", &["vpy"])
                    .set_directory(path)
                    .pick_file();

                if let Some(new_file) = new_file {
                    let mut script_mutex = script.lock();

                    script_mutex.change_script_path(new_file);
                    ctx.request_repaint();

                    ReloadType::Reload
                } else {
                    ReloadType::None
                }
            });

            *promise_mutex = Some(promise);
        }
    }

    pub fn check_misc_finish(&mut self, ctx: &egui::Context) {
        let mut reload_type = None;

        if let Some(mutex) = self.misc_promise.try_lock() {
            if let Some(promise) = &*mutex {
                if let Some(rt) = promise.ready() {
                    reload_type = Some(*rt);
                }
            };
        }

        // Reload handles the promise reset, to avoid rendering other frames
        if let Some(reload_type) = reload_type {
            match reload_type {
                ReloadType::Reload => self.reload(ctx.clone()),
                ReloadType::Reprocess => {
                    self.reprocess_outputs(true, false);
                    *self.misc_promise.lock() = None;
                }
                ReloadType::None => *self.misc_promise.lock() = None,
            }
        }
    }

    pub fn init_transforms(&mut self) {
        let mut transforms = self.transforms.lock();

        if let Some(icc) = transforms.icc.as_mut() {
            icc.setup();
        }
    }

    pub fn change_icc_profile(&mut self, ctx: &egui::Context) {
        let ctx = ctx.clone();
        let transforms = self.transforms.clone();

        if let Some(mut promise_mutex) = self.misc_promise.try_lock() {
            let promise = Promise::spawn_thread("change_icc", move || {
                let new_file = rfd::FileDialog::new()
                    .set_title("Select a ICC profile file")
                    .add_filter("ICC", &["icc", "icm"])
                    .pick_file();

                if let Some(new_file) = new_file {
                    let mut transforms = transforms.lock();

                    let mut profile = IccProfile::srgb(new_file);
                    profile.setup();

                    transforms.icc = Some(profile);

                    ctx.request_repaint();

                    ReloadType::Reprocess
                } else {
                    ReloadType::None
                }
            });

            *promise_mutex = Some(promise);
        }
    }
}
