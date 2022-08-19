use std::fmt::Display;

use fast_image_resize as fir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum PreviewFilterType {
    Gpu,
    Point,
    Bilinear,
    Hamming,
    CatmullRom,
    Mitchell,
    Lanczos3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum PreviewTextureFilterType {
    Linear,
    Nearest,
}

/// Filter type to use with fast_image_resize
impl Default for PreviewFilterType {
    fn default() -> Self {
        Self::Gpu
    }
}

impl Default for PreviewTextureFilterType {
    fn default() -> Self {
        Self::Linear
    }
}

impl From<&PreviewFilterType> for fir::FilterType {
    fn from(f: &PreviewFilterType) -> Self {
        match f {
            // Placeholder but it wouldn't be used
            PreviewFilterType::Gpu => fir::FilterType::Box,
            PreviewFilterType::Point => fir::FilterType::Box,
            PreviewFilterType::Bilinear => fir::FilterType::Bilinear,
            PreviewFilterType::Hamming => fir::FilterType::Hamming,
            PreviewFilterType::CatmullRom => fir::FilterType::CatmullRom,
            PreviewFilterType::Mitchell => fir::FilterType::Mitchell,
            PreviewFilterType::Lanczos3 => fir::FilterType::Lanczos3,
        }
    }
}

impl From<&PreviewTextureFilterType> for eframe::egui::TextureFilter {
    fn from(f: &PreviewTextureFilterType) -> Self {
        match f {
            PreviewTextureFilterType::Linear => eframe::egui::TextureFilter::Linear,
            PreviewTextureFilterType::Nearest => eframe::egui::TextureFilter::Nearest,
        }
    }
}

impl Display for PreviewFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            PreviewFilterType::Gpu => "GPU",
            PreviewFilterType::Point => "Point",
            PreviewFilterType::Bilinear => "Bilinear",
            PreviewFilterType::Hamming => "Hamming",
            PreviewFilterType::CatmullRom => "CatmullRom",
            PreviewFilterType::Mitchell => "Mitchell",
            PreviewFilterType::Lanczos3 => "Lanczos3",
        };

        f.write_str(val)
    }
}

impl Display for PreviewTextureFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            PreviewTextureFilterType::Linear => "Linear",
            PreviewTextureFilterType::Nearest => "Nearest",
        };

        f.write_str(val)
    }
}
