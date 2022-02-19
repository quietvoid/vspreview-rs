use std::sync::Arc;

use eframe::{
    egui,
    epaint::{Color32, ColorImage},
};
use image::DynamicImage;
use itertools::izip;
use vapoursynth::prelude::{ColorFamily, FrameRef};

use crate::previewer::PreviewState;

pub const MIN_ZOOM: f32 = 0.125;
pub const MAX_ZOOM: f32 = 50.0;

/// ColorImage from 24 bits RGB
pub fn frame_to_colorimage(frame: FrameRef) -> ColorImage {
    let format = frame.format();

    // Gray or RGB
    assert!(matches!(
        format.color_family(),
        ColorFamily::Gray | ColorFamily::RGB
    ));

    let plane_count = frame.format().plane_count();
    assert!(plane_count == 1 || plane_count == 3);

    // Assumes all planes are the same resolution
    let (w, h) = (frame.width(0), frame.height(0));

    let pixels = if plane_count == 1 {
        let gray: &[u8] = frame.plane(0).unwrap();

        gray.iter().map(|p| Color32::from_gray(*p)).collect()
    } else {
        let (r, g, b): (&[u8], &[u8], &[u8]) = (
            frame.plane(0).unwrap(),
            frame.plane(1).unwrap(),
            frame.plane(2).unwrap(),
        );

        izip!(r, g, b)
            .map(|(r, g, b)| Color32::from_rgb(*r, *g, *b))
            .collect()
    };

    ColorImage {
        size: [w, h],
        pixels,
    }
}

pub fn process_image(
    orig: Arc<ColorImage>,
    state: PreviewState,
    final_size: eframe::epaint::Vec2,
) -> ColorImage {
    let size = orig.size;
    let mut img = DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(
        size[0] as u32,
        size[1] as u32,
        |x, y| image::Rgba(orig[(x as usize, y as usize)].to_array()),
    ));

    let zoom_factor = state.zoom_factor;
    let (tx, ty) = (state.translate_x, state.translate_y);
    let scale_to_win = state.scale_to_window;

    if zoom_factor != 1.0 && zoom_factor >= MIN_ZOOM {
        let mut w = size[0] as f32;
        let mut h = size[1] as f32;

        if zoom_factor > 1.0 {
            w /= zoom_factor;
            h /= zoom_factor;

            img = img.crop_imm(tx, ty, w.ceil() as u32, h.ceil() as u32);
        };

        let (w, h) = (w * zoom_factor, h * zoom_factor);

        img = img.resize(w.ceil() as u32, h.ceil() as u32, image::imageops::Nearest);
    }

    if scale_to_win && final_size.min_elem() > 0.0 {
        img = img.resize(
            final_size.x as u32,
            final_size.y as u32,
            image::imageops::Nearest,
        );
    }

    let new_size = [img.width() as usize, img.height() as usize];
    let processed = egui::ColorImage::from_rgba_unmultiplied(new_size, img.as_bytes());

    processed
}
