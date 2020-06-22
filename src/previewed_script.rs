use std::path::PathBuf;
use std::time::Instant;

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

        let script_file = script_path.into_os_string().into_string().unwrap();

        let env = Environment::from_file(&script_file, EvalFlags::SetWorkingDir).unwrap();

        let node = env.get_output(0).unwrap().0;
        let info = node.info();

        let summary = get_summary(info);

        let (fr_num, fr_denom) = match info.framerate {
            Property::Constant(fr) => (fr.numerator, fr.denominator),
            Property::Variable => panic!("Only supports constant framerate!"),
        };

        Self {
            env: Environment::from_file(&script_file, EvalFlags::SetWorkingDir).unwrap(),
            script_file,
            script_dir,
            num_frames: info.num_frames as u32,
            frame_rate_num: (fr_num as f64 / fr_denom as f64).ceil() as u32,
            summary,
        }
    }

    pub fn reload(&mut self) {
        let env = &mut self.env;
        env.eval_file(&self.script_file, EvalFlags::SetWorkingDir)
            .unwrap();

        self.update_fields();
    }

    pub fn get_frame(&self, frame_no: u32) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        let now = Instant::now();

        let env = &self.env;
        let mut node = env.get_output(0).unwrap().0;

        let resize_plugin = env
            .get_core()
            .unwrap()
            .get_plugin_by_id("com.vapoursynth.resize")
            .unwrap()
            .unwrap();

        let mut args = OwnedMap::new(API::get().unwrap());
        args.set_node("clip", &node).unwrap();
        args.set_int("format", RGB24_FORMAT as i64).unwrap();

        let rgb = resize_plugin.invoke("Point", &args).unwrap();
        node = rgb.get_node("clip").unwrap();

        let frame = node.get_frame(frame_no as usize).unwrap();

        let (r, g, b): (&[u8], &[u8], &[u8]) = (
            frame.plane(0).unwrap(),
            frame.plane(1).unwrap(),
            frame.plane(2).unwrap(),
        );

        println!("Got frame in {}ms", now.elapsed().as_millis());

        self.to_rgba_buf(frame.resolution(0), r, g, b)
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

        let node = env.get_output(0).unwrap().0;
        let info = node.info();

        let (fr_num, fr_denom) = match info.framerate {
            Property::Constant(fr) => (fr.numerator, fr.denominator),
            Property::Variable => panic!("Only supports constant framerate!"),
        };

        self.num_frames = info.num_frames as u32;
        self.frame_rate_num = (fr_num as f64 / fr_denom as f64).ceil() as u32;

        self.summary = get_summary(info);
    }
}
