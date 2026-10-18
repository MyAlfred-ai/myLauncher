//! Version comparison tolerant to the formats used across MyAlfred apps:
//! `1.5.0.4`, `v1.2.0.139`, `0.1.0`, or a bare commit hash.

use std::cmp::Ordering;

/// Parses `v1.2.3.4` / `1.2.3-beta` into numeric parts. Returns `None` for
/// anything that does not start with a number (commit hashes, branch names).
pub fn parse_numeric(v: &str) -> Option<Vec<u64>> {
    let v = v.trim();
    let v = v
        .strip_prefix('v')
        .or_else(|| v.strip_prefix('V'))
        .unwrap_or(v);
    let core = v.split(['-', '+', ' ']).next().unwrap_or("");
    if core.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for p in core.split('.') {
        parts.push(p.parse::<u64>().ok()?);
    }
    Some(parts)
}

pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    let (pa, pb) = (parse_numeric(a)?, parse_numeric(b)?);
    let n = pa.len().max(pb.len());
    for i in 0..n {
        let (x, y) = (
            pa.get(i).copied().unwrap_or(0),
            pb.get(i).copied().unwrap_or(0),
        );
        match x.cmp(&y) {
            Ordering::Equal => continue,
            o => return Some(o),
        }
    }
    Some(Ordering::Equal)
}

/// Decides whether `latest` is newer than what is installed.
/// Numeric versions are compared numerically; commit-based apps (`by_commit`)
/// and non-numeric versions compare commits.
pub fn is_update(
    latest_version: &str,
    latest_commit: Option<&str>,
    installed_version: Option<&str>,
    installed_commit: Option<&str>,
    by_commit: bool,
) -> bool {
    if let (false, Some(iv)) = (by_commit, installed_version) {
        if let Some(ord) = compare(latest_version, iv) {
            return ord == Ordering::Greater;
        }
    }
    match (latest_commit, installed_commit) {
        (Some(lc), Some(ic)) if !lc.is_empty() && !ic.is_empty() => !same_commit(lc, ic),
        _ => match installed_version {
            Some(iv) => !iv.is_empty() && iv.trim() != latest_version.trim(),
            None => false,
        },
    }
}

fn same_commit(a: &str, b: &str) -> bool {
    let n = a.len().min(b.len()).max(7);
    a.get(..n.min(a.len())) == b.get(..n.min(b.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_numeric("v1.2.0.139"), Some(vec![1, 2, 0, 139]));
        assert_eq!(parse_numeric("1.5.0.4\n"), Some(vec![1, 5, 0, 4]));
        assert_eq!(parse_numeric("2.0.0-beta.1"), Some(vec![2, 0, 0]));
        assert_eq!(parse_numeric("2c68389"), None);
        assert_eq!(parse_numeric(""), None);
    }

    #[test]
    fn compares_versions() {
        assert_eq!(compare("1.5.0.10", "1.5.0.9"), Some(Ordering::Greater));
        assert_eq!(compare("1.5", "1.5.0"), Some(Ordering::Equal));
        assert_eq!(compare("v1.2.0.139", "1.3.0.90"), Some(Ordering::Less));
        assert_eq!(compare("abc", "1.0"), None);
    }

    #[test]
    fn detects_updates() {
        assert!(is_update("1.5.0.5", None, Some("1.5.0.4"), None, false));
        assert!(!is_update("1.5.0.4", None, Some("1.5.0.4"), None, false));
        // Never offer a downgrade.
        assert!(!is_update("1.5.0.3", None, Some("1.5.0.4"), None, false));
        // Commit-based apps.
        assert!(is_update(
            "2c68389",
            Some("2c68389aaaa"),
            Some("1a2b3c4"),
            Some("1a2b3c4dddd"),
            true
        ));
        assert!(!is_update(
            "2c68389",
            Some("2c68389aaaa"),
            Some("2c68389"),
            Some("2c68389aaaa"),
            true
        ));
        // All-digit hashes must not be compared as numbers.
        assert!(is_update(
            "1234567",
            Some("1234567ab"),
            Some("7654321"),
            Some("7654321cd"),
            true
        ));
        // Nothing known about the install: stay quiet.
        assert!(!is_update("1.0.0", None, None, None, false));
    }
}
