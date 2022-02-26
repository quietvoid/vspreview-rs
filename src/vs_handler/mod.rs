use std::collections::HashMap;
use std::path::PathBuf;

use vapoursynth::prelude::*;

use crate::utils::frame_to_dynimage;

pub mod vsframe;
pub mod vsnode;
pub mod vstransform;
pub mod zimg_map;

pub use vsframe::{VSFrame, VSFrameProps};
pub use vsnode::VSNode;
pub use vstransform::*;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct PreviewedScript {
    script_file: String,
    script_dir: PathBuf,

    #[serde(skip)]
    env: Option<Environment>,
}

#[derive(Default, Clone, Debug)]
pub struct VSOutput {
    pub index: i32,
    pub node_info: VSNode,
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

        if let Err(e) = env.eval_file(&self.script_file, EvalFlags::SetWorkingDir) {
            println!("{:?}", e);
        };
    }

    pub fn get_outputs(&mut self) -> HashMap<i32, VSOutput> {
        let env = self.env.get_or_insert(Environment::new().unwrap());

        (0..9)
            .map(|i| {
                env.get_output(i).map(|(node, _alpha)| {
                    let out = VSOutput {
                        index: i,
                        node_info: VSNode::from_videoinfo(node.info()),
                    };

                    (i, out)
                })
            })
            .filter_map(Result::ok)
            .collect()
    }

    pub fn get_frame(
        &mut self,
        output: i32,
        frame_no: u32,
        opts: &VSTransformOptions,
    ) -> Option<VSFrame> {
        let env = self.env.as_ref().unwrap();

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
                    let is_rgb24 = id == PresetFormat::RGB24 as i32;

                    // Disable dither for RGB24 src
                    // Always dither for GRAY/YUV src
                    if opts.enable_dithering && !is_rgb24 && f.bitsPerSample >= 8 {
                        args.set_data("dither_type", opts.dither_algo.as_str().as_bytes())
                            .unwrap();
                    }

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
                            if !is_rgb24 {
                                args.set_int("format", PresetFormat::RGB24 as i64).unwrap();
                                true
                            } else {
                                false
                            }
                        }
                        _ => panic!("Invalid frame color family for preview!"),
                    };

                    if modified {
                        let rgb = resize_plugin.invoke(opts.resizer.as_str(), &args).unwrap();
                        node = rgb.get_node("clip").unwrap();
                    }
                } else {
                    panic!("Invalid format: must be constant");
                }

                let frame = node.get_frame(frame_no as usize).unwrap();
                let props = VSFrameProps::from_mapref(frame.props());
                let image = frame_to_dynimage(&frame);

                Some(VSFrame { image, props })
            }
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }

    pub fn get_original_props(&mut self, output: i32, frame_no: u32) -> Option<VSFrameProps> {
        let env = self.env.as_ref().unwrap();

        match env.get_output(output) {
            Ok((node, _alpha)) => {
                let frame = node.get_frame(frame_no as usize).unwrap();

                Some(VSFrameProps::from_mapref(frame.props()))
            }
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }

    pub fn get_script_dir(&self) -> PathBuf {
        self.script_dir.clone()
    }
}
