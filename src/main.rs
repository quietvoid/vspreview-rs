extern crate image;
extern crate piston_window;

#[macro_use]
extern crate conrod_core;
extern crate conrod_piston;

use std::path::PathBuf;

use conrod_core::{color, widget, Colorable, Positionable, Sizeable, Widget};
use piston_window::texture::UpdateTexture;
use piston_window::*;
use structopt::StructOpt;

mod previewer;

use previewer::{PreviewedScript, Previewer};

#[derive(StructOpt, Debug)]
#[structopt(name = "vspreview-rs", about = "Vapoursynth script previewer")]
struct Opt {
    #[structopt(name = "input", parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    let opt = Opt::from_args();

    let script = PreviewedScript::new(opt.input);

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("VS Preview", [WIDTH, HEIGHT])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    //window.set_lazy(true);

    let frame_no: u32 = 119;
    let mut previewer = Previewer::new(&mut window, script, frame_no);

    // UI
    let (width, height) = previewer.get_size();
    let mut ui = conrod_core::UiBuilder::new([width as f64, height as f64]).build();
    let ids = Ids::new(ui.widget_id_generator());

    ui.fonts
        .insert_from_file(PathBuf::from("assets/FiraSans-Regular.ttf"))
        .unwrap();

    let mut texture_context = window.create_texture_context();

    let mut text_vertex_data = Vec::new();
    let (mut texture_cache, mut glyph_cache) = {
        let cache = conrod_core::text::GlyphCache::builder()
            .dimensions(width, height)
            .scale_tolerance(0.1)
            .position_tolerance(0.1)
            .build();

        let buffer_len = width as usize * height as usize;
        let init = vec![128; buffer_len];
        let settings = TextureSettings::new();
        let texture =
            G2dTexture::from_memory_alpha(&mut texture_context, &init, width, height, &settings)
                .unwrap();

        (texture, cache)
    };

    let image_map = conrod_core::image::Map::new();

    while let Some(e) = window.next() {
        match e {
            Event::Input(Input::Button(input), _opt) => match (input.button, input.state) {
                (Button::Keyboard(k), ButtonState::Press) => {
                    previewer.handle_key_press(&window, &k);
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
            Event::Loop(render) => {
                if let Loop::Render(_ra) = render {
                    previewer.rerender(&mut window, &e);
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
                    .color(color::TRANSPARENT)
                    .set(ids.canvas, ui);

                let current_frame = previewer.get_current_no();
                let max = previewer.get_clip_length();
                let slider_width = size.width / 1.5;
                let pointer_width = -50.0 + (current_frame as f64 / max as f64) * slider_width;

                if let Some(val) = widget::Slider::new(current_frame as f32, 0.0, max as f32)
                    .mid_bottom_with_margin(35.0)
                    .w_h(slider_width, 20.0)
                    .rgba(0.7, 0.7, 0.7, 0.50)
                    .set(ids.slider, ui)
                {
                    previewer.seek_to(val.into());
                }

                widget::Text::new(&format!("{}", current_frame))
                    .bottom_left_with_margins_on(ids.slider, 30.0, pointer_width)
                    .rgba(0.7, 0.7, 0.7, 0.75)
                    .font_size(32)
                    .set(ids.min_label, ui);
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

widget_ids!(struct Ids { canvas, slider, min_label, max_label });
