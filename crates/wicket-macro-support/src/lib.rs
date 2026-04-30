/// FNV-1a hash algorithm.
pub const fn hash_string(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut hash: u32 = 0x811c9dc5; // Offset basis
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x01000193); // FNV Prime
        i += 1;
    }
    hash
}
