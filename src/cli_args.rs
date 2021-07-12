use clap::{App, Arg, ArgMatches, SubCommand};

pub fn get_arg_matches() -> ArgMatches<'static> {
    App::new("中文 vocab")
        .version("0.1")
        .subcommand(
            SubCommand::with_name("add")
                .about("Adds known vocabulary from file")
                .arg(
                    Arg::with_name("filename")
                        .required(true)
                        .help("path to file with one word per line"),
                ),
        )
        .subcommand(
            SubCommand::with_name("add-ignore")
                .about("Adds vocabulary to be ignored from file")
                .arg(
                    Arg::with_name("filename")
                        .required(true)
                        .help("path to file with one word per line"),
                ),
        )
        .subcommand(SubCommand::with_name("sync").about("Syncs data with Anki"))
        .subcommand(SubCommand::with_name("anki-stats").about("Prints anki statistics"))
        .subcommand(
            SubCommand::with_name("analyze")
                .about("Analyze vocabulary of epub")
                .arg(
                    Arg::with_name("filename")
                        .required(true)
                        .help("path to epub file"),
                )
                .arg(
                    Arg::with_name("dict-only")
                        .required(false)
                        .short("d")
                        .long("dict-only")
                        .help("segmentation mode: dict-only"),
                ),
        )
        .subcommand(
            SubCommand::with_name("show")
                .about("Prints vocabulary items (known words by default)")
                .arg(
                    Arg::with_name("status")
                        .takes_value(true)
                        .required(false)
                        .short("s")
                        .long("status")
                        .help(
                            "status of vocab items, one of 'known_external', 'suspended_unknown'",
                        ),
                )
                .arg(
                    Arg::with_name("kind")
                        .takes_value(true)
                        .required(false)
                        .short("k")
                        .long("kind")
                        .help("one of 'words', 'chars'"),
                ),
        )
        .get_matches()
}
