-- Add NES header fields needed for file reconstruction
ALTER TABLE nodes ADD COLUMN mapper INTEGER;
ALTER TABLE nodes ADD COLUMN mirroring INTEGER;
ALTER TABLE nodes ADD COLUMN has_battery INTEGER;
ALTER TABLE nodes ADD COLUMN is_nes2 INTEGER;
ALTER TABLE nodes ADD COLUMN nes2_submapper INTEGER;
