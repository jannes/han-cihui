use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::segmentation::BookSegmentation;

const INSERT_BOOK_QUERY: &str = "
INSERT INTO books
(book_name, author_name, book_json)
VALUES (?1, ?2, ?3)";

const SELECT_ALL_BOOKS_QUERY: &str = "
SELECT book_name, author_name, book_json
FROM books";

const DELETE_BOOK_QUERY: &str = "
DELETE FROM books
WHERE book_name = ?1 AND author_name = ?2";

pub fn db_books_select_all(
    data_conn: &Connection,
) -> Result<Vec<(String, String, BookSegmentation)>> {
    let mut stmt = data_conn.prepare(SELECT_ALL_BOOKS_QUERY)?;
    let res = stmt
        .query_map([], |row| {
            let title: String = row.get(0)?;
            let author: String = row.get(1)?;
            let book_json: String = row.get(2)?;
            let book: BookSegmentation =
                serde_json::from_str(&book_json).expect("failed to deserialize book");
            Ok((title, author, book))
        })?
        .collect::<Result<Vec<(String, String, BookSegmentation)>, _>>();
    res.context("sql error when selecting all books")
}

pub fn db_books_insert(
    data_conn: &Connection,
    title: &str,
    author: &str,
    book: &BookSegmentation,
) -> Result<()> {
    let book_json = serde_json::to_string(book).expect("failed to serialize segmented book");
    data_conn.execute(INSERT_BOOK_QUERY, params![title, author, book_json])?;
    Ok(())
}

pub fn db_books_delete(data_conn: &Connection, title: &str, author: &str) -> Result<()> {
    data_conn.execute(DELETE_BOOK_QUERY, params![title, author])?;
    Ok(())
}
