use anyhow::{bail, Result};
use std::path::Path;

pub fn run(_root: &Path, _cwd: &Path, _task_id: i64, _inputs: &[String], _force: bool) -> Result<i32> {
    bail!("not implemented")
}
