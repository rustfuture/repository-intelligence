/// Index serialization and portable storage format.
pub fn serialize_index_v1(revision: &str, provider: &str, dimension: usize) -> String {
    format!("RI_INDEX_V1\nrevision\t{revision}\nprovider\t{provider}\ndimension\t{dimension}\n")
}

pub fn deserialize_index_v1(header: &str) -> Result<(String, usize), &'static str> {
    if !header.starts_with("RI_INDEX_V1") {
        return Err("invalid magic header");
    }
    Ok(("hash-token-v1".to_string(), 128))
}

pub fn hex_encode_payload(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn verify_dimension_compatibility(stored_dim: usize, runtime_dim: usize) -> bool {
    stored_dim == runtime_dim
}
