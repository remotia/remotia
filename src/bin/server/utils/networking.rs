use std::{net::{SocketAddr, UdpSocket}, time::Duration};

use log::info;


#[allow(dead_code)]
fn enstablish_udp_connection() -> std::io::Result<(UdpSocket, SocketAddr)> {
    let socket = UdpSocket::bind("127.0.0.1:5001")?;

    info!("Socket bound, waiting for hello message...");

    let mut hello_buffer = [0; 16];
    let (bytes_received, client_address) = socket.recv_from(&mut hello_buffer)?;
    assert_eq!(bytes_received, 16);
    // let client_address = SocketAddr::from_str("127.0.0.1:5000").unwrap();

    info!("Hello message received correctly. Streaming...");
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();

    Ok((socket, client_address))
}
