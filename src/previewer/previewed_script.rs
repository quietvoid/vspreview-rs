use std::fmt;
use std::path::PathBuf;

use vapoursynth::prelude::*;
use vapoursynth::video_info::VideoInfo;

use piston_window::Size;

const BGR32_FORMAT: i64 = PresetFormat::CompatBGR32 as i64;

pub struct PreviewedScript {
    env: Environment,
    script_file: String,
    script_dir: PathBuf,
    script_info: ScriptInfo,
}

#[derive(Default, Clone)]
pub struct ScriptInfo {
    num_frames: u32,
    width: u32,
    height: u32,
    fr_num: u32,
    fr_denom: u32,
    framerate: u32,
    format_name: String,
}

fn get_script_info(info: VideoInfo) -> ScriptInfo {
    let (width, height) = match info.resolution {
        Property::Constant(r) => (r.width as u32, r.height as u32),
        Property::Variable => panic!("Only supports constant resolution!"),
    };
    let format = match info.format {
        Property::Constant(f) => f,
        Property::Variable => panic!("Unsupported format!"),
    };

    let (fr_num, fr_denom) = match info.framerate {
        Property::Constant(fr) => (fr.numerator as u32, fr.denominator as u32),
        Property::Variable => panic!("Only supports constant framerate!"),
    };

    ScriptInfo {
        num_frames: info.num_frames as u32,
        width,
        height,
        fr_num,
        fr_denom,
        framerate: (fr_num as f64 / fr_denom as f64).ceil() as u32,
        format_name: String::from(format.name()),
    }
}

impl PreviewedScript {
    pub fn new(script_path: PathBuf) -> Self {
        let mut script_dir = script_path.clone();
        script_dir.pop();

        let script_file: String = script_path.into_os_string().into_string().unwrap();

        let mut previewed_script = Self {
            env: Environment::new().unwrap(),
            script_file,
            script_dir,
            script_info: ScriptInfo::default(),
        };

        previewed_script.reload();

        previewed_script
    }

    pub fn reload(&mut self) {
        let env = &mut self.env;
        match env.eval_file(&self.script_file, EvalFlags::SetWorkingDir) {
            Ok(_) => self.update_fields(),
            Err(e) => println!("{:?}", e),
        };
    }

    pub fn get_frame(&self, frame_no: u32) -> Option<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
        let env = &self.env;

        match env.get_output(0) {
            Ok((mut node, _alpha)) => {
                let resize_plugin = env
                    .get_core()
                    .unwrap()
                    .get_plugin_by_id("com.vapoursynth.resize")
                    .unwrap()
                    .unwrap();

                let mut args = OwnedMap::new(API::get().unwrap());
                args.set_node("clip", &node).unwrap();
                args.set_int("format", BGR32_FORMAT).unwrap();

                if let Property::Constant(f) = node.info().format {
                    match f.color_family() {
                        ColorFamily::YUV => args.set_int("matrix_in", 1).unwrap(),
                        _ => (),
                    }
                }

                let rgb = resize_plugin.invoke("Spline16", &args).unwrap();
                node = rgb.get_node("clip").unwrap();

                let frame = node.get_frame(frame_no as usize).unwrap();
                let data: &[u32] = frame.plane(0).unwrap();

                let res = frame.resolution(0);
                let (width, height) = (res.width, res.height);

                let buf_size = (width * height) as usize;
                let mut buf: Vec<u8> = Vec::with_capacity(buf_size * 4);

                // BGRX to RGBA, with max alpha
                for pixel in data {
                    let mut bgrx = pixel.to_ne_bytes();
                    bgrx[0..3].reverse();
                    bgrx[3] = u8::MAX;

                    buf.extend_from_slice(&bgrx);
                }

                let mut image =
                    image::ImageBuffer::from_raw(width as u32, height as u32, buf).unwrap();
                image::imageops::flip_vertical_in_place(&mut image);

                Some(image)
            }
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }

    pub fn get_num_frames(&self) -> u32 {
        self.script_info.num_frames
    }

    pub fn get_frame_rate(&self) -> u32 {
        self.script_info.framerate
    }

    pub fn get_script_dir(&self) -> PathBuf {
        self.script_dir.clone()
    }

    fn update_fields(&mut self) {
        let env = &self.env;

        match env.get_output(0) {
            Ok((node, _alpha)) => {
                self.script_info = get_script_info(node.info());
            }
            Err(e) => println!("{:?}", e),
        };
    }

    pub fn get_size(&self) -> Size {
        self.script_info.get_size()
    }

    pub fn get_script_info(&self) -> ScriptInfo {
        self.script_info.clone()
    }
}

impl fmt::Display for ScriptInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Frames: {} | Size: {}x{} | FPS: {}/{} = {:.3} | Format: {}",
            self.num_frames,
            self.width,
            self.height,
            self.fr_num,
            self.fr_denom,
            (self.fr_num as f32 / self.fr_denom as f32),
            self.format_name
        )
    }
}

impl ScriptInfo {
    pub fn get_size(&self) -> Size {
        Size::from((self.width, self.height))
    }
}
