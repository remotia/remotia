pub fn setup_packed_bgr_frame_buffer(width: usize, height: usize) -> Vec<u8> {
    let frame_size = width * height * 3;
    let packed_bgr_frame_buffer: Vec<u8> = vec![0; frame_size];
    packed_bgr_frame_buffer
}

pub fn packed_bgra_to_packed_bgr(packed_bgra_buffer: &[u8], packed_bgr_buffer: &mut [u8]) {
    let pixels_count = packed_bgra_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_bgr_buffer[i * 3] = packed_bgra_buffer[i * 4];
        packed_bgr_buffer[i * 3 + 1] = packed_bgra_buffer[i * 4 + 1];
        packed_bgr_buffer[i * 3 + 2] = packed_bgra_buffer[i * 4 + 2];
    }
}
