extern crate image;
extern crate piston_window;

use super::preview::Preview;
use super::previewed_script::PreviewedScript;

use super::image::ImageBuffer;
use piston_window::*;
use std::collections::HashSet;

use super::required_window_size;

const MIN_ZOOM: f64 = 1.0;
const MAX_ZOOM: f64 = 30.0;

pub struct Previewer {
    script: PreviewedScript,
    preview: Preview,
    cur_frame_no: u32,
    zoom_factor: f64,
    vertical_offset: f64,
    horizontal_offset: f64,
    keys_pressed: HashSet<Key>,
    rerender: bool,
}

impl Previewer {
    pub fn new(
        window: &mut PistonWindow,
        script: PreviewedScript,
        initial_frame: u32,
        font: conrod_core::text::Font,
    ) -> Self {
        let zoom_factor = 1.0;
        let vertical_offset = 0.0;
        let horizontal_offset = 0.0;

        let preview = Preview::new(window, &script, initial_frame, font);
        let window_size = required_window_size(&window, &preview);

        let previewer = Self {
            script,
            preview,
            cur_frame_no: initial_frame,
            zoom_factor,
            vertical_offset,
            horizontal_offset,
            keys_pressed: HashSet::new(),
            rerender: true,
        };

        window.set_size(window_size);
        window.set_title(format!(
            "VS Preview - Frame {}, Zoom: {:.0}x",
            initial_frame, zoom_factor
        ));

        previewer
    }

    pub fn rerender(&mut self, window: &mut PistonWindow, event: &Event) {
        let frame_no = self.cur_frame_no;

        window.set_title(format!(
            "VS Preview - Frame {}/{}, Zoom: {:.0}x",
            frame_no,
            self.script.get_num_frames(),
            self.zoom_factor,
        ));

        if self.rerender {
            match self.script.get_frame(frame_no) {
                Some(image) => self.preview.update(image),
                None => self.preview.update(ImageBuffer::new(0, 0)),
            };

            self.rerender = false;
        }

        self.preview.draw_image(
            window,
            event,
            (self.horizontal_offset, self.vertical_offset),
            self.zoom_factor,
        );

        if self.show_osd() {
            let text = self.script.get_summary();

            self.preview.draw_text(window, event, text);
        }
    }

    pub fn handle_key_press(&mut self, window: &PistonWindow, key: &Key) {
        match key {
            Key::Right | Key::Left | Key::Down | Key::Up => self.seek(key),
            Key::F5 => {
                self.reload_script();
                let new_max_frames = self.script.get_num_frames();

                if self.cur_frame_no > new_max_frames {
                    self.cur_frame_no = new_max_frames;
                }
            }
            Key::LCtrl | Key::LShift => {
                self.keys_pressed.insert(*key);
            }
            Key::S => self.save_screenshot(),
            Key::Home | Key::End => {
                let change = match key {
                    Key::Home => 1.0,
                    Key::End => -1.0,
                    _ => 0.0,
                };

                self.translate_horizontally(window, change);
            }
            Key::PageUp | Key::PageDown => {
                let change = match key {
                    Key::PageUp => 1.0,
                    Key::PageDown => -1.0,
                    _ => 0.0,
                };

                self.translate_vertically(window, change);
            }
            Key::I => {
                if !self.keys_pressed.contains(key) {
                    self.keys_pressed.insert(*key);
                } else {
                    self.keys_pressed.remove(key);
                }
            }
            _ => (),
        };
    }

    pub fn handle_key_release(&mut self, key: &Key) {
        match key {
            Key::LCtrl | Key::LShift => {
                self.keys_pressed.remove(key);
            }
            _ => (),
        }
    }

    pub fn handle_mouse_scroll(&mut self, window: &PistonWindow, ticks: [f64; 2]) {
        let change = ticks.last().unwrap();

        if self.keys_pressed.contains(&Key::LCtrl) {
            let (img_w, draw_w) = (self.preview.get_width() as f64, window.draw_size().width);
            let (img_h, draw_h) = (self.preview.get_height() as f64, window.draw_size().height);

            self.zoom_factor += change;

            if self.zoom_factor < MIN_ZOOM {
                self.zoom_factor = MIN_ZOOM;
            } else if self.zoom_factor > MAX_ZOOM {
                self.zoom_factor = MAX_ZOOM;
            }

            self.set_vertical_offset(img_h, draw_h);
            self.set_horizontal_offset(img_w, draw_w);
        } else if self.keys_pressed.contains(&Key::LShift) {
            self.translate_horizontally(window, *change);
        } else {
            self.translate_vertically(window, *change);
        }
    }

    fn reload_script(&mut self) {
        self.script.reload();
        self.rerender = true;
    }

    fn seek(&mut self, key: &Key) {
        if !self.rerender {
            let script = &self.script;
            let frame_write = self.cur_frame_no;
            let mut current = frame_write;

            let num_frames = script.get_num_frames();
            let frame_rate_num = script.get_frame_rate();

            match key {
                Key::Right => {
                    if current < num_frames {
                        current += 1
                    } else {
                        current = num_frames
                    }
                }
                Key::Left => {
                    if current > 0 {
                        current -= 1
                    } else {
                        current = 0
                    }
                }
                Key::Up => {
                    if current > frame_rate_num {
                        current -= frame_rate_num
                    } else {
                        current = 0
                    }
                }
                Key::Down => current += frame_rate_num,
                _ => (),
            }

            if current > num_frames {
                self.cur_frame_no = num_frames;
            } else if current != frame_write {
                self.cur_frame_no = current;
                self.rerender = true;
            }
        }
    }

    fn translate_horizontally(&mut self, window: &PistonWindow, change: f64) {
        let (img_w, draw_w) = (self.preview.get_width() as f64, window.draw_size().width);

        if !self.preview.fits_in_view(&window, self.zoom_factor) {
            self.horizontal_offset += (draw_w / 2.5) * change;
        }

        self.set_horizontal_offset(img_w, draw_w);
    }

    fn translate_vertically(&mut self, window: &PistonWindow, change: f64) {
        let (img_h, draw_h) = (self.preview.get_height() as f64, window.draw_size().height);

        if !self.preview.fits_in_view(&window, self.zoom_factor) {
            self.vertical_offset += (draw_h / 2.5) * change;
        }

        self.set_vertical_offset(img_h, draw_h);
    }

    fn save_screenshot(&self) {
        let frame_write = self.cur_frame_no;
        let img = image::DynamicImage::ImageRgba8(self.preview.cloned_frame()).to_rgb();
        let mut save_path = self.script.get_script_dir();

        let screen_file = format!("vspreview-{}.png", frame_write);
        save_path.push(screen_file);

        img.save_with_format(&save_path, image::ImageFormat::Png)
            .unwrap();

        println!("Screenshot saved to {}", &save_path.to_str().unwrap());
    }

    fn set_vertical_offset(&mut self, img_h: f64, draw_h: f64) {
        let mut max_off = (-1.0 * self.zoom_factor * img_h) + draw_h;

        if max_off.is_sign_positive() {
            max_off = 0.0;
        }

        if self.vertical_offset.is_sign_positive() {
            self.vertical_offset = 0.0;
        } else if self.vertical_offset < max_off {
            self.vertical_offset = max_off;
        }
    }

    fn set_horizontal_offset(&mut self, img_w: f64, draw_w: f64) {
        let mut max_off = (-1.0 * self.zoom_factor * img_w) + draw_w;

        if max_off.is_sign_positive() {
            max_off = 0.0;
        }

        if self.horizontal_offset.is_sign_positive() {
            self.horizontal_offset = 0.0;
        } else if self.horizontal_offset < max_off {
            self.horizontal_offset = max_off;
        }
    }

    pub fn get_size(&self) -> (u32, u32) {
        (
            self.preview.get_width() as u32,
            self.preview.get_height() as u32,
        )
    }

    pub fn get_current_no(&self) -> u32 {
        self.cur_frame_no
    }

    pub fn get_clip_length(&self) -> u32 {
        self.script.get_num_frames()
    }

    pub fn seek_to(&mut self, frame_no: f64) {
        self.cur_frame_no = frame_no as u32;
        self.rerender = true;
    }

    pub fn show_osd(&self) -> bool {
        self.keys_pressed.contains(&Key::I)
    }
}
