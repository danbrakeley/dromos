use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use crate::error::Result;
use crate::rom::types::{Mirroring, NesHeader};

pub fn parse_nes_header(reader: &mut BufReader<File>) -> Result<Option<NesHeader>> {
    let mut header = [0u8; 16];
    reader.read_exact(&mut header)?;

    // Check for "NES\x1A" magic bytes
    if &header[0..4] != b"NES\x1a" {
        return Ok(None);
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

    Ok(Some(NesHeader {
        prg_rom_size,
        chr_rom_size,
        has_trainer,
        mapper,
        mirroring,
        has_battery,
        is_nes2,
        submapper,
    }))
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

pub fn skip_trainer_if_present(reader: &mut BufReader<File>, header: &NesHeader) -> Result<()> {
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
