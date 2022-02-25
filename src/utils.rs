use std::{collections::HashMap, num::NonZeroU32};

use eframe::epaint::{Color32, ColorImage, Vec2};

use fast_image_resize as fr;
use image::DynamicImage;
use itertools::izip;
use vapoursynth::prelude::{ColorFamily, FrameRef};

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

pub fn image_from_colorimage(ci: &ColorImage) -> DynamicImage {
    let (src_w, src_h) = (ci.size[0] as f32, ci.size[1] as f32);

    DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(
        src_w as u32,
        src_h as u32,
        |x, y| image::Rgba(ci[(x as usize, y as usize)].to_array()),
    ))
}

// Normalize from max translate value to float with range [-1, 1]
pub fn translate_norm_coeffs(size: &Vec2, win_size: &Vec2, zoom_factor: f32) -> Vec2 {
    // Clips left and right
    let max_tx = if zoom_factor > 1.0 {
        // When zooming, the image is cropped to smallest bound
        size.x - (win_size.x.min(size.x) / zoom_factor)
    } else if zoom_factor < 1.0 {
        // When unzooming, we want reduce the image size
        // That way it might fit within the window
        (size.x * zoom_factor) - win_size.x
    } else {
        size.x - win_size.x
    };

    // Clips vertically at the bottom only
    let max_ty = if zoom_factor > 1.0 {
        size.y - (win_size.y.min(size.y) / zoom_factor)
    } else if zoom_factor < 1.0 {
        (size.y * zoom_factor) - win_size.y
    } else {
        size.y - win_size.y
    };

    Vec2::from([max_tx, max_ty])
}

pub fn translate_norm_to_pixels(
    translate_norm: &Vec2,
    size: &Vec2,
    win_size: &Vec2,
    zoom_factor: f32,
) -> Vec2 {
    let coeffs = translate_norm_coeffs(size, win_size, zoom_factor);

    Vec2::from([
        (translate_norm.x * coeffs.x).round(),
        (translate_norm.y * coeffs.y).round(),
    ])
}

pub const fn icon_color_for_bool(value: bool) -> (&'static str, Color32) {
    if value {
        ("✅", Color32::from_rgb(0, 128, 0))
    } else {
        ("✖", Color32::from_rgb(200, 0, 0))
    }
}

pub fn update_input_key_state<'a>(
    map: &mut HashMap<&'a str, bool>,
    key: &'a str,
    val: bool,
    res: &eframe::egui::Response,
) -> bool {
    if let Some(current) = map.get_mut(key) {
        *current |= val;
    } else {
        map.insert(key, val);
    }

    release_on_focus_lost(map, key, res)
}

fn release_on_focus_lost<'a>(
    map: &mut HashMap<&'a str, bool>,
    key: &'a str,
    res: &eframe::egui::Response,
) -> bool {
    if !res.has_focus() && (res.drag_released() || res.lost_focus()) {
        map.insert(key, false);

        true
    } else {
        false
    }
}
