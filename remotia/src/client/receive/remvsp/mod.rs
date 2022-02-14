#[cfg(test)]
mod tests;

mod reconstruct;
mod state;

use std::{collections::HashMap, fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;

use log::{debug, info};
use socket2::{Domain, Socket, Type};
use tokio::{
    net::UdpSocket,
    sync::{Mutex, MutexGuard},
    time::Instant,
};

use crate::{
    error::DropReason,
    common::{
        feedback::FeedbackMessage,
        network::remvsp::{RemVSPFrameFragment, RemVSPFrameHeader},
    },
};

use self::{reconstruct::FrameReconstructionState, state::RemVSPReceptionState};

use super::{FrameReceiver, ReceivedFrame};

pub struct RemVSPFrameReceiverConfiguration {
    pub frame_pull_interval: Duration,
    pub delayable_threshold: u128,
}

impl Default for RemVSPFrameReceiverConfiguration {
    fn default() -> Self {
        Self {
            frame_pull_interval: Duration::from_millis(10),
            delayable_threshold: 100,
        }
    }
}

pub struct RemVSPFrameReceiver {
    socket: Arc<UdpSocket>,
    server_address: SocketAddr,
    config: RemVSPFrameReceiverConfiguration,
    state: Arc<Mutex<RemVSPReceptionState>>,
}

impl RemVSPFrameReceiver {
    pub async fn connect(
        port: i16,
        server_address: SocketAddr,
        config: RemVSPFrameReceiverConfiguration,
    ) -> Self {
        let bind_address: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        let bind_address = bind_address.into();

        let raw_socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();
        raw_socket.set_nonblocking(true).unwrap();
        raw_socket.set_recv_buffer_size(256 * 1024 * 1024).unwrap();
        raw_socket.bind(&bind_address).unwrap();

        let udp_socket: std::net::UdpSocket = raw_socket.into();
        let socket = UdpSocket::from_std(udp_socket).unwrap();

        let hello_buffer = [0; 16];
        socket.send_to(&hello_buffer, server_address).await.unwrap();

        let socket = Arc::new(socket);

        let mut obj = Self {
            socket,
            server_address,
            config,
            state: Arc::new(Mutex::new(RemVSPReceptionState::default())),
        };

        obj.run_reception_loop();

        obj
    }

    fn run_reception_loop(&mut self) {
        let mut bin_fragment_buffer = vec![0 as u8; 1024];

        let socket = self.socket.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            loop {
                let received_bytes = socket.recv(&mut bin_fragment_buffer).await.unwrap();

                let frame_fragment = bincode::deserialize::<RemVSPFrameFragment>(
                    &bin_fragment_buffer[..received_bytes],
                )
                .unwrap();

                debug!(
                    "Received frame fragment #{}: {:?}",
                    frame_fragment.fragment_id, frame_fragment.frame_header
                );

                state.lock().await.register_frame_fragment(frame_fragment);
            }
        });
    }
}

#[async_trait]
impl FrameReceiver for RemVSPFrameReceiver {
    async fn receive_encoded_frame(
        &mut self,
        encoded_frame_buffer: &mut [u8],
    ) -> Result<ReceivedFrame, DropReason> {
        let result = {
            debug!("Pulling frame...");
            let received_frame = self
                .state
                .lock()
                .await
                .pull_frame(encoded_frame_buffer, self.config.delayable_threshold);

            match received_frame {
                Some(v) => Ok(v),
                None => Err(DropReason::NoCompleteFrames),
            }
        };

        if let Err(DropReason::NoCompleteFrames) = result {
            debug!("Null pulled frame, throttling...");
            tokio::time::sleep(self.config.frame_pull_interval).await;
        }

        result
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
