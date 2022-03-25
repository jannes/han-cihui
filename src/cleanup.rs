use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::Result;
use han_cihui::{
    config::get_data_dir,
    db::vocab::{db_words_select_all, Vocab},
    extraction::contains_hanzi,
};
use rusqlite::Connection;

pub fn main() -> Result<()> {
    let data_dir = PathBuf::from(get_data_dir());
    let db_path: PathBuf = [data_dir.as_path(), Path::new("data.db")].iter().collect();
    let data_conn = Connection::open(db_path)?;
    let vocabs = db_words_select_all(&data_conn)?;
    let amount_before = vocabs.len();

    let mut n_cleaned_single = 0;
    let mut n_garbage = 0;
    let mut n_cleaned_mult = 0;

    let mut to_delete: HashSet<String> = HashSet::new();
    let mut to_add: HashSet<Vocab> = HashSet::new();

    for vocab in vocabs {
        let split: Vec<&str> = vocab
            .word
            .split(|c: char| !contains_hanzi(&c.to_string()) && c != '，')
            .filter(|w| !w.is_empty())
            .collect();

        if split.is_empty() {
            to_delete.insert(vocab.word.clone());
            n_garbage += 1;
        } else if split.len() == 1 && split[0].len() < vocab.word.len() {
            to_delete.insert(vocab.word.clone());
            n_cleaned_single += 1;
            to_add.insert(Vocab {
                word: split[0].to_string(),
                status: vocab.status,
            });
        } else if split.len() > 1 {
            to_delete.insert(vocab.word.clone());
            n_cleaned_mult += 1;
            for word in split {
                to_add.insert(Vocab {
                    word: word.to_string(),
                    status: vocab.status,
                });
            }
        }
    }

    let amount_after = amount_before + to_add.len() - to_delete.len();

    println!("{} before", amount_before);
    println!("{} after", amount_after);
    println!("{} single cleaned", n_cleaned_single);
    println!("{} multi cleaned", n_cleaned_mult);
    println!("{} deleted", n_garbage);
    println!("===== ADDING ======");
    for v in to_add {
        println!("{}", v.word);
    }
    println!("===== DELETING ======");
    for v in to_delete {
        println!("{}", v);
    }
    Ok(())
}
