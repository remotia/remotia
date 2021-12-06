use std::time::{SystemTime, UNIX_EPOCH};

use futures::{TryStreamExt};
use log::info;
use srt_tokio::SrtSocketBuilder;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    info!("Connecting...");

    let mut srt_socket = SrtSocketBuilder::new_connect("127.0.0.1:3333")
        .connect()
        .await?;

    info!("Connected");

    while let Some((instant, bytes)) = srt_socket.try_next().await? {
        let (packet_id, packet_timestamp, data) =
            bincode::deserialize::<(i32, u128, Vec<u8>)>(&bytes).unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        info!(
            "Received packet #{} of size {} (timestamp: {}, difference: {}, instant difference: {})",
            packet_id,
            data.len(),
            packet_timestamp,
            timestamp - packet_timestamp,
            instant.elapsed().as_millis()
        );
    }

    Ok(())
}
