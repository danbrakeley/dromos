use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomType {
    Nes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal = 0,
    Vertical = 1,
    FourScreen = 2,
}

impl From<u8> for Mirroring {
    fn from(value: u8) -> Self {
        match value {
            1 => Mirroring::Vertical,
            2 => Mirroring::FourScreen,
            _ => Mirroring::Horizontal,
        }
    }
}

impl From<Mirroring> for u8 {
    fn from(m: Mirroring) -> u8 {
        m as u8
    }
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
    pub mapper: u16,
    pub mirroring: Mirroring,
    pub has_battery: bool,
    pub is_nes2: bool,
    pub submapper: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct RomMetadata {
    pub rom_type: RomType,
    pub sha256: [u8; 32],
    pub filename: Option<String>,
    pub nes_header: Option<NesHeader>,
    /// Raw file header bytes for byte-identical reconstruction
    pub source_file_header: Option<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirroring_from_u8() {
        assert_eq!(Mirroring::from(0), Mirroring::Horizontal);
        assert_eq!(Mirroring::from(1), Mirroring::Vertical);
        assert_eq!(Mirroring::from(2), Mirroring::FourScreen);
        // Unknown values default to Horizontal
        assert_eq!(Mirroring::from(99), Mirroring::Horizontal);
        assert_eq!(Mirroring::from(255), Mirroring::Horizontal);
    }

    #[test]
    fn test_mirroring_to_u8() {
        assert_eq!(u8::from(Mirroring::Horizontal), 0);
        assert_eq!(u8::from(Mirroring::Vertical), 1);
        assert_eq!(u8::from(Mirroring::FourScreen), 2);
    }

    #[test]
    fn test_rom_type_from_str() {
        assert_eq!("nes".parse::<RomType>(), Ok(RomType::Nes));
        assert_eq!("NES".parse::<RomType>(), Ok(RomType::Nes));
        assert_eq!("Nes".parse::<RomType>(), Ok(RomType::Nes));
        assert_eq!("nEs".parse::<RomType>(), Ok(RomType::Nes));
        assert!("snes".parse::<RomType>().is_err());
        assert!("".parse::<RomType>().is_err());
    }

    #[test]
    fn test_rom_type_display() {
        assert_eq!(format!("{}", RomType::Nes), "NES");
    }

    #[test]
    fn test_rom_type_as_str() {
        assert_eq!(RomType::Nes.as_str(), "NES");
    }

    #[test]
    fn test_rom_type_round_trip() {
        let original = RomType::Nes;
        let as_str = original.as_str();
        let parsed: RomType = as_str.parse().unwrap();
        assert_eq!(original, parsed);
    }
}
