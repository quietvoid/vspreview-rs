extern crate image;
extern crate piston_window;

#[macro_use]
extern crate conrod_core;
extern crate conrod_piston;

#[macro_use] extern crate conrod_derive;

use std::path::PathBuf;

use piston_window::texture::UpdateTexture;
use piston_window::*;
use structopt::StructOpt;

mod previewer;
mod custom_widgets;

use image::ImageFormat;
use previewer::{scaled_size, PreviewedScript, Previewer, preview_ui};

#[derive(StructOpt, Debug)]
#[structopt(name = "vspreview-rs", about = "Vapoursynth script previewer")]
struct Opt {
    #[structopt(name = "input", parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    // Icon
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon_img = image::load_from_memory_with_format(icon_bytes, ImageFormat::Png)
        .expect("loading icon")
        .to_rgba();
    let (icon_width, icon_height) = icon_img.dimensions();
    let icon = Some(glutin::Icon::from_rgba(icon_img.into_raw(), icon_width, icon_height).unwrap());

    // Font
    let font_bytes = include_bytes!("../assets/FiraSans-Regular.ttf");
    let font = conrod_core::text::Font::from_bytes(&font_bytes[0..font_bytes.len()]).unwrap();

    let opt = Opt::from_args();

    // Get the DPI of the primary display
    let dpi = glutin::EventsLoop::new()
        .get_primary_monitor()
        .get_hidpi_factor();

    // Load script to get frame dimensions
    let script = PreviewedScript::new(opt.input);
    let frame_size = script.get_size();
    let (frame_width, frame_height) = (frame_size.width as u32, frame_size.height as u32);

    let scaled_size = scaled_size(frame_size, dpi);
    let (window_width, window_height) = (scaled_size.width, scaled_size.height);

    let mut previewer = Previewer::new(script);

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("VS Preview", [window_width, window_height])
        .exit_on_esc(false)
        .graphics_api(opengl)
        .build()
        .unwrap();

    // ?? Set icon
    window.window.ctx.window().set_window_icon(icon);

    // Init preview with window now that it's created
    previewer.initialize(&mut window);

    // UI
    let mut ui = conrod_core::UiBuilder::new([window_width, window_height]).build();
    let ids = preview_ui::Ids::new(ui.widget_id_generator());

    ui.fonts.insert(font);

    let mut texture_context = window.create_texture_context();

    let mut text_vertex_data = Vec::new();
    let (mut texture_cache, mut glyph_cache) = {
        let cache = conrod_core::text::GlyphCache::builder()
            .dimensions(frame_width, frame_height)
            .scale_tolerance(0.1)
            .position_tolerance(0.1)
            .build();

        let buffer_len = frame_width as usize * frame_height as usize;
        let init = vec![128; buffer_len];
        let settings = TextureSettings::new();
        let texture = G2dTexture::from_memory_alpha(
            &mut texture_context,
            &init,
            frame_width as u32,
            frame_height as u32,
            &settings,
        )
        .unwrap();

        (texture, cache)
    };

    let image_map = conrod_core::image::Map::new();

    let mut preview_ui = preview_ui::PreviewUi::new(previewer.get_current_no().to_string(), 150.0 / dpi);

    let mut script_info = previewer.get_script_info();

    while let Some(e) = window.next() {

        // TODO: Make the image canvas a conrod widget to avoid handling events twice
        match e {
            Event::Input(Input::Button(input), _opt) => match (input.button, input.state) {
                (Button::Keyboard(k), ButtonState::Press) => {
                    previewer.handle_key_press(&mut window, &k, &mut preview_ui);
                }
                (Button::Keyboard(k), ButtonState::Release) => {
                    previewer.handle_key_release(&k);
                }
                _ => (),
            },
            Event::Input(Input::Move(motion), _opt) => {
                if let Motion::MouseScroll(ticks) = motion {
                    previewer.handle_mouse_scroll(&window, ticks);
                }
            }
            Event::Input(Input::Close(_args), _opt) => {
                previewer.handle_window_close();
            }
            Event::Loop(render) => {
                if let Loop::Render(_ra) = render {
                    previewer.rerender(&mut window, &e);

                    if previewer.show_osd() {
                        script_info = previewer.get_script_info();
                    }
                };
            }
            _ => {}
        };

        if previewer.show_osd() {
            let size = window.size();
            let (win_w, win_h) = (
                size.width as conrod_core::Scalar,
                size.height as conrod_core::Scalar,
            );

            if let Some(e) = conrod_piston::event::convert(e.clone(), win_w, win_h) {
                ui.handle_event(e);
            }

            e.update(|_| {
                let ui = &mut ui.set_widgets();

                preview_ui.gui(ui, &ids, &mut previewer, &script_info.to_string(), win_w);
            });

            window.draw_2d(&e, |context, graphics, device| {
                let primitives = ui.draw();
                // A function used for caching glyphs to the texture cache.
                let cache_queued_glyphs = |_graphics: &mut G2d,
                                           cache: &mut G2dTexture,
                                           rect: conrod_core::text::rt::Rect<u32>,
                                           data: &[u8]| {
                    let offset = [rect.min.x, rect.min.y];
                    let size = [rect.width(), rect.height()];
                    let format = piston_window::texture::Format::Rgba8;
                    text_vertex_data.clear();
                    text_vertex_data.extend(data.iter().flat_map(|&b| vec![255, 255, 255, b]));
                    UpdateTexture::update(
                        cache,
                        &mut texture_context,
                        format,
                        &text_vertex_data[..],
                        offset,
                        size,
                    )
                    .expect("failed to update texture")
                };

                // Specify how to get the drawable texture from the image. In this case, the image
                // *is* the texture.
                fn texture_from_image<T>(img: &T) -> &T {
                    img
                }

                // Draw the conrod `render::Primitives`.
                conrod_piston::draw::primitives(
                    primitives,
                    context,
                    graphics,
                    &mut texture_cache,
                    &mut glyph_cache,
                    &image_map,
                    cache_queued_glyphs,
                    texture_from_image,
                );

                texture_context.encoder.flush(device);
            });
        }
    }
}