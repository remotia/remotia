use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures::SinkExt;
use log::info;
use rand::Rng;
use srt_tokio::SrtSocketBuilder;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    info!("Listening...");

    let mut srt_socket = SrtSocketBuilder::new_listen()
        .local_port(3333)
        .latency(Duration::from_millis(10))
        .connect()
        .await?;

    info!("Connected");

    const MAX_PACKET_SIZE: usize = 1024 * 32;

    let mut packet_id = 0;
    let mut rng = rand::thread_rng();

    loop {
        let packet_size: usize = rng.gen::<usize>() % MAX_PACKET_SIZE;
        let packet_data = vec![0 as u8; packet_size];

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let packet = (packet_id, timestamp, packet_data);

        let binarized_packet = Bytes::from(bincode::serialize(&packet).unwrap());

        info!("Sending packet #{} of size {} (timestamp: {})", packet_id, packet_size, timestamp);

        srt_socket.send((Instant::now(), binarized_packet)).await.unwrap();

        packet_id += 1;

        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}
