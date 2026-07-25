use crate::{db, paths};
use anyhow::Result;
use std::fs;
use std::path::Path;

const BLOCK_BEGIN: &str = "<!-- coord:begin -->";
const BLOCK_END: &str = "<!-- coord:end -->";
const BLOCK_BODY: &str = include_str!("../../../templates/AGENTS-block.md");

pub fn run(cwd: &Path) -> Result<()> {
    let root = paths::find_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    db::open(&root)?;
    ensure_gitignore(&root)?;
    inject_agents_block(&root)?;
    println!("coord initialized at {}", root.display());
    println!("  state:    .agentcoord/state.db (gitignored)");
    println!("  protocol: AGENTS.md (managed block)");
    Ok(())
}

fn ensure_gitignore(root: &Path) -> Result<()> {
    let path = root.join(".gitignore");
    let current = fs::read_to_string(&path).unwrap_or_default();
    if !current.lines().any(|l| l.trim() == ".agentcoord/") {
        let mut next = current;
        if !next.is_empty() && !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str(".agentcoord/\n");
        fs::write(&path, next)?;
    }
    Ok(())
}

fn inject_agents_block(root: &Path) -> Result<()> {
    let path = root.join("AGENTS.md");
    let block = format!("{BLOCK_BEGIN}\n{}\n{BLOCK_END}", BLOCK_BODY.trim_end());
    let current = fs::read_to_string(&path).unwrap_or_default();
    let next = match (current.find(BLOCK_BEGIN), current.find(BLOCK_END)) {
        (Some(start), Some(end)) if end > start => {
            format!(
                "{}{}{}",
                &current[..start],
                block,
                &current[end + BLOCK_END.len()..]
            )
        }
        _ => {
            let mut s = current;
            if !s.is_empty() {
                if !s.ends_with('\n') {
                    s.push('\n');
                }
                s.push('\n');
            }
            format!("{s}{block}\n")
        }
    };
    fs::write(&path, next)?;
    Ok(())
}
