use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use rayon::prelude::*;
use walkdir::WalkDir;

/// The `jfind` command is a streamlined find command. You can simply do
/// `jfind query` and it'll recurisvely search the current directory for files
/// matching that query. Use --help for all flags.
#[derive(Debug, Parser)]
struct Args {
    /// The directory to search within.
    #[clap(short, long, default_value = ".")]
    directory: String,

    /// The depth to search within (default is 10).
    #[clap(long, default_value_t = 10)]
    depth: usize,

    /// Case-sensitive search.
    #[clap(short, long, default_value_t = false)]
    case_sensitive: bool,

    /// The query to search for
    #[clap(default_value = "")]
    query: String,
}

fn check_and_colorize_match(path: &str, query: &str, case_sensitive: bool) -> Option<String> {
    let start = if !case_sensitive {
        path.to_lowercase().find(&query.to_lowercase())
    } else {
        path.find(&query)
    };

    if let Some(start) = start {
        let end = start + query.len();
        Some(format!(
            "{}{}{}",
            &path[..start],
            path[start..end].bright_red(),
            &path[end..]
        ))
    } else {
        None
    }
}

fn find(args: Args) -> Result<String> {
    let directory = args.directory;

    let mut ret = WalkDir::new(directory)
        .max_depth(args.depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .par_bridge()
        .map(|e| e.path().display().to_string())
        .filter_map(|path| check_and_colorize_match(&path, &args.query, args.case_sensitive))
        .collect::<Vec<String>>();

    ret.sort();

    Ok(ret.join("\n"))
}

fn main() {
    let args = Args::parse();
    let output = find(args).unwrap();
    println!("{}", output);
}
