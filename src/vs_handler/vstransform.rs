use std::fmt::Display;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct VSTransformOptions {
    pub resizer: VSResizer,
    pub enable_dithering: bool,
    pub dither_algo: VSDitherAlgo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum VSResizer {
    Bilinear,
    Bicubic,
    Point,
    Lanczos,
    Spline16,
    Spline36,
    Spline64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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
        Self::None
    }
}

impl VSResizer {
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Bilinear => "Bilinear",
            Self::Bicubic => "Bicubic",
            Self::Point => "Point",
            Self::Lanczos => "Lanczos",
            Self::Spline16 => "Spline16",
            Self::Spline36 => "Spline36",
            Self::Spline64 => "Spline64",
        }
    }
}

impl VSDitherAlgo {
    pub const fn as_str(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Ordered => "ordered",
            Self::Random => "random",
            Self::ErrorDiffusion => "error_diffusion",
        }
    }
}

impl Display for VSResizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Display for VSDitherAlgo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            Self::None => "None",
            Self::Ordered => "Ordered",
            Self::Random => "Random",
            Self::ErrorDiffusion => "Error Diffusion",
        };

        f.write_str(v)
    }
}
