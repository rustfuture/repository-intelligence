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
            let path = entry?.path();
            let rel = path.strip_prefix(root).expect("walked under root");
            if path.is_dir() {
                if !matches!(
                    rel.file_name().and_then(|n| n.to_str()),
                    Some(".git" | "target" | "node_modules")
                ) {
                    self.walk(root, &path)?;
                }
            } else if is_indexable(rel) {
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
    let revision = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!revision.is_empty()).then_some(revision)
}

pub fn current_git_revision(root: &Path) -> Option<String> {
    git_revision(root)
}

fn is_indexable(path: &Path) -> bool {
    !matches!(
        path.extension().and_then(|x| x.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "lock" | "bin")
    )
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
}
