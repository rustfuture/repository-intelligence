use crate::Evidence;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationRef {
    EvidenceId(usize),
    Span {
        path: String,
        start: usize,
        end: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationError {
    Empty,
    Malformed(String),
    ReversedRange(usize, usize),
    NonNumeric(String),
    EmptyPath,
}

impl std::fmt::Display for CitationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty citation"),
            Self::Malformed(s) => write!(f, "malformed citation: {s}"),
            Self::ReversedRange(s, e) => write!(f, "reversed range {s}-{e}"),
            Self::NonNumeric(s) => write!(f, "non-numeric line range: {s}"),
            Self::EmptyPath => write!(f, "empty file path in citation"),
        }
    }
}

/// Extract and parse all bracketed citation expressions `[...]` from a single line.
pub fn parse_citations(line: &str) -> Vec<Result<CitationRef, CitationError>> {
    let mut results = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(close_offset) = line[i + 1..].find(']') {
                let close_idx = i + 1 + close_offset;
                let inside = line[i + 1..close_idx].trim();
                if looks_like_citation(inside) {
                    results.push(parse_single_citation(inside));
                }
                i = close_idx + 1;
            } else {
                let rest = &line[i + 1..];
                if looks_like_citation(rest) {
                    results.push(Err(CitationError::Malformed(format!("[{rest}"))));
                }
                break;
            }
        } else {
            i += 1;
        }
    }
    results
}

fn looks_like_citation(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    // [E1], [E12]
    if (s.starts_with('E') || s.starts_with('e'))
        && s[1..].chars().all(|c| c.is_ascii_digit())
        && s.len() > 1
    {
        return true;
    }
    // [path:line] or [path:start-end]
    if s.contains(':') {
        return true;
    }
    // Common file extensions without colon (e.g. malformed citation attempt)
    if s.ends_with(".rs") || s.ends_with(".md") || s.ends_with(".toml") || s.ends_with(".json") {
        return true;
    }
    false
}

fn parse_single_citation(inside: &str) -> Result<CitationRef, CitationError> {
    let trimmed = inside.trim();
    if trimmed.is_empty() {
        return Err(CitationError::Empty);
    }
    // Check [E1], [E2]
    if (trimmed.starts_with('E') || trimmed.starts_with('e'))
        && trimmed[1..].chars().all(|c| c.is_ascii_digit())
        && trimmed.len() > 1
    {
        let id = trimmed[1..]
            .parse::<usize>()
            .map_err(|_| CitationError::NonNumeric(trimmed.to_string()))?;
        if id == 0 {
            return Err(CitationError::Malformed(
                "Evidence ID cannot be 0".to_string(),
            ));
        }
        return Ok(CitationRef::EvidenceId(id));
    }

    // Check path:start-end or path:line
    let (path_part, range_part) = match trimmed.rsplit_once(':') {
        Some((p, r)) => (p.trim(), r.trim()),
        None => return Err(CitationError::Malformed(trimmed.to_string())),
    };

    let clean_path = path_part.trim_start_matches("./");
    if clean_path.is_empty() {
        return Err(CitationError::EmptyPath);
    }

    let parts: Vec<&str> = range_part.split('-').collect();
    match parts.as_slice() {
        [single] => {
            let line = single
                .parse::<usize>()
                .map_err(|_| CitationError::NonNumeric((*single).to_string()))?;
            if line == 0 {
                return Err(CitationError::Malformed(
                    "line number cannot be 0".to_string(),
                ));
            }
            Ok(CitationRef::Span {
                path: clean_path.to_string(),
                start: line,
                end: line,
            })
        }
        [start_str, end_str] => {
            let start = start_str
                .parse::<usize>()
                .map_err(|_| CitationError::NonNumeric((*start_str).to_string()))?;
            let end = end_str
                .parse::<usize>()
                .map_err(|_| CitationError::NonNumeric((*end_str).to_string()))?;
            if start == 0 {
                return Err(CitationError::Malformed(
                    "start line cannot be 0".to_string(),
                ));
            }
            if end < start {
                return Err(CitationError::ReversedRange(start, end));
            }
            Ok(CitationRef::Span {
                path: clean_path.to_string(),
                start,
                end,
            })
        }
        _ => Err(CitationError::Malformed(format!(
            "too many range components in '{range_part}'"
        ))),
    }
}

const STOPWORDS: &[&str] = &[
    "a",
    "about",
    "above",
    "after",
    "again",
    "against",
    "all",
    "am",
    "an",
    "and",
    "any",
    "are",
    "aren't",
    "as",
    "at",
    "be",
    "because",
    "been",
    "before",
    "being",
    "below",
    "between",
    "both",
    "but",
    "by",
    "can",
    "cannot",
    "could",
    "couldn't",
    "did",
    "didn't",
    "do",
    "does",
    "doesn't",
    "doing",
    "don't",
    "down",
    "during",
    "each",
    "few",
    "for",
    "from",
    "further",
    "had",
    "hadn't",
    "has",
    "hasn't",
    "have",
    "haven't",
    "having",
    "he",
    "her",
    "here",
    "hers",
    "herself",
    "him",
    "himself",
    "his",
    "how",
    "i",
    "if",
    "in",
    "into",
    "is",
    "isn't",
    "it",
    "it's",
    "its",
    "itself",
    "let's",
    "me",
    "more",
    "most",
    "mustn't",
    "my",
    "myself",
    "no",
    "nor",
    "not",
    "of",
    "off",
    "on",
    "once",
    "only",
    "or",
    "other",
    "ought",
    "our",
    "ours",
    "ourselves",
    "out",
    "over",
    "own",
    "same",
    "shan't",
    "she",
    "should",
    "shouldn't",
    "so",
    "some",
    "such",
    "than",
    "that",
    "the",
    "their",
    "theirs",
    "them",
    "themselves",
    "then",
    "there",
    "these",
    "they",
    "this",
    "those",
    "through",
    "to",
    "too",
    "under",
    "until",
    "up",
    "very",
    "was",
    "wasn't",
    "we",
    "were",
    "weren't",
    "what",
    "when",
    "where",
    "which",
    "while",
    "who",
    "whom",
    "why",
    "with",
    "won't",
    "would",
    "wouldn't",
    "you",
    "your",
    "yours",
    "yourself",
    "yourselves",
    // LLM conversational filler words
    "according",
    "code",
    "snippet",
    "statement",
    "evidence",
    "file",
    "function",
    "shows",
    "states",
    "indicates",
    "returns",
    "defined",
    "contains",
    "value",
];

pub fn resolve_citation<'a>(
    citation: &CitationRef,
    evidence: &'a [Evidence],
) -> Option<&'a Evidence> {
    match citation {
        CitationRef::EvidenceId(id) => {
            if *id >= 1 && *id <= evidence.len() {
                Some(&evidence[*id - 1])
            } else {
                None
            }
        }
        CitationRef::Span { path, start, end } => {
            let norm_path = Path::new(path.trim_start_matches("./"));
            evidence.iter().find(|item| {
                let item_norm =
                    Path::new(item.path.to_str().unwrap_or("").trim_start_matches("./"));
                item_norm == norm_path
                    && *start >= item.start_line
                    && *end <= item.end_line
                    && *start <= *end
            })
        }
    }
}

pub fn citation_matches_evidence(citation: &CitationRef, evidence: &[Evidence]) -> bool {
    resolve_citation(citation, evidence).is_some()
}

pub fn extract_claim_text(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut in_bracket = false;
    for c in line.chars() {
        if c == '[' {
            in_bracket = true;
        } else if c == ']' {
            in_bracket = false;
        } else if !in_bracket {
            result.push(c);
        }
    }
    result.trim().to_string()
}

pub fn tokenize_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map(|t| t.trim_matches('_').to_ascii_lowercase())
        .filter(|t| t.len() >= 2)
        .collect()
}

/// Verdict for a single atomic claim against one evidence span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportOutcome {
    Supported,
    Unsupported(String),
}

/// Per-line verification record that keeps *citation resolution* (does the cited
/// span exist in the retrieved evidence?) separate from *claim support* (does
/// the cited span actually entail the claim?).
///
/// Citation presence alone is **not** correctness evidence. A claim may resolve
/// to a real retrieved span and still be unsupported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimAssessment {
    pub claim: String,
    pub citations: Vec<String>,
    pub citation_resolved: bool,
    pub supported: bool,
    pub reason: String,
}

/// Structured verification result for a whole answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerAssessment {
    pub accepted: bool,
    pub verified_text: String,
    pub declared_citations: usize,
    pub resolved_citations: usize,
    pub verified_citations: usize,
    pub claims: Vec<ClaimAssessment>,
    pub reason: String,
}

/// Explicit negation words. Intentionally narrow: words such as "rejects" or
/// "invalid" are not treated as polarity markers because they carry their own
/// meaning and over-triggering them caused false rejects.
const NEGATION_CUES: &[&str] = &[
    "not", "no", "never", "without", "cannot", "cant", "dont", "doesnt", "isnt", "arent", "wont",
    "shouldnt", "nor", "neither",
];

fn is_negation_cue(token: &str) -> bool {
    NEGATION_CUES.contains(&token)
}

/// Tokenize while turning code negation `!` into an explicit `not` token so that
/// `!is_symlink` reads as a negated predicate.
fn tokenize_with_negation(text: &str) -> Vec<String> {
    tokenize_words(&text.replace('!', " not "))
}

/// True when the two tokens are the same word or a conservative morphological
/// variant: a shared prefix of at least four characters covering at least half
/// of the shorter token. This deliberately does **not** relate `safe` to
/// `unsafe`, so a claim cannot borrow support from a negated form.
fn tokens_related(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let shortest = left.len().min(right.len());
    if shortest < 4 {
        return false;
    }
    let shared = left
        .bytes()
        .zip(right.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    shared >= 4 && shared * 2 >= shortest
}

fn normalize_number(raw: &str) -> String {
    let raw = raw.trim_end_matches('.');
    match raw.split_once('.') {
        Some((int_part, frac)) => {
            let int_trimmed = int_part.trim_start_matches('0');
            let int_out = if int_trimmed.is_empty() {
                "0"
            } else {
                int_trimmed
            };
            let frac_trimmed = frac.trim_end_matches('0');
            if frac_trimmed.is_empty() {
                int_out.to_string()
            } else {
                format!("{int_out}.{frac_trimmed}")
            }
        }
        None => {
            let trimmed = raw.trim_start_matches('0');
            if trimmed.is_empty() {
                "0".to_string()
            } else {
                trimmed.to_string()
            }
        }
    }
}

fn extract_numbers(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let raw: String = chars[start..i].iter().collect();
            out.push(normalize_number(&raw));
        } else {
            i += 1;
        }
    }
    out
}

fn extract_quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for quote in ['"', '\'', '`'] {
        let mut parts = text.split(quote);
        let _ = parts.next();
        while let (Some(inner), Some(_)) = (parts.next(), parts.next()) {
            let inner = inner.trim();
            if inner.len() >= 3 && inner.chars().any(|c| c.is_ascii_alphabetic()) {
                out.push(inner.to_ascii_lowercase());
            }
        }
    }
    out
}

/// Identifier-like tokens that must appear verbatim in the cited evidence:
/// underscore names, ALL-CAPS names, and letter/digit mixes such as `f32`.
fn is_identifier_anchor(token: &str) -> bool {
    if token.len() < 3 {
        return false;
    }
    if token.contains('_') {
        return true;
    }
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    let has_alpha = token.chars().any(|c| c.is_ascii_alphabetic());
    let all_upper = token.chars().all(|c| !c.is_ascii_lowercase());
    (has_digit && has_alpha) || all_upper
}

/// Split a claim into atomic clauses so that one fabricated conjunct cannot hide
/// behind a supported one.
fn split_clauses(claim: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    for sentence in claim.split(['.', ';', '!', '?', '\n']) {
        for part in sentence.split(',') {
            for sub in part.split(" and ") {
                let trimmed = sub.trim();
                if !trimmed.is_empty() {
                    clauses.push(trimmed.to_string());
                }
            }
        }
    }
    clauses
}

fn negation_flags(tokens: &[String]) -> Vec<bool> {
    // A two-token window. A wider window produced false rejects on ordinary code:
    // `if !header.starts_with("RI_INDEX_V1")` would mark the quoted constant as
    // negated. Scope handling is inherently approximate without a parser; this is
    // the conservative middle ground.
    tokens
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let start = index.saturating_sub(2);
            tokens[start..index].iter().any(|t| is_negation_cue(t))
        })
        .collect()
}

/// True when `candidate` is the negated/antonym form of `base` (`unsafe` for
/// `safe`). This blocks a claim from being grounded by its opposite even when the
/// rest of the sentence overlaps.
fn is_negated_form_of(candidate: &str, base: &str) -> bool {
    ["un", "in", "im", "dis", "non", "ir", "anti"]
        .iter()
        .any(|prefix| {
            candidate.starts_with(prefix)
                && candidate.len() > prefix.len() + 2
                && &candidate[prefix.len()..] == base
        })
}

fn clause_support(
    clause: &str,
    evidence: &Evidence,
    evidence_tokens: &[String],
) -> Result<(), String> {
    let claim_tokens = tokenize_with_negation(clause);
    let informative: Vec<(usize, &String)> = claim_tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| !STOPWORDS.contains(&token.as_str()) && token.len() >= 2)
        .collect();
    if informative.is_empty() {
        return Ok(());
    }

    let claim_negated = negation_flags(&claim_tokens);
    let evidence_negated = negation_flags(evidence_tokens);

    let mut matches: Vec<(usize, usize)> = Vec::new();
    for (claim_index, token) in &informative {
        if let Some(evidence_index) = evidence_tokens
            .iter()
            .position(|candidate| tokens_related(token, candidate))
        {
            if claim_negated[*claim_index] != evidence_negated[evidence_index] {
                return Err(format!(
                    "negation polarity differs between the claim and the cited evidence for '{token}'"
                ));
            }
            matches.push((*claim_index, evidence_index));
        }
    }

    if matches.is_empty() {
        return Err(format!(
            "no informative token from '{clause}' appears in the cited evidence"
        ));
    }

    // Antonym guard: an unmatched content word whose opposite form is present in
    // the evidence means the claim asserts the negation of what the source states.
    for (claim_index, token) in &informative {
        if matches.iter().any(|(index, _)| index == claim_index) {
            continue;
        }
        if let Some(opposite) = evidence_tokens
            .iter()
            .find(|candidate| is_negated_form_of(candidate, token))
        {
            return Err(format!(
                "claim states '{token}' while the cited evidence states its negated form '{opposite}'"
            ));
        }
    }

    if matches.len() * 2 < informative.len() {
        return Err(format!(
            "only {}/{} informative tokens from '{clause}' appear in the cited evidence",
            matches.len(),
            informative.len()
        ));
    }

    // Relation direction: matched claim tokens must appear in the same relative
    // order in the evidence. A reversed relation flips this order.
    let mut previous: Option<usize> = None;
    for (_, evidence_index) in &matches {
        if let Some(last) = previous {
            if *evidence_index < last {
                return Err(format!(
                    "claim reverses the relation order found in the cited evidence for '{clause}'"
                ));
            }
        }
        previous = Some(*evidence_index);
    }

    let _ = evidence;
    Ok(())
}

/// Conservative, dependency-free claim support check.
///
/// It is deliberately stricter than word overlap: numbers and identifiers must
/// appear verbatim, negation polarity must agree, matched terms must keep their
/// order, and every atomic clause must be at least half covered. Word overlap by
/// itself is not treated as proof of correctness, and a refusal here means "not
/// verified", not "false".
pub fn assess_claim_support(claim: &str, evidence_item: &Evidence) -> SupportOutcome {
    let claim = claim.trim();
    if claim.is_empty() {
        return SupportOutcome::Unsupported("claim is empty".to_string());
    }

    let mut evidence_text = evidence_item.text.clone();
    evidence_text.push(' ');
    evidence_text.push_str(&evidence_item.kind);
    if let Some(ref symbol) = evidence_item.symbol {
        evidence_text.push(' ');
        evidence_text.push_str(symbol);
    }
    if let Some(path) = evidence_item.path.to_str() {
        evidence_text.push(' ');
        evidence_text.push_str(path);
    }
    let evidence_tokens = tokenize_with_negation(&evidence_text);
    let evidence_numbers = extract_numbers(&evidence_text);
    let evidence_lower = evidence_text.to_ascii_lowercase();

    // 1. Hard anchors: numbers, quoted strings, identifier-like names.
    for number in extract_numbers(claim) {
        if !evidence_numbers.contains(&number) {
            return SupportOutcome::Unsupported(format!(
                "claim states the number '{number}', which is absent from the cited evidence"
            ));
        }
    }
    for quoted in extract_quoted(claim) {
        if !evidence_lower.contains(&quoted) {
            return SupportOutcome::Unsupported(format!(
                "claim quotes '{quoted}', which is absent from the cited evidence"
            ));
        }
    }
    for token in tokenize_words(claim) {
        if is_identifier_anchor(&token) && !evidence_tokens.iter().any(|t| t == &token) {
            return SupportOutcome::Unsupported(format!(
                "claim names '{token}', which is absent from the cited evidence"
            ));
        }
    }

    // 2. Atomic clauses: every clause must be at least half covered.
    for clause in split_clauses(claim) {
        if let Err(reason) = clause_support(&clause, evidence_item, &evidence_tokens) {
            return SupportOutcome::Unsupported(reason);
        }
    }

    SupportOutcome::Supported
}

pub fn evidence_supports_claim(claim: &str, evidence_item: &Evidence) -> bool {
    matches!(
        assess_claim_support(claim, evidence_item),
        SupportOutcome::Supported
    )
}

#[derive(Debug, PartialEq, Eq)]
pub enum VerificationResult {
    Accepted {
        verified_text: String,
        verified_citations: usize,
    },
    Refused {
        reason: String,
    },
}

/// Assess an answer and return the structured per-claim record.
pub fn assess_answer_citations(raw_answer: &str, evidence: &[Evidence]) -> AnswerAssessment {
    let trimmed = raw_answer.trim();
    if trimmed.is_empty() || trimmed.contains("Insufficient repository evidence") {
        return AnswerAssessment {
            accepted: false,
            verified_text: String::new(),
            declared_citations: 0,
            resolved_citations: 0,
            verified_citations: 0,
            claims: Vec::new(),
            reason: "Model signaled insufficient evidence or returned empty answer".to_string(),
        };
    }

    let mut verified_lines = Vec::new();
    let mut claims = Vec::new();
    let mut declared = 0usize;
    let mut resolved = 0usize;

    for line in trimmed.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }

        let citations = parse_citations(line_trimmed);
        let claim_text = extract_claim_text(line_trimmed);
        if citations.is_empty() {
            claims.push(ClaimAssessment {
                claim: claim_text,
                citations: Vec::new(),
                citation_resolved: false,
                supported: false,
                reason: "line contains an uncited assertion".to_string(),
            });
            return AnswerAssessment {
                accepted: false,
                verified_text: String::new(),
                declared_citations: declared,
                resolved_citations: resolved,
                verified_citations: 0,
                claims,
                reason: format!("Line contains uncited assertion: '{line_trimmed}'"),
            };
        }

        let mut line_citations = Vec::new();
        let mut line_supported = false;
        let mut line_reason = String::new();
        let mut line_resolved = true;

        for citation in citations {
            declared += 1;
            match citation {
                Ok(reference) => {
                    let rendered = format!("{reference:?}");
                    let resolved_item = resolve_citation(&reference, evidence);
                    match resolved_item {
                        Some(item) => {
                            resolved += 1;
                            line_citations.push(item.citation());
                            match assess_claim_support(&claim_text, item) {
                                SupportOutcome::Supported => line_supported = true,
                                SupportOutcome::Unsupported(reason) => {
                                    if line_reason.is_empty() {
                                        line_reason = reason;
                                    }
                                }
                            }
                        }
                        None => {
                            line_resolved = false;
                            line_reason =
                                format!("citation '{rendered}' is not found in retrieved evidence");
                            break;
                        }
                    }
                }
                Err(error) => {
                    line_resolved = false;
                    line_reason = format!("invalid citation format: {error}");
                    break;
                }
            }
        }

        let supported = line_resolved && line_supported;
        claims.push(ClaimAssessment {
            claim: claim_text.clone(),
            citations: line_citations,
            citation_resolved: line_resolved,
            supported,
            reason: if supported {
                "supported by the cited evidence".to_string()
            } else if line_reason.is_empty() {
                "no cited span supports this claim".to_string()
            } else {
                line_reason.clone()
            },
        });

        if !line_resolved {
            return AnswerAssessment {
                accepted: false,
                verified_text: String::new(),
                declared_citations: declared,
                resolved_citations: resolved,
                verified_citations: 0,
                claims,
                reason: format!("Citation could not be resolved on line: '{line_trimmed}'"),
            };
        }
        if !supported {
            return AnswerAssessment {
                accepted: false,
                verified_text: String::new(),
                declared_citations: declared,
                resolved_citations: resolved,
                verified_citations: 0,
                claims,
                reason: format!("Cited evidence does not support claim: '{claim_text}'"),
            };
        }
        verified_lines.push(line_trimmed.to_string());
    }

    if verified_lines.is_empty() || resolved == 0 {
        return AnswerAssessment {
            accepted: false,
            verified_text: String::new(),
            declared_citations: declared,
            resolved_citations: resolved,
            verified_citations: 0,
            claims,
            reason: "Zero valid citations verified across entire answer".to_string(),
        };
    }

    AnswerAssessment {
        accepted: true,
        verified_text: verified_lines.join("\n"),
        declared_citations: declared,
        resolved_citations: resolved,
        verified_citations: resolved,
        claims,
        reason: "all cited lines resolved and were supported by the cited evidence".to_string(),
    }
}

pub fn verify_answer_citations(raw_answer: &str, evidence: &[Evidence]) -> VerificationResult {
    let assessment = assess_answer_citations(raw_answer, evidence);
    if assessment.accepted {
        VerificationResult::Accepted {
            verified_text: assessment.verified_text,
            verified_citations: assessment.verified_citations,
        }
    } else {
        VerificationResult::Refused {
            reason: assessment.reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_evidence() -> Vec<Evidence> {
        vec![
            Evidence {
                path: PathBuf::from("security.rs"),
                start_line: 2,
                end_line: 4,
                score: 1.0,
                source: "hybrid".to_string(),
                kind: "fn validate_relative_path".to_string(),
                symbol: Some("validate_relative_path".to_string()),
                text: "pub fn validate_relative_path(path: &str) -> bool {\n    !path.starts_with('/') && !path.contains(\"..\") && !path.is_empty()\n}".to_string(),
            },
            Evidence {
                path: PathBuf::from("src/storage.rs"),
                start_line: 10,
                end_line: 25,
                score: 0.9,
                source: "hybrid".to_string(),
                kind: "struct Store".to_string(),
                symbol: Some("Store".to_string()),
                text: "pub struct Store {\n    pub count: usize,\n}".to_string(),
            },
        ]
    }

    #[test]
    fn parse_evidence_id() {
        let citations = parse_citations("This is proven by [E1].");
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0], Ok(CitationRef::EvidenceId(1)));
    }

    #[test]
    fn parse_path_line_and_span() {
        let citations = parse_citations("Checked at [security.rs:2-4] and [security.rs:3].");
        assert_eq!(citations.len(), 2);
        assert_eq!(
            citations[0],
            Ok(CitationRef::Span {
                path: "security.rs".to_string(),
                start: 2,
                end: 4,
            })
        );
        assert_eq!(
            citations[1],
            Ok(CitationRef::Span {
                path: "security.rs".to_string(),
                start: 3,
                end: 3,
            })
        );
    }

    #[test]
    fn parse_adjacent_citations() {
        let citations = parse_citations("Supported by [E1][E2] together.");
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0], Ok(CitationRef::EvidenceId(1)));
        assert_eq!(citations[1], Ok(CitationRef::EvidenceId(2)));
    }

    #[test]
    fn reject_reversed_range() {
        let citations = parse_citations("Invalid span [security.rs:10-4].");
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0], Err(CitationError::ReversedRange(10, 4)));
    }

    #[test]
    fn reject_malformed_range() {
        let citations = parse_citations("Malformed [security.rs:1-2-3].");
        assert_eq!(citations.len(), 1);
        assert!(matches!(citations[0], Err(CitationError::Malformed(_))));
    }

    #[test]
    fn reject_non_numeric() {
        let citations = parse_citations("Non numeric [security.rs:one-two].");
        assert_eq!(citations.len(), 1);
        assert!(matches!(citations[0], Err(CitationError::NonNumeric(_))));
    }

    fn api_evidence() -> Vec<Evidence> {
        vec![Evidence {
            path: PathBuf::from("api.rs"),
            start_line: 1,
            end_line: 3,
            score: 1.0,
            source: "hybrid".to_string(),
            kind: "api module".to_string(),
            symbol: None,
            text: "pub fn health() -> &'static str { \"status ok\" }\npub fn search(query: &str) -> &'static str { \"path line score source text\" }\npub fn reload(commit: &str) -> &'static str { \"reloaded commit\" }".to_string(),
        }]
    }

    #[test]
    fn verify_answer_accepts_valid() {
        let ev = sample_evidence();
        let ans = "The validate_relative_path function rejects empty paths [E1].\nStorage keeps a count field [src/storage.rs:10-20].";
        let res = verify_answer_citations(ans, &ev);
        assert!(
            matches!(res, VerificationResult::Accepted { .. }),
            "expected accepted, got: {res:?}"
        );
    }

    #[test]
    fn claim_support_accepts_grounded_statement() {
        let ev = api_evidence();
        assert_eq!(
            assess_claim_support("The health function returns status ok", &ev[0]),
            SupportOutcome::Supported
        );
    }

    #[test]
    fn claim_support_rejects_wrong_number() {
        let ev = api_evidence();
        let outcome = assess_claim_support("The health function returns 5 status codes", &ev[0]);
        assert!(
            matches!(outcome, SupportOutcome::Unsupported(_)),
            "fabricated number must be rejected, got: {outcome:?}"
        );
    }

    #[test]
    fn claim_support_rejects_reversed_negation() {
        let ev = api_evidence();
        let outcome = assess_claim_support("The health function does not return status ok", &ev[0]);
        assert!(
            matches!(outcome, SupportOutcome::Unsupported(_)),
            "reversed negation must be rejected, got: {outcome:?}"
        );
    }

    #[test]
    fn claim_support_rejects_reversed_relation() {
        let ev = api_evidence();
        let outcome = assess_claim_support("status ok returns the health function", &ev[0]);
        assert!(
            matches!(outcome, SupportOutcome::Unsupported(_)),
            "reversed relation must be rejected, got: {outcome:?}"
        );
    }

    #[test]
    fn claim_support_rejects_extra_fabricated_claim() {
        let ev = api_evidence();
        let outcome = assess_claim_support(
            "The health function returns status ok and the reload function deletes the database",
            &ev[0],
        );
        assert!(
            matches!(outcome, SupportOutcome::Unsupported(_)),
            "extra fabricated conjunct must be rejected, got: {outcome:?}"
        );
    }

    #[test]
    fn claim_support_rejects_fabricated_identifier() {
        let ev = api_evidence();
        let outcome = assess_claim_support("The health function calls RI_SECRET_HEADER", &ev[0]);
        assert!(
            matches!(outcome, SupportOutcome::Unsupported(_)),
            "fabricated identifier must be rejected, got: {outcome:?}"
        );
    }

    #[test]
    fn citation_resolution_is_separate_from_claim_support() {
        let ev = api_evidence();
        // The cited span exists, but it does not support the claim.
        let unsupported =
            assess_answer_citations("The health function deletes all files [api.rs:1-3].", &ev);
        assert!(!unsupported.accepted);
        assert_eq!(unsupported.claims.len(), 1);
        assert!(unsupported.claims[0].citation_resolved);
        assert!(!unsupported.claims[0].supported);

        // Same span, supported claim: citation resolution and support both hold.
        let supported =
            assess_answer_citations("The health function returns status ok [api.rs:1-3].", &ev);
        assert!(supported.accepted, "reason: {}", supported.reason);
        assert!(supported.claims[0].citation_resolved);
        assert!(supported.claims[0].supported);
        assert_eq!(supported.resolved_citations, 1);
    }

    #[test]
    fn claim_support_does_not_borrow_from_negated_word_form() {
        // "safe" must not be satisfied by "unsafe": the token relation is not a
        // substring match.
        let ev = Evidence {
            path: PathBuf::from("security.rs"),
            start_line: 1,
            end_line: 1,
            score: 1.0,
            source: "lexical".to_string(),
            kind: "note".to_string(),
            symbol: None,
            text: "The scanner marks the input unsafe.".to_string(),
        };
        let outcome = assess_claim_support("The scanner marks the input safe", &ev);
        assert!(
            matches!(outcome, SupportOutcome::Unsupported(_)),
            "safe must not be grounded by unsafe, got: {outcome:?}"
        );
    }

    #[test]
    fn verify_answer_refuses_uncited_line() {
        let ev = sample_evidence();
        let ans = "The validate_relative_path function rejects empty paths [E1].\nAnd this is an unverified assertion.";
        let res = verify_answer_citations(ans, &ev);
        assert!(matches!(res, VerificationResult::Refused { .. }));
    }

    #[test]
    fn verify_answer_refuses_when_line_has_mixed_valid_and_invalid() {
        let ev = sample_evidence();
        let ans = "Validation is here [E1] but also hallucinated [fake.rs:99].";
        let res = verify_answer_citations(ans, &ev);
        assert!(matches!(res, VerificationResult::Refused { .. }));
    }

    #[test]
    fn verify_answer_refuses_out_of_bounds_citation() {
        let ev = sample_evidence();
        let ans = "Out of range span [security.rs:1-100].";
        let res = verify_answer_citations(ans, &ev);
        assert!(matches!(res, VerificationResult::Refused { .. }));
    }

    #[test]
    fn verify_answer_refuses_insufficient_text() {
        let ev = sample_evidence();
        let ans = "Insufficient repository evidence to answer this question.";
        let res = verify_answer_citations(ans, &ev);
        assert!(matches!(res, VerificationResult::Refused { .. }));
    }

    #[test]
    fn verify_answer_refuses_irrelevant_citation_for_claim() {
        let ev = sample_evidence();
        // security.rs:2-4 contains validate_relative_path, completely irrelevant to reload git diff
        let ans = "The reload endpoint applies a Git diff for added, modified, and deleted paths [security.rs:2-4].";
        let res = verify_answer_citations(ans, &ev);
        assert!(
            matches!(res, VerificationResult::Refused { .. }),
            "Expected Refused for irrelevant citation, got: {:?}",
            res
        );
    }
}
