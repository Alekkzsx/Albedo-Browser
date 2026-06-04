/// Codifica bytes em string hexadecimal.
pub fn encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}
