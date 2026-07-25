use anyhow::{anyhow, bail, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Nearest ancestor containing `.agentcoord/`, else nearest containing `.git`, else None.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.join(".agentcoord").is_dir() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.join(".git").exists() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

/// Accepts "12", "T12", "t12".
pub fn parse_task_id(s: &str) -> Result<i64> {
    let t = s.trim();
    let t = t
        .strip_prefix('T')
        .or_else(|| t.strip_prefix('t'))
        .unwrap_or(t);
    t.parse::<i64>()
        .map_err(|_| anyhow!("invalid task id: {s} (expected e.g. T12)"))
}

/// True when one path equals or lexically contains the other (file ≺ directory).
pub fn overlaps(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
}

/// Lexically resolve `input` (relative to `cwd` unless absolute) into a
/// root-relative, `/`-separated path with no trailing slash.
/// Purely lexical: claimed paths may not exist yet.
pub fn normalize(root: &Path, cwd: &Path, input: &str) -> Result<String> {
    let p = Path::new(input.trim_end_matches('/'));
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    let mut parts: Vec<OsString> = Vec::new();
    for comp in abs.components() {
        use std::path::Component::*;
        match comp {
            CurDir => {}
            ParentDir => {
                if parts.pop().is_none() {
                    bail!("path escapes the filesystem root: {input}");
                }
            }
            Normal(c) => parts.push(c.to_os_string()),
            RootDir | Prefix(_) => parts.clear(),
        }
    }
    let mut resolved = PathBuf::from("/");
    for part in parts {
        resolved.push(part);
    }
    let rel = resolved
        .strip_prefix(root)
        .map_err(|_| anyhow!("path is outside the repository: {input}"))?;
    let s = rel.to_string_lossy().to_string();
    if s.is_empty() {
        bail!("cannot claim the repository root itself");
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_ids() {
        assert_eq!(parse_task_id("12").unwrap(), 12);
        assert_eq!(parse_task_id("T12").unwrap(), 12);
        assert_eq!(parse_task_id("t7").unwrap(), 7);
        assert!(parse_task_id("abc").is_err());
        assert!(parse_task_id("").is_err());
    }

    #[test]
    fn overlaps_rules() {
        assert!(overlaps("a/b.rs", "a/b.rs"));
        assert!(overlaps("src", "src/auth.ts"));
        assert!(overlaps("src/auth.ts", "src"));
        assert!(!overlaps("src/auth.ts", "src/auth.ts.bak"));
        assert!(!overlaps("src", "srclib/x.rs"));
        assert!(!overlaps("a/b", "a/c"));
    }

    #[test]
    fn normalize_basic() {
        let root = Path::new("/repo");
        assert_eq!(
            normalize(root, Path::new("/repo"), "src/a.rs").unwrap(),
            "src/a.rs"
        );
        assert_eq!(
            normalize(root, Path::new("/repo/src"), "./a.rs").unwrap(),
            "src/a.rs"
        );
        assert_eq!(
            normalize(root, Path::new("/repo/src"), "../lib/x.rs").unwrap(),
            "lib/x.rs"
        );
        assert_eq!(
            normalize(root, Path::new("/repo"), "/repo/a/b").unwrap(),
            "a/b"
        );
        assert_eq!(normalize(root, Path::new("/repo"), "dir/").unwrap(), "dir");
    }

    #[test]
    fn normalize_rejects_escapes() {
        let root = Path::new("/repo");
        assert!(normalize(root, Path::new("/repo"), "../outside.rs").is_err());
        assert!(normalize(root, Path::new("/repo"), "/etc/passwd").is_err());
        assert!(normalize(root, Path::new("/repo"), ".").is_err()); // repo root itself
    }

    #[test]
    fn find_root_prefers_agentcoord_over_git() {
        let tmp = tempfile::tempdir().unwrap();
        let outer = tmp.path();
        let inner = outer.join("sub/dir");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::create_dir_all(outer.join(".git")).unwrap();
        assert_eq!(find_root(&inner).unwrap(), outer);
        std::fs::create_dir_all(outer.join("sub/.agentcoord")).unwrap();
        assert_eq!(find_root(&inner).unwrap(), outer.join("sub"));
    }
}
