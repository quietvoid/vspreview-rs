use vapoursynth::{prelude::Property, video_info::VideoInfo};

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct VSNode {
    pub num_frames: u32,
    pub width: u32,
    pub height: u32,
    pub fr_num: u32,
    pub fr_denom: u32,
    pub framerate: u32,
    pub format_name: String,
}

impl VSNode {
    pub fn from_videoinfo(info: VideoInfo) -> VSNode {
        let (width, height) = match info.resolution {
            Property::Constant(r) => (r.width as u32, r.height as u32),
            Property::Variable => panic!("Only supports constant resolution!"),
        };
        let format = info.format;

        let (fr_num, fr_denom) = match info.framerate {
            Property::Constant(fr) => (fr.numerator as u32, fr.denominator as u32),
            Property::Variable => panic!("Only supports constant framerate!"),
        };

        VSNode {
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

impl std::fmt::Display for VSNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
