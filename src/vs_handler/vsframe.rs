use eframe::epaint::ColorImage;
use vapoursynth::map::MapRef;

#[derive(Default, Clone)]
pub struct VSFrame {
    pub frame_image: ColorImage,
    pub props: VSFrameProps,
}

#[derive(Default, Clone)]
pub struct VSFrameProps {
    frame_type: String,
    color_range: String,

    primaries: String,
    matrix: String,
    transfer: String,

    chromaloc: String,

    is_scenecut: Option<bool>,
}

impl VSFrameProps {
    // Only reserved frame props
    pub fn from_mapref(map: MapRef) -> Self {
        let mut props = VSFrameProps::default();

        props.frame_type = if let Ok(frame_type) = map.get_data("_PictType") {
            std::str::from_utf8(frame_type).unwrap().to_string()
        } else {
            "N/A".to_string()
        };

        props
    }
}
