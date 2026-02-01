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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_apply_diff() {
        let temp_dir = tempdir().unwrap();
        let diff_path = temp_dir.path().join("test.bsdiff");

        let old = b"Hello, World!";
        let new = b"Hello, Rust World!";

        // Create diff
        let diff_size = create_diff(old, new, &diff_path).unwrap();
        assert!(diff_size > 0);
        assert!(diff_path.exists());

        // Apply diff
        let result = apply_diff(old, &diff_path).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_diff_identical_files() {
        let temp_dir = tempdir().unwrap();
        let diff_path = temp_dir.path().join("identical.bsdiff");

        let data = b"This is identical content that won't change";

        let diff_size = create_diff(data, data, &diff_path).unwrap();
        // Just verify the diff was created and can be applied
        assert!(diff_size > 0);

        let result = apply_diff(data, &diff_path).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_diff_completely_different() {
        let temp_dir = tempdir().unwrap();
        let diff_path = temp_dir.path().join("different.bsdiff");

        let old = vec![0xAA; 1024];
        let new = vec![0xBB; 1024];

        let diff_size = create_diff(&old, &new, &diff_path).unwrap();
        assert!(diff_size > 0);

        let result = apply_diff(&old, &diff_path).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_diff_empty_to_content() {
        let temp_dir = tempdir().unwrap();
        let diff_path = temp_dir.path().join("empty_to_content.bsdiff");

        let old = b"";
        let new = b"Some new content";

        create_diff(old, new, &diff_path).unwrap();
        let result = apply_diff(old, &diff_path).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_diff_content_to_empty() {
        let temp_dir = tempdir().unwrap();
        let diff_path = temp_dir.path().join("content_to_empty.bsdiff");

        let old = b"Some existing content";
        let new = b"";

        create_diff(old, new, &diff_path).unwrap();
        let result = apply_diff(old, &diff_path).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_diff_large_similar_content() {
        let temp_dir = tempdir().unwrap();
        let diff_path = temp_dir.path().join("large_similar.bsdiff");

        // Create large files with small differences (typical ROM hack scenario)
        let old = vec![0u8; 32 * 1024]; // 32KB
        let mut new = old.clone();

        // Make small changes
        new[100] = 0xFF;
        new[1000] = 0xAB;
        new[10000] = 0xCD;

        let diff_size = create_diff(&old, &new, &diff_path).unwrap();
        assert!(diff_size > 0);

        // Verify the diff applies correctly - this is the important part
        let result = apply_diff(&old, &diff_path).unwrap();
        assert_eq!(result, new);
    }
}
