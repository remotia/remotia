use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
