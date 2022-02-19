use std::fmt;
use std::path::PathBuf;

use eframe::epaint::ColorImage;
use vapoursynth::prelude::*;
use vapoursynth::video_info::VideoInfo;

use crate::utils::frame_to_colorimage;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct PreviewedScript {
    script_file: String,
    script_dir: PathBuf,

    #[serde(skip)]
    env: Option<Environment>,
}

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct NodeInfo {
    pub num_frames: u32,
    pub width: u32,
    pub height: u32,
    pub fr_num: u32,
    pub fr_denom: u32,
    pub framerate: u32,
    pub format_name: String,
}

#[derive(Default, Clone, Debug)]
pub struct VSOutput {
    pub index: i32,
    pub node_info: NodeInfo,
}

#[derive(Default, Clone)]
pub struct VSFrame {
    pub frame_image: ColorImage,
    pub frame_type: String,
}

impl PreviewedScript {
    pub fn new(script_path: PathBuf) -> Self {
        let mut script_dir = script_path.clone();
        script_dir.pop();

        let script_file: String = script_path.into_os_string().into_string().unwrap();

        Self {
            env: None,
            script_file,
            script_dir,
        }
    }

    pub fn reload(&mut self) {
        let env = self.env.get_or_insert(Environment::new().unwrap());
        println!("reloading script");

        if let Err(e) = env.eval_file(&self.script_file, EvalFlags::SetWorkingDir) {
            println!("{:?}", e);
        };
    }

    pub fn get_outputs(&mut self) -> Vec<VSOutput> {
        let env = self.env.get_or_insert(Environment::new().unwrap());

        (0..9)
            .map(|i| {
                env.get_output(i).map(|(node, _alpha)| VSOutput {
                    index: i,
                    node_info: NodeInfo::from_videoinfo(node.info()),
                })
            })
            .filter_map(Result::ok)
            .collect()
    }

    pub fn get_frame(&mut self, output: i32, frame_no: u32) -> Option<VSFrame> {
        let env = self.env.get_or_insert(Environment::new().unwrap());

        match env.get_output(output) {
            Ok((mut node, _alpha)) => {
                let resize_plugin = env
                    .get_core()
                    .unwrap()
                    .get_plugin_by_id("com.vapoursynth.resize")
                    .unwrap()
                    .unwrap();

                let mut args = OwnedMap::new(API::get().unwrap());
                args.set_node("clip", &node).unwrap();

                if let Property::Constant(f) = node.info().format {
                    let id = i32::from(f.id());

                    let modified = match f.color_family() {
                        ColorFamily::Gray => {
                            if id != PresetFormat::Gray8 as i32 {
                                args.set_int("format", PresetFormat::Gray8 as i64).unwrap();
                                true
                            } else {
                                false
                            }
                        }
                        ColorFamily::YUV => {
                            args.set_int("format", PresetFormat::RGB24 as i64).unwrap();
                            args.set_int("matrix_in", 1).unwrap();

                            true
                        }
                        ColorFamily::RGB => {
                            if id != PresetFormat::RGB24 as i32 {
                                args.set_int("format", PresetFormat::RGB24 as i64).unwrap();
                                true
                            } else {
                                false
                            }
                        }
                        _ => panic!("Invalid frame color family for preview!"),
                    };

                    if modified {
                        let rgb = resize_plugin.invoke("Spline16", &args).unwrap();
                        node = rgb.get_node("clip").unwrap();
                    }
                } else {
                    panic!("Invalid format: must be constant");
                }

                let frame = node.get_frame(frame_no as usize).unwrap();

                let frame_type = if let Ok(frame_type) = frame.props().get_data("_PictType") {
                    std::str::from_utf8(frame_type).unwrap().to_string()
                } else {
                    "N/A".to_string()
                };

                let frame_image = frame_to_colorimage(frame);

                Some(VSFrame {
                    frame_image,
                    frame_type,
                })
            }
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.env.is_some()
    }

    pub fn get_script_dir(&self) -> PathBuf {
        self.script_dir.clone()
    }
}

impl NodeInfo {
    fn from_videoinfo(info: VideoInfo) -> NodeInfo {
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

        NodeInfo {
            num_frames: info.num_frames as u32,
            width,
            height,
            fr_num,
            fr_denom,
            framerate: (fr_num as f64 / fr_denom as f64).ceil() as u32,
            format_name: String::from(format.name()),
        }
    }
}

impl fmt::Display for NodeInfo {
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
            self.format_name,
        )
    }
}
