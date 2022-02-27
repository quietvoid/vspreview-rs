use std::{collections::HashMap, sync::Arc};

use eframe::{egui, epaint::Vec2};
use image::DynamicImage;
use parking_lot::{Mutex, RwLock};
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

type VSPreviewFrame = Arc<RwLock<PreviewFrame>>;
type FramePromise = Promise<Option<VSPreviewFrame>>;
type PropsPromise = Promise<Option<VSFrameProps>>;
type ReloadPromise = Promise<Option<HashMap<i32, VSOutput>>>;

/// TODO:
///   - Canvas background color
///   - ?
#[derive(Debug, Default, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PreviewState {
    pub show_gui: bool,

    pub cur_output: i32,
    pub cur_frame_no: u32,

    pub zoom_factor: f32,

    #[serde(skip)]
    pub translate_changed: bool,

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

    pub rendered_frame: Option<VSPreviewFrame>,
    pub original_props: Option<VSFrameProps>,

    pub force_reprocess: bool,
    pub last_frame_no: u32,
}

pub struct PreviewFrame {
    pub vsframe: VSFrame,

    /// Can't be moved out of `VSFrame` without a copy
    /// As an Option, we can check which image to use
    pub processed_image: Option<DynamicImage>,
    pub texture: Mutex<Option<egui::TextureHandle>>,
}
