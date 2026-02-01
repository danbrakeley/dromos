use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{DromosError, Result};
use crate::rom::nes::{parse_nes_header, skip_trainer_if_present};
use crate::rom::types::{RomMetadata, RomType};

fn detect_rom_type(path: &Path) -> Option<RomType> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "nes" => Some(RomType::Nes),
        _ => None,
    }
}

fn hash_remaining(reader: &mut BufReader<File>) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().into())
}

pub fn hash_rom_file(path: &Path) -> Result<RomMetadata> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let filename = path.file_name().map(|s| s.to_string_lossy().into_owned());

    match detect_rom_type(path) {
        Some(RomType::Nes) => {
            match parse_nes_header(&mut reader)? {
                Some(header) => {
                    skip_trainer_if_present(&mut reader, &header)?;
                    let sha256 = hash_remaining(&mut reader)?;

                    Ok(RomMetadata {
                        rom_type: RomType::Nes,
                        sha256,
                        filename,
                        nes_header: Some(header),
                    })
                }
                None => {
                    // Not a valid NES file despite extension
                    Err(DromosError::InvalidNesFile {
                        path: path.to_path_buf(),
                    })
                }
            }
        }
        None => {
            let extension = path
                .extension()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "none".to_string());
            Err(DromosError::UnsupportedRomType { extension })
        }
    }
}

pub fn format_hash(hash: &[u8; 32]) -> String {
    hex::encode(hash)
}

pub fn parse_hash(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let bytes = hex::decode(s).ok()?;
    bytes.try_into().ok()
}

pub fn read_rom_bytes(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    match detect_rom_type(path) {
        Some(RomType::Nes) => match parse_nes_header(&mut reader)? {
            Some(header) => {
                skip_trainer_if_present(&mut reader, &header)?;
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes)?;
                Ok(bytes)
            }
            None => Err(DromosError::InvalidNesFile {
                path: path.to_path_buf(),
            }),
        },
        None => {
            // For unknown types, read the whole file
            reader.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            Ok(bytes)
        }
    }
}
