#![allow(non_camel_case_types)]
use std::fmt::Display;

use num_enum::FromPrimitive;

// Color range
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, num_enum::Default)]
#[repr(u8)]
pub enum VSColorRange {
    Full = 0,
    Limited,
    #[num_enum(default)]
    Unspecfied,
}

// Mapping zimg color matrices
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, num_enum::Default)]
#[repr(u8)]
pub enum VSMatrix {
    Rgb = 0,
    BT709,
    #[num_enum(default)]
    Unspecified,
    Reserved3,
    Fcc,
    BT470bg,
    ST170M,
    ST240M,
    YCgCo,
    BT2020Ncl,
    BT2020cl,
    ST2085,
    ChromaNcl,
    Chromacl,
    ICtCp,
}

// Mapping zimg transfer characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, num_enum::Default)]
#[repr(u8)]
pub enum VSTransferCharacteristics {
    Reserved0 = 0,
    BT709,
    #[num_enum(default)]
    Unspecified,
    Reserved3,
    BT470m,
    BT470bg,
    BT601,
    ST240M,
    Linear,
    Log100,
    Log316,
    xvYCC,
    BT1361,
    sRgb,
    BT2020_10,
    BT2020_12,
    ST2084,
    ST428,
    STD_B67,
}

// Primaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, num_enum::Default)]
#[repr(u8)]
pub enum VSPrimaries {
    Reserved0 = 0,
    BT709,
    #[num_enum(default)]
    Unspecified,
    Reserved3,
    BT470m,
    BT470bg,
    ST170M,
    ST240M,
    Film,
    BT2020,
    Xyz,
    DCIP3,
    DCIP3_D65,
    Reserved13,
    Reserved14,
    Reserved15,
    Reserved16,
    Reserved17,
    Reserved18,
    Reserved19,
    Reserved20,
    Reserved21,
    JEDEC_P22, // EBU3213
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, num_enum::Default)]
#[repr(u8)]
pub enum VSChromaLocation {
    Left = 0,
    Center,
    TopLeft,
    Top,
    BottomLeft,
    Bottom,
    #[num_enum(default)]
    Unspecified,
}

impl Display for VSColorRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::Limited => "Limited",
            Self::Full => "Full",
            Self::Unspecfied => "Unspecified",
        };

        f.write_str(val)
    }
}

impl Display for VSChromaLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::Left => "Left",
            Self::Center => "Center",
            Self::TopLeft => "Top left",
            Self::Top => "Top",
            Self::BottomLeft => "Bottom left",
            Self::Bottom => "Bottom",
            Self::Unspecified => "Unspecified",
        };

        f.write_str(val)
    }
}

impl Display for VSMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::Rgb => "RGB",
            Self::BT709 => "BT.709",
            Self::Unspecified => "Unspecified",
            Self::Reserved3 => "Reserved",
            Self::Fcc => "FCC",
            Self::BT470bg => "BT.470bg",
            Self::ST170M => "ST 170M",
            Self::ST240M => "ST 240M",
            Self::YCgCo => "YCgCo",
            Self::BT2020Ncl => "BT.2020 non-constant luminance",
            Self::BT2020cl => "BT.2020 constant luminance",
            Self::ST2085 => "ST2085",
            Self::ChromaNcl => "Chromaticity derived non-constant luminance",
            Self::Chromacl => "Chromaticity derived constant luminance",
            Self::ICtCp => "ICtCp",
        };

        f.write_str(val)
    }
}

impl Display for VSTransferCharacteristics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::Reserved0 | Self::Reserved3 => "Reserved",
            Self::BT709 => "BT.709",
            Self::Unspecified => "Unspecified",
            Self::BT470m => "BT.470m",
            Self::BT470bg => "BT.470bg",
            Self::BT601 => "BT.601",
            Self::ST240M => "ST 240M",
            Self::Linear => "Linear",
            Self::Log100 => "Log 1:100 contrast",
            Self::Log316 => "Log 1:316 contrast",
            Self::xvYCC => "xvYCC",
            Self::BT1361 => "BT.1361",
            Self::sRgb => "sRGB",
            Self::BT2020_10 => "BT.2020_10",
            Self::BT2020_12 => "BT.2020_12",
            Self::ST2084 => "ST 2084 (PQ)",
            Self::ST428 => "ST 428",
            Self::STD_B67 => "ARIB std-b67 (HLG)",
        };

        f.write_str(val)
    }
}

impl Display for VSPrimaries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::Reserved0
            | Self::Reserved3
            | Self::Reserved13
            | Self::Reserved14
            | Self::Reserved15
            | Self::Reserved16
            | Self::Reserved17
            | Self::Reserved18
            | Self::Reserved19
            | Self::Reserved20
            | Self::Reserved21 => "Reserved",
            Self::BT709 => "BT.709",
            Self::Unspecified => "Unspecified",
            Self::BT470m => "BT.470m",
            Self::BT470bg => "BT.470bg",
            Self::ST170M => "ST 170M",
            Self::ST240M => "ST 240M",
            Self::Film => "Film",
            Self::BT2020 => "BT.2020",
            Self::Xyz => "XYZ",
            Self::DCIP3 => "DCI-P3, DCI white point",
            Self::DCIP3_D65 => "DCI-P3 D65 white point",
            Self::JEDEC_P22 => "JEDEC P22",
        };

        f.write_str(val)
    }
}
