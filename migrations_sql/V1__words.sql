CREATE TABLE IF NOT EXISTS words (
    word text primary key, 
    status integer not null
);
CREATE INDEX IF NOT EXISTS word_index ON words(word);