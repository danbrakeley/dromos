use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::error::{DromosError, Result};

pub fn create_diff(old: &[u8], new: &[u8], diff_path: &Path) -> Result<u64> {
    let mut patch = Vec::new();
    bsdiff::diff(old, new, &mut patch).map_err(|e| DromosError::DiffCreation(e.to_string()))?;

    let file = File::create(diff_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&patch)?;
    writer.flush()?;

    Ok(patch.len() as u64)
}

pub fn apply_diff(old: &[u8], diff_path: &Path) -> Result<Vec<u8>> {
    let file = File::open(diff_path)?;
    let mut reader = BufReader::new(file);
    let mut patch = Vec::new();
    reader.read_to_end(&mut patch)?;

    let mut new = Vec::new();
    bsdiff::patch(old, &mut patch.as_slice(), &mut new)
        .map_err(|e| DromosError::DiffApplication(e.to_string()))?;

    Ok(new)
}
