use clap::Parser;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "dromos")]
#[command(about = "ROM image management through binary diffs")]
struct Cli {
    /// File to hash
    file: PathBuf,
}

#[derive(Debug)]
enum RomType {
    Nes,
}

#[derive(Debug)]
struct NesHeader {
    prg_rom_size: usize,
    chr_rom_size: usize,
    has_trainer: bool,
}

fn detect_rom_type(path: &Path) -> Option<RomType> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "nes" => Some(RomType::Nes),
        _ => None,
    }
}

fn parse_nes_header(reader: &mut BufReader<File>) -> std::io::Result<Option<NesHeader>> {
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

fn hash_remaining(reader: &mut BufReader<File>) -> std::io::Result<[u8; 32]> {
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

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let file = File::open(&cli.file)?;
    let mut reader = BufReader::new(file);

    let hash = match detect_rom_type(&cli.file) {
        Some(RomType::Nes) => {
            match parse_nes_header(&mut reader)? {
                Some(header) => {
                    // Skip trainer if present
                    if header.has_trainer {
                        reader.seek(SeekFrom::Current(512))?;
                    }

                    let hash = hash_remaining(&mut reader)?;

                    println!("Hash: {:x}", sha2::digest::generic_array::GenericArray::from(hash));
                    println!("Type: NES");
                    println!("PRG ROM: {} KB", header.prg_rom_size / 1024);
                    println!("CHR ROM: {} KB", header.chr_rom_size / 1024);
                    println!("Trainer: {}", if header.has_trainer { "Yes" } else { "No" });

                    return Ok(());
                }
                None => {
                    // Not a valid NES file despite extension, hash full file
                    reader.seek(SeekFrom::Start(0))?;
                    hash_remaining(&mut reader)?
                }
            }
        }
        None => hash_remaining(&mut reader)?,
    };

    println!("{:x}", sha2::digest::generic_array::GenericArray::from(hash));

    Ok(())
}
