use std::sync::{Arc, RwLock};

use eframe::{egui, epaint::Vec2};
use poll_promise::Promise;

mod epi_app;
mod preview_filter_type;
mod ui;
mod vs_previewer;

use ui::*;

use preview_filter_type::PreviewFilterType;
pub use vs_previewer::VSPreviewer;

use super::vs_handler::{vstransform, PreviewedScript, VSFrame, VSFrameProps, VSOutput};
use vstransform::VSTransformOptions;

use crate::utils::{
    dimensions_for_window, resize_fast, translate_norm_coeffs, update_input_key_state,
};

pub const MIN_ZOOM: f32 = 0.125;
pub const MAX_ZOOM: f32 = 64.0;

type APreviewFrame = Arc<RwLock<PreviewFrame>>;
type FramePromise = Promise<APreviewFrame>;

/// TODO:
///   - Canvas background color
///   - ?
#[derive(Default, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PreviewState {
    pub show_gui: bool,

    pub cur_output: i32,
    pub cur_frame_no: u32,

    pub zoom_factor: f32,

    pub translate: Vec2,
    pub translate_norm: Vec2,

    pub frame_transform_opts: VSTransformOptions,

    // Only upscales
    pub upscale_to_window: bool,
    /// Defaults to Point for performance
    pub upsampling_filter: PreviewFilterType,

    pub zoom_multiplier: f32,

    pub scroll_multiplier: f32,
    pub canvas_margin: f32,
}

#[derive(Default)]
pub struct PreviewOutput {
    pub vsoutput: VSOutput,

    pub frame_promise: Option<FramePromise>,
    pub original_props_promise: Option<Promise<Option<VSFrameProps>>>,

    pub force_reprocess: bool,
    pub last_frame_no: u32,
}

#[derive(Clone)]
pub struct PreviewFrame {
    pub vsframe: VSFrame,
    pub texture: egui::TextureHandle,
}
