use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use crate::error::Result;
use crate::rom::types::NesHeader;

pub fn parse_nes_header(reader: &mut BufReader<File>) -> Result<Option<NesHeader>> {
    let mut header = [0u8; 16];
    reader.read_exact(&mut header)?;

    // Check for "NES\x1A" magic bytes
    if &header[0..4] != b"NES\x1a" {
        return Ok(None);
    }

    let prg_rom_size = header[4] as usize * 16 * 1024; // 16 KB units
    let chr_rom_size = header[5] as usize * 8 * 1024; // 8 KB units
    let has_trainer = (header[6] & 0x04) != 0;

    Ok(Some(NesHeader {
        prg_rom_size,
        chr_rom_size,
        has_trainer,
    }))
}

pub fn skip_trainer_if_present(reader: &mut BufReader<File>, header: &NesHeader) -> Result<()> {
    if header.has_trainer {
        reader.seek(SeekFrom::Current(512))?;
    }
    Ok(())
}
