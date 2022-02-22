use eframe::egui::style::Margin;
use eframe::egui::{Key, Visuals};
use eframe::epaint::{self, Color32, Stroke, Vec2};
use eframe::{
    egui::{self, Frame},
    epi,
};

use super::*;
use vstransform::VSDitherAlgo;

impl epi::App for Previewer {
    fn name(&self) -> &str {
        "vspreview-rs"
    }

    fn setup(
        &mut self,
        ctx: &egui::Context,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        if let Some(storage) = _storage {
            self.state = epi::get_value(storage, epi::APP_KEY).unwrap_or(PreviewState {
                upscale_to_window: true,
                zoom_factor: 1.0,
                zoom_multiplier: 1.0,
                scroll_multiplier: 1.0,
                canvas_margin: 0.0,
                upsample_filter: PreviewFilterType::Bilinear,
                ..Default::default()
            })
        }

        self.state.cur_frame_no = 12345;
        self.state.upscale_to_window = true;
        self.state.zoom_factor = 1.0;
        self.state.zoom_multiplier = 1.0;
        self.state.translate = Vec2::ZERO;
        self.state.translate_norm = Vec2::ZERO;
        self.state.scroll_multiplier = 1.0;
        self.state.canvas_margin = 0.0;
        self.state.upsample_filter = PreviewFilterType::Bilinear;

        self.state.frame_transform_opts.add_dither = false;
        self.state.frame_transform_opts.dither_algo = VSDitherAlgo::None;

        if self.state.scroll_multiplier <= 0.0 {
            self.state.scroll_multiplier = 1.0;
        }

        // Limit to 2.0 multiplier every zoom, should be plenty
        if self.state.zoom_multiplier < 1.0 {
            self.state.zoom_multiplier = 1.0;
        } else if self.state.zoom_multiplier > 2.0 {
            self.state.zoom_multiplier = 2.0;
        }

        self.reload(ctx.clone(), frame.clone(), true);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let cur_output = self.state.cur_output;

        // Initial callback
        self.check_reload_finish();

        // We want a new frame
        // Previously rendering frames must have completed
        self.check_rerender(ctx, frame);

        // Poll new requested frame, replace old if ready
        if let Some(promise) = self.replace_frame_promise.as_ref() {
            if promise.poll().is_ready() {
                let output = self.outputs.get_mut(&cur_output).unwrap();
                output.frame_promise = Some(self.replace_frame_promise.take().unwrap());

                // Update last output once the new frame is rendered
                self.last_output_key = cur_output;
            }
        }

        let has_current_output = !self.outputs.is_empty() && self.outputs.contains_key(&cur_output);
        let new_frame = Frame::default()
            .fill(Color32::from_gray(51))
            .margin(Margin::symmetric(
                self.state.canvas_margin,
                self.state.canvas_margin,
            ))
            .stroke(Stroke::none());

        let mut global_visuals = Visuals::dark();
        global_visuals.window_shadow = epaint::Shadow::small_light();

        egui::CentralPanel::default()
            .frame(new_frame)
            .show(ctx, |ui| {
                ui.ctx().set_visuals(global_visuals);

                if !self.any_input_focused() {
                    if ui.input().key_pressed(Key::Q) || ui.input().key_pressed(Key::Escape) {
                        frame.quit();
                    } else if ui.input().key_pressed(Key::I) {
                        self.state.show_gui = !self.state.show_gui;

                        // Clear if the GUI is hidden
                        if !self.state.show_gui {
                            self.inputs_focused.clear();
                        }
                    }
                }

                // React on canvas resolution change
                if self.available_size != ui.available_size() {
                    self.available_size = ui.available_size();

                    self.reprocess_outputs();
                }

                if self.state.show_gui {
                    self.draw_state_window(ctx);
                }

                self.draw_centered_image(ctx, frame, ui);

                if self.state.show_gui && has_current_output {
                    self.draw_bottom_ui(ctx);
                }

                // Check at the end of frame for reprocessing
                self.check_rerender(ctx, frame);
            });
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, &self.state);
    }
}

impl Previewer {
    fn draw_state_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("State").show(ctx, |ui| {
            ui.label("Hello World!");
        });
    }

    fn draw_centered_image(&mut self, ctx: &egui::Context, frame: &epi::Frame, ui: &mut egui::Ui) {
        let cur_output = self.state.cur_output;
        let has_current_output = !self.outputs.is_empty() && self.outputs.contains_key(&cur_output);

        // If the outputs differ in frame index, we should wait for the render
        // instead of rendering the old frame
        let output_diff_frame = if has_current_output {
            let cur_output = self.outputs.get(&cur_output).unwrap();
            let last_output = self.outputs.get(&self.last_output_key).unwrap();

            last_output.last_frame_no != cur_output.last_frame_no
        } else {
            false
        };

        let zoom_delta = ui.input().zoom_delta();
        let scroll_delta = ui.input().scroll_delta;

        ui.centered_and_justified(|ui| {
            // Acquire frame texture to render now
            let frame_promise = if has_current_output {
                let output = self.outputs.get(&cur_output).unwrap();

                if output_diff_frame {
                    None
                } else {
                    output.frame_promise.as_ref()
                }
            } else {
                None
            };

            if self.reload_data.is_some() || frame_promise.is_none() {
                ui.add(egui::Spinner::new().size(200.0));
            } else if let Some(promise) = frame_promise {
                if let Some(pf) = promise.ready() {
                    let mut image_size: Option<[f32; 2]> = None;

                    if let Ok(pf) = &pf.read() {
                        image_size = Some(pf.vsframe.frame_image.size.map(|i| i as f32));

                        let tex_size = pf.texture.size_vec2();
                        ui.image(&pf.texture, tex_size);
                    }

                    if !self.any_input_focused() {
                        // We could read the image rendered
                        if let Some(image_size) = image_size {
                            if !self.rerender && self.replace_frame_promise.is_none() {
                                let size = Vec2::from(image_size);

                                self.handle_move_inputs(ui, &size, zoom_delta, scroll_delta);
                                self.handle_keypresses(ui);

                                if ui.input().key_pressed(Key::R) {
                                    self.reload(ctx.clone(), frame.clone(), true)
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    fn draw_bottom_ui(&mut self, ctx: &egui::Context) {
        let output = self.outputs.get_mut(&self.state.cur_output).unwrap();
        let node_info = &output.vsoutput.node_info;

        let transparent_frame =
            Frame::default()
                .fill(Color32::from_black_alpha(96))
                .margin(Margin {
                    left: 20.0,
                    right: 20.0,
                    top: 10.0,
                    bottom: 10.0,
                });

        egui::TopBottomPanel::bottom("BottomInfo")
            .frame(transparent_frame)
            .show(ctx, |ui| {
                // Add slider
                ui.spacing_mut().slider_width = self.available_size.x / 2.0;

                let mut slider_frame_no = self.state.cur_frame_no;

                let frames_slider =
                    egui::Slider::new(&mut slider_frame_no, 0..=(node_info.num_frames - 1))
                        .integer();

                let slider_res = ui.add(frames_slider);
                let in_use = slider_res.has_focus() || slider_res.drag_started();

                if let Some(current) = self.inputs_focused.get_mut("frame_slider") {
                    *current |= in_use;
                } else {
                    self.inputs_focused.insert("frame_slider", in_use);
                }

                // Released/changed value
                if !slider_res.has_focus()
                    && (slider_res.drag_released() || slider_res.lost_focus())
                {
                    output.last_frame_no = self.state.cur_frame_no;

                    self.state.cur_frame_no = slider_frame_no;

                    self.rerender = true;
                    self.inputs_focused.insert("frame_slider", false);
                } else if slider_frame_no != self.state.cur_frame_no {
                    self.state.cur_frame_no = slider_frame_no;
                }

                let output_info = format!("Output {} - {}", output.vsoutput.index, node_info);

                let node_info_label = egui::RichText::new(output_info)
                    .color(Color32::from_gray(200))
                    .size(20.0);
                ui.label(node_info_label);
            });
    }
}
