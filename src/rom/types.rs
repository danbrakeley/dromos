use std::fmt;
use std::str::FromStr;

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

impl FromStr for RomType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NES" => Ok(RomType::Nes),
            _ => Err(()),
        }
    }
}

impl RomType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RomType::Nes => "NES",
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
