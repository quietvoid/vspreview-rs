mod preview;
mod previewed_script;
mod previewer;
pub mod preview_ui;

use super::image;
use super::piston_window;

use piston_window::*;

pub use preview::Preview;
pub use previewed_script::PreviewedScript;
pub use previewer::Previewer;

pub fn get_scaling(window: &PistonWindow) -> (f64, f64) {
    let size = window.size();
    let draw_size = window.draw_size();

    let dx = size.width as f64 / draw_size.width as f64;
    let dy = size.height as f64 / draw_size.height as f64;

    (dx, dy)
}

pub fn scaled_size(size: Size, dpi: f64) -> Size {
    Size::from((size.width / dpi, size.height as f64 / dpi))
}
