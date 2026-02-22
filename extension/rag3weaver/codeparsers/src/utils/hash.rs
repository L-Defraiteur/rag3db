pub fn content_hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}

pub fn hash_to_uuid(hash_hex: &str) -> String {
    format!(
        "{}-{}-{}-{}-{}",
        &hash_hex[0..8],
        &hash_hex[8..12],
        &hash_hex[12..16],
        &hash_hex[16..20],
        &hash_hex[20..32]
    )
}

pub fn blake3_uuid(input: &str) -> String {
    hash_to_uuid(&content_hash(input))
}
