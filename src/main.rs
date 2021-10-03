use clap::{App, Arg, Shell, SubCommand};
use serde_json::json;
use std::env;
use std::io;
use std::path::PathBuf;
use xdg;

mod indexer;
use indexer::{Hash, SqliteIndex};

const PROG_NAME: &'static str = "rindexer";

fn xdg_path() -> String {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(PROG_NAME).unwrap();
    let db_path = xdg_dirs
        .place_data_file("db.sqlite")
        .expect("cannot create configuration directory");

    db_path.to_str().unwrap().to_owned()
}

fn app() -> App<'static, 'static> {
    App::new(PROG_NAME)
        .version("1.0")
        .author("Benjamin Gentil <benjamin@gentil.io>")
        .about("Index files in a sqlite database")
        .arg(
            Arg::with_name("database")
                .short("d")
                .long("database")
                .value_name("DATABASE")
                .help("Define the database path")
                .takes_value(true)
                .default_value("<xdg_path>"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("run indexation")
                .arg(
                    Arg::with_name("path")
                        .short("p")
                        .long("path")
                        .value_name("PATH")
                        .help("Define the path to index")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("algorithm")
                        .short("a")
                        .long("algorithm")
                        .value_name("HASH")
                        .help("Define the hash algorithm to use")
                        .takes_value(true)
                        .possible_values(&["md5", "sha1", "sha256"])
                        .default_value("sha256"),
                )
                .arg(
                    Arg::with_name("hash_method")
                        .short("m")
                        .long("method")
                        .value_name("HASH_METHOD")
                        .help("Define the hash method to use")
                        .takes_value(true)
                        .possible_values(&["native", "external"])
                        .default_value("external"),
                )
                .arg(
                    Arg::with_name("recursive")
                        .short("r")
                        .help("Index subfolders"),
                )
                .arg(
                    Arg::with_name("force")
                        .short("f")
                        .help("Index even if the modification date hasn't changed"),
                )
                .arg(
                    Arg::with_name("nodelete")
                        .short("D")
                        .help("Keep deleted files in index DB"),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("list indexed files")
                .subcommand(SubCommand::with_name("duplicates").about("list duplicates files")),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("search indexed file by name")
                .arg(Arg::with_name("NAME").required(true)),
        )
        .subcommand(
            SubCommand::with_name("completion")
                .about("generates shells completion")
                .subcommand(SubCommand::with_name("fish").about("generates fish completion"))
                .subcommand(SubCommand::with_name("bash").about("generates bash completion"))
                .subcommand(SubCommand::with_name("zsh").about("generates zsh completion")),
        )
}

fn main() -> Result<(), ()> {
    let matches = app().get_matches();

    let db_path = match matches.value_of("database") {
        Some("<xdg_path>") => xdg_path(),
        Some(path) => path.to_owned(),
        None => panic!("Unexpected empty database path"),
    };

    let mut index = SqliteIndex::new(db_path);

    match matches.subcommand() {
        ("run", Some(run_matches)) => {
            let path = match run_matches.value_of("path") {
                Some(p) => PathBuf::from(p),
                _ => env::current_dir().unwrap(),
            };
            let hash = Hash::new(
                run_matches.value_of("algorithm").unwrap(),
                run_matches.value_of("hash_method").unwrap(),
            );
            index.index(
                path,
                hash,
                run_matches.is_present("recursive"),
                run_matches.is_present("force"),
            )?;
        }
        ("list", Some(list_matches)) => match list_matches.subcommand() {
            ("duplicates", Some(_)) => {
                println!(
                    "{}",
                    json!({
                        "duplicates": index.list_duplicates().unwrap()
                    })
                    .to_string(),
                );
            }
            ("", None) => {
                println!(
                    "{}",
                    json!({
                        "list": index.list(None).unwrap()
                    })
                    .to_string(),
                );
            }
            _ => panic!("Unexpected subcommand"),
        },
        ("search", Some(search_matches)) => {
            let mut name = "%".to_string();
            name += search_matches.value_of("NAME").unwrap();

            println!(
                "{}",
                json!({
                    "results": index.list(Some(name.as_str())).unwrap()
                })
                .to_string(),
            );
        }
        ("completion", Some(completion_matches)) => match completion_matches.subcommand() {
            ("fish", Some(_)) => {
                app().gen_completions_to(PROG_NAME, Shell::Fish, &mut io::stdout());
            }
            ("bash", Some(_)) => {
                app().gen_completions_to(PROG_NAME, Shell::Bash, &mut io::stdout());
            }
            ("zsh", Some(_)) => {
                app().gen_completions_to(PROG_NAME, Shell::Zsh, &mut io::stdout());
            }
            ("", None) => {
                println!("Please specify a shell");
            }
            _ => panic!("Unexpected shell"),
        },
        ("", None) => {}
        _ => panic!("Unexpected subcommand"),
    }

    Ok(())
}
