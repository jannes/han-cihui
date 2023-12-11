-- vocabulary words that are manually added
CREATE TABLE words_external (
    word text primary key
);
CREATE INDEX words_external_index ON words_external(word);

-- vocabulary words that are extracted from Anki
CREATE TABLE words_anki (
    word text primary key, 
    status integer not null
);
CREATE INDEX words_anki_index ON words_anki(word);

-- books that are saved for analysis/creation of word lists
CREATE TABLE books (
    book_name text not null,
    author_name text not null,
    book_json text not null,
    PRIMARY KEY ( book_name, author_name )
);

-- word lists created from book analysis
CREATE TABLE word_lists (
    id integer primary key,
    book_name text not null,
    author_name text not null,
    create_time integer not null,
    min_occurrence_words integer not null,
    min_occurrence_chars integer,
    word_list_json text not null
);
CREATE INDEX book_name_index ON word_lists(book_name);
CREATE INDEX create_time_index ON word_lists(create_time);
