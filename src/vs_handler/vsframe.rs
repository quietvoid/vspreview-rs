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
    frame_type: String,

    color_range: VSPixelRange,
    chromaloc: VSChromaLoc,

    primaries: VSPrimaries,
    matrix: VSMatrix,
    transfer: VSTransferCharacteristics,

    is_scenecut: Option<bool>,
}

impl VSFrameProps {
    // Only reserved frame props
    pub fn from_mapref(map: MapRef) -> Self {
        let frame_type = if let Ok(frame_type) = map.get_data("_PictType") {
            std::str::from_utf8(frame_type).unwrap().to_string()
        } else {
            "N/A".to_string()
        };

        let color_range = map
            .get_int("_ColorRange")
            .map(|v| v as u8)
            .map_or(VSPixelRange::default(), VSPixelRange::from);

        let chromaloc = map
            .get_int("_ChromaLocation")
            .map(|v| v as u8)
            .map_or(VSChromaLoc::default(), VSChromaLoc::from);

        let primaries = map
            .get_int("_Primaries")
            .map(|v| v as u8)
            .map_or(VSPrimaries::default(), VSPrimaries::from);

        let matrix = map
            .get_int("_Matrix")
            .map(|v| v as u8)
            .map_or(VSMatrix::default(), VSMatrix::from);

        let transfer = map.get_int("_Transfer").map(|v| v as u8).map_or(
            VSTransferCharacteristics::default(),
            VSTransferCharacteristics::from,
        );

        let is_scenecut = map
            .get_int("_SceneChangePrev")
            .map_or(None, |v| Some(v != 0));

        VSFrameProps {
            frame_type,
            color_range,
            chromaloc,
            primaries,
            matrix,
            transfer,
            is_scenecut,
        }
    }
}
