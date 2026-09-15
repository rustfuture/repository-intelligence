pub mod citation;
pub mod llm;

use std::{
    collections::{HashMap, HashSet},
    fmt, fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

/// A line-level lexical match retained for backwards-compatible CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub score: usize,
    pub text: String,
}

/// Retrieval strategy used by [`Index::search_evidence`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    Lexical,
    Semantic,
    Hybrid,
}

impl RetrievalMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lexical => "lexical",
            Self::Semantic => "semantic",
            Self::Hybrid => "hybrid",
        }
    }
}

/// A source span that can be shown to a user or passed to an answer provider.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub score: f32,
    pub source: String,
    pub kind: String,
    pub symbol: Option<String>,
    pub text: String,
}

impl Evidence {
    pub fn citation(&self) -> String {
        if self.start_line == self.end_line {
            format!("{}:{}", self.path.display(), self.start_line)
        } else {
            format!(
                "{}:{}-{}",
                self.path.display(),
                self.start_line,
                self.end_line
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecord {
    pub sha: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Clone)]
struct Chunk {
    path: PathBuf,
    start_line: usize,
    end_line: usize,
    text: String,
    kind: String,
    symbol: Option<String>,
}

/// Provider abstraction for semantic retrieval.
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&self, text: &str) -> io::Result<Vec<f32>>;
}

/// A stable hashed-token embedding. It is not a language model, but gives the
/// product a reproducible semantic-retrieval baseline and a replaceable seam
/// for a real local or hosted embedding service.
#[derive(Debug, Clone)]
pub struct HashEmbedding {
    dimension: usize,
}

impl HashEmbedding {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension: dimension.max(8),
        }
    }
}

impl Default for HashEmbedding {
    fn default() -> Self {
        Self::new(128)
    }
}

impl EmbeddingProvider for HashEmbedding {
    fn name(&self) -> &str {
        "hash-token-v1"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(&self, text: &str) -> io::Result<Vec<f32>> {
        let mut vector = vec![0.0; self.dimension];
        for token in tokenize(text) {
            let hash = stable_hash(token.as_bytes());
            let slot = (hash as usize) % self.dimension;
            let sign = if hash & 1 == 0 { 1.0 } else { -1.0 };
            vector[slot] += sign;
        }
        normalize(&mut vector);
        Ok(vector)
    }
}

/// A local neural embedding provider backed by an Ollama daemon.
/// Defaults to `nomic-embed-text` with dimension 768 on localhost:11434.
#[derive(Debug, Clone)]
pub struct OllamaEmbedding {
    model: String,
    dimension: usize,
    endpoint: String,
}

impl OllamaEmbedding {
    pub fn new(model: impl Into<String>, dimension: usize, endpoint: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            dimension,
            endpoint: endpoint.into(),
        }
    }

    pub fn nomic_default() -> Self {
        let endpoint =
            std::env::var("RI_EMBEDDING_URL").unwrap_or_else(|_| "127.0.0.1:11434".to_owned());
        let model =
            std::env::var("RI_EMBEDDING_MODEL").unwrap_or_else(|_| "nomic-embed-text".to_owned());
        Self::new(model, 768, endpoint)
    }

    pub fn try_embed(&self, text: &str) -> io::Result<Vec<f32>> {
        http_post_embed(&self.endpoint, &self.model, text, self.dimension)
    }

    pub fn is_available(&self) -> bool {
        self.try_embed("ping").is_ok()
    }
}

impl Default for OllamaEmbedding {
    fn default() -> Self {
        Self::nomic_default()
    }
}

impl EmbeddingProvider for OllamaEmbedding {
    fn name(&self) -> &str {
        &self.model
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed(&self, text: &str) -> io::Result<Vec<f32>> {
        self.try_embed(text)
    }
}

pub fn provider_from_name(name: &str) -> io::Result<Arc<dyn EmbeddingProvider>> {
    let lower = name.trim().to_ascii_lowercase();
    if lower == "hash" || lower == "hash-token-v1" {
        Ok(Arc::new(HashEmbedding::default()))
    } else if lower.starts_with("nomic") || lower.starts_with("ollama") {
        Ok(Arc::new(OllamaEmbedding::default()))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "unknown embedding provider '{}'; supported: 'hash', 'nomic-embed-text'",
                name
            ),
        ))
    }
}

pub fn default_provider_from_env() -> io::Result<Arc<dyn EmbeddingProvider>> {
    if let Ok(provider) = std::env::var("RI_EMBEDDING_PROVIDER") {
        return provider_from_name(&provider);
    }
    Ok(Arc::new(HashEmbedding::default()))
}

pub struct Index {
    files: HashMap<PathBuf, Vec<String>>,
    terms: HashMap<String, HashSet<(PathBuf, usize)>>,
    chunks: HashMap<PathBuf, Vec<Chunk>>,
    file_hashes: HashMap<PathBuf, u64>,
    revision: Option<String>,
    commits: Vec<CommitRecord>,
    embedding: Arc<dyn EmbeddingProvider>,
    vectors: HashMap<PathBuf, Vec<Vec<f32>>>,
}

impl Default for Index {
    fn default() -> Self {
        Self::with_embedding(Arc::new(HashEmbedding::default()))
    }
}

impl Index {
    pub fn with_embedding(embedding: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            files: HashMap::new(),
            terms: HashMap::new(),
            chunks: HashMap::new(),
            file_hashes: HashMap::new(),
            revision: None,
            commits: Vec::new(),
            embedding,
            vectors: HashMap::new(),
        }
    }

    pub fn build(root: &Path) -> io::Result<Self> {
        let provider = default_provider_from_env()?;
        Self::build_with_provider(root, provider)
    }

    pub fn build_with_provider(
        root: &Path,
        embedding: Arc<dyn EmbeddingProvider>,
    ) -> io::Result<Self> {
        let mut index = Self::with_embedding(embedding);
        index.rebuild(root)?;
        index.revision = git_revision(root);
        index.refresh_commits(root);
        Ok(index)
    }

    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    pub fn embedding_provider(&self) -> &str {
        self.embedding.name()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.values().map(Vec::len).sum()
    }

    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    /// Apply Git changes between revisions. Renames are handled as a delete
    /// followed by an update. A missing/unrelated history falls back to a
    /// safe worktree refresh.
    pub fn sync_git(&mut self, root: &Path, new_revision: &str) -> io::Result<()> {
        if new_revision.ends_with("-dirty") {
            self.sync_worktree(root)?;
            self.refresh_commits(root);
            self.revision = Some(new_revision.to_owned());
            return Ok(());
        }
        let Some(old_revision) = self.revision.clone() else {
            self.rebuild(root)?;
            self.revision = Some(new_revision.to_owned());
            self.refresh_commits(root);
            return Ok(());
        };
        // A dirty snapshot may contain edits that are absent from the next
        // commit diff (for example, the worktree was restored). Reconcile the
        // actual worktree before comparing clean revisions so stale terms are
        // removed and the clean SHA is recorded.
        if old_revision.ends_with("-dirty") {
            self.sync_worktree(root)?;
            self.refresh_commits(root);
            self.revision = Some(new_revision.to_owned());
            return Ok(());
        }
        let old_revision = old_revision.trim_end_matches("-dirty");
        let output = Command::new("git")
            .args([
                "-C",
                root.to_str().unwrap_or("."),
                "diff",
                "--name-status",
                "-z",
                "--find-renames",
                old_revision,
                new_revision.trim_end_matches("-dirty"),
            ])
            .output()?;
        if !output.status.success() {
            self.sync_worktree(root)?;
            self.refresh_commits(root);
            return Ok(());
        }
        let diff_output = String::from_utf8_lossy(&output.stdout);
        let fields: Vec<_> = diff_output
            .split('\0')
            .filter(|field| !field.is_empty())
            .collect();
        let mut cursor = 0;
        while cursor < fields.len() {
            let status = fields[cursor];
            cursor += 1;
            match status.chars().next() {
                Some('D') => {
                    if let Some(path) = fields.get(cursor) {
                        self.remove_file(Path::new(path));
                    }
                    cursor += 1;
                }
                Some('R') => {
                    if let Some(old_path) = fields.get(cursor) {
                        self.remove_file(Path::new(old_path));
                    }
                    cursor += 1;
                    if let Some(new_path) = fields.get(cursor) {
                        self.update_file(root, Path::new(new_path))?;
                    }
                    cursor += 1;
                }
                Some('C') => {
                    cursor += 1;
                    if let Some(new_path) = fields.get(cursor) {
                        self.update_file(root, Path::new(new_path))?;
                    }
                    cursor += 1;
                }
                Some('A' | 'M' | 'T') => {
                    if let Some(path) = fields.get(cursor) {
                        self.update_file(root, Path::new(path))?;
                    }
                    cursor += 1;
                }
                _ => cursor += 1,
            }
        }
        self.revision = Some(new_revision.to_owned());
        self.refresh_commits(root);
        Ok(())
    }

    /// Compare the current worktree against stored file hashes and only read
    /// changed files. This also removes files that disappeared outside Git.
    pub fn sync_worktree(&mut self, root: &Path) -> io::Result<()> {
        let mut current = HashMap::new();
        collect_files(root, root, &mut current)?;
        let previous: Vec<_> = self.file_hashes.keys().cloned().collect();
        for path in previous {
            if !current.contains_key(&path) {
                self.remove_file(&path);
            }
        }
        for (path, hash) in current {
            if self.file_hashes.get(&path) != Some(&hash) {
                self.update_file(root, &path)?;
            }
        }
        self.revision = git_revision(root);
        self.refresh_commits(root);
        Ok(())
    }

    pub fn rebuild(&mut self, root: &Path) -> io::Result<()> {
        self.files.clear();
        self.terms.clear();
        self.chunks.clear();
        self.file_hashes.clear();
        self.vectors.clear();
        self.commits.clear();
        self.walk(root, root)?;
        self.refresh_commits(root);
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
            if let Some(content) = read_source(&path)? {
                self.add_file(relative.to_path_buf(), content)?;
            }
        }
        Ok(())
    }

    pub fn remove_file(&mut self, relative: &Path) {
        self.files.remove(relative);
        self.file_hashes.remove(relative);
        self.chunks.remove(relative);
        self.vectors.remove(relative);
        self.terms.retain(|_, refs| {
            refs.retain(|(p, _)| p != relative);
            !refs.is_empty()
        });
    }

    pub fn analytics(&self) -> String {
        let file_count = self.files.len();
        let term_count = self.terms.len();
        let total_lines: usize = self.files.values().map(Vec::len).sum();
        let chunk_count = self.chunk_count();
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
            r#"{{"files": {}, "lines": {}, "chunks": {}, "commits": {}, "unique_terms": {}, "embedding_provider": "{}", "embedding_dimension": {}, "top_terms": "{}"}}"#,
            file_count,
            total_lines,
            chunk_count,
            self.commit_count(),
            term_count,
            json_escape(self.embedding.name()),
            self.embedding.dimension(),
            json_escape(&top_terms_str)
        )
    }

    /// Save source content and metadata in a portable, deterministic index
    /// format. Embeddings are recomputed on load through the provider seam.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = String::from("RI_INDEX_V2\n");
        output.push_str("revision\t");
        output.push_str(&hex_encode(self.revision.as_deref().unwrap_or("")));
        output.push('\n');
        output.push_str("provider\t");
        output.push_str(&hex_encode(self.embedding.name()));
        output.push('\n');
        output.push_str("dimension\t");
        output.push_str(&hex_encode(&self.embedding.dimension().to_string()));
        output.push('\n');
        output.push_str("normalized\t");
        output.push_str(&hex_encode("true"));
        output.push('\n');
        let mut paths: Vec<_> = self.files.keys().collect();
        paths.sort();
        for relative in paths {
            let content = self
                .files
                .get(relative)
                .map(|lines| lines.join("\n"))
                .unwrap_or_default();
            output.push_str("file\t");
            output.push_str(&hex_encode(&relative.to_string_lossy()));
            output.push('\t');
            output.push_str(&hex_encode(&content));
            output.push('\n');
        }
        for commit in &self.commits {
            output.push_str("commit\t");
            output.push_str(&hex_encode(&commit.sha));
            output.push('\t');
            output.push_str(&hex_encode(&commit.date));
            output.push('\t');
            output.push_str(&hex_encode(&commit.subject));
            output.push('\n');
        }
        fs::write(path, output)
    }

    pub fn load_from(path: &Path) -> io::Result<Self> {
        let provider = default_provider_from_env()?;
        Self::load_from_with_embedding(path, provider)
    }

    pub fn load_from_with_embedding(
        path: &Path,
        embedding: Arc<dyn EmbeddingProvider>,
    ) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut lines = content.lines();
        let header = lines.next().unwrap_or("");
        if header != "RI_INDEX_V2" && header != "RI_INDEX_V1" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported repository-intelligence index format",
            ));
        }
        let mut index = Self::with_embedding(embedding);
        for line in lines {
            let fields: Vec<_> = line.split('\t').collect();
            match fields.as_slice() {
                ["revision", encoded] => {
                    let revision = hex_decode(encoded)?;
                    if !revision.is_empty() {
                        index.revision = Some(revision);
                    }
                }
                ["provider", encoded] => {
                    let saved_provider = hex_decode(encoded)?;
                    if !saved_provider.is_empty() && saved_provider != index.embedding.name() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "embedding provider mismatch: index specifies provider '{}', but current provider is '{}'. Re-index the repository or specify the matching provider.",
                                saved_provider,
                                index.embedding.name()
                            ),
                        ));
                    }
                }
                ["dimension", encoded] => {
                    let dim_str = hex_decode(encoded)?;
                    if let Ok(dim) = dim_str.parse::<usize>() {
                        if dim != index.embedding.dimension() {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!(
                                    "embedding dimension mismatch: index specifies {}, but provider '{}' has {}",
                                    dim,
                                    index.embedding.name(),
                                    index.embedding.dimension()
                                ),
                            ));
                        }
                    }
                }
                ["normalized", _] => {}
                ["file", encoded_path, encoded_content] => {
                    if encoded_path.len() > 4096 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "index path field exceeds safety limit",
                        ));
                    }
                    let relative = PathBuf::from(hex_decode(encoded_path)?);
                    let source = hex_decode(encoded_content)?;
                    let file_name = relative
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("");
                    if is_safe_relative(&relative)
                        && is_indexable(&relative)
                        && !is_sensitive_file_name(file_name)
                    {
                        index.remove_file(&relative);
                        index.add_file(relative, source)?;
                    }
                }
                ["commit", encoded_sha, encoded_date, encoded_subject] => {
                    index.commits.push(CommitRecord {
                        sha: hex_decode(encoded_sha)?,
                        date: hex_decode(encoded_date)?,
                        subject: hex_decode(encoded_subject)?,
                    });
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "malformed repository-intelligence index record",
                    ));
                }
            }
        }
        Ok(index)
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

    pub fn search_evidence(&self, query: &str, limit: usize, mode: RetrievalMode) -> Vec<Evidence> {
        if tokenize(query).is_empty() || limit == 0 {
            return Vec::new();
        }
        match mode {
            RetrievalMode::Lexical => self.lexical_evidence(query, limit),
            RetrievalMode::Semantic => self.semantic_evidence(query, limit),
            RetrievalMode::Hybrid => self.hybrid_evidence(query, limit),
        }
    }

    pub fn build_evidence(&self, query: &str, limit: usize) -> Vec<Evidence> {
        let mut evidence = self.search_evidence(query, limit, RetrievalMode::Hybrid);
        if limit == 0 {
            return evidence;
        }
        // Grounded answer context must contain a lexical anchor. A vector
        // provider may legitimately rank a semantically similar distractor
        // above the exact source span, so preserve one lexical hit explicitly.
        if let Some(anchor) = self.lexical_evidence(query, 1).into_iter().next() {
            let already_present = evidence
                .iter()
                .any(|item| item.path == anchor.path && item.start_line == anchor.start_line);
            if !already_present {
                if evidence.len() >= limit {
                    evidence.pop();
                }
                evidence.insert(0, anchor);
            }
        }
        evidence
    }

    pub fn answer_context(&self, query: &str, limit: usize) -> Option<String> {
        let evidence = self.build_evidence(query, limit);
        // The deterministic hash embedding is deliberately conservative: an
        // answer requires at least one lexical anchor as well as fused hits.
        // This prevents an unrelated positive vector collision from becoming
        // model context for an unanswerable question.
        if evidence.is_empty() || self.lexical_evidence(query, 1).is_empty() {
            return None;
        }
        Some(format_evidence(&evidence))
    }

    pub fn search_commits(&self, query: &str, limit: usize) -> Vec<CommitRecord> {
        let terms = tokenize(query);
        if terms.is_empty() {
            return Vec::new();
        }
        let mut ranked: Vec<(usize, &CommitRecord)> = self
            .commits
            .iter()
            .filter_map(|commit| {
                let text = format!("{} {}", commit.subject, commit.date);
                let score = terms
                    .iter()
                    .map(|term| {
                        tokenize(&text)
                            .iter()
                            .filter(|token| *token == term)
                            .count()
                    })
                    .sum::<usize>();
                (score > 0).then_some((score, commit))
            })
            .collect();
        ranked.sort_by(|(a_score, a), (b_score, b)| {
            b_score.cmp(a_score).then_with(|| a.sha.cmp(&b.sha))
        });
        ranked
            .into_iter()
            .take(limit)
            .map(|(_, commit)| commit.clone())
            .collect()
    }

    fn lexical_evidence(&self, query: &str, limit: usize) -> Vec<Evidence> {
        let terms = tokenize(query);
        let mut scored: Vec<(usize, &Chunk)> = self
            .chunks
            .values()
            .flatten()
            .filter_map(|chunk| {
                let score = terms
                    .iter()
                    .map(|term| tokenize(&chunk.text).iter().filter(|t| *t == term).count())
                    .sum::<usize>();
                (score > 0).then_some((score, chunk))
            })
            .collect();
        scored.sort_by(|(a_score, a), (b_score, b)| {
            b_score
                .cmp(a_score)
                .then_with(|| a.path.cmp(&b.path))
                .then(a.start_line.cmp(&b.start_line))
        });
        scored
            .into_iter()
            .take(limit)
            .map(|(score, chunk)| evidence_from_chunk(chunk, score as f32, "lexical"))
            .collect()
    }

    fn semantic_evidence(&self, query: &str, limit: usize) -> Vec<Evidence> {
        let query_vector = match self.embedding.embed(query) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("semantic embedding failed: {err}");
                return Vec::new();
            }
        };
        let mut scored = Vec::new();
        for (path, chunks) in &self.chunks {
            let Some(vectors) = self.vectors.get(path) else {
                continue;
            };
            for (chunk, vector) in chunks.iter().zip(vectors) {
                let score = cosine(&query_vector, vector);
                if score > 0.0 {
                    scored.push((score, chunk));
                }
            }
        }
        scored.sort_by(|(a_score, a), (b_score, b)| {
            b_score
                .partial_cmp(a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
                .then(a.start_line.cmp(&b.start_line))
        });
        scored
            .into_iter()
            .take(limit)
            .map(|(score, chunk)| evidence_from_chunk(chunk, score, "semantic"))
            .collect()
    }

    fn hybrid_evidence(&self, query: &str, limit: usize) -> Vec<Evidence> {
        let candidate_limit = limit.saturating_mul(4).max(20);
        let lexical = self.lexical_evidence(query, candidate_limit);
        let semantic = self.semantic_evidence(query, candidate_limit);
        let mut fused: HashMap<(PathBuf, usize), (f32, Evidence)> = HashMap::new();
        // Reciprocal Rank Fusion uses a documented constant k=60; no tuned
        // magic weight can hide whether either retriever helped.
        for (rank, item) in lexical.into_iter().enumerate() {
            let key = (item.path.clone(), item.start_line);
            let entry = fused.entry(key).or_insert((0.0, item.clone()));
            entry.0 += 1.0 / (60.0 + rank as f32 + 1.0);
            entry.1.source = "hybrid".to_owned();
        }
        for (rank, item) in semantic.into_iter().enumerate() {
            let key = (item.path.clone(), item.start_line);
            let entry = fused.entry(key).or_insert((0.0, item.clone()));
            entry.0 += 1.0 / (60.0 + rank as f32 + 1.0);
            entry.1.source = "hybrid".to_owned();
        }
        let mut ranked: Vec<_> = fused.into_values().collect();
        ranked.sort_by(|(a_score, a), (b_score, b)| {
            b_score
                .partial_cmp(a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
                .then(a.start_line.cmp(&b.start_line))
        });
        ranked
            .into_iter()
            .take(limit)
            .map(|(score, mut evidence)| {
                evidence.score = score;
                evidence
            })
            .collect()
    }

    fn walk(&mut self, root: &Path, dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
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
                if file_name.starts_with('.')
                    || matches!(file_name, "target" | "node_modules" | "build" | "dist")
                {
                    continue;
                }
                self.walk(root, &path)?;
            } else if is_indexable(rel) && !is_sensitive_file_name(file_name) {
                if let Some(content) = read_source(&path)? {
                    self.add_file(rel.to_path_buf(), content)?;
                }
            }
        }
        Ok(())
    }

    fn add_file(&mut self, relative: PathBuf, content: String) -> io::Result<()> {
        let hash = stable_hash(content.as_bytes());
        let lines: Vec<String> = content.lines().map(String::from).collect();
        for (number, line) in lines.iter().enumerate() {
            for term in tokenize(line) {
                self.terms
                    .entry(term)
                    .or_default()
                    .insert((relative.clone(), number + 1));
            }
        }
        let chunks = chunk_file(&relative, &lines);
        let mut vectors = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            vectors.push(self.embedding.embed(&chunk.text)?);
        }
        self.file_hashes.insert(relative.clone(), hash);
        self.chunks.insert(relative.clone(), chunks);
        self.vectors.insert(relative.clone(), vectors);
        self.files.insert(relative, lines);
        Ok(())
    }

    fn refresh_commits(&mut self, root: &Path) {
        self.commits.clear();
        let Ok(output) = Command::new("git")
            .args([
                "-C",
                root.to_str().unwrap_or("."),
                "log",
                "-n",
                "100",
                "--date=short",
                "--format=%H%x09%ad%x09%s",
            ])
            .output()
        else {
            return;
        };
        if !output.status.success() {
            return;
        }
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some((sha, rest)) = line.split_once('\t') else {
                continue;
            };
            let Some((date, subject)) = rest.split_once('\t') else {
                continue;
            };
            self.commits.push(CommitRecord {
                sha: sha.to_owned(),
                date: date.to_owned(),
                subject: subject.to_owned(),
            });
        }
    }
}

fn evidence_from_chunk(chunk: &Chunk, score: f32, source: &str) -> Evidence {
    Evidence {
        path: chunk.path.clone(),
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        score,
        source: source.to_owned(),
        kind: chunk.kind.clone(),
        symbol: chunk.symbol.clone(),
        text: chunk.text.clone(),
    }
}

pub fn format_evidence(evidence: &[Evidence]) -> String {
    evidence
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            format!(
                "[E{}] [{}] {}\n{}",
                idx + 1,
                item.citation(),
                item.kind,
                item.text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn chunk_file(path: &Path, lines: &[String]) -> Vec<Chunk> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut declarations = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some((kind, symbol)) = declaration(line) {
            declarations.push((index, kind, symbol));
        }
    }
    if declarations.is_empty() {
        return lines
            .chunks(40)
            .enumerate()
            .map(|(chunk_index, block)| {
                let start = chunk_index * 40 + 1;
                Chunk {
                    path: path.to_path_buf(),
                    start_line: start,
                    end_line: start + block.len() - 1,
                    text: block.join("\n"),
                    kind: "text".to_owned(),
                    symbol: None,
                }
            })
            .collect();
    }
    let mut chunks = Vec::new();
    let mut cursor = 0;
    for (position, (start, kind, symbol)) in declarations.iter().enumerate() {
        if *start > cursor {
            chunks.extend(generic_chunks(path, &lines[cursor..*start], cursor + 1));
        }
        let next = declarations
            .get(position + 1)
            .map(|item| item.0)
            .unwrap_or(lines.len());
        let end = next.min(start + 80);
        let block = &lines[*start..end];
        chunks.push(Chunk {
            path: path.to_path_buf(),
            start_line: *start + 1,
            end_line: *start + block.len(),
            text: block.join("\n"),
            kind: kind.clone(),
            symbol: symbol.clone(),
        });
        cursor = end;
    }
    if cursor < lines.len() {
        chunks.extend(generic_chunks(path, &lines[cursor..], cursor + 1));
    }
    chunks
}

fn generic_chunks(path: &Path, lines: &[String], first_line: usize) -> Vec<Chunk> {
    lines
        .chunks(40)
        .enumerate()
        .map(|(chunk_index, block)| {
            let start = first_line + chunk_index * 40;
            Chunk {
                path: path.to_path_buf(),
                start_line: start,
                end_line: start + block.len() - 1,
                text: block.join("\n"),
                kind: "text".to_owned(),
                symbol: None,
            }
        })
        .collect()
}

fn declaration(line: &str) -> Option<(String, Option<String>)> {
    let trimmed = line.trim_start();
    let patterns = [
        ("function", "fn "),
        ("function", "def "),
        ("class", "class "),
        ("struct", "struct "),
        ("trait", "trait "),
        ("impl", "impl "),
        ("function", "function "),
    ];
    for (kind, marker) in patterns {
        if let Some(position) = trimmed.find(marker) {
            if position > 0 && trimmed.as_bytes()[position - 1].is_ascii_alphanumeric() {
                continue;
            }
            let rest = &trimmed[position + marker.len()..];
            let name: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            return Some((kind.to_owned(), (!name.is_empty()).then_some(name)));
        }
    }
    None
}

fn collect_files(root: &Path, dir: &Path, files: &mut HashMap<PathBuf, u64>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if meta.file_type().is_symlink() {
                continue;
            }
        }
        let relative = path.strip_prefix(root).expect("walked under root");
        if !is_safe_relative(relative) {
            continue;
        }
        let name = relative
            .file_name()
            .and_then(|item| item.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if !name.starts_with('.')
                && !matches!(name, "target" | "node_modules" | "build" | "dist")
            {
                collect_files(root, &path, files)?;
            }
        } else if is_indexable(relative) && !is_sensitive_file_name(name) {
            if let Some(content) = read_source(&path)? {
                files.insert(relative.to_path_buf(), stable_hash(content.as_bytes()));
            }
        }
    }
    Ok(())
}

fn read_source(path: &Path) -> io::Result<Option<String>> {
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(content) => Ok(Some(content)),
        Err(_) => Ok(None),
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
        Some("png" | "jpg" | "jpeg" | "gif" | "lock" | "bin" | "ico" | "pdf" | "zip" | "ri")
    )
}

fn is_sensitive_file_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (name.starts_with('.') && name != ".github" && name != ".gitignore")
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.ends_with(".p12")
        || lower.ends_with(".pfx")
        || lower.ends_with(".keystore")
        || matches!(
            lower.as_str(),
            "id_rsa"
                | "id_ed25519"
                | "credentials.json"
                | "service-account.json"
                | ".npmrc"
                | ".netrc"
                | ".env"
        )
        || lower.starts_with(".env.")
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 2)
        .map(str::to_ascii_lowercase)
        .collect()
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> io::Result<String> {
    if value.len() % 2 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "odd-length hex field",
        ));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let text = std::str::from_utf8(pair)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid hex field"))?;
        let byte = u8::from_str_radix(text, 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid hex field"))?;
        bytes.push(byte);
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-utf8 index field"))
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 16);
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn http_post_embed(
    endpoint: &str,
    model: &str,
    text: &str,
    expected_dim: usize,
) -> io::Result<Vec<f32>> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    if endpoint.starts_with("https://") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "HTTPS endpoint is not supported for local plaintext Ollama HTTP connection; specify http:// or host:port",
        ));
    }

    let host_port = endpoint.trim_start_matches("http://").trim_end_matches('/');

    let addr = host_port.to_socket_addrs()?.next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::AddrNotAvailable, "invalid endpoint address")
    })?;

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let escaped_text = json_escape(text);
    let escaped_model = json_escape(model);
    let body = format!(
        r#"{{"model":"{}","input":"{}"}}"#,
        escaped_model, escaped_text
    );

    let request = format!(
        "POST /api/embed HTTP/1.0\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        host_port,
        body.len(),
        body
    );

    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    (&mut stream)
        .take(10 * 1024 * 1024)
        .read_to_end(&mut response)?;

    let response_str = String::from_utf8_lossy(&response);
    let mut header_and_body = response_str.splitn(2, "\r\n\r\n");
    let header = header_and_body.next().unwrap_or("");
    let body_str = header_and_body.next().unwrap_or("");

    let status_line = header.lines().next().unwrap_or("");
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    if status_code != 200 {
        return Err(io::Error::other(format!(
            "Ollama HTTP error (status {}): {}\nbody: {}",
            status_code,
            status_line,
            body_str.chars().take(200).collect::<String>()
        )));
    }

    let vector_str = if let Some(start_idx) = body_str.find("\"embeddings\":[[") {
        let slice = &body_str[start_idx + "\"embeddings\":[[".len()..];
        let end_idx = slice.find("]]").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "malformed embeddings array in Ollama response",
            )
        })?;
        &slice[..end_idx]
    } else if let Some(start_idx) = body_str.find("\"embedding\":[") {
        let slice = &body_str[start_idx + "\"embedding\":[".len()..];
        let end_idx = slice.find(']').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "malformed embedding array in Ollama response",
            )
        })?;
        &slice[..end_idx]
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "no embeddings found in Ollama response. status: '{}', body: '{}'",
                status_line,
                body_str.chars().take(200).collect::<String>()
            ),
        ));
    };

    let mut vector = Vec::with_capacity(expected_dim);
    for token in vector_str.split(',') {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            let val = trimmed.parse::<f32>().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to parse embedding float '{}': {}", trimmed, e),
                )
            })?;
            if val.is_nan() || val.is_infinite() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "embedding vector contains NaN or infinite value",
                ));
            }
            vector.push(val);
        }
    }

    if vector.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "empty embedding vector received from provider",
        ));
    }
    if vector.len() != expected_dim {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "embedding dimension mismatch: expected {} dimensions from model '{}', but received {}",
                expected_dim, model, vector.len()
            ),
        ));
    }

    normalize(&mut vector);
    Ok(vector)
}

impl fmt::Display for RetrievalMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
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
        assert!(index.commit_count() >= 2);
        assert_eq!(index.search_commits("changed", 5).len(), 1);
    }

    #[test]
    fn analytics_returns_chunk_and_embedding_stats() {
        let root = fixture();
        let index = Index::build(&root).unwrap();
        let stats = index.analytics();
        assert!(stats.contains(r#""files": 1"#));
        assert!(stats.contains(r#""chunks": 1"#));
        assert!(stats.contains("hash-token-v1"));
    }

    #[test]
    fn semantic_and_hybrid_return_citable_evidence() {
        let root = fixture();
        let index = Index::build(&root).unwrap();
        let semantic = index.search_evidence("bounded worker", 5, RetrievalMode::Semantic);
        assert!(!semantic.is_empty());
        assert_eq!(semantic[0].citation(), "lib.rs:1");
        let hybrid = index.build_evidence("bounded worker", 5);
        assert!(!hybrid.is_empty());
        assert_eq!(hybrid[0].source, "hybrid");
    }

    #[test]
    fn no_evidence_is_explicitly_unanswerable() {
        let root = fixture();
        let index = Index::build(&root).unwrap();
        assert!(index
            .answer_context("quantum database migration", 5)
            .is_none());
    }

    #[test]
    fn save_and_load_preserve_search_and_revision() {
        let root = fixture();
        let index = Index::build(&root).unwrap();
        let path = root.join("ri.index");
        index.save_to(&path).unwrap();
        let loaded = Index::load_from(&path).unwrap();
        assert_eq!(
            loaded.search("bounded worker", 5),
            index.search("bounded worker", 5)
        );
        assert_eq!(loaded.revision(), index.revision());
    }

    #[test]
    fn loading_an_index_reapplies_sensitive_file_policy() {
        let root = fixture();
        let mut index = Index::build(&root).unwrap();
        index
            .add_file(
                PathBuf::from("credentials.json"),
                "should-not-leak".to_owned(),
            )
            .unwrap();
        let path = root.join("saved.ri");
        index.save_to(&path).unwrap();
        let loaded = Index::load_from(&path).unwrap();
        assert!(loaded.search("should not leak", 5).is_empty());
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
        index
            .add_file(PathBuf::from("credentials.json"), "legacycanary".to_owned())
            .unwrap();
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
