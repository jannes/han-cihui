CREATE TABLE books (
    book_name text not null,
    author_name text not null,
    book_json text not null,
    PRIMARY KEY ( book_name, author_name )
);
