use std::{collections::HashSet, fs};

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use rusqlite::Connection;

use crate::db::vocab::{
    db_words_add_external, db_words_delete, db_words_select_all, db_words_select_known,
    AddedExternal,
};

pub fn get_arg_matches() -> ArgMatches {
    Command::new("中文 vocab")
        .version("0.1")
        .subcommand(
            Command::new("add")
                .about("Adds known vocabulary from file")
                .arg(
                    Arg::new("filename")
                        .required(true)
                        .help("path to file with one word per line"),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Deletes known vocabulary from file")
                .arg(
                    Arg::new("filename")
                        .required(true)
                        .help("path to file with one word per line"),
                ),
        )
        .subcommand(
            Command::new("add-ignore")
                .about("Adds vocabulary to be ignored from file")
                .arg(
                    Arg::new("filename")
                        .required(true)
                        .help("path to file with one word per line"),
                ),
        )
        .subcommand(Command::new("show").about("Prints known words"))
        .get_matches()
}

pub fn perform_add_external(
    data_conn: &Connection,
    filename: &str,
    kind: AddedExternal,
) -> Result<()> {
    let file_str = fs::read_to_string(filename)?;
    let words_to_add: HashSet<String> = file_str
        .split('\n')
        .map(|line| String::from(line.trim()))
        .filter(|trimmed| !trimmed.is_empty())
        .collect();
    let words_known: HashSet<String> = db_words_select_all(data_conn)?
        .iter()
        .map(|vocab| String::from(&vocab.word))
        .collect();
    let words_unknown: &HashSet<&str> = &words_to_add
        .difference(&words_known)
        .map(|s| s.as_str())
        .collect();
    println!("amount saved: {}", &words_known.len());
    println!("amount to add: {}", &words_to_add.len());
    println!("amount new: {}", &words_unknown.len());
    db_words_add_external(data_conn, words_unknown, kind)
}

pub fn perform_delete_external(data_conn: &Connection, filename: &str) -> Result<()> {
    let file_str = fs::read_to_string(filename)?;
    let words_to_delete: HashSet<String> = file_str
        .split('\n')
        .map(|line| String::from(line.trim()))
        .filter(|trimmed| !trimmed.is_empty())
        .collect();
    println!("amount to delete: {}", &words_to_delete.len());
    db_words_delete(data_conn, &words_to_delete)
}

pub fn show(conn: &Connection) -> Result<()> {
    let known_words = db_words_select_known(conn)?;
    for item in known_words {
        println!("{}", item);
    }
    Ok(())
}
