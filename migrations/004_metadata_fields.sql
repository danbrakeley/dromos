-- Add metadata fields for ROM nodes
ALTER TABLE nodes ADD COLUMN source_url TEXT;
ALTER TABLE nodes ADD COLUMN version TEXT;
ALTER TABLE nodes ADD COLUMN release_date TEXT;
ALTER TABLE nodes ADD COLUMN tags TEXT;
ALTER TABLE nodes ADD COLUMN description TEXT;
