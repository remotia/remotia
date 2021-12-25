use std::env;
use std::net::TcpStream;

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::ops::ControlFlow;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use beryllium::*;

use chrono::Utc;
use clap::Parser;
use log::info;
use log::{debug, error, warn};
use pixels::wgpu;
use pixels::PixelsBuilder;
use pixels::{wgpu::Surface, Pixels, SurfaceTexture};
use zstring::zstr;

use crate::client::decode::Decoder;
use crate::client::error::ClientError;
use crate::client::profiling::logging::console::ReceptionRoundConsoleLogger;
use crate::client::profiling::logging::csv::ReceptionRoundCSVLogger;
use crate::client::profiling::ReceivedFrameStats;
use crate::client::profiling::ReceptionRoundStats;
use crate::client::receive::FrameReceiver;
use crate::client::utils::decoding::packed_bgr_to_packed_rgba;
use crate::client::utils::profilation::setup_round_stats;
use crate::common::window::create_gl_window;

pub struct WaterfallClientConfiguration {
    pub decoder: Box<dyn Decoder>,
    pub frame_receiver: Box<dyn FrameReceiver>,

    pub canvas_width: u32,
    pub canvas_height: u32,

    pub maximum_consecutive_connection_losses: u32,

    pub target_fps: u32,

    pub console_profiling: bool,
    pub csv_profiling: bool,
}

pub struct WaterfallClientState {
    pub pixels: Pixels,
    pub consecutive_connection_losses: u32,
    pub encoded_frame_buffer: Vec<u8>,
    pub raw_frame_buffer: Vec<u8>,
}

pub struct WaterfallClientPipeline {
    config: WaterfallClientConfiguration,
}

impl WaterfallClientPipeline {
    pub fn new(config: WaterfallClientConfiguration) -> Self {
        Self { config }
    }

    pub async fn run(mut self) {
        let expected_frame_size: usize =
            (self.config.canvas_width as usize) * (self.config.canvas_height as usize) * 3;

        // Init display
        let gl_win = create_gl_window(
            self.config.canvas_width as i32,
            self.config.canvas_height as i32,
        );
        let window = &*gl_win;

        let mut state = WaterfallClientState {
            pixels: {
                let surface_texture = SurfaceTexture::new(
                    self.config.canvas_width,
                    self.config.canvas_height,
                    &window,
                );
                PixelsBuilder::new(
                    self.config.canvas_width,
                    self.config.canvas_height,
                    surface_texture,
                )
                .build()
                .unwrap()
            },
            consecutive_connection_losses: 0,
            encoded_frame_buffer: vec![0 as u8; expected_frame_size],
            raw_frame_buffer: vec![0 as u8; expected_frame_size],
        };

        state.pixels.render().unwrap();

        info!("Starting to receive stream...");

        let round_duration = Duration::from_secs(1);
        let mut round_stats =
            setup_round_stats(self.config.csv_profiling, self.config.console_profiling).unwrap();

        let target_fps = self.config.target_fps as f64;
        let mut fps: f64 = recalculate_fps(0.0, target_fps, None);

        let mut last_frame_dispatching_time = 0;

        loop {
            let spin_time = (1000 / std::cmp::max(fps as i64, 1)) - last_frame_dispatching_time;
            std::thread::sleep(Duration::from_millis(std::cmp::max(0, spin_time) as u64));

            let frame_dispatch_start_time = Instant::now();

            match self.receive_frame(&mut state).await {
                ControlFlow::Continue(frame_stats) => {
                    fps = recalculate_fps(fps, target_fps, frame_stats.error.as_ref());

                    round_stats.profile_frame(frame_stats);

                    let current_round_duration = round_stats.start_time.elapsed();

                    if current_round_duration.gt(&round_duration) {
                        round_stats.log();
                        round_stats.reset();
                    }
                }
                ControlFlow::Break(_) => break,
            };

            last_frame_dispatching_time = frame_dispatch_start_time.elapsed().as_millis() as i64;
        }
    }

    async fn receive_frame(
        &mut self,
        state: &mut WaterfallClientState,
    ) -> ControlFlow<(), ReceivedFrameStats> {
        debug!("Waiting for next frame...");

        let total_start_time = Instant::now();

        let reception_start_time = Instant::now();
        let receive_result = self
            .config
            .frame_receiver
            .receive_encoded_frame(&mut state.encoded_frame_buffer)
            .await;
        let reception_time = reception_start_time.elapsed().as_millis();

        let (reception_delay, frame_delay) = if receive_result.is_ok() {
            let received_frame = receive_result.as_ref().unwrap();
            let capture_timestamp = received_frame.capture_timestamp;
            let frame_delay = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                - capture_timestamp;

            (received_frame.reception_delay, frame_delay)
        } else {
            (0, 0)
        };

        let decoding_start_time = Instant::now();
        let decode_result = receive_result.and_then(|received_frame| {
            decode_task(
                &mut self.config.decoder,
                &mut state.raw_frame_buffer,
                &mut state.encoded_frame_buffer[..received_frame.buffer_size],
            )
        });
        let decoding_time = decoding_start_time.elapsed().as_millis();

        let rendering_start_time = Instant::now();
        let render_result = decode_result.and_then(|_| {
            render_task(
                &mut state.raw_frame_buffer,
                &mut state.pixels,
                &mut state.consecutive_connection_losses,
            )
        });
        let rendering_time = rendering_start_time.elapsed().as_millis();

        let error = render_result.map_or_else(
            |e| {
                handle_error(&e, &mut state.consecutive_connection_losses);
                Some(e)
            },
            |_| None,
        );

        if state.consecutive_connection_losses >= self.config.maximum_consecutive_connection_losses
        {
            error!("Too much consecutive connection losses, closing stream");
            return ControlFlow::Break(());
        }

        let total_time = total_start_time.elapsed().as_millis();

        ControlFlow::Continue(ReceivedFrameStats {
            reception_time,
            decoding_time,
            rendering_time,
            total_time,
            frame_delay,
            reception_delay,
            error,

            ..Default::default()
        })
    }
}

fn recalculate_fps(current_fps: f64, target_fps: f64, frame_error: Option<&ClientError>) -> f64 {
    if let Some(error) = frame_error {
        match error {
            ClientError::Timeout => current_fps * 0.6,
            _ => current_fps,
        }
    } else {
        let fps_increment = (target_fps - current_fps) / 2.0;
        let next_round_fps = current_fps + fps_increment;
        next_round_fps
    }
}

fn handle_error(error: &ClientError, consecutive_connection_losses: &mut u32) {
    match error {
        ClientError::InvalidWholeFrameHeader | ClientError::StaleFrame => {
            *consecutive_connection_losses = 0
        }
        ClientError::FFMpegSendPacketError => {
            debug!("H264 Send packet error")
        }
        _ => *consecutive_connection_losses += 1,
    }

    debug!(
        "Error while receiving frame: {}, dropping (consecutive connection losses: {})",
        error, consecutive_connection_losses
    );
}

fn decode_task(
    decoder: &mut Box<dyn Decoder>,
    encoded_frame_buffer: &mut [u8],
    raw_frame_buffer: &mut [u8],
) -> Result<usize, ClientError> {
    debug!("Decoding {} received bytes", encoded_frame_buffer.len());
    decoder.decode(encoded_frame_buffer, raw_frame_buffer)
}

fn render_task(
    raw_frame_buffer: &mut [u8],
    pixels: &mut Pixels,
    consecutive_connection_losses: &mut u32,
) -> Result<(), ClientError> {
    packed_bgr_to_packed_rgba(raw_frame_buffer, pixels.get_frame());

    *consecutive_connection_losses = 0;
    pixels.render().unwrap();
    debug!("[SUCCESS] Frame rendered on screen");

    Ok(())
}
