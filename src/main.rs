extern crate image;
extern crate piston_window;

#[macro_use]
extern crate conrod_core;
extern crate conrod_piston;

use std::path::PathBuf;

use conrod_core::{widget, Colorable, Positionable, Sizeable, Widget};
use piston_window::texture::UpdateTexture;
use piston_window::*;
use structopt::StructOpt;

mod previewer;

use image::ImageFormat;
use previewer::{scaled_size, PreviewedScript, Previewer};

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
    let scaled_ui = 150.0 / dpi;

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
    let ids = Ids::new(ui.widget_id_generator());

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

    let mut script_info = previewer.get_script_info();

    while let Some(e) = window.next() {
        match e {
            Event::Input(Input::Button(input), _opt) => match (input.button, input.state) {
                (Button::Keyboard(k), ButtonState::Press) => {
                    previewer.handle_key_press(&mut window, &k);
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

        let size = window.size();
        let (win_w, win_h) = (
            size.width as conrod_core::Scalar,
            size.height as conrod_core::Scalar,
        );
        if let Some(e) = conrod_piston::event::convert(e.clone(), win_w, win_h) {
            ui.handle_event(e);
        }

        if previewer.show_osd() {
            e.update(|_| {
                let ui = &mut ui.set_widgets();

                widget::Canvas::new()
                    .rgba(0.25, 0.25, 0.25, 0.50)
                    .mid_bottom()
                    .w(win_w)
                    .h(scaled_ui)
                    .set(ids.canvas, ui);

                let current_frame = previewer.get_current_no();
                let max = previewer.get_clip_length();
                let slider_width = size.width / 1.5;
                let pointer_width = -50.0 + (current_frame as f64 / max as f64) * slider_width;

                if let Some(val) = widget::Slider::new(current_frame as f32, 0.0, max as f32)
                    .mid_bottom_with_margin(55.0)
                    .w_h(slider_width, 20.0)
                    .rgba(0.75, 0.75, 0.75, 1.00)
                    .set(ids.slider, ui)
                {
                    previewer.seek_to(val.into());
                }

                widget::Text::new(&format!("{}", current_frame))
                    .bottom_left_with_margins_on(ids.slider, 25.0, pointer_width)
                    .rgba(0.75, 0.75, 0.75, 1.00)
                    .font_size(32)
                    .set(ids.min_label, ui);

                widget::Text::new(&script_info.to_string())
                    .bottom_left_with_margins_on(ids.canvas, 15.0, 10.0)
                    .rgba(0.75, 0.75, 0.75, 1.00)
                    .font_size(26)
                    .set(ids.frame_info, ui);
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

widget_ids!(struct Ids { canvas, slider, min_label, frame_info });
