use crate::{cmd, paths};
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "coord", version, about = "A shared task ledger coordinating parallel coding agents")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Initialize coordination in this repository
    Init,
    /// Manage tasks
    #[command(subcommand)]
    Task(TaskCmd),
    /// Register files you are about to modify
    Claim {
        /// Task id (e.g. T12)
        #[arg(short = 't', long = "task")]
        task: String,
        /// Files or directories to claim
        #[arg(required = true)]
        paths: Vec<String>,
        /// Register co-editing even if a live task holds the path
        #[arg(long)]
        force: bool,
    },
    /// Show who is editing what
    Status,
    /// Commit only the files claimed by a task
    Commit {
        /// Task id (e.g. T12)
        #[arg(short = 't', long = "task")]
        task: String,
        /// Commit message
        #[arg(short = 'm', long = "message")]
        message: String,
    },
}

#[derive(Subcommand)]
enum TaskCmd {
    /// Register a new task; prints its id
    Start { description: String },
    /// Finish a task and release all its claims
    Done { id: String },
}

pub fn run() -> Result<i32> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    match cli.cmd {
        Cmd::Init => {
            cmd::init::run(&cwd)?;
            Ok(0)
        }
        Cmd::Task(TaskCmd::Start { description }) => {
            cmd::task::start(&require_root(&cwd)?, &description)?;
            Ok(0)
        }
        Cmd::Task(TaskCmd::Done { id }) => {
            cmd::task::done(&require_root(&cwd)?, paths::parse_task_id(&id)?)?;
            Ok(0)
        }
        Cmd::Claim { task, paths: ps, force } => {
            cmd::claim::run(&require_root(&cwd)?, &cwd, paths::parse_task_id(&task)?, &ps, force)
        }
        Cmd::Status => {
            cmd::status::run(&require_root(&cwd)?)?;
            Ok(0)
        }
        Cmd::Commit { task, message } => {
            cmd::commit::run(&require_root(&cwd)?, paths::parse_task_id(&task)?, &message)?;
            Ok(0)
        }
    }
}

fn require_root(cwd: &Path) -> Result<PathBuf> {
    match paths::find_root(cwd) {
        Some(root) if root.join(".agentcoord").is_dir() => Ok(root),
        _ => bail!("coord is not initialized here; run `coord init` at the repository root"),
    }
}
