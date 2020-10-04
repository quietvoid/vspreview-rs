use super::image::{ImageBuffer, Rgba};

use super::previewed_script::PreviewedScript;

use super::get_scaling;
use piston_window::*;

pub struct Preview {
    cur_frame: ImageBuffer<Rgba<u8>, Vec<u8>>,
    texture_context: Option<G2dTextureContext>,
    texture: Option<G2dTexture>,
}

impl Preview {
    pub fn new(script: &PreviewedScript, initial_frame: u32) -> Self {
        let cur_frame = match script.get_frame(initial_frame) {
            Some(frame) => frame,
            None => ImageBuffer::new(0, 0),
        };

        Self {
            cur_frame,
            texture_context: None,
            texture: None,
        }
    }

    pub fn initialize(&mut self, window: &mut PistonWindow) {
        let mut texture_context = window.create_texture_context();
        let texture: G2dTexture = Texture::from_image(
            &mut texture_context,
            &self.cur_frame,
            &TextureSettings::new().mag(texture::Filter::Nearest),
        )
        .unwrap();

        self.texture_context = Some(texture_context);
        self.texture = Some(texture);
    }

    pub fn update(&mut self, image: ImageBuffer<Rgba<u8>, Vec<u8>>) {
        self.cur_frame = image;
        let texture = self.texture.as_mut().unwrap();
        let mut texture_context = self.texture_context.as_mut().unwrap();

        if texture.get_width() != self.cur_frame.width()
            || texture.get_height() != self.cur_frame.height()
        {
            *texture = G2dTexture::from_image(
                &mut texture_context,
                &self.cur_frame,
                &TextureSettings::new().mag(texture::Filter::Nearest),
            )
            .unwrap()
        } else {
            texture
                .update(&mut texture_context, &self.cur_frame)
                .unwrap();
        }

        self.texture = Some(texture.to_owned());
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

            piston_window::image(self.texture.as_ref().unwrap(), img_transform, graphics);

            // Flush to GPU
            self.texture_context.as_mut().unwrap().encoder.flush(device);
        });
    }

    pub fn fits_in_view(
        &self,
        window: &PistonWindow,
        zoom_factor: f64,
        horizontally: bool,
    ) -> bool {
        let image_w = self.get_width() * zoom_factor;
        let image_h = self.get_height() as f64 * zoom_factor;

        let draw_size = window.draw_size();

        if horizontally {
            draw_size.width >= image_w
        } else {
            draw_size.height >= image_h
        }
    }

    pub fn get_width(&self) -> f64 {
        self.texture.as_ref().unwrap().get_width() as f64
    }

    pub fn get_height(&self) -> f64 {
        self.texture.as_ref().unwrap().get_height() as f64
    }

    pub fn cloned_frame(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        self.cur_frame.to_owned()
    }
}
