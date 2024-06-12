use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::Parser;
use crossterm::terminal::size;
use std::{fs, os::unix::fs::MetadataExt};
use users::{get_group_by_gid, get_user_by_uid};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
const SIZES: [&str; 6] = ["", "K", "M", "G", "T", "P"];

#[derive(Debug, Parser)]
#[clap(disable_help_flag = true)]
struct Args {
    #[clap(long, action = clap::ArgAction::HelpLong)]
    help: Option<bool>,

    /// Show files starting with '.'
    #[clap(short, long, default_value_t = false)]
    all: bool,

    /// Show files in a list format
    #[clap(short, long, default_value_t = false)]
    list: bool,

    /// Show the data in human-readable format
    #[clap(short, long, default_value_t = false)]
    human: bool,

    /// The path to list
    #[clap(default_value = ".")]
    path: String,
}

fn get_file_group(metadata: &fs::Metadata) -> Result<String> {
    let gid = metadata.gid();
    let group = get_group_by_gid(gid).context("Attempting to get group by gdi")?;
    Ok(group.name().to_string_lossy().into())
}

fn get_file_owner(metadata: &fs::Metadata) -> Result<String> {
    let uid = metadata.uid();
    let user = get_user_by_uid(uid).context("Attempting to get user by uid")?;
    Ok(user.name().to_string_lossy().into())
}

fn get_last_modified(metadata: &fs::Metadata) -> Result<String> {
    let modified = metadata.modified()?;
    let human_readable_time: DateTime<Local> = modified.into();
    Ok(human_readable_time.format("%b %e %H:%M").to_string())
}

#[cfg(unix)]
fn get_mode(metadata: &fs::Metadata) -> String {
    let mode = metadata.permissions().mode();
    fn mode_to_string(mode: u32, is_dir: bool) -> String {
        let mut s = String::with_capacity(11);

        s.push(if is_dir { 'd' } else { '-' });
        s.push(if mode & 0o400 == 0o400 { 'r' } else { '-' });
        s.push(if mode & 0o200 == 0o200 { 'w' } else { '-' });
        s.push(if mode & 0o100 == 0o100 { 'x' } else { '-' });
        s.push(if mode & 0o040 == 0o040 { 'r' } else { '-' });
        s.push(if mode & 0o020 == 0o020 { 'w' } else { '-' });
        s.push(if mode & 0o010 == 0o010 { 'x' } else { '-' });
        s.push(if mode & 0o004 == 0o004 { 'r' } else { '-' });
        s.push(if mode & 0o002 == 0o002 { 'w' } else { '-' });
        s.push(if mode & 0o001 == 0o001 { 'x' } else { '-' });

        s
    }
    mode_to_string(mode, metadata.is_dir())
}

#[cfg(unix)]
fn get_nlink(metadata: &fs::Metadata) -> String {
    metadata.nlink().to_string()
}

#[cfg(unix)]
fn get_size_bytes(metadata: &fs::Metadata) -> String {
    metadata.size().to_string()
}

#[cfg(unix)]
fn get_size_human_readable(metadata: &fs::Metadata) -> String {
    let bytes = metadata.size();
    if bytes == 0 {
        return "0".to_string();
    }

    // If we just have some absolutely ridiculous number, just do bytes
    let magnitude = bytes.ilog(1024) as usize;
    let adjusted = (bytes as f64) / (1u64 << (magnitude * 10)) as f64;

    if magnitude > SIZES.len() || magnitude == 0 {
        format!("{:.1}", adjusted)
    } else {
        format!("{:.1}{}", adjusted, SIZES[magnitude])
    }
}

#[cfg(not(unix))]
fn get_metadata(path: &String) -> Result<String> {
    Ok("".to_string())
}

fn ls(args: Args) -> Result<String> {
    let path = args.path;

    // If the provided path is not a directory just print the name.
    if !fs::metadata(&path)?.is_dir() {
        return Ok(path);
    }

    let mut entries_paths = {
        let all_files = fs::read_dir(path)?.map(|entry| entry.unwrap().path());

        if args.all {
            all_files.collect::<Vec<_>>()
        } else {
            all_files
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map_or(false, |name| !name.starts_with('.'))
                })
                .collect::<Vec<_>>()
        }
    };

    entries_paths.sort();
    let entries_strs = entries_paths
        .iter()
        .map(|e| e.to_str().unwrap().to_string())
        .collect::<Vec<String>>();

    if args.list {
        #[cfg(unix)]
        {
            let metadatas = entries_paths
                .iter()
                .map(|entry| fs::metadata(entry).unwrap())
                .collect::<Vec<fs::Metadata>>();
            let modes = metadatas.iter().map(get_mode).collect::<Vec<String>>();
            let nlinks = metadatas.iter().map(get_nlink).collect::<Vec<String>>();
            let owners = metadatas
                .iter()
                .map(|metadata| get_file_owner(metadata).unwrap())
                .collect::<Vec<String>>();
            let groups = metadatas
                .iter()
                .map(|metadata| get_file_group(metadata).unwrap())
                .collect::<Vec<String>>();
            let sizes = metadatas
                .iter()
                .map(|m| {
                    if args.human {
                        get_size_human_readable(m)
                    } else {
                        get_size_bytes(m)
                    }
                })
                .collect::<Vec<String>>();
            let last_modified = metadatas
                .iter()
                .map(|metadata| get_last_modified(metadata).unwrap())
                .collect::<Vec<String>>();

            let max_nlink_width = nlinks.iter().map(|nlink| nlink.len()).max().unwrap_or(0);
            let max_owner_width = owners.iter().map(|owner| owner.len()).max().unwrap_or(0);
            let max_group_width = groups.iter().map(|group| group.len()).max().unwrap_or(0);
            let max_size_width = sizes.iter().map(|size| size.len()).max().unwrap_or(0);

            Ok(entries_paths
                .into_iter()
                .enumerate()
                .map(|(i, entry)| {
                    format!("{:<10} {:>width$} {:<owner_width$} {:<group_width$} {:>size_width$} {} {entry}",
                            modes[i], nlinks[i], owners[i], groups[i], sizes[i], last_modified[i],
                            entry = entry.file_name().unwrap().to_str().unwrap(),
                            width = max_nlink_width,
                            owner_width = max_owner_width,
                            group_width = max_group_width,
                            size_width = max_size_width)
                })
                .collect::<Vec<String>>()
                .join("\n"))
        }

        #[cfg(not(unix))]
        Ok(entries_paths.join("\n"))
    } else {
        // Iterate over the terminal width
        let terminal_width = size()?.0 as usize;

        // The max entry width is the longest string in the entry set
        let max_entry_width = entries_strs
            .iter()
            .map(|entry| entry.len())
            .max()
            .unwrap_or(0)
            + 2; // +2 for padding

        // The number of columns is the maximum extent to which we can it values, given the longest entry.
        let columns = terminal_width / max_entry_width;

        // We want the longest columns to be on the left side, so we round up by adding columns - 1.
        let rows = (entries_paths.len() + columns - 1) / columns;

        let mut output = String::new();

        // Now, build the output column by column in alphabetical order (just like coreutils ls does it.)
        for row in 0..rows {
            for column in 0..columns {
                if let Some(entry) = entries_strs.get(row + column * rows) {
                    output += entry;
                    // Add spaces only if we're *not* in the end column.
                    if column != columns - 1 {
                        output += &" ".repeat(max_entry_width - entry.len());
                    }
                }
            }
            if row != rows - 1 {
                output.push('\n');
            }
        }
        Ok(output)
    }
}

fn main() {
    let args = Args::parse();
    let output = ls(args).unwrap();
    println!("{}", output);
}
