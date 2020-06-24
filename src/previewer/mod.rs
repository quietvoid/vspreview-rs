mod preview;
mod previewed_script;
mod previewer;

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

pub fn required_window_size(window: &PistonWindow, preview: &Preview) -> Size {
    let (dx, dy) = get_scaling(window);

    let new_width = preview.get_width() as f64 * dx;
    let new_height = preview.get_height() as f64 * dy;

    Size::from((new_width, new_height))
}
