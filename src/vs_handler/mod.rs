use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use vapoursynth::api::MessageHandlerId;
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
    #[serde(skip)]
    message_handler_id: Option<MessageHandlerId>,

    #[serde(skip)]
    pub vs_messages: Arc<Mutex<Vec<VSMessage>>>,
}

#[derive(Default, Clone, Debug)]
pub struct VSOutput {
    pub index: i32,
    pub node_info: VSNode,
}

#[derive(Clone, Debug)]
pub struct VSMessage {
    pub message_type: MessageType,
    pub message: String,
}

impl PreviewedScript {
    pub fn new(script_path: PathBuf) -> Self {
        let mut script_dir = script_path.clone();
        script_dir.pop();

        let script_file: String = script_path
            .into_os_string()
            .into_string()
            .expect("Invalid script file path!");

        Self {
            script_file,
            script_dir,
            env: None,
            message_handler_id: None,
            vs_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn reload(&mut self) -> Result<()> {
        let env = if let Some(env) = self.env.as_mut() {
            env.clear();

            env
        } else {
            self.env.get_or_insert(Environment::new()?)
        };

        if self.message_handler_id.is_none() {
            let api = API::get().ok_or(anyhow!("Couldn't retrieve API object"))?;

            let vserrors = self.vs_messages.clone();
            let id = api.add_message_handler(move |message_type, message| {
                let message = message.to_str().unwrap().to_string();

                let mut errors = vserrors.lock();
                errors.push(VSMessage {
                    message_type,
                    message,
                });
            });

            self.message_handler_id = Some(id);
        }

        env.eval_file(&self.script_file, EvalFlags::SetWorkingDir)?;

        Ok(())
    }

    pub fn get_outputs(&mut self) -> Result<HashMap<i32, VSOutput>> {
        let env = self.env.get_or_insert(Environment::new()?);

        let outputs: HashMap<i32, VSOutput> = (0..9)
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
            .collect();

        if !outputs.is_empty() {
            Ok(outputs)
        } else {
            bail!("VapourSynth script has not set any output node!");
        }
    }

    pub fn get_frame(
        &mut self,
        output: i32,
        frame_no: u32,
        opts: &VSTransformOptions,
    ) -> Result<VSFrame> {
        let env = self
            .env
            .as_ref()
            .ok_or(anyhow!("Cannot request VS frame without environment"))?;

        let (mut node, _alpha) = env.get_output(output)?;

        // std plugin, should always exist
        let resize_plugin = env
            .get_core()?
            .get_plugin_by_id("com.vapoursynth.resize")?
            .unwrap();

        let mut args = OwnedMap::new(API::get().ok_or(anyhow!("Couldn't initialize VS API"))?);
        args.set_node("clip", &node)?;

        if let Property::Constant(f) = node.info().format {
            let id = i32::from(f.id());
            let is_rgb24 = id == PresetFormat::RGB24 as i32;

            // Disable dither for RGB24 src
            // Always dither for GRAY/YUV src
            if opts.enable_dithering && !is_rgb24 && f.bitsPerSample >= 8 {
                args.set_data("dither_type", opts.dither_algo.as_str().as_bytes())?;
            }

            let modified = match f.color_family() {
                ColorFamily::Gray => {
                    if id != PresetFormat::Gray8 as i32 {
                        args.set_int("format", PresetFormat::Gray8 as i64)?;
                        true
                    } else {
                        false
                    }
                }
                ColorFamily::YUV => {
                    args.set_int("format", PresetFormat::RGB24 as i64)?;
                    args.set_int("matrix_in", 1)?;

                    true
                }
                ColorFamily::RGB => {
                    if !is_rgb24 {
                        args.set_int("format", PresetFormat::RGB24 as i64)?;
                        true
                    } else {
                        false
                    }
                }
                _ => panic!("Invalid frame color family for preview!"),
            };

            if modified {
                let rgb = resize_plugin.invoke(opts.resizer.as_str(), &args)?;
                node = rgb.get_node("clip")?;
            }
        } else {
            panic!("Invalid format: must be constant");
        }

        let frame = node.get_frame(frame_no as usize)?;
        let props = VSFrameProps::from_mapref(frame.props());
        let image = frame_to_dynimage(&frame);

        Ok(VSFrame { image, props })
    }

    pub fn get_original_props(&mut self, output: i32, frame_no: u32) -> Result<VSFrameProps> {
        let env = self
            .env
            .as_ref()
            .ok_or(anyhow!("Cannot request VS frame without environment"))?;

        let (node, _alpha) = env.get_output(output)?;
        let frame = node.get_frame(frame_no as usize)?;

        Ok(VSFrameProps::from_mapref(frame.props()))
    }

    pub fn get_script_dir(&self) -> PathBuf {
        self.script_dir.clone()
    }

    pub fn add_vs_error<T>(&mut self, res: &Result<T>) {
        if let Err(e) = res {
            let mut messages = self.vs_messages.lock();

            messages.push(VSMessage {
                message_type: MessageType::Fatal,
                message: format!("{:?}", e),
            });
        }
    }
}

impl Drop for PreviewedScript {
    fn drop(&mut self) {
        if let Some(handler_id) = self.message_handler_id {
            let api = API::get().unwrap();
            api.remove_message_handler(handler_id);
        }
    }
}
