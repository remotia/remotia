use std::{io::Read, net::{TcpListener, TcpStream}, sync::Arc};

use log::{debug, info};
use tokio::sync::Mutex;

use async_trait::async_trait;

use crate::common::feedback::FeedbackMessage;

use super::ServerProfiler;

pub struct TCPServerProfiler {
    feedbacks: Arc<Mutex<Vec<FeedbackMessage>>>
}

impl TCPServerProfiler {
    pub fn connect() -> Self {
        let obj = Self {
            feedbacks: Arc::new(Mutex::new(Vec::new()))
        };

        obj.run_reception_loop();

        obj
    }

    fn run_reception_loop(&self) {
        let feedbacks = self.feedbacks.clone();

        tokio::spawn(async move {
            let listener = TcpListener::bind("127.0.0.1:5011").unwrap();
            info!("Waiting for client profiler connection...");
            let (mut socket, _) = listener.accept().unwrap();
            
            let mut read_buffer = vec![0; 512];

            loop {
                let read_bytes = socket.read(&mut read_buffer).unwrap();
                let binarized_obj_buffer = &read_buffer[..read_bytes];
                let message: FeedbackMessage = bincode::deserialize(binarized_obj_buffer).unwrap();

                feedbacks.lock().await.push(message);
            }
        });
    }
}

#[async_trait]
impl ServerProfiler for TCPServerProfiler {
    async fn pull_feedback(&mut self) -> Option<FeedbackMessage> {
        self.feedbacks.lock().await.pop()
    }
}
