/// Semantic and lexical retrieval fusion implementation.
pub fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let dot: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    let norm_l: f32 = left.iter().map(|v| v * v).sum::<f32>().sqrt();
    let norm_r: f32 = right.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm_l > 0.0 && norm_r > 0.0 {
        dot / (norm_l * norm_r)
    } else {
        0.0
    }
}

pub fn reciprocal_rank_fusion(lexical_ranks: &[usize], semantic_ranks: &[usize], k: f32) -> f32 {
    let score_lex = lexical_ranks.iter().map(|r| 1.0 / (k + *r as f32 + 1.0)).sum::<f32>();
    let score_sem = semantic_ranks.iter().map(|r| 1.0 / (k + *r as f32 + 1.0)).sum::<f32>();
    score_lex + score_sem
}

pub fn anchor_lexical_hit(candidates: &mut Vec<usize>, anchor: usize) {
    if !candidates.contains(&anchor) {
        candidates.insert(0, anchor);
    }
}
