use super::image::{ImageBuffer, Rgba};

use super::previewed_script::PreviewedScript;

use super::get_scaling;
use piston_window::*;

pub struct Preview {
    cur_frame: ImageBuffer<Rgba<u8>, Vec<u8>>,
    glyphs: Glyphs,
    texture_context: G2dTextureContext,
    texture: G2dTexture,
}

impl Preview {
    pub fn new(
        window: &mut PistonWindow,
        script: &PreviewedScript,
        initial_frame: u32,
        font: conrod_core::text::Font,
    ) -> Self {
        let cur_frame = script.get_frame(initial_frame);

        let mut texture_context = window.create_texture_context();
        let texture: G2dTexture = Texture::from_image(
            &mut texture_context,
            &cur_frame,
            &TextureSettings::new().mag(texture::Filter::Nearest),
        )
        .unwrap();

        let glyphs = Glyphs::from_font(
            font,
            window.create_texture_context(),
            TextureSettings::new(),
        );

        Self {
            cur_frame,
            glyphs,
            texture_context,
            texture,
        }
    }

    pub fn update(&mut self, image: ImageBuffer<Rgba<u8>, Vec<u8>>) {
        self.cur_frame = image;

        if self.texture.get_width() != self.cur_frame.width()
            || self.texture.get_height() != self.cur_frame.height()
        {
            self.texture = G2dTexture::from_image(
                &mut self.texture_context,
                &self.cur_frame,
                &TextureSettings::new().mag(texture::Filter::Nearest),
            )
            .unwrap()
        } else {
            self.texture
                .update(&mut self.texture_context, &self.cur_frame)
                .unwrap();
        }
    }

    pub fn draw_image(
        &mut self,
        window: &mut PistonWindow,
        event: &Event,
        offsets: (f64, f64),
        zoom_factor: f64,
    ) {
        let (dx, dy) = get_scaling(window);
        let (horizontal_offset, vertical_offset) = offsets;

        window.draw_2d(event, |context, graphics, device| {
            clear([0.2, 0.2, 0.2, 1.0], graphics);

            let img_transform = context
                .transform
                .scale(dx, dy)
                .trans(horizontal_offset, vertical_offset)
                .zoom(zoom_factor);

            piston_window::image(&self.texture, img_transform, graphics);

            // Flush to GPU
            self.texture_context.encoder.flush(device);
        });
    }

    pub fn draw_text(&mut self, window: &mut PistonWindow, event: &Event, text: &str) {
        let (_dx, dy) = get_scaling(window);
        let osd_y = (window.draw_size().height * dy) - 12.0;

        window.draw_2d(event, |context, graphics, device| {
            let transform = context.transform.trans(10.0, osd_y).zoom(0.5);
            text::Text::new_color([0.7, 0.7, 0.7, 0.75], 48)
                .draw(
                    &text,
                    &mut self.glyphs,
                    &context.draw_state,
                    transform,
                    graphics,
                )
                .unwrap();

            self.glyphs.factory.encoder.flush(device);
        });
    }

    pub fn fits_in_view(&self, window: &PistonWindow, zoom_factor: f64) -> bool {
        let image_w = self.get_width() * zoom_factor;
        let image_h = self.get_height() as f64 * zoom_factor;

        let draw_size = window.draw_size();

        draw_size.width >= image_w || draw_size.height >= image_h
    }

    pub fn get_width(&self) -> f64 {
        self.texture.get_width() as f64
    }

    pub fn get_height(&self) -> f64 {
        self.texture.get_height() as f64
    }

    pub fn cloned_frame(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        self.cur_frame.to_owned()
    }
}
