CREATE TABLE word_lists (
    id integer primary key autoincrement,
    book_name text not null,
    author_name text not null,
    create_time integer not null,
    min_occurrence_words integer not null,
    min_occurrence_chars integer,
    word_list_json text not null
);
CREATE INDEX book_name_index ON word_lists(book_name);
CREATE INDEX create_time_index ON word_lists(create_time);
