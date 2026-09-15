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

pub fn citation_matches_evidence(citation: &CitationRef, evidence: &[Evidence]) -> bool {
    match citation {
        CitationRef::EvidenceId(id) => *id >= 1 && *id <= evidence.len(),
        CitationRef::Span { path, start, end } => {
            let norm_path = Path::new(path.trim_start_matches("./"));
            evidence.iter().any(|item| {
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

pub fn verify_answer_citations(raw_answer: &str, evidence: &[Evidence]) -> VerificationResult {
    let trimmed = raw_answer.trim();
    if trimmed.is_empty() || trimmed.contains("Insufficient repository evidence") {
        return VerificationResult::Refused {
            reason: "Model signaled insufficient evidence or returned empty answer".to_string(),
        };
    }

    let mut verified_lines = Vec::new();
    let mut total_verified_citations = 0;

    for line in trimmed.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }

        let citations = parse_citations(line_trimmed);
        if citations.is_empty() {
            return VerificationResult::Refused {
                reason: format!("Line contains uncited assertion: '{line_trimmed}'"),
            };
        }

        let mut line_citations_count = 0;
        for c_res in citations {
            match c_res {
                Ok(c) => {
                    if !citation_matches_evidence(&c, evidence) {
                        return VerificationResult::Refused {
                            reason: format!(
                                "Citation '{:?}' is not supported by retrieved evidence",
                                c
                            ),
                        };
                    }
                    line_citations_count += 1;
                }
                Err(err) => {
                    return VerificationResult::Refused {
                        reason: format!("Invalid citation format on line: {err}"),
                    };
                }
            }
        }

        total_verified_citations += line_citations_count;
        verified_lines.push(line_trimmed.to_string());
    }

    if total_verified_citations == 0 || verified_lines.is_empty() {
        return VerificationResult::Refused {
            reason: "Zero valid citations verified across entire answer".to_string(),
        };
    }

    VerificationResult::Accepted {
        verified_text: verified_lines.join("\n"),
        verified_citations: total_verified_citations,
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

    #[test]
    fn verify_answer_accepts_valid() {
        let ev = sample_evidence();
        let ans =
            "Validation ensures safe paths [E1].\nStorage holds items [src/storage.rs:10-20].";
        let res = verify_answer_citations(ans, &ev);
        assert!(matches!(res, VerificationResult::Accepted { .. }));
    }

    #[test]
    fn verify_answer_refuses_uncited_line() {
        let ev = sample_evidence();
        let ans = "Validation ensures safe paths [E1].\nAnd this is an unverified assertion.";
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
}
