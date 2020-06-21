extern crate piston_window;
extern crate image;

use std::path::PathBuf;

use piston_window::*;
use structopt::StructOpt;

mod previewer;
mod previewed_script;

use previewer::*;
use previewed_script::PreviewedScript;

#[derive(StructOpt, Debug)]
#[structopt(name = "vspreview-rs", about = "Vapoursynth script previewer")]
struct Opt {
    #[structopt(name = "input", parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow =
        WindowSettings::new("VS Preview", [800,600])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    window.set_lazy(true);

    let frame_no: u32 = 119;
    let script = PreviewedScript::new(opt.input);
    let mut previewer = Previewer::new(&mut window, script, frame_no);

    while let Some(e) = window.next() {
        match e {
            Event::Input(Input::Button(input), _opt) => {
                match (input.button, input.state) {
                    (Button::Keyboard(k), ButtonState::Press) => {
                        previewer.handle_key_press(&window, &k);
                    },
                    (Button::Keyboard(k), ButtonState::Release) => {
                        previewer.handle_key_release(&k);
                    },
                    _ => (),
                }
            },
            Event::Input(Input::Move(motion), _opt) => {
                match motion {
                    Motion::MouseScroll(ticks) => {
                        previewer.handle_mouse_scroll(&window, ticks);
                    }
                    _ => (),
                }
            },
            Event::Loop(render) => {
                match render {
                    Loop::Render(_ra) => {
                        previewer.rerender(&mut window, &e);
                    },
                    _ => {},
                };
            },
            _ => {},
        };
    }
}