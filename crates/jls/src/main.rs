use anyhow::Result;
use clap::Parser;
use std::fs;

#[derive(Debug, Parser)]
struct Args {
    /// Show files starting with '.'
    #[clap(short, long, default_value_t = false)]
    all: bool,

    /// Show files in a list format
    #[clap(short, long, default_value_t = false)]
    list: bool,

    /// The path to list
    #[clap(required = true)]
    path: String,
}

fn ls(args: Args) -> Result<String> {
    let path = args.path;

    let mut entries = {
        let all_files = fs::read_dir(path)?
            .into_iter()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap());

        if !args.all {
            all_files
                .filter(|filename| !filename.starts_with('.'))
                .collect::<Vec<String>>()
        } else {
            all_files.collect::<Vec<String>>()
        }
    };

    entries.sort();

    if args.list {
        Ok(entries.join("\n"))
    } else {
        Ok(entries.join(" "))
    }
}

fn main() {
    let args = Args::parse();
    let output = ls(args).unwrap();
    println!("{}", output);
}
