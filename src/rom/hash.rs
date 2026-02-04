use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{DromosError, Result};
use crate::rom::nes::{parse_nes_header_bytes, skip_trainer_if_present};
use crate::rom::types::{RomMetadata, RomType};

/// Hash bytes directly using SHA-256. Pure function for testability.
pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn detect_rom_type(path: &Path) -> Option<RomType> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "nes" => Some(RomType::Nes),
        _ => None,
    }
}

fn hash_remaining(reader: &mut impl Read) -> Result<[u8; 32]> {
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
            // Read raw header bytes first
            let mut header_bytes = [0u8; 16];
            reader.read_exact(&mut header_bytes)?;

            match parse_nes_header_bytes(&header_bytes) {
                Some(header) => {
                    skip_trainer_if_present(&mut reader, &header)?;
                    let sha256 = hash_remaining(&mut reader)?;

                    Ok(RomMetadata {
                        rom_type: RomType::Nes,
                        sha256,
                        filename,
                        nes_header: Some(header),
                        source_file_header: Some(header_bytes.to_vec()),
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
        Some(RomType::Nes) => {
            // Read raw header bytes
            let mut header_bytes = [0u8; 16];
            reader.read_exact(&mut header_bytes)?;

            match parse_nes_header_bytes(&header_bytes) {
                Some(header) => {
                    skip_trainer_if_present(&mut reader, &header)?;
                    let mut bytes = Vec::new();
                    reader.read_to_end(&mut bytes)?;
                    Ok(bytes)
                }
                None => Err(DromosError::InvalidNesFile {
                    path: path.to_path_buf(),
                }),
            }
        }
        None => {
            // For unknown types, read the whole file
            reader.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            Ok(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes_deterministic() {
        let data = b"Hello, World!";
        let hash1 = hash_bytes(data);
        let hash2 = hash_bytes(data);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_bytes_different_input() {
        let data1 = b"Hello, World!";
        let data2 = b"Hello, World?";

        let hash1 = hash_bytes(data1);
        let hash2 = hash_bytes(data2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_bytes_known_value() {
        // Known SHA-256 hash for empty input
        let empty_hash = hash_bytes(b"");
        // SHA-256 of empty string is e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            format_hash(&empty_hash),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_format_hash() {
        let mut hash = [0u8; 32];
        hash[0] = 0xAB;
        hash[1] = 0xCD;
        hash[31] = 0xEF;

        let formatted = format_hash(&hash);
        assert_eq!(formatted.len(), 64);
        assert!(formatted.starts_with("abcd"));
        assert!(formatted.ends_with("ef"));
    }

    #[test]
    fn test_parse_hash_valid() {
        let hex_str = "abcd0000000000000000000000000000000000000000000000000000000000ef";
        let parsed = parse_hash(hex_str).expect("Should parse valid hash");

        assert_eq!(parsed[0], 0xAB);
        assert_eq!(parsed[1], 0xCD);
        assert_eq!(parsed[31], 0xEF);
    }

    #[test]
    fn test_parse_hash_invalid_length() {
        assert!(parse_hash("abc").is_none());
        assert!(parse_hash("").is_none());
        assert!(
            parse_hash("abcd00000000000000000000000000000000000000000000000000000000000").is_none()
        ); // 63 chars
        assert!(
            parse_hash("abcd000000000000000000000000000000000000000000000000000000000000f")
                .is_none()
        ); // 65 chars
    }

    #[test]
    fn test_parse_hash_invalid_chars() {
        // Contains 'g' which is not valid hex
        assert!(
            parse_hash("ghij0000000000000000000000000000000000000000000000000000000000ef")
                .is_none()
        );
    }

    #[test]
    fn test_format_parse_round_trip() {
        let mut original = [0u8; 32];
        for i in 0..32 {
            original[i] = i as u8;
        }

        let formatted = format_hash(&original);
        let parsed = parse_hash(&formatted).expect("Should parse formatted hash");

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_detect_rom_type() {
        use std::path::Path;

        assert_eq!(detect_rom_type(Path::new("game.nes")), Some(RomType::Nes));
        assert_eq!(detect_rom_type(Path::new("game.NES")), Some(RomType::Nes));
        assert_eq!(detect_rom_type(Path::new("game.Nes")), Some(RomType::Nes));
        assert_eq!(detect_rom_type(Path::new("game.snes")), None);
        assert_eq!(detect_rom_type(Path::new("game")), None);
    }
}
