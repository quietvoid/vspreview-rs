extern crate image;
extern crate piston_window;

use super::previewed_script::PreviewedScript;

use std::collections::HashSet;

use image::{ImageBuffer, Rgba};
use piston_window::*;

const MIN_ZOOM: f64 = 1.0;
const MAX_ZOOM: f64 = 30.0;

pub struct Previewer {
    script: PreviewedScript,
    cur_frame: ImageBuffer<Rgba<u8>, Vec<u8>>,
    cur_frame_no: u32,

    glyphs: Glyphs,
    texture_context: G2dTextureContext,
    texture: G2dTexture,
    zoom_factor: f64,
    vertical_offset: f64,
    horizontal_offset: f64,
    keys_pressed: HashSet<Key>,
    rerender: bool,
}

impl Previewer {
    pub fn new(window: &mut PistonWindow, script: PreviewedScript, initial_frame: u32) -> Self {
        let font = std::include_bytes!("../assets/FiraSans-Regular.ttf");

        let zoom_factor = 1.0;
        let vertical_offset = 0.0;
        let horizontal_offset = 0.0;

        let cur_frame = script.get_frame(initial_frame);

        let mut texture_context = window.create_texture_context();
        let image: G2dTexture = Texture::from_image(
            &mut texture_context,
            &cur_frame,
            &TextureSettings::new().mag(texture::Filter::Nearest),
        )
        .unwrap();

        let glyphs = Glyphs::from_bytes(
            font,
            window.create_texture_context(),
            TextureSettings::new(),
        )
        .unwrap();

        let previewer = Self {
            script,
            cur_frame,
            cur_frame_no: initial_frame,
            glyphs,
            texture_context,
            texture: image,
            zoom_factor,
            vertical_offset,
            horizontal_offset,
            keys_pressed: HashSet::new(),
            rerender: true,
        };

        let window_size = previewer.get_window_size(&window);
        window.set_size(window_size);
        window.set_title(format!(
            "VS Preview - Frame {}, Zoom: {:.0}x",
            initial_frame, zoom_factor
        ));

        previewer
    }

    pub fn rerender(&mut self, window: &mut PistonWindow, event: &Event) {
        let (dx, dy) = self.get_scaling(window);
        let osd_y = (window.draw_size().height * dy) - 12.0;

        let frame_no = self.cur_frame_no;

        if self.rerender {
            self.cur_frame = self.script.get_frame(frame_no);
            self.texture
                .update(&mut self.texture_context, &self.cur_frame)
                .unwrap();

            self.rerender = false;
        }

        window.set_title(format!(
            "VS Preview - Frame {}/{}, Zoom: {:.0}x",
            frame_no,
            self.script.get_num_frames(),
            self.zoom_factor,
        ));

        window.draw_2d(event, |c, g, device| {
            clear([0.20; 4], g);

            let img_transform = c
                .transform
                .scale(dx, dy)
                .trans(self.horizontal_offset, self.vertical_offset)
                .zoom(self.zoom_factor);

            piston_window::image(&self.texture, img_transform, g);

            if self.keys_pressed.contains(&Key::I) {
                let transform = c.transform.trans(10.0, osd_y).zoom(0.5);
                text::Text::new_color([0.85, 0.85, 0.85, 1.0], 32)
                    .draw(
                        self.script.get_summary(),
                        &mut self.glyphs,
                        &c.draw_state,
                        transform,
                        g,
                    )
                    .unwrap();

                self.glyphs.factory.encoder.flush(device);
            }

            // Flush to GPU
            self.texture_context.encoder.flush(device);
        });
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
            let (img_w, draw_w) = (self.texture.get_width() as f64, window.draw_size().width);
            let (img_h, draw_h) = (self.texture.get_height() as f64, window.draw_size().height);

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

    fn get_window_size(&self, window: &PistonWindow) -> Size {
        let (dx, dy) = self.get_scaling(window);

        let new_width = self.texture.get_width() as f64 * dx;
        let new_height = self.texture.get_height() as f64 * dy;

        Size::from((new_width, new_height))
    }

    fn get_scaling(&self, window: &PistonWindow) -> (f64, f64) {
        let size = window.size();
        let draw_size = window.draw_size();

        let dx = size.width as f64 / draw_size.width as f64;
        let dy = size.height as f64 / draw_size.height as f64;

        (dx, dy)
    }

    fn fits_in_view(&self, window: &PistonWindow) -> bool {
        let image_w = self.texture.get_width() as f64 * self.zoom_factor;
        let image_h = self.texture.get_height() as f64 * self.zoom_factor;

        let draw_size = window.draw_size();

        draw_size.width >= image_w || draw_size.height >= image_h
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
        let (img_w, draw_w) = (self.texture.get_width() as f64, window.draw_size().width);

        if !self.fits_in_view(&window) {
            self.horizontal_offset += draw_w * change;
        }

        self.set_horizontal_offset(img_w, draw_w);
    }

    fn translate_vertically(&mut self, window: &PistonWindow, change: f64) {
        let (img_h, draw_h) = (self.texture.get_height() as f64, window.draw_size().height);

        if !self.fits_in_view(&window) {
            self.vertical_offset += draw_h * change;
        }

        self.set_vertical_offset(img_h, draw_h);
    }

    fn save_screenshot(&self) {
        let frame_write = self.cur_frame_no;
        let img = image::DynamicImage::ImageRgba8(self.cur_frame.to_owned()).to_rgb();
        let mut save_path = self.script.get_script_dir();

        let screen_file = format!("vspreview-{}.png", frame_write);
        save_path.push(screen_file);

        img.save_with_format(save_path, image::ImageFormat::Png)
            .unwrap();

        println!("Screenshot ");
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
}
