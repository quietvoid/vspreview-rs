use std::path::PathBuf;

use lcms2::{CIExyY, CIExyYTRIPLE, Flags, Intent, PixelFormat, Profile, ToneCurve, Transform};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct IccProfile {
    pub icc_file: PathBuf,

    #[serde(skip)]
    pub target_profile: Option<Profile>,

    #[serde(skip)]
    pub input_profile: Option<Profile>,

    pub input_whitepoint: XyYCoords,
    pub input_primaries: XyYTriple,

    #[serde(skip)]
    pub transform: Option<Transform<image::Rgb<u8>, image::Rgb<u8>>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct XyYCoords {
    x: f64,
    y: f64,
    y2: f64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct XyYTriple {
    red: XyYCoords,
    green: XyYCoords,
    blue: XyYCoords,
}

// Correct image from BT.1886 Rec709 to the target profile
impl IccProfile {
    pub fn srgb(icc_file: PathBuf) -> Self {
        Self {
            icc_file,
            target_profile: None,
            input_profile: None,
            input_whitepoint: XyYCoords::d65(),
            input_primaries: XyYTriple::rec709(),
            transform: None,
        }
    }

    pub fn setup(&mut self) {
        let target_profile = Profile::new_file(&self.icc_file).unwrap();
        let intent = Intent::RelativeColorimetric;

        let bp = target_profile.detect_black_point(intent).unwrap();

        // Target contract with profile black point
        let lw: f64 = 1.0;
        let lb: f64 = bp.Y;

        // Input assumed BT.1886
        let wp = CIExyY::from(&self.input_whitepoint);
        let prim = CIExyYTRIPLE::from(&self.input_primaries);

        let lwy = lw.powf(1.0 / 2.4);
        let lby = lb.powf(1.0 / 2.4);

        let tc = &ToneCurve::new_parametric(6, &[2.4, lwy - lby, lby, 0.0]).unwrap();
        let curves = [tc, tc, tc];

        let input_profile = Profile::new_rgb(&wp, &prim, &curves).unwrap();

        let transform = Transform::new_flags(
            &input_profile,
            PixelFormat::RGB_8,
            &target_profile,
            PixelFormat::RGB_8,
            intent,
            Flags::default() | Flags::BLACKPOINT_COMPENSATION,
        )
        .unwrap();

        self.input_profile = Some(input_profile);
        self.target_profile = Some(target_profile);
        self.transform = Some(transform);
    }
}

impl XyYCoords {
    pub fn d65() -> Self {
        Self {
            x: 0.31271,
            y: 0.32902,
            y2: 1.0,
        }
    }
}

impl XyYTriple {
    pub fn rec709() -> Self {
        XyYTriple {
            red: XyYCoords {
                x: 0.64,
                y: 0.33,
                y2: 1.0,
            },
            green: XyYCoords {
                x: 0.30,
                y: 0.60,
                y2: 1.0,
            },
            blue: XyYCoords {
                x: 0.15,
                y: 0.06,
                y2: 1.0,
            },
        }
    }
}

impl From<&XyYCoords> for CIExyY {
    fn from(xyy: &XyYCoords) -> Self {
        CIExyY {
            x: xyy.x,
            y: xyy.y,
            Y: xyy.y2,
        }
    }
}

impl From<&XyYTriple> for CIExyYTRIPLE {
    fn from(prim: &XyYTriple) -> Self {
        Self {
            Red: CIExyY::from(&prim.red),
            Green: CIExyY::from(&prim.green),
            Blue: CIExyY::from(&prim.blue),
        }
    }
}
