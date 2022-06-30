use std::{collections::HashMap, sync::Arc};

use eframe::{egui, epaint::Vec2};
use image::DynamicImage;
use parking_lot::{Mutex, RwLock};
use poll_promise::Promise;

mod eframe_app;
mod preview_filter_type;
mod transforms;
mod ui;
mod vs_previewer;

use ui::*;

use preview_filter_type::{PreviewFilterType, PreviewTextureFilterType};
pub use vs_previewer::VSPreviewer;

use super::vs_handler::{vstransform, PreviewedScript, VSFrame, VSFrameProps, VSOutput};
use vstransform::VSTransformOptions;

use crate::utils::{
    dimensions_for_window, resize_fast, translate_norm_coeffs, update_input_key_state,
};

use transforms::icc::IccProfile;

pub const MIN_ZOOM: f32 = 0.125;
pub const MAX_ZOOM: f32 = 64.0;

type VSPreviewFrame = Arc<RwLock<PreviewFrame>>;
type FramePromise = Promise<Option<VSPreviewFrame>>;
type PropsPromise = Promise<Option<VSFrameProps>>;
type ReloadPromise = Promise<Option<HashMap<i32, VSOutput>>>;

/// TODO:
///   - Canvas background color
///   - ?
#[derive(Debug, Copy, Clone, serde::Deserialize, serde::Serialize)]
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

    /// Texture filter (for GPU scaling)
    pub texture_filter: PreviewTextureFilterType,
    // Only upscales
    pub upscale_to_window: bool,
    /// Defaults to Point for performance
    pub upsampling_filter: PreviewFilterType,
    /// Fit the texture before painting
    pub fit_to_window: bool,

    pub zoom_multiplier: f32,

    pub scroll_multiplier: f32,
    pub canvas_margin: f32,

    pub icc_enabled: bool,
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

pub struct FetchImageState {
    ctx: egui::Context,
    frame_mutex: Arc<Mutex<Option<FramePromise>>>,
    script: Arc<Mutex<PreviewedScript>>,
    state: PreviewState,
    pf: Option<VSPreviewFrame>,
    reprocess: bool,
    win_size: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub enum ReloadType {
    None,
    Reload,
    Reprocess,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PreviewTransforms {
    pub icc: Option<IccProfile>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            zoom_factor: 1.0,
            zoom_multiplier: 1.0,
            scroll_multiplier: 1.0,
            canvas_margin: 0.0,
            fit_to_window: true,
            show_gui: Default::default(),
            cur_output: Default::default(),
            cur_frame_no: Default::default(),
            translate_changed: Default::default(),
            translate: Default::default(),
            translate_norm: Default::default(),
            frame_transform_opts: Default::default(),
            upscale_to_window: Default::default(),
            upsampling_filter: Default::default(),
            icc_enabled: Default::default(),
            texture_filter: Default::default(),
        }
    }
}
