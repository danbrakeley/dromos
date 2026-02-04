-- Add raw file header storage for byte-identical ROM reconstruction
ALTER TABLE nodes ADD COLUMN source_file_header BLOB;
