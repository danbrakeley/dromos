ALTER TABLE nodes ADD COLUMN title TEXT;
UPDATE nodes SET title = filename WHERE title IS NULL;
