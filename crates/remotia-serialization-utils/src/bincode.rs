use async_trait::async_trait;
use bincode::{Decode, Encode};
use remotia_buffer_utils::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

pub struct BincodeSerializer<K> {
    buffer_key: K,
}

impl<K> BincodeSerializer<K> {
    pub fn new(buffer_key: K) -> Self {
        Self { buffer_key }
    }
}

// TODO: Find a safe way to implement writing to BytesMut
#[async_trait]
impl<K, F> FrameProcessor<F> for BincodeSerializer<K>
where
    K: Copy + Send,
    F: PullableFrameProperties<K, BytesMut> + Encode + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut output_buffer = frame_data.pull(&self.buffer_key).unwrap();

        unsafe {
            output_buffer.set_len(output_buffer.capacity());
        }

        let output_slice = &mut output_buffer;

        let written_bytes =
            bincode::encode_into_slice(&frame_data, output_slice, bincode::config::standard())
                .unwrap();

        unsafe {
            output_buffer.set_len(written_bytes);
        }

        frame_data.push(self.buffer_key, output_buffer);

        Some(frame_data)
    }
}

pub struct BincodeDeserializer<K> {
    buffer_key: K,
}

impl<K> BincodeDeserializer<K> {
    pub fn new(buffer_key: K) -> Self {
        Self { buffer_key }
    }
}

#[async_trait]
impl<K, F> FrameProcessor<F> for BincodeDeserializer<K>
where
    K: Copy + Send,
    F: PullableFrameProperties<K, BytesMut> + Decode + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let input_buffer = frame_data.pull(&self.buffer_key).unwrap();

        let (mut decoded_frame_data, _) : (F, usize) =
            bincode::decode_from_slice(&input_buffer, bincode::config::standard()).unwrap();

        decoded_frame_data.push(self.buffer_key, input_buffer);

        Some(decoded_frame_data)
    }
}
