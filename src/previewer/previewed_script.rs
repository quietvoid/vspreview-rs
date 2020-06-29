use std::path::PathBuf;

use itertools::izip;
use vapoursynth::prelude::*;
use vapoursynth::video_info::Resolution;
use vapoursynth::video_info::VideoInfo;

const RGB24_FORMAT: i32 = PresetFormat::RGB24 as i32;

pub struct PreviewedScript {
    env: Environment,
    script_file: String,
    script_dir: PathBuf,
    num_frames: u32,
    frame_rate_num: u32,
    summary: String,
}

fn get_summary(info: VideoInfo) -> String {
    let (width, height) = match info.resolution {
        Property::Constant(r) => (r.width, r.height),
        Property::Variable => panic!("Only supports constant resolution!"),
    };
    let format = match info.format {
        Property::Constant(f) => f,
        Property::Variable => panic!("Unsupported format!"),
    };

    let (fr_num, fr_denom) = match info.framerate {
        Property::Constant(fr) => (fr.numerator, fr.denominator),
        Property::Variable => panic!("Only supports constant framerate!"),
    };

    let summary = format!(
        "Frames: {} | Size: {}x{} | FPS: {}/{} = {:.3} | Format: {}",
        info.num_frames,
        width,
        height,
        fr_num,
        fr_denom,
        (fr_num as f32 / fr_denom as f32),
        format.name(),
    );

    summary
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
            num_frames: 0,
            frame_rate_num: 0,
            summary: String::new(),
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
                args.set_int("format", RGB24_FORMAT as i64).unwrap();
        
                if let Property::Constant(f) = node.info().format {
                    match f.color_family() {
                        ColorFamily::YUV => args.set_int("matrix_in", 1).unwrap(),
                        _ => (),
                    }
                }
        
                let rgb = resize_plugin.invoke("Point", &args).unwrap();
                node = rgb.get_node("clip").unwrap();
        
                let frame = node.get_frame(frame_no as usize).unwrap();
        
                let (r, g, b): (&[u8], &[u8], &[u8]) = (
                    frame.plane(0).unwrap(),
                    frame.plane(1).unwrap(),
                    frame.plane(2).unwrap(),
                );
        
                Some(self.to_rgba_buf(frame.resolution(0), r, g, b))
            },
            Err(e) => {
                println!("{:?}", e);
                None
            },
        }
    }

    fn to_rgba_buf(
        &self,
        res: Resolution,
        r: &[u8],
        g: &[u8],
        b: &[u8],
    ) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        let (width, height) = (res.width, res.height);

        let buf_size = (width * height) as usize;

        let mut rgba: Vec<u8> = Vec::with_capacity(buf_size * 4);
        let alpha: u8 = 255;

        for (r, g, b) in izip!(r, g, b) {
            rgba.extend_from_slice(&[*r, *g, *b, alpha]);
        }

        image::ImageBuffer::from_raw(width as u32, height as u32, rgba).unwrap()
    }

    pub fn get_num_frames(&self) -> u32 {
        self.num_frames
    }

    pub fn get_frame_rate(&self) -> u32 {
        self.frame_rate_num
    }

    pub fn get_script_dir(&self) -> PathBuf {
        self.script_dir.clone()
    }

    pub fn get_summary(&self) -> &str {
        &self.summary
    }

    fn update_fields(&mut self) {
        let env = &self.env;

        match env.get_output(0) {
            Ok((node, _alpha)) => {
                let info = node.info();
                let (fr_num, fr_denom) = match info.framerate {
                    Property::Constant(fr) => (fr.numerator, fr.denominator),
                    Property::Variable => panic!("Only supports constant framerate!"),
                };
        
                self.num_frames = info.num_frames as u32;
                self.frame_rate_num = (fr_num as f64 / fr_denom as f64).ceil() as u32;
        
                self.summary = get_summary(info);
            },
            Err(e) => println!("{:?}", e),
        };
    }
}
