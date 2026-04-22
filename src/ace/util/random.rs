pub fn get_random_bytes(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        *b = 0;
    }
}
