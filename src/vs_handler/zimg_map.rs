#![allow(non_camel_case_types)]
use num_enum::FromPrimitive;

// Pixel range
#[derive(Debug, Clone, FromPrimitive, num_enum::Default)]
#[repr(u8)]
pub enum VSPixelRange {
    Limited = 0,
    Full,
    #[num_enum(default)]
    Unspecfied,
}

// Mapping zimg color matrices
#[derive(Debug, Clone, FromPrimitive, num_enum::Default)]
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
#[derive(Debug, Clone, FromPrimitive, num_enum::Default)]
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
#[derive(Debug, Clone, FromPrimitive, num_enum::Default)]
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

#[derive(Debug, Clone, FromPrimitive, num_enum::Default)]
#[repr(u8)]
pub enum VSChromaLoc {
    Left = 0,
    Center,
    TopLeft,
    Top,
    BottomLeft,
    Bottom,
    #[num_enum(default)]
    Unspecified,
}
