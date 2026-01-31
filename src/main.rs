use clap::Parser;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dromos")]
#[command(about = "ROM image management through binary diffs")]
struct Cli {
    /// File to hash
    file: PathBuf,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let file = File::open(&cli.file)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    println!("{:x}", hash);

    Ok(())
}
