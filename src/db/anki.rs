extern crate rusqlite;
extern crate serde_json;

use std::{collections::HashMap, time::Instant};

use anyhow::{Context, Result};
use jieba_rs::Jieba;
use rusqlite::{params, Connection};

use crate::{
    config::{get_config, Config},
    db::vocab::select_max_modified,
    fan2jian::get_mapping,
    segmentation::extract_words,
};

use super::vocab::{db_words_insert_overwrite, VocabStatus};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NoteStatus {
    Active,
    Suspended,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Note {
    pub fields_raw: String,
    pub status: NoteStatus,
    pub modified_timestamp: i64,
}

pub fn db_sync_anki_data(data_conn: &mut Connection) -> Result<()> {
    let start_extract = Instant::now();
    let Config {
        anki_db_path,
        anki_notes,
        ..
    } = get_config();

    let conn =
        Connection::open_with_flags(anki_db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    // let max_mod = select_max_modified(&conn).context("failed to select maximum latest modified")?;
    let current_latest_mod = select_max_modified(&data_conn)?;
    let zh_notes =
        get_zh_notes(&conn, anki_notes, current_latest_mod).context("failed to select notes")?;

    // append notes into big long text for each status type
    let mut text_active = String::new();
    let mut text_suspended = String::new();
    let mut new_latest_mod = 0;
    for note in zh_notes {
        new_latest_mod = std::cmp::max(new_latest_mod, note.modified_timestamp);
        match note.status {
            NoteStatus::Active => text_active.push_str(&note.fields_raw),
            NoteStatus::Suspended => text_suspended.push_str(&note.fields_raw),
        }
    }

    eprintln!("current latest mod: {current_latest_mod}");
    eprintln!("new latest mod: {new_latest_mod}");

    // extract words from each big text and construct vocab
    // any word that is both active and inactive counts as active
    let jieba = Jieba::new();
    let fan2jian = get_mapping(true);
    let jian2fan = get_mapping(false);
    let mut vocab: HashMap<String, VocabStatus> = HashMap::new();

    for word in extract_words(&text_suspended, &jieba, &fan2jian, &jian2fan) {
        vocab.insert(word, VocabStatus::Inactive);
    }
    for word in extract_words(&text_active, &jieba, &fan2jian, &jian2fan) {
        vocab.insert(word, VocabStatus::Active);
    }
    let duration = start_extract.elapsed();
    eprintln!("anki sync extraction duration: {duration:#?}");
    eprintln!("vocab len: {}", vocab.len());

    if vocab.len() > 0 {
        let start_insert = Instant::now();
        db_words_insert_overwrite(data_conn, &vocab, Some(new_latest_mod))?;
        let duration = start_insert.elapsed();
        eprintln!("anki sync insert duration: {duration:#?}");
    }
    Ok(())
}

/**
-------------- PRIVATE ----------------
*/

#[derive(Debug)]
struct Notetype {
    id: i64,
    name: String,
}

// get (note id, note fields, maximum modfification of note and its active cards) tuples
// if any of a note's card is active, it is included here
const SELECT_ACTIVE_SQL: &str =
    "SELECT n.id, n.flds, MAX(COALESCE(n.mod, 0), COALESCE(c.mod, 0)) AS max_mod \
     FROM notes n LEFT JOIN cards c ON n.id = c.nid \
     WHERE n.mid = ?1 \
     AND c.queue != -1 \
     GROUP BY n.id, n.flds \
     HAVING max_mod > ?2";

// get (note id, note fields, maximum modfification of note and its inactive cards) tuples
// if any of a note's card is inactive, it is included here
const SELECT_INACTIVE_SQL: &str =
    "SELECT n.id, n.flds, MAX(COALESCE(n.mod, 0), COALESCE(c.mod, 0)) AS max_mod \
     FROM notes n LEFT JOIN cards c ON n.id = c.nid \
     WHERE n.mid = ?1 \
     AND c.queue == -1 \
     GROUP BY n.id, n.flds \
     HAVING max_mod > ?2";

const SELECT_NOTETYPES_SQL: &str = "SELECT notetypes.id, notetypes.name FROM notetypes";

fn get_zh_notes(conn: &Connection, notetypes: Vec<String>, min_modified: i64) -> Result<Vec<Note>> {
    let notetypes = get_zh_notetypes(conn, notetypes)?;
    let mut all_notes: Vec<Note> = Vec::new();
    for Notetype {
        id: notetype_id, ..
    } in notetypes
    {
        all_notes.extend(select_notes(
            conn,
            notetype_id,
            NoteStatus::Active,
            min_modified,
        )?);
        all_notes.extend(select_notes(
            conn,
            notetype_id,
            NoteStatus::Suspended,
            min_modified,
        )?);
    }
    Ok(all_notes)
}

fn get_zh_notetypes(conn: &Connection, zh_notetype_names: Vec<String>) -> Result<Vec<Notetype>> {
    let mut notetypes_query = conn.prepare(SELECT_NOTETYPES_SQL)?;
    let all_notetypes = notetypes_query.query_map(params![], |row| {
        Ok(Notetype {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    let res: Result<_, _> = all_notetypes
        .filter(|nt| {
            if nt.is_ok() {
                zh_notetype_names.contains(&nt.as_ref().unwrap().name)
            } else {
                false
            }
        })
        .collect();
    res.context("failed to select notetypes")
}

fn select_notes(
    conn: &Connection,
    notetype_id: i64,
    status: NoteStatus,
    min_modified: i64,
) -> Result<Vec<Note>, rusqlite::Error> {
    let params = params![notetype_id, min_modified];
    let mut stmt = match status {
        NoteStatus::Active => conn.prepare(SELECT_ACTIVE_SQL)?,
        NoteStatus::Suspended => conn.prepare(SELECT_INACTIVE_SQL)?,
    };
    let res = stmt
        .query_map(params, |row| {
            Ok(Note {
                fields_raw: row.get(1)?,
                status,
                modified_timestamp: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<Note>, _>>();
    res
}
