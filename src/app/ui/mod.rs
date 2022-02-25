use super::{update_input_key_state, PreviewFilterType, VSPreviewer, MAX_ZOOM, MIN_ZOOM};
use eframe::{egui, epaint, epi};

mod bottom_panel;
mod controls;
mod frame_props;
mod preferences;
mod preview_image;
mod state_window;

pub use bottom_panel::UiBottomPanel;
use controls::UiControls;
use frame_props::UiFrameProps;
use preferences::UiPreferences;
pub use preview_image::UiPreviewImage;
pub use state_window::UiStateWindow;

const STATE_LABEL_COLOR: epaint::Color32 = epaint::Color32::from_gray(160);
