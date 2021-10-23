ALTER TABLE words 
ADD COLUMN last_changed integer;

UPDATE words
SET last_changed = strftime('%s','now');