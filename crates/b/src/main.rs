use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use std::fs::{self, DirEntry};
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Parser)]
struct Args {
    /// Format the project that we're in
    #[clap(short, long, default_value_t = false)]
    format: bool,

    /// Build the project that we're in
    #[clap(short, long, default_value_t = false)]
    build: bool,

    /// Whether or not to perform a release build (only works with -b, --build)
    #[clap(short, long, default_value_t = false)]
    release: bool,

    /// Verbose output
    #[clap(short, long, default_value_t = false)]
    verbose: bool,
}

/// Projects for work with special build commands
const WORK_PROJECTS: [&str; 5] = [
    "hotshot",
    "espresso-sequencer",
    "hotshot-builder-core",
    "hotshot-query-service",
    "hotshot-events-service",
];

#[derive(Debug, Eq, PartialEq)]
enum ProjectType {
    Rust,
    Python,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProjectType::Rust => write!(f, "ProjectType(Rust)"),
            ProjectType::Python => write!(f, "ProjectType(Python)"),
        }
    }
}

#[derive(Debug)]
struct Project {
    name: String,
    work: bool,
    project_type: ProjectType,
}

impl std::fmt::Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Project(name={}, is_work_project={}, project_type={})",
            self.name, self.work, self.project_type
        )
    }
}

impl Project {
    pub fn new(name: &str, work: bool, project_type: ProjectType) -> Self {
        Self {
            name: name.to_string(),
            work,
            project_type,
        }
    }

    pub fn build(&self, release: bool) -> Result<()> {
        match self.project_type {
            ProjectType::Rust => self.build_rust(release),
            ProjectType::Python => {
                bail!("Cannot build this project type yet.")
            }
        }
    }

    pub fn format(&self) -> Result<()> {
        match self.project_type {
            ProjectType::Rust => self.format_rust(),
            ProjectType::Python => {
                bail!("Cannot format this project type yet.")
            }
        }
    }

    fn format_rust(&self) -> Result<()> {
        let cmds = match self.project_type {
            ProjectType::Rust => {
                if self.work && self.name.as_str() == "hotshot" {
                    vec!["just", "async-std", "fmt_lint"]
                } else {
                    vec!["cargo", "fmt"]
                }
            }
            ProjectType::Python => bail!("Not supported yet."),
        };

        self.run_cmd(cmds[0], cmds[1..].to_vec())
    }

    fn build_rust(&self, release: bool) -> Result<()> {
        let mut cmds = match self.project_type {
            ProjectType::Rust => {
                if self.work && self.name.as_str() == "hotshot" {
                    vec!["just", "async-std", "build"]
                } else {
                    vec!["cargo", "build"]
                }
            }
            ProjectType::Python => bail!("Not supported yet."),
        };

        if release {
            cmds.push("--release");
        }

        self.run_cmd(cmds[0], cmds[1..].to_vec())
    }

    fn run_cmd(&self, command_name: &str, args: Vec<&str>) -> Result<()> {
        let mut child = Command::new(command_name)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = io::BufReader::new(stdout);
        let stderr_reader = io::BufReader::new(stderr);

        let stdout_handle = std::thread::spawn(move || {
            for line in stdout_reader.lines() {
                match line {
                    Ok(line) => println!("{line}"),
                    Err(e) => eprintln!("{e}"),
                }
            }
        });

        let stderr_handle = std::thread::spawn(move || {
            for line in stderr_reader.lines() {
                match line {
                    Ok(line) => println!("{line}"),
                    Err(e) => eprintln!("{e}"),
                }
            }
        });

        stdout_handle.join().unwrap();
        stderr_handle.join().unwrap();

        let status = child.wait()?;
        if status.success() {
            println!(
                "Exited with code: {}",
                status.code().unwrap().to_string().green()
            );
        } else {
            eprintln!(
                "Exited with code: {}",
                status.code().unwrap().to_string().bright_red()
            );
        }

        Ok(())
    }
}

fn detect_work_project(current_dir: &PathBuf, paths: &Vec<DirEntry>) -> Option<String> {
    let dirname = current_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_lowercase();

    if !WORK_PROJECTS.contains(&dirname.as_str()) {
        return None;
    }

    // Check if this is a work project.
    for pname in WORK_PROJECTS.iter() {
        for entry in paths {
            if entry
                .path()
                .components()
                .any(|c| c.as_os_str().eq_ignore_ascii_case(pname))
            {
                return Some(pname.to_string());
            }
        }
    }

    None
}

fn detect_project_type(paths: &Vec<DirEntry>) -> Option<ProjectType> {
    for entry in paths {
        if let Some(os_str) = entry.path().file_name() {
            match os_str.to_str() {
                Some(s) => match s.to_lowercase().as_str() {
                    "cargo.toml" => return Some(ProjectType::Rust),
                    "requirements.txt" | "pyproject.toml" => return Some(ProjectType::Python),
                    _ => {}
                },
                None => {}
            }
        }

        if let Some(os_str) = entry.path().extension() {
            match os_str.to_str() {
                Some("rs") => return Some(ProjectType::Rust),
                Some("py") => return Some(ProjectType::Python),
                _ => {}
            }
        }
    }

    None
}

fn find_project_name(current_dir: &PathBuf) -> &str {
    current_dir.file_name().unwrap().to_str().unwrap()
}

fn main() -> Result<()> {
    let args = Args::parse();

    // What directory are we in?
    let current_dir = std::env::current_dir().map_err(anyhow::Error::from)?;

    // Get all the good files.
    let paths = fs::read_dir(&current_dir)
        .map_err(anyhow::Error::from)?
        .filter_map(Result::ok)
        .collect::<Vec<DirEntry>>();

    // Get the project name
    let project = if let Some(name) = detect_work_project(&current_dir, &paths) {
        Project::new(&name, true, ProjectType::Rust)
    } else {
        if let Some(typ) = detect_project_type(&paths) {
            Project::new(&find_project_name(&current_dir), false, typ)
        } else {
            bail!("Couldn't detect project type, or go an invalid project type.");
        }
    };

    if args.build {
        if args.verbose {
            println!("{}", format!("{}", project).blue());
            println!("{}", format!("{}", "Initiating build".blue()));
        }
        project.build(args.release)?;
    }

    if args.format {
        project.format()?;
    }

    Ok(())
}
