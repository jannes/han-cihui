use std::time::{Duration, SystemTime};

use anyhow::Result;

use rusqlite::{params, Connection};

use crate::{
    analysis::AnalysisQuery,
    word_lists::{ChapterWords, WordList, WordListMetadata},
};

const INSERT_WORD_LIST_QUERY: &str = "
INSERT INTO word_lists
(book_name, author_name, create_time, min_occurrence_words, min_occurrence_chars, word_list_json)
VALUES (?1, ?2, strftime('%s', 'now'), ?3, ?4, ?5)";

const SELECT_ALL_WORD_LISTS_QUERY: &str = "
SELECT id, book_name, author_name, create_time, min_occurrence_words, min_occurrence_chars
FROM word_lists";

const SELECT_WORD_LIST_QUERY: &str = "
SELECT (word_list_json)
FROM word_lists WHERE id = ?1";

pub fn db_wlist_insert(conn: &Connection, word_list: WordList) -> Result<()> {
    let book_name = word_list.metadata.book_name;
    let author_name = word_list.metadata.author_name;
    let min_occ_words = word_list.metadata.analysis_query.min_occurrence_words;
    let min_occ_chars = word_list
        .metadata
        .analysis_query
        .min_occurrence_unknown_chars;
    let word_list_json = serde_json::to_string(&word_list.words_per_chapter)
        .expect("failed to serialize words per chapter lists");
    conn.execute(
        INSERT_WORD_LIST_QUERY,
        params![
            book_name,
            author_name,
            min_occ_words,
            min_occ_chars,
            word_list_json
        ],
    )?;
    Ok(())
}

pub fn db_wlist_select_all_mdata(conn: &Connection) -> Result<Vec<WordListMetadata>> {
    let mut query = conn.prepare(SELECT_ALL_WORD_LISTS_QUERY)?;
    let res = query
        .query_map([], |row| {
            let create_time = SystemTime::UNIX_EPOCH
                .checked_add(Duration::from_secs(row.get(3)?))
                .expect("system time should not be out of bounds");
            let min_occurrence_words = row.get(4)?;
            let min_occurrence_unknown_chars = row.get(5)?;
            let analysis_query = AnalysisQuery {
                min_occurrence_words,
                min_occurrence_unknown_chars,
            };
            Ok(WordListMetadata {
                id: row.get(0)?,
                book_name: row.get(1)?,
                author_name: row.get(2)?,
                create_time,
                analysis_query,
            })
        })?
        .collect::<Result<Vec<WordListMetadata>, _>>()?;
    Ok(res)
}

pub fn db_wlist_select_by_id(
    conn: &Connection,
    word_list_id: u64,
) -> Result<Option<Vec<ChapterWords>>> {
    let mut query = conn.prepare(SELECT_WORD_LIST_QUERY)?;
    let res = query
        .query_map([word_list_id], |row| {
            let words_per_chapter_json: String = row.get(0)?;
            let words_per_chapter: Vec<ChapterWords> =
                serde_json::from_str(&words_per_chapter_json)
                    .expect("failed to deserialize words per chapter lists");
            Ok(words_per_chapter)
        })?
        .next()
        .transpose()?;
    Ok(res)
}
