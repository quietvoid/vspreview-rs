use eframe::egui::style::Margin;
use eframe::egui::{DragValue, Key, RichText, Visuals};
use eframe::epaint::{self, Color32, Stroke, Vec2};
use eframe::{
    egui::{self, Frame},
    epi,
};

use itertools::Itertools;

use super::*;
use vstransform::VSDitherAlgo;

const STATE_LABEL_COLOR: Color32 = Color32::from_gray(160);

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
                upscale_to_window: false,
                zoom_factor: 1.0,
                zoom_multiplier: 1.0,
                scroll_multiplier: 1.0,
                canvas_margin: 0.0,
                upsample_filter: PreviewFilterType::Bilinear,
                ..Default::default()
            })
        }

        self.state.cur_frame_no = 12345;
        self.state.upscale_to_window = false;
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
                    self.draw_state_window(ctx, frame);
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
    fn draw_state_window(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let has_current_output =
            !self.outputs.is_empty() && self.outputs.contains_key(&self.state.cur_output);

        egui::Window::new("State")
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                egui::Grid::new("node_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        self.output_select_ui(ui);
                        ui.end_row();

                        self.zoom_slider_ui(ui);
                        ui.end_row();

                        self.translate_drag_ui(ui);
                        ui.end_row();
                    });

                if has_current_output {
                    ui.separator();

                    self.frameprops_ui(frame, ui);
                }
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

                // We want a bit more precision to within ~50 frames
                let frames_slider =
                    egui::Slider::new(&mut slider_frame_no, 0..=(node_info.num_frames - 1))
                        .smart_aim(false)
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

    pub fn output_select_ui(&mut self, ui: &mut egui::Ui) {
        let old_output = self.state.cur_output;
        let mut new_output = old_output;

        ui.label(RichText::new("Output").color(STATE_LABEL_COLOR));

        egui::ComboBox::from_id_source(egui::Id::new("output_select"))
            .selected_text(format!("Output {}", new_output))
            .show_ui(ui, |ui| {
                for i in self.outputs.keys().sorted() {
                    ui.selectable_value(&mut new_output, *i, format!("Output {}", i));
                }
            });

        // Changed output
        if new_output != old_output {
            self.state.cur_output = new_output;

            {
                let out = self.outputs.get_mut(&old_output).unwrap();
                out.original_props_promise = None;
            }

            if self.output_needs_rerender(old_output) {
                self.rerender = true;
            }
        }
    }

    pub fn zoom_slider_ui(&mut self, ui: &mut egui::Ui) {
        let old_zoom = self.state.zoom_factor;
        let mut new_zoom = old_zoom;

        let zoom_range = MIN_ZOOM..=MAX_ZOOM;
        let frames_slider = egui::Slider::new(&mut new_zoom, zoom_range).max_decimals(3);

        ui.label(RichText::new("Zoom factor").color(STATE_LABEL_COLOR));
        ui.add(frames_slider);

        if new_zoom != old_zoom {
            self.state.zoom_factor = new_zoom;
            self.rerender = true;

            self.correct_translate_for_current_output();
        }
    }

    pub fn translate_drag_ui(&mut self, ui: &mut egui::Ui) {
        let old_translate = self.state.translate_norm;
        let mut new_translate = old_translate;

        ui.label(RichText::new("Translate").color(STATE_LABEL_COLOR));
        ui.horizontal(|ui| {
            let x_drag = DragValue::new(&mut new_translate.x)
                .speed(0.01)
                .clamp_range(-0.01..=1.0)
                .max_decimals(3);

            let y_drag = DragValue::new(&mut new_translate.y)
                .speed(0.01)
                .clamp_range(-0.01..=1.0)
                .max_decimals(3);

            ui.label(RichText::new("x").color(STATE_LABEL_COLOR));
            ui.add(x_drag);
            ui.label(RichText::new("y").color(STATE_LABEL_COLOR));
            ui.add(y_drag);
        });

        if new_translate != old_translate && new_translate.length() > 0.0 {
            self.state.translate_norm = new_translate;

            self.update_pixels_translation_for_current_output();
            self.rerender = true;
        }
    }

    pub fn frameprops_ui(&mut self, frame: &epi::Frame, ui: &mut egui::Ui) {
        let output = self.outputs.get(&self.state.cur_output).unwrap();
        let mut props = None;

        if let Some(promise) = &output.frame_promise {
            if let Some(pf) = promise.ready() {
                if let Ok(pf) = &pf.read() {
                    props = Some(pf.vsframe.props.clone())
                }
            }
        }

        // Overwrite from original if available
        if let Some(promise) = &output.original_props_promise {
            if let Some(Some(p)) = promise.ready() {
                props = Some(p.clone());
            }
        }

        if let Some(props) = props {
            let header = RichText::new("Frame props").color(STATE_LABEL_COLOR);

            egui::CollapsingHeader::new(header).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                egui::Grid::new("props_grid")
                    .num_columns(2)
                    .spacing([8.0, -2.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Frame type").color(STATE_LABEL_COLOR));
                        ui.label(props.frame_type);
                        ui.end_row();

                        ui.label(RichText::new("Color range").color(STATE_LABEL_COLOR));
                        ui.label(props.color_range.to_string());
                        ui.end_row();

                        ui.label(RichText::new("Chroma location").color(STATE_LABEL_COLOR));
                        ui.label(props.chroma_location.to_string());
                        ui.end_row();

                        ui.label(RichText::new("Primaries").color(STATE_LABEL_COLOR));
                        ui.label(props.primaries.to_string());
                        ui.end_row();

                        ui.label(RichText::new("Matrix").color(STATE_LABEL_COLOR));
                        ui.label(props.matrix.to_string());
                        ui.end_row();

                        ui.label(RichText::new("Transfer").color(STATE_LABEL_COLOR));
                        ui.label(props.transfer.to_string());
                        ui.end_row();

                        if let Some(sc) = props.is_scenecut {
                            let (v, color) = crate::utils::icon_color_for_bool(sc);

                            ui.label(RichText::new("Scene cut").color(STATE_LABEL_COLOR));
                            ui.label(RichText::new(v).size(20.0).color(color));
                            ui.end_row();
                        }

                        if let Some(hdr10_meta) = props.hdr10_metadata {
                            ui.label(RichText::new("Mastering display").color(STATE_LABEL_COLOR));

                            let prim_label =
                                egui::Label::new(hdr10_meta.mastering_display.to_string())
                                    .sense(egui::Sense::click());
                            let mdcv_res = ui.add(prim_label);

                            ui.scope(|ui| {
                                if mdcv_res
                                    .on_hover_text("Click to copy x265 setting")
                                    .clicked()
                                {
                                    let arg = format!(
                                        "--master-display \"{}\"",
                                        hdr10_meta.mastering_display.x265_string()
                                    );
                                    ui.output().copied_text = arg;
                                }
                            });
                            ui.end_row();

                            if let (Some(maxcll), Some(maxfall)) =
                                (hdr10_meta.maxcll, hdr10_meta.maxfall)
                            {
                                ui.label(
                                    RichText::new("Content light level").color(STATE_LABEL_COLOR),
                                );

                                let cll_label = egui::Label::new(format!(
                                    "MaxCLL: {maxcll}, MaxFALL: {maxfall}"
                                ))
                                .sense(egui::Sense::click());
                                let cll_res = ui.add(cll_label);

                                ui.scope(|ui| {
                                    if cll_res
                                        .on_hover_text("Click to copy x265 setting")
                                        .clicked()
                                    {
                                        let arg = format!("--maxcll \"{},{}\"", maxcll, maxfall);
                                        ui.output().copied_text = arg;
                                    }
                                });
                                ui.end_row();
                            }
                        }

                        let (v, color) = crate::utils::icon_color_for_bool(props.is_dolbyvision);
                        ui.label(RichText::new("Dolby Vision").color(STATE_LABEL_COLOR));
                        ui.label(RichText::new(v).size(20.0).color(color));
                        ui.end_row();

                        if let Some(cambi) = props.cambi_score {
                            let rounded = egui::emath::round_to_decimals(cambi, 4);
                            ui.label(RichText::new("CAMBI score").color(STATE_LABEL_COLOR));
                            ui.label(rounded.to_string());
                            ui.end_row();
                        }

                        ui.label("");
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            let reload_btn = ui.button("Reload original props");

                            if reload_btn.clicked() {
                                self.fetch_original_props(frame);
                            }
                        });
                        ui.end_row();
                    });
            });
        }
    }
}
