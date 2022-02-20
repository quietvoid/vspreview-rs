use std::{num::NonZeroU32, sync::Arc};

use eframe::epaint::Vec2;
use eframe::{
    egui,
    epaint::{Color32, ColorImage},
};

use fast_image_resize as fr;
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
    win_size: eframe::epaint::Vec2,
) -> ColorImage {
    let (src_w, src_h) = (orig.size[0] as u32, orig.size[1] as u32);

    let mut img = DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(src_w, src_h, |x, y| {
        image::Rgba(orig[(x as usize, y as usize)].to_array())
    }));

    let zoom_factor = state.zoom_factor;
    let (tx, ty) = (
        state.translate.x.round() as u32,
        state.translate.y.round() as u32,
    );

    // Rounded up
    let win_size = win_size.round();

    if zoom_factor != 1.0 && zoom_factor >= MIN_ZOOM {
        let mut w = src_w as f32;
        let mut h = src_h as f32;

        if zoom_factor > 1.0 {
            w /= zoom_factor;
            h /= zoom_factor;

            img = img.crop_imm(0, 0, w.round() as u32, h.round() as u32);
        }

        let (w, h) = (w * zoom_factor, h * zoom_factor);
        let orig_size = Vec2::new(w as f32, h as f32).round();

        let target_size = if state.scale_to_window {
            // Crop and resize up to max size of window
            dimensions_for_window(win_size, orig_size).round()
        } else {
            orig_size
        };

        img = resize_fast(
            img,
            target_size.x.round() as u32,
            target_size.y.round() as u32,
            fr::FilterType::Box,
        );
    }

    if state.scale_to_window {
        // Image size after crop
        let orig_size = Vec2::new(img.width() as f32, img.height() as f32);

        // Scaled size to window bounds
        let target_size = dimensions_for_window(win_size, orig_size).round();

        if orig_size != target_size {
            let fr_filter = fr::FilterType::from(state.scale_filter);
            img = resize_fast(img, target_size.x as u32, target_size.y as u32, fr_filter);
        }
    }

    let new_size = [img.width() as usize, img.height() as usize];
    let processed = egui::ColorImage::from_rgba_unmultiplied(new_size, img.as_bytes());

    processed
}

pub fn resize_fast(
    img: DynamicImage,
    dst_width: u32,
    dst_height: u32,
    filter_type: fr::FilterType,
) -> DynamicImage {
    let width = NonZeroU32::new(img.width()).unwrap();
    let height = NonZeroU32::new(img.height()).unwrap();

    let src_image = fr::Image::from_vec_u8(
        width,
        height,
        img.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )
    .unwrap();

    let mut dst_image = fr::Image::new(
        NonZeroU32::new(dst_width).unwrap(),
        NonZeroU32::new(dst_height).unwrap(),
        src_image.pixel_type(),
    );
    // Get mutable view of destination image data
    let mut dst_view = dst_image.view_mut();

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(filter_type));
    resizer.resize(&src_image.view(), &mut dst_view).unwrap();

    let buf =
        image::ImageBuffer::from_raw(dst_width, dst_height, dst_image.buffer().to_vec()).unwrap();
    DynamicImage::ImageRgba8(buf)
}

pub fn dimensions_for_window(win_size: Vec2, orig_size: Vec2) -> Vec2 {
    let mut size = orig_size;

    // Fit to width
    if orig_size.x != win_size.x {
        size.x = win_size.x;
        size.y = (size.x * orig_size.y) / orig_size.x;
    }

    // Fit to height
    if size.y > win_size.y {
        size.y = win_size.y;
        size.x = (size.y * orig_size.x) / orig_size.y;
    }

    size
}
