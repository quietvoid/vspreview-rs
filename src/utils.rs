use std::num::NonZeroU32;

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
pub const MAX_ZOOM: f32 = 64.0;

/// ColorImage from 24 bits RGB
pub fn frame_to_colorimage(frame: &FrameRef) -> ColorImage {
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
    orig: &ColorImage,
    state: &PreviewState,
    win_size: eframe::epaint::Vec2,
) -> ColorImage {
    let (src_w, src_h) = (orig.size[0] as f32, orig.size[1] as f32);

    let mut img = DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(
        src_w as u32,
        src_h as u32,
        |x, y| image::Rgba(orig[(x as usize, y as usize)].to_array()),
    ));

    let zoom_factor = state.zoom_factor;
    let (tx, ty) = (state.translate.x.round(), state.translate.y.round());

    // Rounded up
    let win_size = win_size.round();
    let (mut w, mut h) = (src_w as f32, src_h as f32);

    // Unzoom first and foremost
    if zoom_factor < 1.0 {
        w *= zoom_factor;
        h *= zoom_factor;

        img = resize_fast(img, w.round() as u32, h.round() as u32, fr::FilterType::Box);
    }

    // Positive = crop right part
    let x = if tx.is_sign_negative() { 0.0 } else { tx.abs() };
    let y = if ty.is_sign_negative() { 0.0 } else { ty.abs() };

    if w > win_size.x || h > win_size.y || zoom_factor > 1.0 {
        if (tx.abs() > 0.0 || ty.abs() > 0.0) && zoom_factor <= 1.0 {
            w -= tx.abs();
            h -= ty.abs();
        }

        // Limit to window size
        w = w.min(win_size.x);
        h = h.min(win_size.y);

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

        let target_size = if state.scale_to_window {
            // Resize up to max size of window
            dimensions_for_window(win_size, new_size).round()
        } else {
            new_size
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
            let fr_filter = fr::FilterType::from(&state.scale_filter);
            img = resize_fast(img, target_size.x as u32, target_size.y as u32, fr_filter);
        }
    }

    let new_size = [img.width() as usize, img.height() as usize];
    let processed = egui::ColorImage::from_rgba_unmultiplied(new_size, img.as_bytes());

    processed
}

// Based on fast_image_resize example doc
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
