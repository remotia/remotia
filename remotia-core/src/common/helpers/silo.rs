use std::time::Instant;

use bytes::BytesMut;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn channel_pull<T>(receiver: &mut UnboundedReceiver<T>) -> Option<(T, u128)> {
    let wait_start_time = Instant::now();
    let object = receiver.recv().await;
    let wait_time = wait_start_time.elapsed().as_millis();

    object.as_ref()?;

    let object = object.unwrap();

    Some((object, wait_time))
}
