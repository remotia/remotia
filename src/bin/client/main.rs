#![allow(unused_imports)]

mod decode;
mod error;
mod profiling;
mod receive;

mod utils;

use std::env;
use std::net::TcpStream;

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::ops::ControlFlow;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;

use beryllium::*;

use chrono::Utc;
use log::info;
use log::{debug, error, warn};
use pixels::wgpu;
use pixels::PixelsBuilder;
use pixels::{wgpu::Surface, Pixels, SurfaceTexture};
use profiling::ReceivedFrameStats;
use receive::tcp::TCPFrameReceiver;
use zstring::zstr;

use crate::decode::Decoder;
use crate::error::ClientError;
use crate::profiling::logging::console::ReceptionRoundConsoleLogger;
use crate::profiling::logging::csv::ReceptionRoundCSVLogger;
use crate::profiling::ReceptionRoundStats;
use crate::receive::udp::UDPFrameReceiver;
use crate::receive::FrameReceiver;
use crate::utils::decoding::packed_bgr_to_packed_rgba;
use crate::utils::decoding::setup_decoding_env;

#[allow(dead_code)]
fn enstablish_udp_connection(server_address: &SocketAddr) -> std::io::Result<UdpSocket> {
    let socket = UdpSocket::bind("127.0.0.1:5002")?;
    socket
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();

    let hello_buffer = [0; 16];
    socket.send_to(&hello_buffer, server_address).unwrap();

    Ok(socket)
}

fn parse_canvas_resolution_arg(arg: &String) -> (u32, u32) {
    let canvas_resolution_split: Vec<&str> = arg.split("x").collect();

    let width_str = canvas_resolution_split[0];
    let height_str = canvas_resolution_split[1];

    let canvas_width: u32 = u32::from_str(width_str)
        .unwrap_or_else(|e| panic!("Unable to parse width '{}': {}", width_str, e));

    let canvas_height: u32 = u32::from_str(height_str)
        .unwrap_or_else(|e| panic!("Unable to parse height '{}': {}", height_str, e));

    (canvas_width, canvas_height)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    let (canvas_width, canvas_height) = parse_canvas_resolution_arg(&args[1]);
    let expected_frame_size: usize = (canvas_width as usize) * (canvas_height as usize) * 3;

    // Init display
    let sdl = SDL::init(InitFlags::default())?;
    let window = sdl.create_raw_window(
        "Remotia client",
        WindowPosition::Centered,
        canvas_width,
        canvas_height,
        0,
    )?;

    let mut pixels = {
        // let surface = Surface::create(&window);
        let surface_texture = SurfaceTexture::new(canvas_width, canvas_height, &window);
        PixelsBuilder::new(canvas_width, canvas_height, surface_texture)
            // .texture_format(wgpu::TextureFormat::Bgra8Unorm)
            .build()?
        // Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    pixels.render()?;

    let server_address = SocketAddr::from_str("127.0.0.1:5001")?;

    /*let socket = enstablish_udp_connection(&server_address)?;
    let mut frame_receiver = UDPFrameReceiver::create(&socket, &server_address);*/

    let mut stream = TcpStream::connect(server_address)?;
    let mut frame_receiver = TCPFrameReceiver::create(&mut stream);

    let mut decoder = setup_decoding_env(canvas_width, canvas_height, &args[2]);

    let mut consecutive_connection_losses = 0;

    let mut encoded_frame_buffer = vec![0 as u8; expected_frame_size];

    info!("Starting to receive stream...");

    let round_duration = Duration::from_secs(1);
    let mut round_stats = setup_round_stats()?;

    loop {
        match receive_frame(
            &mut decoder,
            &mut pixels,
            &mut frame_receiver,
            &mut encoded_frame_buffer,
            &mut consecutive_connection_losses,
        ) {
            ControlFlow::Continue(frame_stats) => {
                round_stats.profile_frame(frame_stats);

                let current_round_duration = round_stats.start_time.elapsed();

                if current_round_duration.gt(&round_duration) {
                    round_stats.log();
                    round_stats.reset();
                }
            }
            ControlFlow::Break(_) => break,
        };
    }

    Ok(())
}

fn setup_round_stats() -> Result<ReceptionRoundStats, std::io::Error> {
    let round_stats: ReceptionRoundStats = {
        let datetime = Utc::now();

        ReceptionRoundStats {
            loggers: vec![
                Box::new(ReceptionRoundCSVLogger::new(
                    format!("csv_logs/client/{}.csv", datetime).as_str(),
                )?),
                Box::new(ReceptionRoundConsoleLogger::default()),
            ],

            ..Default::default()
        }
    };
    Ok(round_stats)
}

fn receive_frame(
    decoder: &mut Box<dyn Decoder>,
    pixels: &mut Pixels,
    frame_receiver: &mut TCPFrameReceiver,
    encoded_frame_buffer: &mut Vec<u8>,
    consecutive_connection_losses: &mut i32,
) -> ControlFlow<(), ReceivedFrameStats> {
    debug!("Waiting for next frame...");

    let total_start_time = Instant::now();

    let reception_start_time = Instant::now();
    let receive_result = frame_receiver.receive_encoded_frame(encoded_frame_buffer);
    let reception_time = reception_start_time.elapsed().as_millis();

    let decoding_start_time = Instant::now();
    let decode_result = receive_result.and_then(|received_data_length| {
        decode_task(decoder, &mut encoded_frame_buffer[..received_data_length])
    });
    let decoding_time = decoding_start_time.elapsed().as_millis();

    let rendering_start_time = Instant::now();
    let render_result =
        decode_result.and_then(|_| render_task(decoder, pixels, consecutive_connection_losses));
    let rendering_time = rendering_start_time.elapsed().as_millis();

    let rendered = render_result.is_ok();
    render_result.unwrap_or_else(|e| {
        handle_error(e, consecutive_connection_losses);
    });

    if *consecutive_connection_losses >= 100 {
        error!("Too much consecutive connection losses, closing stream");
        return ControlFlow::Break(());
    }

    let total_time = total_start_time.elapsed().as_millis();

    ControlFlow::Continue(ReceivedFrameStats {
        reception_time,
        decoding_time,
        rendering_time,
        total_time,
        rendered,
    })
}

fn handle_error(error: ClientError, consecutive_connection_losses: &mut i32) {
    match error {
        ClientError::InvalidWholeFrameHeader => *consecutive_connection_losses = 0,
        ClientError::FFMpegSendPacketError => {
            debug!("H264 Send packet error")
        }
        _ => *consecutive_connection_losses += 1,
    }

    warn!(
        "Error while receiving frame: {}, dropping (consecutive connection losses: {})",
        error, consecutive_connection_losses
    );
}

fn decode_task(
    decoder: &mut Box<dyn Decoder>,
    encoded_frame_buffer: &mut [u8],
) -> Result<usize, ClientError> {
    debug!("Decoding {} received bytes", encoded_frame_buffer.len());
    decoder.decode(encoded_frame_buffer)
}

fn render_task(
    decoder: &mut Box<dyn Decoder>,
    pixels: &mut Pixels,
    consecutive_connection_losses: &mut i32,
) -> Result<(), ClientError> {
    packed_bgr_to_packed_rgba(decoder.get_decoded_frame(), pixels.get_frame());

    *consecutive_connection_losses = 0;
    pixels.render().unwrap();
    debug!("[SUCCESS] Frame rendered on screen");

    Ok(())
}
