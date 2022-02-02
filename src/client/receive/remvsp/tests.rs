use rand::Rng;

use crate::common::network::remvsp::{RemVSPFrameFragment, RemVSPFrameHeader};

use super::FrameReconstructionState;

#[test]
fn reconstruct_simple_test() {
    let mut reconstruction_state = FrameReconstructionState::default();

    let header = RemVSPFrameHeader {
        frame_fragments_count: 2,
        fragment_size: 4,
        capture_timestamp: 0,
    };
    
    let fragment0 = RemVSPFrameFragment {
        frame_header: header,
        fragment_id: 0,
        data: vec![0, 0, 0, 0],
    };

    let fragment1 = RemVSPFrameFragment {
        frame_header: header,
        fragment_id: 1,
        data: vec![0, 1, 0, 1],
    };

    reconstruction_state.register_fragment(fragment0);
    reconstruction_state.register_fragment(fragment1);

    let mut output_buffer = vec![0 as u8; 8];

    reconstruction_state.reconstruct(&mut output_buffer);

    let expected_output = vec![0, 0, 0, 0, 0, 1, 0, 1];

    assert_eq!(output_buffer, expected_output);
}

fn reconstruct_message_test(message: &[u8], fragment_size: u16) {
    let mut reconstruction_state = FrameReconstructionState::default();

    let frame_fragments_count = message.len() as u16 / fragment_size;

    let header = RemVSPFrameHeader {
        frame_fragments_count,
        fragment_size,
        capture_timestamp: 0,
    };

    let chunks = message.chunks(fragment_size as usize);
    for (idx, chunk) in chunks.enumerate() {
        reconstruction_state.register_fragment(RemVSPFrameFragment {
            frame_header: header,
            fragment_id: idx as u16,
            data: chunk.to_vec(),
        });
    }

    let mut output_buffer = vec![0 as u8; message.len()];

    reconstruction_state.reconstruct(&mut output_buffer);

    assert_eq!(output_buffer, message);
}

#[test]
fn reconstruct_trailing_bytes_test() {
    let message = [120, 18, 226, 150, 154, 168, 231, 12, 251, 64, 207, 188, 103, 122, 61];
    reconstruct_message_test(&message, 4);
}

#[test]
fn reconstruct_random_message_test() {
    let mut rng = rand::thread_rng();
    let mut message = vec![0 as u8; 16];

    for i in 0..message.len() {
        message[i] = rng.gen::<u8>();
    }

    reconstruct_message_test(&message, 4);
}