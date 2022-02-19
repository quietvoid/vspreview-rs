use std::sync::Arc;
use std::sync::Mutex;

use eframe::egui::style::Margin;
use eframe::egui::Key;
use eframe::egui::Ui;
use eframe::egui::Visuals;
use eframe::epaint::Color32;
use eframe::epaint::ColorImage;
use eframe::epaint::Stroke;
use eframe::{
    egui::{self, Frame},
    epi,
};
use poll_promise::Promise;

use crate::utils::{process_image, MAX_ZOOM, MIN_ZOOM};
use crate::vs_handler::PreviewedScript;
use crate::vs_handler::VSOutput;

type APreviewFrame = Arc<PreviewFrame>;
type FramePromise = Promise<APreviewFrame>;

#[derive(Default)]
pub struct Previewer {
    pub script: Arc<Mutex<PreviewedScript>>,
    pub reload_data: Option<Promise<(Vec<VSOutput>, APreviewFrame)>>,
    pub state: PreviewState,

    pub initialized: bool,

    pub outputs: Vec<PreviewOutput>,
    pub last_output_index: usize,

    pub rerender: bool,
    pub reprocess: bool,
    pub replace_frame_promise: Option<FramePromise>,

    pub available_size: eframe::epaint::Vec2,
}

#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PreviewState {
    pub cur_output: i32,
    pub cur_frame_no: u32,

    pub scale_to_window: bool,

    pub zoom_factor: f32,
    pub translate_x: u32,
    pub translate_y: u32,
}

#[derive(Default)]
pub struct PreviewOutput {
    pub vsoutput: VSOutput,

    pub frame_promise: Option<FramePromise>,

    pub force_reprocess: bool,
    pub last_frame_no: u32,
}

#[derive(Clone)]
pub struct PreviewFrame {
    // Thread safe as always immutable
    pub image: Arc<ColorImage>,

    pub texture: egui::TextureHandle,
    pub frame_type: String,
}

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
                scale_to_window: true,
                zoom_factor: 1.0,
                ..Default::default()
            })
        }

        self.state.cur_frame_no = 12345;
        self.state.zoom_factor = 1.0;
        self.state.scale_to_window = false;

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
                let output = self.outputs.get_mut(cur_output as usize).unwrap();
                output.frame_promise = Some(self.replace_frame_promise.take().unwrap());

                // Update last output once the new frame is rendered
                self.last_output_index = cur_output as usize;
            }
        }

        let num_outputs = self.outputs.len();

        // If the outputs differ in frame index, we should wait for the render
        // instead of rendering the old frame
        let output_diff_frame = if num_outputs > 0 {
            let cur_output = self.outputs.get(cur_output as usize).unwrap();
            let last_output = self.outputs.get(self.last_output_index).unwrap();

            last_output.last_frame_no != cur_output.last_frame_no
        } else {
            false
        };

        let mut new_frame = Frame::default()
            .fill(Color32::from_gray(24))
            .margin(Margin::symmetric(1.0, 1.0));

        // Remove margin & stroke to keep original image intact
        if !self.state.scale_to_window {
            new_frame.margin = Margin::symmetric(0.0, 0.0);
            new_frame.stroke = Stroke::none();
        }

        egui::CentralPanel::default()
            .frame(new_frame)
            .show(ctx, |ui| {
                *ui.visuals_mut() = Visuals::dark();

                // React on canvas resolution change
                if self.available_size != ui.available_size() {
                    self.available_size = ui.available_size();
                    self.outputs
                        .iter_mut()
                        .for_each(|o| o.force_reprocess = true);
                }

                ui.centered_and_justified(|ui| {
                    // Acquire frame texture to render now
                    let frame_promise = if num_outputs > 0 {
                        let output = self.outputs.get_mut(cur_output as usize).unwrap();

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
                            ui.image(&pf.texture, pf.texture.size_vec2());

                            if !self.rerender && self.replace_frame_promise.is_none() {
                                self.handle_keypresses(ui);
                                self.handle_mouse_inputs(ui);

                                if ui.input().key_pressed(Key::R) {
                                    self.reload(ctx.clone(), frame.clone(), true)
                                } else if ui.input().key_pressed(Key::Q)
                                    || ui.input().key_pressed(Key::Escape)
                                {
                                    frame.quit();
                                }
                            }
                        }
                    }
                });
            });
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, &self.state);
    }
}

impl Previewer {
    fn reload(&mut self, ctx: egui::Context, frame: epi::Frame, force_reload: bool) {
        let state = self.state.clone();
        let cur_output = state.cur_output;
        let cur_frame_no = state.cur_frame_no;

        let script = self.script.clone();

        let size = self.available_size;

        self.reload_data = Some(poll_promise::Promise::spawn_thread(
            "initialization/reload",
            move || {
                // This is OK because we didn't have an initial texture
                let mut mutex = script.lock().unwrap();

                if force_reload || !mutex.is_initialized() {
                    mutex.reload();
                }

                let outputs = mutex.get_outputs();

                let vsframe = mutex.get_frame(cur_output, cur_frame_no).unwrap();
                let image = Arc::new(vsframe.frame_image);
                let processed_image = process_image(image.clone(), state, size);

                let pf = PreviewFrame {
                    image,
                    texture: ctx.load_texture("initial_frame", processed_image),
                    frame_type: vsframe.frame_type,
                };

                frame.request_repaint();

                (outputs, Arc::new(pf))
            },
        ));
    }

    fn check_reload_finish(&mut self) {
        if let Some(promise) = &self.reload_data {
            if let Some(data) = promise.ready() {
                let cur_output = self.state.cur_output;

                self.outputs = data
                    .0
                    .iter()
                    .map(|o| PreviewOutput {
                        vsoutput: o.clone(),
                        ..Default::default()
                    })
                    .collect();

                println!("Got outputs: {:?}", &self.outputs.len());
                self.outputs
                    .iter()
                    .for_each(|o| println!("{:?}", o.vsoutput));

                let output = self.outputs.get_mut(cur_output as usize).unwrap();
                let node_info = &output.vsoutput.node_info;

                let (sender, promise) = Promise::new();
                sender.send(data.1.clone());

                output.frame_promise = Some(promise);

                self.reload_data = None;
                self.last_output_index = cur_output as usize;

                if self.state.cur_frame_no >= node_info.num_frames {
                    self.state.cur_frame_no = node_info.num_frames - 1;
                }

                // First reload
                if !self.initialized {
                    self.initialized = true;

                    // Force rerender once we have the initial window size
                    if self.state.scale_to_window {
                        self.rerender = true;
                    }
                }
            }
        }
    }

    fn check_rerender(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        if !self.outputs.is_empty() {
            let output = self
                .outputs
                .get_mut(self.state.cur_output as usize)
                .unwrap();

            if output.force_reprocess {
                self.rerender = true;
                self.reprocess = true;
                output.force_reprocess = false;
            }
        }

        if self.rerender && self.replace_frame_promise.is_none() {
            self.rerender = false;

            let reprocess = self.reprocess;
            self.reprocess = false;

            let state = self.state.clone();
            let size = self.available_size;

            let ctx = ctx.clone();
            let frame = frame.clone();

            let script = self.script.clone();

            let pf = if reprocess {
                self.get_current_frame()
            } else {
                None
            };

            self.replace_frame_promise = Some(poll_promise::Promise::spawn_thread(
                "fetch_frame",
                move || Self::get_preview_image(ctx, frame, script, state, pf, reprocess, size),
            ));
        }
    }

    fn get_current_frame(&self) -> Option<APreviewFrame> {
        if !self.outputs.is_empty() {
            let output = self.outputs.get(self.state.cur_output as usize).unwrap();

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

    fn get_preview_image(
        ctx: egui::Context,
        frame: epi::Frame,
        script: Arc<Mutex<PreviewedScript>>,
        state: PreviewState,
        pf: Option<APreviewFrame>,
        reprocess: bool,
        size: eframe::epaint::Vec2,
    ) -> APreviewFrame {
        // This is fine because only one promise may be executing at a time
        let mut mutex = script.lock().unwrap();

        let have_existing_frame = pf.is_some();

        // Reuse existing image, process and recreate texture
        let pf = if reprocess && have_existing_frame {
            let pf = pf.unwrap();
            let processed_image = process_image(pf.image.clone(), state, size);

            PreviewFrame {
                image: pf.image.clone(),
                texture: ctx.load_texture("frame", processed_image),
                frame_type: pf.frame_type.clone(),
            }
        } else {
            // Request new frame, process and recreate texture
            let vsframe = mutex
                .get_frame(state.cur_output, state.cur_frame_no)
                .unwrap();
            let image = Arc::new(vsframe.frame_image);
            let processed_image = process_image(image.clone(), state, size);

            PreviewFrame {
                image,
                texture: ctx.load_texture("frame", processed_image),
                frame_type: vsframe.frame_type,
            }
        };

        // Once frame is ready
        frame.request_repaint();

        Arc::new(pf)
    }

    fn handle_keypresses(&mut self, ui: &mut Ui) {
        let mut rerender = self.check_update_seek(ui);
        rerender |= self.check_update_output(ui);

        self.rerender = rerender;
    }

    /// Returns whether to rerender
    fn check_update_seek(&mut self, ui: &mut Ui) -> bool {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return false;
        }

        let output = self
            .outputs
            .get_mut(self.state.cur_output as usize)
            .unwrap();
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

    fn check_update_output(&mut self, ui: &mut Ui) -> bool {
        // Must not have modifiers
        if !ui.input().modifiers.is_none() {
            return false;
        }

        let cur_output = self.state.cur_output;
        let num_outputs = self.outputs.len();

        let mut res = if ui.input().key_pressed(Key::Num1) && cur_output != 0 && num_outputs >= 1 {
            self.state.cur_output = 0;
            true
        } else if ui.input().key_pressed(Key::Num2) && cur_output != 1 && num_outputs >= 2 {
            self.state.cur_output = 1;
            true
        } else if ui.input().key_pressed(Key::Num3) && cur_output != 2 && num_outputs >= 3 {
            self.state.cur_output = 2;
            true
        } else if ui.input().key_pressed(Key::Num4) && cur_output != 3 && num_outputs >= 4 {
            self.state.cur_output = 3;
            true
        } else if ui.input().key_pressed(Key::Num5) && cur_output != 4 && num_outputs >= 5 {
            self.state.cur_output = 4;
            true
        } else if ui.input().key_pressed(Key::Num6) && cur_output != 5 && num_outputs >= 6 {
            self.state.cur_output = 5;
            true
        } else if ui.input().key_pressed(Key::Num7) && cur_output != 6 && num_outputs >= 7 {
            self.state.cur_output = 6;
            true
        } else if ui.input().key_pressed(Key::Num8) && cur_output != 7 && num_outputs >= 8 {
            self.state.cur_output = 7;
            true
        } else if ui.input().key_pressed(Key::Num9) && cur_output != 8 && num_outputs >= 9 {
            self.state.cur_output = 8;
            true
        } else if ui.input().key_pressed(Key::Num0) && cur_output != 9 && num_outputs >= 10 {
            self.state.cur_output = 9;
            true
        } else {
            false
        };

        // Changed output
        if res {
            let out = self.outputs.get(cur_output as usize).unwrap();
            let new = self.outputs.get(self.state.cur_output as usize).unwrap();

            // Rerender every time just because it's simpler than syncing across outputs
            let rerender = self.state.zoom_factor != 1.0
                || self.state.translate_x != 0
                || self.state.translate_y != 0;

            res = out.last_frame_no != new.last_frame_no || rerender;
        }

        res
    }

    fn handle_mouse_inputs(&mut self, ui: &mut Ui) {
        if ui.input().modifiers.ctrl {
            // Zoom
            let mut delta = ui.input().zoom_delta();
            let mut new_factor = self.state.zoom_factor;

            let zoom_modifier = if ui.input().key_pressed(Key::ArrowDown) {
                delta = 0.0;
                0.1
            } else if ui.input().key_pressed(Key::ArrowUp) {
                delta = 2.0;
                0.1
            } else {
                1.0
            };

            // Ignore 1.0 delta, means no zoom done
            if delta < 1.0 {
                // Smaller unzooming when below 1.0
                if new_factor <= 1.0 {
                    new_factor -= 0.125;
                } else {
                    new_factor -= zoom_modifier;
                }

                new_factor = new_factor.clamp(MIN_ZOOM, MAX_ZOOM);
            } else if delta > 1.0 {
                if new_factor < 1.0 {
                    // Zoom back from a unzoomed state
                    // Go back to no zoom
                    new_factor += 0.125;
                } else {
                    new_factor += zoom_modifier;
                }

                new_factor = new_factor.clamp(MIN_ZOOM, MAX_ZOOM);
            }

            if delta != 1.0 && new_factor != self.state.zoom_factor {
                self.state.zoom_factor = f32::trunc(new_factor * 1000.0) / 1000.0;

                self.rerender = true;
                self.reprocess = true;

                println!("Zoom factor {}", self.state.zoom_factor);
            }
        } else if ui.input().modifiers.shift {
            // Horizontal scroll
            let dx = ui.input().scroll_delta.x;
        } else {
            // Vertical scroll
            let dy = ui.input().scroll_delta.y;
        }
    }
}
