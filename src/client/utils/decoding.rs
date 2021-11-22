pub fn packed_bgr_to_packed_rgba(packed_bgr_buffer: &[u8], packed_bgra_buffer: &mut [u8]) {
    let pixels_count = packed_bgra_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_bgra_buffer[i * 4 + 2] = packed_bgr_buffer[i * 3];
        packed_bgra_buffer[i * 4 + 1] = packed_bgr_buffer[i * 3 + 1];
        packed_bgra_buffer[i * 4] = packed_bgr_buffer[i * 3 + 2];
    }
}