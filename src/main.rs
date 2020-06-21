extern crate image;
extern crate piston_window;

use std::path::PathBuf;

use piston_window::*;
use structopt::StructOpt;

mod previewed_script;
mod previewer;

use previewed_script::PreviewedScript;
use previewer::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "vspreview-rs", about = "Vapoursynth script previewer")]
struct Opt {
    #[structopt(name = "input", parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let script = PreviewedScript::new(opt.input);

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("VS Preview", [800, 600])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    window.set_lazy(true);

    let frame_no: u32 = 119;
    let mut previewer = Previewer::new(&mut window, script, frame_no);

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
    }
}
