extern crate rusqlite;
extern crate serde_json;

use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

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

/// Whether the note is active (i.e one of its cards is) or suspended (all cards are suspended)
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NoteStatus {
    Active,
    Inactive,
}

impl From<i64> for NoteStatus {
    fn from(value: i64) -> Self {
        // Anki card's queue field value meanings:
        // -- -3=user buried(In scheduler 2),
        // -- -2=sched buried (In scheduler 2),
        // -- -2=buried(In scheduler 1),
        // -- -1=suspended,
        // -- 0=new, 1=learning, 2=review (as for type)
        // -- 3=in learning, next rev in at least a day after the previous review
        // -- 4=preview
        if value > 0 {
            Self::Active
        } else {
            Self::Inactive
        }
    }
}

/// An Anki note, its text content and some metadata
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Note {
    /// All fields appended into one string
    pub fields_raw: String,
    /// Status of the note, which impacts status of contained words
    pub status: NoteStatus,
    /// When was the last modification/(un)suspension of the note or one of its cards?
    pub last_modified: i64,
}

/// Synchronize local vocabulary data with current Anki state
///
/// Reads all relevant note data from Anki, recomputes Anki-based vocabulary state
/// and updates the vocabulary that has changed state since last sync with Anki
pub fn db_sync_anki_data(data_conn: &mut Connection) -> Result<()> {
    let start_extract = Instant::now();
    let Config {
        anki_db_path,
        anki_notes,
        ..
    } = get_config();

    let conn =
        Connection::open_with_flags(anki_db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;

    // get latest modification time from previous sync and all note data from Anki DB
    let current_latest_mod = select_max_modified(&data_conn)?;
    let all_notes = get_zh_notes(&conn, anki_notes).context("failed to select notes")?;

    let jieba = Jieba::new();
    let fan2jian = get_mapping(true);
    let jian2fan = get_mapping(false);
    let mut all_vocab: HashMap<String, VocabStatus> = HashMap::new();
    let mut mod_words: HashSet<String> = HashSet::new();
    let mut new_latest_mod = 0;

    // extract words from each note and construct vocab
    // any word that is both active and inactive counts as active
    // record all words that are contained on recently modified notes
    for note in all_notes {
        new_latest_mod = std::cmp::max(new_latest_mod, note.last_modified);
        let words = extract_words(&note.fields_raw, &jieba, &fan2jian, &jian2fan);
        // if note is recently modified, record all its words in the modified set
        if note.last_modified > current_latest_mod {
            for word in &words {
                mod_words.insert(word.clone());
            }
        }
        let vocab_status = VocabStatus::from(note.status);
        // record & update word statuses
        for word in words {
            all_vocab
                .entry(word)
                .and_modify(|status| {
                    // only ever update words that are currently marked inactive
                    // to ensure any word that appears in active and inactive notes is marked active
                    if matches!(status, VocabStatus::Inactive) {
                        *status = vocab_status;
                    }
                })
                .or_insert(vocab_status);
        }
    }

    eprintln!("current latest mod: {current_latest_mod}");
    eprintln!("new latest mod: {new_latest_mod}");

    let duration = start_extract.elapsed();

    let mod_vocab: HashMap<String, VocabStatus> = all_vocab
        .into_iter()
        .filter(|(word, _)| mod_words.contains(word))
        .collect();

    eprintln!("anki sync extraction duration: {duration:#?}");
    eprintln!("modified vocab len: {}", mod_vocab.len());

    if mod_vocab.len() > 0 {
        let start_insert = Instant::now();
        db_words_insert_overwrite(data_conn, &mod_vocab, Some(new_latest_mod))?;
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

// get (note id, note fields, maximum modfification of note and its active cards, max queue) tuples
// selecting max queue means obtaining the more "active" status as positive values identify active
// cards and negative values identify inactive cards
//
const SELECT_NOTES: &str =
    "SELECT n.id, n.flds, MAX(COALESCE(n.mod, 0), COALESCE(c.mod, 0)) AS max_mod, MAX(c.queue) AS queue \
     FROM notes n JOIN cards c ON n.id = c.nid \
     WHERE n.mid = ?1 \
     GROUP BY n.id, n.flds";

const SELECT_NOTETYPES_SQL: &str = "SELECT notetypes.id, notetypes.name FROM notetypes";

fn get_zh_notes(conn: &Connection, notetypes: Vec<String>) -> Result<Vec<Note>> {
    let notetypes = get_zh_notetypes(conn, notetypes)?;
    let mut all_notes: Vec<Note> = Vec::new();
    for Notetype {
        id: notetype_id, ..
    } in notetypes
    {
        all_notes.extend(select_notes(conn, notetype_id)?);
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

fn select_notes(conn: &Connection, notetype_id: i64) -> Result<Vec<Note>, rusqlite::Error> {
    let params = params![notetype_id];
    let res = conn
        .prepare(SELECT_NOTES)?
        .query_map(params, |row| {
            let status: i64 = row.get(3)?;
            Ok(Note {
                fields_raw: row.get(1)?,
                status: status.into(),
                last_modified: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<Note>, _>>();
    res
}
