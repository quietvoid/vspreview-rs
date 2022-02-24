use eframe::epaint::ColorImage;
use vapoursynth::map::MapRef;

use super::zimg_map::*;

#[derive(Default, Clone)]
pub struct VSFrame {
    pub frame_image: ColorImage,
    pub props: VSFrameProps,
}

#[derive(Default, Debug, Clone)]
pub struct VSFrameProps {
    pub frame_type: String,

    pub color_range: VSColorRange,
    pub chroma_location: Option<VSChromaLocation>,

    pub primaries: VSPrimaries,
    pub matrix: VSMatrix,
    pub transfer: VSTransferCharacteristics,

    pub is_scenecut: Option<bool>,
    pub is_dolbyvision: bool,
    pub cambi_score: Option<f64>,
}

/// Reserved props
const KEY_FRAME_TYPE: &str = "_PictType";
const KEY_COLOR_RANGE: &str = "_ColorRange";
const KEY_CHROMALOC: &str = "_ChromaLocation";
const KEY_PRIMARIES: &str = "_Primaries";
const KEY_MATRIX: &str = "_Matrix";
const KEY_TRANSFER: &str = "_Transfer";
const KEY_SCENE_CUT: &str = "_SceneChangePrev";

/// Potentially relevant props
const KEY_DOVI_RPU: &str = "DolbyVisionRPU";
const KEY_CAMBI: &str = "CAMBI";

impl VSFrameProps {
    // Only reserved frame props
    pub fn from_mapref(map: MapRef) -> Self {
        let frame_type = if let Ok(frame_type) = map.get_data(KEY_FRAME_TYPE) {
            std::str::from_utf8(frame_type).unwrap().to_string()
        } else {
            "N/A".to_string()
        };

        let color_range = map
            .get_int(KEY_COLOR_RANGE)
            .map(|v| v as u8)
            .map_or(VSColorRange::default(), VSColorRange::from);

        let primaries = map
            .get_int(KEY_PRIMARIES)
            .map(|v| v as u8)
            .map_or(VSPrimaries::default(), VSPrimaries::from);

        let matrix = map
            .get_int(KEY_MATRIX)
            .map(|v| v as u8)
            .map_or(VSMatrix::default(), VSMatrix::from);

        let transfer = map.get_int(KEY_TRANSFER).map(|v| v as u8).map_or(
            VSTransferCharacteristics::default(),
            VSTransferCharacteristics::from,
        );

        let chroma_location = map
            .get_int(KEY_CHROMALOC)
            .ok()
            .map(|v| VSChromaLocation::from(v as u8));

        let is_scenecut = map.get_int(KEY_SCENE_CUT).map_or(None, |v| Some(v != 0));
        let is_dolbyvision = map.value_count(KEY_DOVI_RPU).map_or(false, |v| v > 0);
        let cambi_score = map.get_float(KEY_CAMBI).ok();

        VSFrameProps {
            frame_type,
            color_range,
            chroma_location,
            primaries,
            matrix,
            transfer,
            is_scenecut,
            is_dolbyvision,
            cambi_score,
        }
    }
}
