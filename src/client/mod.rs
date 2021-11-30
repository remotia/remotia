#![allow(unused_imports)]

pub mod decode;
pub mod error;
pub mod profiling;
pub mod receive;

pub mod utils;

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
use clap::Parser;
use log::info;
use log::{debug, error, warn};
use pixels::wgpu;
use pixels::PixelsBuilder;
use pixels::{wgpu::Surface, Pixels, SurfaceTexture};
use profiling::ReceivedFrameStats;
use zstring::zstr;

use crate::client::profiling::ReceptionRoundStats;
use crate::client::profiling::logging::console::ReceptionRoundConsoleLogger;
use crate::client::profiling::logging::csv::ReceptionRoundCSVLogger;
use crate::client::utils::profilation::setup_round_stats;
use crate::client::utils::reception::receive_frame;

use self::decode::Decoder;
use self::receive::FrameReceiver;

pub struct ClientConfiguration {
    pub decoder: Box<dyn Decoder>,
    pub frame_receiver: Box<dyn FrameReceiver>,

    pub canvas_width: u32,
    pub canvas_height: u32,

    pub maximum_consecutive_connection_losses: u32,

    pub console_profiling: bool,
    pub csv_profiling: bool,
}

pub struct ClientState {
    pub pixels: Pixels,
    pub consecutive_connection_losses: u32,
    pub encoded_frame_buffer: Vec<u8>
}

pub async fn run_with_configuration(mut config: ClientConfiguration) -> Result<(), Box<dyn std::error::Error>> {
    let expected_frame_size: usize =
        (config.canvas_width as usize) * (config.canvas_height as usize) * 3;

    // Init display
    let sdl = SDL::init(InitFlags::default())?;
    let window = sdl.create_raw_window(
        "Remotia client",
        WindowPosition::Centered,
        config.canvas_width,
        config.canvas_height,
        0,
    )?;

    let mut state = ClientState {
        pixels: {
            let surface_texture = SurfaceTexture::new(config.canvas_width, config.canvas_height, &window);
            PixelsBuilder::new(config.canvas_width, config.canvas_height, surface_texture)
                .build()?
        },
        consecutive_connection_losses: 0,
        encoded_frame_buffer: vec![0 as u8; expected_frame_size],
    };

    state.pixels.render()?;

    info!("Starting to receive stream...");

    let round_duration = Duration::from_secs(1);
    let mut round_stats = setup_round_stats(&config)?;

    loop {
        match receive_frame(
            &mut config,
            &mut state
        ).await {
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
