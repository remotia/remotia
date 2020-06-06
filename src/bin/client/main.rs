// use std::net::UdpSocket;

use std::str::FromStr;
use std::net::{SocketAddr, SocketAddrV4};
use udt::*;

// const PACKET_SIZE: usize = 512;
const FRAME_SIZE: usize = 1920 * 1080 * 4;

fn main() {
    let localhost = std::net::Ipv4Addr::from_str("127.0.0.1").unwrap();

    let sock = UdtSocket::new(SocketFamily::AFInet, SocketType::Stream).unwrap();
    sock.bind(SocketAddr::V4(SocketAddrV4::new(localhost, 5001))).unwrap();
    let my_addr = sock.getsockname().unwrap();
    println!("Server bound to {:?}", my_addr);
    sock.listen(1).unwrap();
    let (connection_socket, peer) = sock.accept().unwrap();
    println!("Received new connection from peer {:?}", peer);

    // let mut packet_buffer: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
    let mut frame_buffer: [u8; FRAME_SIZE] = [0; FRAME_SIZE];

    loop {
        println!("Waiting for next frame...");

        connection_socket.recv(&mut frame_buffer, FRAME_SIZE).unwrap();

        println!("Received a frame");
    }
}