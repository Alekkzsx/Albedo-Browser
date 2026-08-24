use ace_core::collections::ChunkBuffer;

#[test]
fn test_chunk_buffer_streaming_and_slicing() {
    let mut buffer: ChunkBuffer<16> = ChunkBuffer::new(); // Chunks de 16 bytes para teste

    assert!(buffer.is_empty());
    buffer.write_bytes(b"Hello, ");
    buffer.write_bytes(b"World! This is a long stream of data for testing.");

    assert_eq!(buffer.len(), 56);
    assert_eq!(buffer.peek(5), b"Hello");

    let mut dest = [0u8; 13];
    let read_count = buffer.read_bytes(&mut dest);
    assert_eq!(read_count, 13);
    assert_eq!(&dest, b"Hello, World!");

    assert_eq!(buffer.len(), 43);

    let mut remaining = vec![0u8; 100];
    let rem_count = buffer.read_bytes(&mut remaining);
    assert_eq!(rem_count, 43);
    assert_eq!(
        &remaining[..rem_count],
        b" This is a long stream of data for testing."
    );
    assert!(buffer.is_empty());
}
