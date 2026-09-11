pub mod llm;

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub score: usize,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct Index {
    files: HashMap<PathBuf, Vec<String>>,
    terms: HashMap<String, HashSet<(PathBuf, usize)>>,
    revision: Option<String>,
}

impl Index {
    pub fn build(root: &Path) -> io::Result<Self> {
        let mut index = Self::default();
        index.rebuild(root)?;
        index.revision = git_revision(root);
        Ok(index)
    }

    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    /// Apply only Git file changes between the indexed revision and `new_revision`.
    /// A missing or unrelated history falls back to a complete rebuild.
    pub fn sync_git(&mut self, root: &Path, new_revision: &str) -> io::Result<()> {
        let Some(old_revision) = self.revision.clone() else {
            self.rebuild(root)?;
            self.revision = Some(new_revision.to_owned());
            return Ok(());
        };
        let output = Command::new("git")
            .args([
                "-C",
                root.to_str().unwrap_or("."),
                "diff",
                "--name-status",
                &old_revision,
                new_revision,
            ])
            .output()?;
        if !output.status.success() {
            self.rebuild(root)?;
            self.revision = Some(new_revision.to_owned());
            return Ok(());
        }
        let changes = String::from_utf8_lossy(&output.stdout);
        for line in changes.lines() {
            let Some((status, path)) = line.split_once('\t') else {
                continue;
            };
            let relative = Path::new(path);
            if status.starts_with('D') {
                self.remove_file(relative);
            } else {
                self.update_file(root, relative)?;
            }
        }
        self.revision = Some(new_revision.to_owned());
        Ok(())
    }

    pub fn rebuild(&mut self, root: &Path) -> io::Result<()> {
        self.files.clear();
        self.terms.clear();
        self.walk(root, root)?;
        Ok(())
    }

    pub fn update_file(&mut self, root: &Path, relative: &Path) -> io::Result<()> {
        self.remove_file(relative);
        if !is_safe_relative(relative) {
            return Ok(());
        }
        let mut checked = root.to_path_buf();
        for component in relative.components() {
            checked.push(component);
            match fs::symlink_metadata(&checked) {
                Ok(metadata) if metadata.file_type().is_symlink() => return Ok(()),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error),
            }
        }
        let file_name = relative
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if is_sensitive_file_name(file_name) {
            return Ok(());
        }
        let path = root.join(relative);
        if path.is_file() && is_indexable(relative) {
            self.add_file(relative.to_path_buf(), fs::read_to_string(path)?);
        }
        Ok(())
    }

    pub fn remove_file(&mut self, relative: &Path) {
        self.files.remove(relative);
        self.terms.retain(|_, refs| {
            refs.retain(|(p, _)| p != relative);
            !refs.is_empty()
        });
    }

    pub fn analytics(&self) -> String {
        let file_count = self.files.len();
        let term_count = self.terms.len();
        let total_lines: usize = self.files.values().map(|lines| lines.len()).sum();
        let mut top_terms: Vec<_> = self
            .terms
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();
        top_terms.sort_by_key(|(term, count)| (std::cmp::Reverse(*count), term.clone()));
        top_terms.truncate(5);
        let top_terms_str = top_terms
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"{{"files": {}, "lines": {}, "unique_terms": {}, "top_terms": "{}"}}"#,
            file_count, total_lines, term_count, top_terms_str
        )
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let terms: Vec<String> = tokenize(query);
        if terms.is_empty() {
            return Vec::new();
        }
        let mut scores: HashMap<(PathBuf, usize), usize> = HashMap::new();
        for term in terms {
            if let Some(refs) = self.terms.get(&term) {
                for reference in refs {
                    *scores.entry(reference.clone()).or_default() += 1;
                }
            }
        }
        let mut hits: Vec<_> = scores
            .into_iter()
            .filter_map(|((path, line), score)| {
                self.files.get(&path).and_then(|lines| {
                    lines.get(line.saturating_sub(1)).map(|text| Hit {
                        path: path.clone(),
                        line,
                        score,
                        text: text.clone(),
                    })
                })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.path.cmp(&b.path))
                .then(a.line.cmp(&b.line))
        });
        hits.truncate(limit);
        hits
    }

    fn walk(&mut self, root: &Path, dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip symlinks
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.file_type().is_symlink() {
                    continue;
                }
            }

            let rel = path.strip_prefix(root).expect("walked under root");
            if !is_safe_relative(rel) {
                continue;
            }
            let file_name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if path.is_dir() {
                // Sensitive dirs & typical ignores
                if file_name.starts_with('.')
                    || matches!(file_name, "target" | "node_modules" | "build" | "dist")
                {
                    continue;
                }
                self.walk(root, &path)?;
            } else if is_indexable(rel) {
                // Sensitive files
                if is_sensitive_file_name(file_name) {
                    continue;
                }
                self.add_file(rel.to_path_buf(), fs::read_to_string(path)?);
            }
        }
        Ok(())
    }

    fn add_file(&mut self, relative: PathBuf, content: String) {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        for (number, line) in lines.iter().enumerate() {
            for term in tokenize(line) {
                self.terms
                    .entry(term)
                    .or_default()
                    .insert((relative.clone(), number + 1));
            }
        }
        self.files.insert(relative, lines);
    }
}

fn git_revision(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", root.to_str()?, "rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut revision = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if revision.is_empty() {
        return None;
    }

    // Check if dirty
    if let Ok(status) = Command::new("git")
        .args(["-C", root.to_str()?, "status", "--porcelain"])
        .output()
    {
        if !status.stdout.is_empty() {
            revision.push_str("-dirty");
        }
    }
    Some(revision)
}

pub fn current_git_revision(root: &Path) -> Option<String> {
    git_revision(root)
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.components().all(|component| {
            let std::path::Component::Normal(name) = component else {
                return false;
            };
            let Some(name) = name.to_str() else {
                return false;
            };
            !name.starts_with('.')
                && !matches!(
                    name,
                    "target" | "node_modules" | "build" | "dist" | "id_rsa" | "id_ed25519"
                )
                && !name.ends_with(".pem")
                && !name.ends_with(".key")
        })
}

fn is_indexable(path: &Path) -> bool {
    !matches!(
        path.extension().and_then(|x| x.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "lock" | "bin")
    )
}

/// Single source of truth for file names that must never be indexed.
/// Shared by the full walk and incremental updates so a scan and a rebase agree.
fn is_sensitive_file_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    (name.starts_with('.') && name != ".github" && name != ".gitignore")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".p12")
        || name.ends_with(".pfx")
        || name.ends_with(".keystore")
        || name == "id_rsa"
        || name == "id_ed25519"
        || name == "credentials.json"
        || name == "service-account.json"
        || name == ".npmrc"
        || name == ".netrc"
        || name == ".env"
        || name.starts_with(".env.")
}
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ri-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("lib.rs"), "pub fn bounded_worker() {}\n").unwrap();
        fs::create_dir_all(path.join("target")).unwrap();
        fs::write(path.join("target/ignored.rs"), "bounded_worker").unwrap();
        path
    }
    #[test]
    fn returns_line_cited_hits_and_skips_target() {
        let root = fixture();
        let index = Index::build(&root).unwrap();
        let hits = index.search("bounded worker", 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, PathBuf::from("lib.rs"));
        assert_eq!(hits[0].line, 1);
    }
    #[test]
    fn update_and_delete_change_results() {
        let root = fixture();
        let mut index = Index::build(&root).unwrap();
        fs::write(root.join("lib.rs"), "pub fn fresh_index() {}\n").unwrap();
        index.update_file(&root, Path::new("lib.rs")).unwrap();
        assert!(index.search("bounded", 5).is_empty());
        assert_eq!(index.search("fresh", 5).len(), 1);
        index.remove_file(Path::new("lib.rs"));
        assert!(index.search("fresh", 5).is_empty());
    }

    #[test]
    fn git_sync_updates_only_changed_files() {
        let root = fixture();
        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .args(["-C", root.to_str().unwrap()])
                .args(args)
                .output()
                .unwrap()
        };
        assert!(run(&["init", "-q"]).status.success());
        assert!(run(&["config", "user.email", "test@example.invalid"])
            .status
            .success());
        assert!(run(&["config", "user.name", "Test"]).status.success());
        assert!(run(&["add", "lib.rs"]).status.success());
        assert!(run(&["commit", "-qm", "initial"]).status.success());
        let old = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout).unwrap();
        let old = old.trim();
        let mut index = Index::build(&root).unwrap();
        fs::write(root.join("lib.rs"), "pub fn changed_symbol() {}\n").unwrap();
        assert!(run(&["commit", "-am", "changed"]).status.success());
        let new = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout).unwrap();
        index.revision = Some(old.to_owned());
        index.sync_git(&root, new.trim()).unwrap();
        assert!(index.search("bounded", 5).is_empty());
        assert_eq!(index.search("changed symbol", 5).len(), 1);
    }

    #[test]
    fn analytics_returns_stats() {
        let root = fixture();
        let index = Index::build(&root).unwrap();
        let stats = index.analytics();
        assert!(stats.contains(r#""files": 1"#));
        assert!(stats.contains(r#""lines": 1"#));
    }

    #[test]
    fn incremental_updates_reject_sensitive_and_outside_paths() {
        let root = fixture();
        let mut index = Index::build(&root).unwrap();
        for name in [".env", "private.key", "id_ed25519"] {
            fs::write(root.join(name), "sensitivecanary").unwrap();
            index.update_file(&root, Path::new(name)).unwrap();
        }
        index
            .update_file(&root, Path::new("../outside.rs"))
            .unwrap();
        assert!(index.search("sensitivecanary", 5).is_empty());
        assert!(Index::build(&root)
            .unwrap()
            .search("sensitivecanary", 5)
            .is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn incremental_updates_skip_symlink_parents() {
        let root = fixture();
        let outside = fixture();
        fs::write(outside.join("secret.rs"), "outsidecanary").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
        let mut index = Index::build(&root).unwrap();
        index
            .update_file(&root, Path::new("linked/secret.rs"))
            .unwrap();
        assert!(index.search("outsidecanary", 5).is_empty());
    }

    #[test]
    fn incremental_and_full_scan_agree_on_sensitive_files() {
        let root = fixture();
        fs::create_dir_all(root.join("sub")).unwrap();
        let sensitive = [
            "credentials.json",
            "service-account.json",
            "client.p12",
            "client.pfx",
            "release.keystore",
            "sub/credentials.json",
            "sub/client.p12",
            "sub/.env",
            "sub/.npmrc",
        ];
        for name in sensitive {
            fs::write(root.join(name), "sensitivecanary").unwrap();
        }
        fs::write(root.join("sub/keep.rs"), "publiccanary").unwrap();

        let mut index = Index::build(&root).unwrap();
        assert!(index.search("sensitivecanary", 20).is_empty());
        assert_eq!(index.search("publiccanary", 5).len(), 1);

        for name in sensitive {
            index.update_file(&root, Path::new(name)).unwrap();
        }
        index.update_file(&root, Path::new("sub/keep.rs")).unwrap();
        assert!(index.search("sensitivecanary", 20).is_empty());
        assert_eq!(index.search("publiccanary", 5).len(), 1);
    }

    #[test]
    fn previously_indexed_sensitive_file_is_removed_on_update() {
        let root = fixture();
        let mut index = Index::build(&root).unwrap();
        index.add_file(PathBuf::from("credentials.json"), "legacycanary".to_owned());
        assert_eq!(index.search("legacycanary", 5).len(), 1);

        fs::write(root.join("credentials.json"), "legacycanary").unwrap();
        index
            .update_file(&root, Path::new("credentials.json"))
            .unwrap();
        assert!(index.search("legacycanary", 5).is_empty());
    }

    #[test]
    fn sensitive_names_match_case_insensitively() {
        for name in [
            "CREDENTIALS.JSON",
            "Service-Account.json",
            "CLIENT.P12",
            "CLIENT.PFX",
            "RELEASE.KEYSTORE",
            "ID_RSA",
            "ID_ED25519",
            "SERVER.PEM",
            "SERVER.KEY",
            ".ENV",
            ".Env.local",
        ] {
            assert!(is_sensitive_file_name(name), "{name} must be sensitive");
        }
        for name in ["credentials.rs", "service.rs", "keep.p12.bak"] {
            assert!(
                !is_sensitive_file_name(name),
                "{name} must not be sensitive"
            );
        }
    }

    #[test]
    fn full_scan_and_update_agree_on_case_variant_sensitive_names() {
        let root = fixture();
        fs::create_dir_all(root.join("sub")).unwrap();
        let sensitive = [
            "CREDENTIALS.JSON",
            "Service-Account.json",
            "CLIENT.P12",
            "sub/RELEASE.PFX",
            "sub/KEYSTORE.KEYSTORE",
            "ID_RSA",
        ];
        for name in sensitive {
            fs::write(root.join(name), "casecanary").unwrap();
        }

        let mut index = Index::build(&root).unwrap();
        assert!(index.search("casecanary", 20).is_empty());
        for name in sensitive {
            index.update_file(&root, Path::new(name)).unwrap();
        }
        assert!(index.search("casecanary", 20).is_empty());
    }

    #[test]
    fn update_file_case_alias_cannot_bypass_sensitive_policy() {
        let root = fixture();
        fs::write(root.join("credentials.json"), "aliascanary").unwrap();
        let mut index = Index::build(&root).unwrap();
        assert!(index.search("aliascanary", 5).is_empty());

        index
            .update_file(&root, Path::new("CREDENTIALS.JSON"))
            .unwrap();
        assert!(index.search("aliascanary", 5).is_empty());
    }
}
