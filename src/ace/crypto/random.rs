/// Cryptographically Secure Pseudo-Random Number Generator (CSPRNG).
/// 
/// Currently wraps the `getrandom` system call, but will be replaced by 
/// a custom Albedo implementation in AFS-23.
pub fn get_random_bytes(dest: &mut [u8]) {
    getrandom::getrandom(dest).expect("ACE-Crypto: failed to get random bytes from system CSPRNG");
}
