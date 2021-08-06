use eframe::epaint::{Color32, ColorImage};
use itertools::izip;
use vapoursynth::prelude::{ColorFamily, FrameRef};

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
