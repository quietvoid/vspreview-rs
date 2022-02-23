use fast_image_resize as fir;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub enum PreviewFilterType {
    Point,
    Bilinear,
    Hamming,
    CatmullRom,
    Mitchell,
    Lanczos3,
}

/// Filter type to use with fast_image_resize
impl Default for PreviewFilterType {
    fn default() -> Self {
        PreviewFilterType::Point
    }
}

impl From<&PreviewFilterType> for fir::FilterType {
    fn from(f: &PreviewFilterType) -> Self {
        match f {
            PreviewFilterType::Point => fir::FilterType::Box,
            PreviewFilterType::Bilinear => fir::FilterType::Bilinear,
            PreviewFilterType::Hamming => fir::FilterType::Hamming,
            PreviewFilterType::CatmullRom => fir::FilterType::CatmullRom,
            PreviewFilterType::Mitchell => fir::FilterType::Mitchell,
            PreviewFilterType::Lanczos3 => fir::FilterType::Lanczos3,
        }
    }
}
