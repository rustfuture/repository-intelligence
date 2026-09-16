/// Filesystem security and path traversal validation.
pub fn validate_relative_path(path: &str) -> bool {
    !path.starts_with('/') && !path.contains("..") && !path.is_empty()
}

pub fn block_symlink_traversal(is_symlink: bool) -> bool {
    !is_symlink
}

pub fn filter_sensitive_file(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".key") || lower.ends_with(".pem") || lower == "credentials.json"
}

pub fn sanitize_prompt_evidence(text: &str) -> String {
    format!("<repository_evidence>\n{text}\n</repository_evidence>")
}
