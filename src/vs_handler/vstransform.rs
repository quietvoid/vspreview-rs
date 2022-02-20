#[derive(Default, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct VSTransformOptions {
    pub resizer: VSResizer,
    pub add_dither: bool,
    pub dither_algo: VSDitherAlgo,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum VSResizer {
    Bilinear,
    Bicubic,
    Point,
    Lanczos,
    Spline16,
    Spline36,
    Spline64,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum VSDitherAlgo {
    None,
    Ordered,
    Random,
    ErrorDiffusion,
}

impl Default for VSResizer {
    fn default() -> Self {
        Self::Spline16
    }
}

impl Default for VSDitherAlgo {
    fn default() -> Self {
        VSDitherAlgo::None
    }
}

impl VSResizer {
    pub const fn as_str(&self) -> &str {
        match self {
            VSResizer::Bilinear => "Bilinear",
            VSResizer::Bicubic => "Bicubic",
            VSResizer::Point => "Point",
            VSResizer::Lanczos => "Lanczos",
            VSResizer::Spline16 => "Spline16",
            VSResizer::Spline36 => "Spline36",
            VSResizer::Spline64 => "Spline64",
        }
    }
}
impl VSDitherAlgo {
    pub const fn as_str(&self) -> &str {
        match self {
            VSDitherAlgo::None => "none",
            VSDitherAlgo::Ordered => "ordered",
            VSDitherAlgo::Random => "random",
            VSDitherAlgo::ErrorDiffusion => "error_diffusion",
        }
    }
}
