use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use ignore::gitignore::GitignoreBuilder;
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

    /// The depth to search within.
    #[clap(long, default_value_t = 10)]
    depth: usize,

    /// Case-sensitive search.
    #[clap(short, long, default_value_t = false)]
    case_sensitive: bool,

    /// Whether or not to ignore files in .gitignore
    #[clap(short, long, default_value_t = false)]
    ignore_gitingore: bool,

    /// The query to search for
    query: String,
}

fn check_and_colorize_match(path: &str, query: &str, case_sensitive: bool) -> Option<String> {
    let start = if !case_sensitive {
        path.to_lowercase().find(&query.to_lowercase())
    } else {
        path.find(query)
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

    let mut ignore_builder = GitignoreBuilder::new(&directory);
    if args.ignore_gitingore {
        if let Some(e) = ignore_builder.add(format!("{}/.gitignore", directory)) {
            bail!("Error parsing .gitignore: {e}");
        }
    }

    let gitignore = ignore_builder.build()?;

    let mut ret = WalkDir::new(directory)
        .max_depth(args.depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .par_bridge()
        .map(|e| e.path().display().to_string())
        .filter_map(|path| {
            if gitignore.matched(&path, false).is_ignore() {
                return None;
            }

            check_and_colorize_match(&path, &args.query, args.case_sensitive)
        })
        .collect::<Vec<String>>();

    ret.sort();

    Ok(ret.join("\n"))
}

fn main() {
    let args = Args::parse();
    let output = find(args).unwrap();
    println!("{}", output);
}
