//! Functionality required at both compile time and runtime.

/// FNV-1a hash algorithm.
pub const fn hash_string_32(name: &str) -> u32 {
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

pub const fn hash_string(name: &str) -> u16 {
    let bytes = name.as_bytes();
    let mut hash: u32 = 0x811c9dc5; // 32-bit Offset basis
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x01000193); // 32-bit FNV Prime
        i += 1;
    }

    // Fold the 32-bit hash into 16 bits
    // XOR the upper 16 bits with the lower 16 bits
    ((hash >> 16) ^ (hash & 0xFFFF)) as u16
}

/// Return the index of a string in a slice.
pub fn get_string_index<S, L>(value: S, list: Option<&[L]>) -> Option<u8>
where
    S: AsRef<str>,
    L: AsRef<str>,
{
    list.and_then(|l| {
        l.iter()
            .position(|st| st.as_ref() == value.as_ref())
            .and_then(|idx| idx.try_into().ok())
    })
}
