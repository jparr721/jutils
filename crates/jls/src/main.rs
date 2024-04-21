use anyhow::Result;
use clap::Parser;
use crossterm::terminal::size;
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

    // If the provided path is not a directory just print the name.
    if !fs::metadata(&path)?.is_dir() {
        return Ok(path);
    }

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
        // Iterate over the terminal width
        let terminal_width = size()?.0 as usize;

        // The max entry width is the longest string in the entry set
        let max_entry_width = entries.iter().map(|entry| entry.len()).max().unwrap_or(0) + 2; // +2 for padding

        // The number of columns is the maximum extent to which we can it values, given the longest entry.
        let columns = terminal_width / max_entry_width;

        // We want the longest columns to be on the left side, so we round up by adding columns - 1.
        let rows = (entries.len() + columns - 1) / columns;

        let mut output = String::new();

        // Now, build the output column by column in alphabetical order (just like coreutils ls does it.)
        for row in 0..rows {
            for column in 0..columns {
                if let Some(entry) = entries.get(row + column * rows) {
                    output += entry;
                    // Add spaces only if we're *not* in the end column.
                    if column != columns - 1 {
                        output += &" ".repeat(max_entry_width - entry.len());
                    }
                }
            }
            output.push('\n');
        }
        Ok(output)
    }
}

fn main() {
    let args = Args::parse();
    let output = ls(args).unwrap();

    println!("{}", output);
}
