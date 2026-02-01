use std::io::{Read, Seek, SeekFrom};

use crate::error::Result;
use crate::rom::types::{Mirroring, NesHeader};

/// Parse a 16-byte iNES/NES 2.0 header from raw bytes.
/// Returns None if the magic bytes are invalid.
pub fn parse_nes_header_bytes(header: &[u8; 16]) -> Option<NesHeader> {
    // Check for "NES\x1A" magic bytes
    if &header[0..4] != b"NES\x1a" {
        return None;
    }

    let flags6 = header[6];
    let flags7 = header[7];

    // Detect NES 2.0 format: bits 2-3 of byte 7 == 0b10
    let is_nes2 = (flags7 & 0x0C) == 0x08;

    let prg_rom_size = header[4] as usize * 16 * 1024; // 16 KB units
    let chr_rom_size = header[5] as usize * 8 * 1024; // 8 KB units
    let has_trainer = (flags6 & 0x04) != 0;
    let has_battery = (flags6 & 0x02) != 0;

    // Mirroring: bit 3 of flags6 = four-screen, bit 0 = vertical/horizontal
    let mirroring = if (flags6 & 0x08) != 0 {
        Mirroring::FourScreen
    } else if (flags6 & 0x01) != 0 {
        Mirroring::Vertical
    } else {
        Mirroring::Horizontal
    };

    // Mapper number: lower 4 bits from flags6, upper 4 bits from flags7
    let mapper_lo = (flags6 >> 4) as u16;
    let mapper_hi = (flags7 & 0xF0) as u16;
    let mut mapper = mapper_hi | mapper_lo;

    // NES 2.0 extended mapper bits (byte 8, bits 0-3)
    let submapper = if is_nes2 {
        let flags8 = header[8];
        mapper |= ((flags8 & 0x0F) as u16) << 8;
        let sub = (flags8 >> 4) & 0x0F;
        if sub > 0 { Some(sub) } else { None }
    } else {
        None
    };

    Some(NesHeader {
        prg_rom_size,
        chr_rom_size,
        has_trainer,
        mapper,
        mirroring,
        has_battery,
        is_nes2,
        submapper,
    })
}

/// Parse NES header from a reader. Thin I/O wrapper around parse_nes_header_bytes.
pub fn parse_nes_header(reader: &mut impl Read) -> Result<Option<NesHeader>> {
    let mut header = [0u8; 16];
    reader.read_exact(&mut header)?;
    Ok(parse_nes_header_bytes(&header))
}

/// Build a 16-byte iNES/NES 2.0 header from stored metadata.
/// The trainer flag is always cleared regardless of the original value.
pub fn build_nes_header(header: &NesHeader) -> [u8; 16] {
    let mut bytes = [0u8; 16];

    // Magic bytes
    bytes[0] = b'N';
    bytes[1] = b'E';
    bytes[2] = b'S';
    bytes[3] = 0x1A;

    // PRG ROM size in 16 KB units
    bytes[4] = (header.prg_rom_size / (16 * 1024)) as u8;

    // CHR ROM size in 8 KB units
    bytes[5] = (header.chr_rom_size / (8 * 1024)) as u8;

    // Flags 6: mapper lower nibble, mirroring, battery, trainer (always 0)
    let mut flags6 = ((header.mapper & 0x0F) << 4) as u8;
    match header.mirroring {
        Mirroring::Vertical => flags6 |= 0x01,
        Mirroring::FourScreen => flags6 |= 0x08,
        Mirroring::Horizontal => {}
    }
    if header.has_battery {
        flags6 |= 0x02;
    }
    // Note: trainer bit (0x04) is intentionally NOT set
    bytes[6] = flags6;

    // Flags 7: mapper upper nibble, NES 2.0 identifier
    let mut flags7 = (header.mapper & 0xF0) as u8;
    if header.is_nes2 {
        flags7 |= 0x08; // NES 2.0 identifier
    }
    bytes[7] = flags7;

    // Byte 8: NES 2.0 extended mapper and submapper
    if header.is_nes2 {
        let mapper_ext = ((header.mapper >> 8) & 0x0F) as u8;
        let submapper = header.submapper.unwrap_or(0) & 0x0F;
        bytes[8] = mapper_ext | (submapper << 4);
    }

    // Bytes 9-15 remain zero (unused in iNES 1.0, could be extended for NES 2.0)

    bytes
}

pub fn skip_trainer_if_present(reader: &mut (impl Read + Seek), header: &NesHeader) -> Result<()> {
    if header.has_trainer {
        reader.seek(SeekFrom::Current(512))?;
    }
    Ok(())
}

/// Reconstruct a complete NES ROM file from header metadata and raw ROM bytes.
/// The reconstructed file will NOT include trainer data, regardless of whether
/// the original file had it.
pub fn reconstruct_nes_file(header: &NesHeader, rom_bytes: &[u8]) -> Vec<u8> {
    let header_bytes = build_nes_header(header);
    let mut file = Vec::with_capacity(16 + rom_bytes.len());
    file.extend_from_slice(&header_bytes);
    file.extend_from_slice(rom_bytes);
    file
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Create a minimal valid iNES 1.0 header
    fn make_ines_header(prg_banks: u8, chr_banks: u8, flags6: u8, flags7: u8) -> [u8; 16] {
        let mut header = [0u8; 16];
        header[0] = b'N';
        header[1] = b'E';
        header[2] = b'S';
        header[3] = 0x1A;
        header[4] = prg_banks;
        header[5] = chr_banks;
        header[6] = flags6;
        header[7] = flags7;
        header
    }

    #[test]
    fn test_parse_valid_ines_header() {
        // 2 PRG banks (32KB), 1 CHR bank (8KB), mapper 0, horizontal mirroring
        let header = make_ines_header(2, 1, 0x00, 0x00);
        let parsed = parse_nes_header_bytes(&header).expect("Should parse valid header");

        assert_eq!(parsed.prg_rom_size, 32 * 1024);
        assert_eq!(parsed.chr_rom_size, 8 * 1024);
        assert_eq!(parsed.mapper, 0);
        assert_eq!(parsed.mirroring, Mirroring::Horizontal);
        assert!(!parsed.has_trainer);
        assert!(!parsed.has_battery);
        assert!(!parsed.is_nes2);
        assert!(parsed.submapper.is_none());
    }

    #[test]
    fn test_parse_nes2_header() {
        // NES 2.0 header: flags7 bits 2-3 = 0b10
        let mut header = make_ines_header(2, 1, 0x10, 0x08); // mapper 1, NES 2.0 flag
        header[8] = 0x52; // submapper 5, extended mapper bits 2

        let parsed = parse_nes_header_bytes(&header).expect("Should parse NES 2.0 header");

        assert!(parsed.is_nes2);
        assert_eq!(parsed.submapper, Some(5));
        // Mapper = 1 (from flags6/7) | (2 << 8) = 513
        assert_eq!(parsed.mapper, 0x201);
    }

    #[test]
    fn test_parse_invalid_magic() {
        let mut header = [0u8; 16];
        header[0] = b'N';
        header[1] = b'E';
        header[2] = b'S';
        header[3] = 0x00; // Invalid - should be 0x1A

        let parsed = parse_nes_header_bytes(&header);
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_trainer_flag() {
        // flags6 bit 2 = trainer present
        let header = make_ines_header(1, 0, 0x04, 0x00);
        let parsed = parse_nes_header_bytes(&header).expect("Should parse");

        assert!(parsed.has_trainer);
    }

    #[test]
    fn test_parse_mirroring_modes() {
        // Horizontal (default)
        let header_h = make_ines_header(1, 0, 0x00, 0x00);
        assert_eq!(
            parse_nes_header_bytes(&header_h).unwrap().mirroring,
            Mirroring::Horizontal
        );

        // Vertical (bit 0 set)
        let header_v = make_ines_header(1, 0, 0x01, 0x00);
        assert_eq!(
            parse_nes_header_bytes(&header_v).unwrap().mirroring,
            Mirroring::Vertical
        );

        // Four-screen (bit 3 set, takes precedence)
        let header_4 = make_ines_header(1, 0, 0x08, 0x00);
        assert_eq!(
            parse_nes_header_bytes(&header_4).unwrap().mirroring,
            Mirroring::FourScreen
        );
    }

    #[test]
    fn test_parse_battery_flag() {
        // flags6 bit 1 = battery-backed RAM
        let header = make_ines_header(1, 0, 0x02, 0x00);
        let parsed = parse_nes_header_bytes(&header).expect("Should parse");

        assert!(parsed.has_battery);
    }

    #[test]
    fn test_parse_mapper_number() {
        // Mapper 4 (MMC3): low nibble in flags6 bits 4-7, high nibble in flags7 bits 4-7
        // Mapper 4 = 0x04, so flags6 = 0x40, flags7 = 0x00
        let header = make_ines_header(1, 0, 0x40, 0x00);
        assert_eq!(parse_nes_header_bytes(&header).unwrap().mapper, 4);

        // Mapper 69 (Sunsoft FME-7): 0x45 = flags6 = 0x50, flags7 = 0x40
        let header2 = make_ines_header(1, 0, 0x50, 0x40);
        assert_eq!(parse_nes_header_bytes(&header2).unwrap().mapper, 69);
    }

    #[test]
    fn test_build_header_round_trip() {
        let original = NesHeader {
            prg_rom_size: 32 * 1024,
            chr_rom_size: 8 * 1024,
            has_trainer: false,
            mapper: 4,
            mirroring: Mirroring::Vertical,
            has_battery: true,
            is_nes2: false,
            submapper: None,
        };

        let bytes = build_nes_header(&original);
        let parsed = parse_nes_header_bytes(&bytes).expect("Should parse built header");

        assert_eq!(parsed.prg_rom_size, original.prg_rom_size);
        assert_eq!(parsed.chr_rom_size, original.chr_rom_size);
        assert_eq!(parsed.mapper, original.mapper);
        assert_eq!(parsed.mirroring, original.mirroring);
        assert_eq!(parsed.has_battery, original.has_battery);
        assert!(!parsed.has_trainer); // Always cleared
    }

    #[test]
    fn test_build_header_clears_trainer() {
        let original = NesHeader {
            prg_rom_size: 16 * 1024,
            chr_rom_size: 0,
            has_trainer: true, // Original had trainer
            mapper: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            is_nes2: false,
            submapper: None,
        };

        let bytes = build_nes_header(&original);
        let parsed = parse_nes_header_bytes(&bytes).expect("Should parse");

        // Trainer flag should be cleared
        assert!(!parsed.has_trainer);
    }

    #[test]
    fn test_build_header_nes2_round_trip() {
        let original = NesHeader {
            prg_rom_size: 64 * 1024,
            chr_rom_size: 16 * 1024,
            has_trainer: false,
            mapper: 0x105, // Extended mapper number
            mirroring: Mirroring::FourScreen,
            has_battery: true,
            is_nes2: true,
            submapper: Some(3),
        };

        let bytes = build_nes_header(&original);
        let parsed = parse_nes_header_bytes(&bytes).expect("Should parse NES 2.0 header");

        assert_eq!(parsed.mapper, original.mapper);
        assert!(parsed.is_nes2);
        assert_eq!(parsed.submapper, Some(3));
    }

    #[test]
    fn test_reconstruct_nes_file() {
        let header = NesHeader {
            prg_rom_size: 16 * 1024,
            chr_rom_size: 8 * 1024,
            has_trainer: false,
            mapper: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            is_nes2: false,
            submapper: None,
        };

        let rom_bytes = vec![0xAA; 24 * 1024]; // PRG + CHR
        let file = reconstruct_nes_file(&header, &rom_bytes);

        assert_eq!(file.len(), 16 + rom_bytes.len());
        assert_eq!(&file[0..4], b"NES\x1a");
        assert_eq!(&file[16..], rom_bytes.as_slice());
    }

    #[test]
    fn test_parse_nes_header_from_reader() {
        let header_bytes = make_ines_header(2, 1, 0x00, 0x00);
        let mut cursor = Cursor::new(header_bytes);

        let parsed = parse_nes_header(&mut cursor)
            .expect("Should not error")
            .expect("Should parse valid header");

        assert_eq!(parsed.prg_rom_size, 32 * 1024);
    }

    #[test]
    fn test_skip_trainer_if_present() {
        let header_with_trainer = NesHeader {
            prg_rom_size: 16 * 1024,
            chr_rom_size: 0,
            has_trainer: true,
            mapper: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            is_nes2: false,
            submapper: None,
        };

        let header_without_trainer = NesHeader {
            has_trainer: false,
            ..header_with_trainer.clone()
        };

        // Create a cursor with some data
        let data = vec![0u8; 1024];
        let mut cursor = Cursor::new(data);

        // Skip trainer when present
        skip_trainer_if_present(&mut cursor, &header_with_trainer).unwrap();
        assert_eq!(cursor.position(), 512);

        // Reset and try without trainer
        cursor.set_position(0);
        skip_trainer_if_present(&mut cursor, &header_without_trainer).unwrap();
        assert_eq!(cursor.position(), 0);
    }
}
