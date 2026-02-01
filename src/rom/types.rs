use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomType {
    Nes,
}

impl fmt::Display for RomType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomType::Nes => write!(f, "NES"),
        }
    }
}

impl RomType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RomType::Nes => "NES",
        }
    }

    pub fn from_str(s: &str) -> Option<RomType> {
        match s.to_uppercase().as_str() {
            "NES" => Some(RomType::Nes),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NesHeader {
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub has_trainer: bool,
}

#[derive(Debug, Clone)]
pub struct RomMetadata {
    pub rom_type: RomType,
    pub sha256: [u8; 32],
    pub filename: Option<String>,
    pub nes_header: Option<NesHeader>,
}
