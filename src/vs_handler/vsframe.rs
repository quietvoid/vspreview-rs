use std::fmt::Display;

use image::DynamicImage;
use vapoursynth::map::MapRef;

use super::zimg_map::*;

/// Reserved props
const KEY_FRAME_TYPE: &str = "_PictType";
const KEY_COLOR_RANGE: &str = "_ColorRange";
const KEY_CHROMALOC: &str = "_ChromaLocation";
const KEY_PRIMARIES: &str = "_Primaries";
const KEY_MATRIX: &str = "_Matrix";
const KEY_TRANSFER: &str = "_Transfer";
const KEY_SCENE_CUT: &str = "_SceneChangePrev";

/// Potentially relevant props
const KEY_CAMBI: &str = "CAMBI";
const KEY_DOVI_RPU: &str = "DolbyVisionRPU";

// HDR10 related
const KEY_MDCV_PRIM_X: &str = "MasteringDisplayPrimariesX";
const KEY_MDCV_PRIM_Y: &str = "MasteringDisplayPrimariesY";
const KEY_MDCV_WP_X: &str = "MasteringDisplayWhitePointX";
const KEY_MDCV_WP_Y: &str = "MasteringDisplayWhitePointY";
const KEY_MDCV_LUM_MIN: &str = "MasteringDisplayMinLuminance";
const KEY_MDCV_LUM_MAX: &str = "MasteringDisplayMaxLuminance";
const KEY_HDR10_MAXCLL: &str = "ContentLightLevelMax";
const KEY_HDR10_MAXFALL: &str = "ContentLightLevelAverage";

#[derive(Default, Clone)]
pub struct VSFrame {
    pub image: DynamicImage,
    pub props: VSFrameProps,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct VSFrameProps {
    pub frame_type: char,

    pub color_range: VSColorRange,
    pub chroma_location: VSChromaLocation,

    pub primaries: VSPrimaries,
    pub matrix: VSMatrix,
    pub transfer: VSTransferCharacteristics,

    pub is_scenecut: Option<bool>,
    pub cambi_score: Option<f64>,

    pub hdr10_metadata: Option<Hdr10Metadata>,
    pub is_dolbyvision: bool,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Hdr10Metadata {
    pub mastering_display: MdcvMetadata,
    pub maxcll: Option<f64>,
    pub maxfall: Option<f64>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct MdcvMetadata {
    pub lum_min: f64,
    pub lum_max: f64,

    pub red: [f64; 2],
    pub green: [f64; 2],
    pub blue: [f64; 2],
    pub white_point: [f64; 2],
}

impl VSFrameProps {
    // Only reserved frame props
    pub fn from_mapref(map: MapRef) -> Self {
        let frame_type = if let Ok(frame_type) = map.get_data(KEY_FRAME_TYPE) {
            frame_type[0] as char
        } else {
            '?'
        };

        let color_range = map
            .get_int(KEY_COLOR_RANGE)
            .map_or(VSColorRange::default(), |v| VSColorRange::from(v as u8));

        let chroma_location = map
            .get_int(KEY_CHROMALOC)
            .map_or(VSChromaLocation::default(), |v| {
                VSChromaLocation::from(v as u8)
            });

        let primaries = map
            .get_int(KEY_PRIMARIES)
            .map_or(VSPrimaries::default(), |v| VSPrimaries::from(v as u8));

        let matrix = map
            .get_int(KEY_MATRIX)
            .map_or(VSMatrix::default(), |v| VSMatrix::from(v as u8));

        let transfer = map.get_int(KEY_TRANSFER).map(|v| v as u8).map_or(
            VSTransferCharacteristics::default(),
            VSTransferCharacteristics::from,
        );

        let is_scenecut = map.get_int(KEY_SCENE_CUT).map_or(None, |v| Some(v != 0));
        let cambi_score = map.get_float(KEY_CAMBI).ok();

        let hdr10_metadata = Hdr10Metadata::new(&map);
        let is_dolbyvision = map.value_count(KEY_DOVI_RPU).map_or(false, |v| v > 0);

        VSFrameProps {
            frame_type,
            color_range,
            chroma_location,
            primaries,
            matrix,
            transfer,
            is_scenecut,
            cambi_score,
            is_dolbyvision,
            hdr10_metadata,
        }
    }
}

// Requires MDCV minimum
impl Hdr10Metadata {
    fn new(map: &MapRef) -> Option<Self> {
        let mastering_display = MdcvMetadata::new(map)?;

        let maxcll = map.get_float(KEY_HDR10_MAXCLL).ok();
        let maxfall = map.get_float(KEY_HDR10_MAXFALL).ok();

        let meta = Hdr10Metadata {
            mastering_display,
            maxcll,
            maxfall,
        };

        Some(meta)
    }
}

impl MdcvMetadata {
    fn new(map: &MapRef) -> Option<Self> {
        let lum_min = map.get_float(KEY_MDCV_LUM_MIN).ok()?;
        let lum_max = map.get_float(KEY_MDCV_LUM_MAX).ok()?;

        let primaries_x = map.get_float_array(KEY_MDCV_PRIM_X).ok()?;
        assert!(primaries_x.len() == 3);
        let primaries_y = map.get_float_array(KEY_MDCV_PRIM_Y).ok()?;
        assert!(primaries_x.len() == 3);

        let wp_x = map.get_float(KEY_MDCV_WP_X).ok()?;
        let wp_y = map.get_float(KEY_MDCV_WP_Y).ok()?;

        let meta = MdcvMetadata {
            lum_min,
            lum_max,
            red: [primaries_x[0], primaries_y[0]],
            green: [primaries_x[1], primaries_y[1]],
            blue: [primaries_x[2], primaries_y[2]],
            white_point: [wp_x, wp_y],
        };

        Some(meta)
    }

    pub fn x265_string(&self) -> String {
        let Self {
            lum_min,
            lum_max,
            red,
            green,
            blue,
            white_point,
        } = self;
        let [rx, ry] = red.map(|v| (v * 50000.0).round() as u16);
        let [gx, gy] = green.map(|v| (v * 50000.0).round() as u16);
        let [bx, by] = blue.map(|v| (v * 50000.0).round() as u16);
        let [wx, wy] = white_point.map(|v| (v * 50000.0).round() as u16);
        let (max, min) = (
            (*lum_max * 10000.0).round() as usize,
            (*lum_min * 10000.0).round() as usize,
        );

        format!(
            "\
            G({gx},{gy})\
            B({bx},{by})\
            R({rx},{ry})\
            WP({wx},{wy})\
            L({max},{min})\
        "
        )
    }
}

impl Display for MdcvMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            lum_min,
            lum_max,
            red,
            green,
            blue,
            white_point,
        } = self;
        let [rx, ry] = red.map(|v| (v * 50000.0).round() as u16);
        let [gx, gy] = green.map(|v| (v * 50000.0).round() as u16);
        let [bx, by] = blue.map(|v| (v * 50000.0).round() as u16);
        let [wx, wy] = white_point;

        let primaries_str = match (rx, ry, gx, gy, bx, by) {
            (34000, 16000, 8500, 39850, 6550, 2300) => Some("BT.2020"),
            (34000, 16000, 13250, 34500, 7500, 3000) => Some("Display P3"),
            (32000, 16500, 15000, 30000, 7500, 3000) => Some("BT.709"),
            _ => None,
        };

        if let Some(prim) = primaries_str {
            f.write_str(prim)
        } else {
            f.write_str(&format!(
                "\
                G({gx:.03},{gy:.03})\
                B({bx:.03},{by:.03})\
                R({rx:.03},{ry:.03})\
                WP({wx:.04},{wy:.04})\
                L({lum_max:.0},{lum_min:.4})\
            "
            ))
        }
    }
}
