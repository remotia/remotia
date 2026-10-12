//! Winit window renderer driven by frame arrivals.
//!
//! The original `WinitRunner` called `Pixels::render` from the winit event
//! callback but never requested a redraw nor set a non-blocking control flow.
//! With no input events to wake the loop it parked right after startup, so the
//! window kept the frame presented at that moment while every decoded frame
//! landed in the pixel buffer without ever reaching the surface (the buffer
//! was dirty, the window was not).
//!
//! This module owns the event loop instead: [`WinitRenderer::process`] copies
//! each frame into the shared pixel buffer and wakes the loop through an
//! [`EventLoopProxy`], which presents the latest frame. Redraws are coalesced
//! by a pending flag, so a burst of frames results in a single present of the
//! newest buffer. The loop also exits on [`RenderShutdown::request`], so the
//! window can be destroyed programmatically (e.g. before drawing anything else
//! on the screen).
//!
//! Pressing Ctrl-C while the render window has focus requests the shutdown of
//! the pipeline attached with [`WinitRenderer::with_pipeline_shutdown`].

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pixels::{Pixels, SurfaceTexture};
use remotia_buffer_utils::BytesMut;
use remotia_core::common::helpers::time::now_timestamp;
use remotia_core::pipeline::PipelineHandle;
use remotia_core::traits::{BorrowMutFrameProperties, FrameProcessor};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState};
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, WindowBuilder};

/// Message sent to the render window event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopEvent {
    /// A frame was copied into the pixel buffer: present it.
    Present,
    /// The run is over: close the window and stop the loop.
    Shutdown,
}

/// Handle that closes the render window of a running [`WinitRenderer`].
///
/// Cloned out of the renderer before it moves into the pipeline, so the
/// window can be dropped programmatically (e.g. a fullscreen window covering
/// the screen) while the event loop is still running.
#[derive(Clone, Debug)]
pub struct RenderShutdown {
    proxy: EventLoopProxy<LoopEvent>,
}

impl RenderShutdown {
    /// Asks the event loop to exit. Harmless when the window was already
    /// closed by the user, in which case the proxy is simply disconnected.
    pub fn request(&self) {
        if self.proxy.send_event(LoopEvent::Shutdown).is_err() {
            log::debug!("Render window event loop already stopped");
        }
    }
}

/// Pixel buffer shared between the pipeline task and the event loop, plus the
/// identity of the frame it currently holds.
///
/// The frame identity travels under the same lock as the pixels so a present
/// always measures the frame it actually displays, even when the pipeline task
/// refills the buffer concurrently.
struct Surface {
    pixels: Pixels<'static>,
    /// Frame id of the frame copied into the buffer and not presented yet.
    frame_id: i64,
    /// Capture timestamp of that frame (ms since UNIX epoch), when known.
    capture_ts: Option<u128>,
}

/// Shared render surface.
type SharedSurface = Arc<Mutex<Surface>>;

/// One frame actually presented to the window: enough to measure the frame
/// delay (capture -> display).
#[derive(Clone, Copy, Debug)]
pub struct DisplaySample {
    pub frame_id: i64,
    /// Capture timestamp of the frame (ms since UNIX epoch); `None` when the
    /// frame carried none.
    pub capture_ts: Option<u128>,
    /// Wall-clock time the frame was presented (ms since UNIX epoch).
    pub present_ts: u128,
}

/// Frame identity reported to the renderer for the display log.
///
/// Implemented by the pipeline frame DTO so [`WinitRenderer::process`] can
/// remember which frame the pixel buffer holds and [`PresentStats::displays`]
/// can attribute each present to the frame that really reached the screen.
/// DTOs without a meaningful identity return `0` / `None`.
pub trait PresentedFrame {
    fn presented_frame_id(&self) -> i64;
    fn presented_capture_ts(&self) -> Option<u128>;
}

/// Aggregated cost of `Pixels::render` (the actual present), measured on the
/// event loop thread and read after the run.
///
/// Frames handed to the renderer are coalesced (only the newest is presented
/// when the event loop catches up), so the present count is the true displayed
/// frame rate and can be lower than the decoded frame count. Each present also
/// logs a [`DisplaySample`], so the frame delay can be computed only for the
/// frames that really reached the screen.
#[derive(Debug, Default)]
pub struct PresentStats {
    presents: AtomicU64,
    total_us: AtomicU64,
    max_us: AtomicU64,
    /// One sample per presented frame, in present order.
    displays: Mutex<Vec<DisplaySample>>,
}

impl PresentStats {
    /// Number of frames actually presented to the window.
    pub fn presents(&self) -> u64 {
        self.presents.load(Ordering::Relaxed)
    }

    /// Total time spent presenting, in microseconds.
    pub fn total_us(&self) -> u64 {
        self.total_us.load(Ordering::Relaxed)
    }

    /// Longest single present, in microseconds.
    pub fn max_us(&self) -> u64 {
        self.max_us.load(Ordering::Relaxed)
    }

    /// Mean present duration in milliseconds (0 without presents).
    pub fn mean_ms(&self) -> f64 {
        let presents = self.presents();
        if presents == 0 {
            0.0
        } else {
            self.total_us() as f64 / presents as f64 / 1000.0
        }
    }

    /// Samples of the frames actually presented, in present order. Frames
    /// coalesced away by the event loop are not in the list, so the frame
    /// delay is measured over the displayed frames only.
    pub fn displays(&self) -> Vec<DisplaySample> {
        self.displays
            .lock()
            .expect("present display log mutex poisoned")
            .clone()
    }

    fn record(&self, elapsed_us: u64) {
        self.presents.fetch_add(1, Ordering::Relaxed);
        self.total_us.fetch_add(elapsed_us, Ordering::Relaxed);
        self.max_us.fetch_max(elapsed_us, Ordering::Relaxed);
    }

    /// Logs that `frame_id` became visible at `present_ts` (ms since UNIX
    /// epoch).
    pub fn record_display(&self, frame_id: i64, capture_ts: Option<u128>, present_ts: u128) {
        self.displays
            .lock()
            .expect("present display log mutex poisoned")
            .push(DisplaySample {
                frame_id,
                capture_ts,
                present_ts,
            });
    }
}

/// Default window title used when no custom title was set with
/// [`WinitRenderer::with_title`].
pub const DEFAULT_WINDOW_TITLE: &str = "remotia";

/// A window that presents the frames of a pipeline.
pub struct WinitRenderer<K> {
    buffer_key: K,
    title: String,
    surface: Option<SharedSurface>,
    /// Set when a frame has been copied and not presented yet.
    pending: Arc<AtomicBool>,
    /// Wakes the event loop to present the latest frame.
    wake: Option<EventLoopProxy<LoopEvent>>,
    /// Present timings, shared with the event loop thread.
    present_stats: Arc<PresentStats>,
    /// Pipeline handle used to request a shutdown when Ctrl-C is pressed in
    /// the render window; `None` until
    /// [`WinitRenderer::with_pipeline_shutdown`] attaches one.
    pipeline_handle: Option<PipelineHandle>,
}

impl<K> WinitRenderer<K> {
    pub fn new(buffer_key: K) -> Self {
        Self {
            buffer_key,
            title: DEFAULT_WINDOW_TITLE.to_string(),
            surface: None,
            pending: Arc::new(AtomicBool::new(false)),
            wake: None,
            present_stats: Arc::new(PresentStats::default()),
            pipeline_handle: None,
        }
    }

    /// Sets the window title used by [`WinitRenderer::allocate`].
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Handle to the present timings (clone before the renderer is moved into
    /// the pipeline).
    pub fn present_stats(&self) -> Arc<PresentStats> {
        self.present_stats.clone()
    }

    /// Handle that closes the render window; only available after
    /// [`WinitRenderer::allocate`], which creates the event loop.
    pub fn shutdown_handle(&self) -> Option<RenderShutdown> {
        self.wake.as_ref().map(|proxy| RenderShutdown {
            proxy: proxy.clone(),
        })
    }

    /// Attaches the pipeline handle, enabling a graceful shutdown of the run
    /// when Ctrl-C is pressed in the render window.
    ///
    /// Without a handle the keypress is only logged; the run keeps going until
    /// its own shutdown.
    pub fn with_pipeline_shutdown(mut self, handle: PipelineHandle) -> Self {
        self.pipeline_handle = Some(handle);
        self
    }

    /// Creates the window and the pixel buffer sized `width`x`height`, and
    /// returns the runner that drives the event loop.
    ///
    /// When `monitor` is given, the window opens borderless fullscreen: on
    /// the RandR output name (e.g. `HDMI-1-0`) or 0-based index it names, or
    /// on the monitor the window opens on for the `current` spec. The error
    /// lists the available monitors when the spec matches none of them.
    pub fn allocate(
        &mut self,
        width: u32,
        height: u32,
        monitor: Option<&str>,
    ) -> Result<WinitRunner, String> {
        let event_loop = EventLoopBuilder::<LoopEvent>::with_user_event()
            .build()
            .expect("Failed to create the winit event loop");
        let wake = event_loop.create_proxy();

        let mut builder = WindowBuilder::new()
            .with_title(self.title.clone())
            .with_inner_size(PhysicalSize::new(width, height));

        if let Some(spec) = monitor {
            if is_current_monitor(spec) {
                log::info!("Render window: borderless fullscreen on the current monitor");
                builder = builder.with_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                let monitors: Vec<MonitorHandle> = event_loop.available_monitors().collect();
                let target = select_monitor(&monitors, spec)?;
                log::info!(
                    "Render window: borderless fullscreen on monitor '{}' ({}x{}, at {}, {})",
                    target.name().unwrap_or_else(|| "<unnamed>".to_string()),
                    target.size().width,
                    target.size().height,
                    target.position().x,
                    target.position().y,
                );
                builder = builder.with_fullscreen(Some(Fullscreen::Borderless(Some(target))));
            }
        }

        let window = builder
            .build(&event_loop)
            .expect("Failed to create the renderer window");

        let pixels = Pixels::new(width, height, SurfaceTexture::new(width, height, window))
            .expect("Failed to create the pixel buffer");
        let surface = Arc::new(Mutex::new(Surface {
            pixels,
            frame_id: 0,
            capture_ts: None,
        }));

        self.surface = Some(surface.clone());
        self.wake = Some(wake);

        Ok(WinitRunner {
            event_loop,
            surface,
            pending: self.pending.clone(),
            present_stats: self.present_stats.clone(),
            pipeline_handle: self.pipeline_handle.take(),
        })
    }
}

/// The monitor spec that asks for the monitor the window opens on
/// (winit's `Fullscreen::Borderless(None)`) instead of a named/indexed one.
const CURRENT_MONITOR_SPEC: &str = "current";

/// Whether a monitor spec requests fullscreen on the current monitor
/// (case-insensitive).
fn is_current_monitor(spec: &str) -> bool {
    spec.eq_ignore_ascii_case(CURRENT_MONITOR_SPEC)
}

/// Whether the monitor at `index` with output `name` matches a monitor spec:
/// a 0-based index or a RandR output name (case-insensitive).
fn matches_monitor(spec: &str, index: usize, name: Option<&str>) -> bool {
    match spec.parse::<usize>() {
        Ok(wanted) => wanted == index,
        Err(_) => name.is_some_and(|name| name.eq_ignore_ascii_case(spec)),
    }
}

/// Resolves a monitor spec to one of the winit monitors of the render display.
fn select_monitor(monitors: &[MonitorHandle], spec: &str) -> Result<MonitorHandle, String> {
    if let Some(monitor) = monitors
        .iter()
        .enumerate()
        .find(|(index, monitor)| matches_monitor(spec, *index, monitor.name().as_deref()))
        .map(|(_, monitor)| monitor.clone())
    {
        return Ok(monitor);
    }

    let available = monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            format!(
                "{index}: {} ({}x{})",
                monitor.name().unwrap_or_else(|| "<unnamed>".to_string()),
                monitor.size().width,
                monitor.size().height,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "no monitor matches '{spec}'; monitors on the render display: {available}"
    ))
}

/// Drives the winit event loop of a [`WinitRenderer`].
pub struct WinitRunner {
    event_loop: EventLoop<LoopEvent>,
    surface: SharedSurface,
    pending: Arc<AtomicBool>,
    present_stats: Arc<PresentStats>,
    /// Pipeline handle asked to shut down on Ctrl-C in the window.
    pipeline_handle: Option<PipelineHandle>,
}

// SAFETY: the runner is created on the main thread (where winit requires the
// event loop to live on some platforms) and immediately moved to the task
// that runs it. On Linux/X11 targets the event loop can be created and run
// on different threads, which is what the previous runner did as well.
unsafe impl Send for WinitRunner {}

impl WinitRunner {
    pub fn start(self) {
        let Self {
            event_loop,
            surface,
            pending,
            present_stats,
            pipeline_handle,
        } = self;

        event_loop.set_control_flow(ControlFlow::Wait);
        // Modifier state is delivered by its own events, so the Ctrl-C check
        // tracks the latest `ModifiersChanged` while handling key presses.
        let mut modifiers = ModifiersState::empty();
        event_loop
            .run(move |event, target| match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => target.exit(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    if size.width > 0 && size.height > 0 {
                        let resize = surface
                            .lock()
                            .unwrap()
                            .pixels
                            .resize_surface(size.width, size.height);
                        if let Err(error) = resize {
                            log::warn!(
                                "Failed to resize the render surface to {}x{}: {error}",
                                size.width,
                                size.height
                            );
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::ModifiersChanged(new_modifiers),
                    ..
                } => modifiers = new_modifiers.state(),
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event, .. },
                    ..
                } => {
                    if is_ctrl_c(event.state, &event.logical_key, event.repeat, modifiers) {
                        match &pipeline_handle {
                            Some(handle) => {
                                log::info!(
                                    "Ctrl-C pressed in the render window, requesting pipeline shutdown"
                                );
                                handle.request_shutdown();
                            }
                            None => log::debug!(
                                "Ctrl-C pressed in the render window without a pipeline handle attached"
                            ),
                        }
                    }
                }
                // Woken by `WinitRenderer::process` after a frame was copied.
                Event::UserEvent(LoopEvent::Present) => present(&surface, &pending, &present_stats),
                Event::UserEvent(LoopEvent::Shutdown) => target.exit(),
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => present(&surface, &pending, &present_stats),
                _ => {}
            })
            .expect("The winit event loop failed");
    }
}

/// Whether a keyboard event is the Ctrl-C keybinding: a non-repeat press of
/// the `c` key (matched on the logical key, so it follows the active layout)
/// with a Control modifier held.
fn is_ctrl_c(
    state: ElementState,
    logical_key: &Key,
    repeat: bool,
    modifiers: ModifiersState,
) -> bool {
    state == ElementState::Pressed
        && !repeat
        && modifiers.control_key()
        && matches!(logical_key, Key::Character(text) if text.eq_ignore_ascii_case("c"))
}

/// Presents the latest frame, unless it has already been presented.
///
/// The frame identity is read under the surface lock, before `Pixels::render`:
/// the pipeline task cannot refill the buffer while the lock is held, so the
/// logged display sample belongs to the frame that was actually rendered.
fn present(surface: &SharedSurface, pending: &AtomicBool, stats: &PresentStats) {
    if !pending.swap(false, Ordering::AcqRel) {
        return;
    }

    match surface.lock() {
        Ok(surface) => {
            let frame_id = surface.frame_id;
            let capture_ts = surface.capture_ts;

            let started = std::time::Instant::now();
            let result = surface.pixels.render();
            stats.record(started.elapsed().as_micros() as u64);

            if let Err(error) = result {
                log::warn!("Failed to present a frame: {error}");
                return;
            }

            // The frame reached the screen: log it for the frame-delay
            // measurement (capture -> display).
            stats.record_display(frame_id, capture_ts, now_timestamp());
        }
        Err(poisoned) => log::error!("Renderer mutex poisoned: {poisoned}"),
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for WinitRenderer<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + PresentedFrame + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let surface = match self.surface.as_ref() {
            Some(surface) => surface,
            None => {
                log::error!("Renderer received a frame before `allocate`");
                return Some(frame_data);
            }
        };

        // Read the frame identity before borrowing the pixel buffer out of
        // the DTO.
        let frame_id = frame_data.presented_frame_id();
        let capture_ts = frame_data.presented_capture_ts();

        let raw_frame_buffer = match frame_data.get_mut_ref(&self.buffer_key) {
            Some(buffer) => buffer,
            None => {
                log::warn!("Frame carries no render pixel buffer");
                return Some(frame_data);
            }
        };

        {
            let mut surface = surface.lock().unwrap();
            let frame = surface.pixels.frame_mut();
            if frame.len() != raw_frame_buffer.len() {
                log::error!(
                    "Render buffer holds {} bytes but the pixel buffer expects {}",
                    raw_frame_buffer.len(),
                    frame.len()
                );
                return Some(frame_data);
            }
            frame.copy_from_slice(raw_frame_buffer);
            // Remember which frame the buffer now holds: the next present
            // measures its capture-to-display delay.
            surface.frame_id = frame_id;
            surface.capture_ts = capture_ts;
        }

        self.pending.store(true, Ordering::Release);
        if let Some(wake) = &self.wake {
            if let Err(error) = wake.send_event(LoopEvent::Present) {
                log::debug!("Renderer event loop is gone: {error}");
            }
        }

        Some(frame_data)
    }
}

#[cfg(test)]
mod tests {
    use winit::event::ElementState;
    use winit::keyboard::{Key, ModifiersState};

    use super::{is_ctrl_c, is_current_monitor, matches_monitor, DisplaySample, PresentStats};

    #[test]
    fn ctrl_c_matches_only_a_pressed_control_c() {
        let c = Key::Character("c".into());
        let uppercase_c = Key::Character("C".into());
        let d = Key::Character("d".into());

        assert!(is_ctrl_c(
            ElementState::Pressed,
            &c,
            false,
            ModifiersState::CONTROL
        ));
        assert!(is_ctrl_c(
            ElementState::Pressed,
            &uppercase_c,
            false,
            ModifiersState::CONTROL
        ));
        assert!(!is_ctrl_c(
            ElementState::Pressed,
            &d,
            false,
            ModifiersState::CONTROL
        ));
        assert!(!is_ctrl_c(
            ElementState::Pressed,
            &c,
            false,
            ModifiersState::empty()
        ));
        assert!(!is_ctrl_c(
            ElementState::Released,
            &c,
            false,
            ModifiersState::CONTROL
        ));
        // Held keys repeat: request the shutdown once, not once per repeat.
        assert!(!is_ctrl_c(
            ElementState::Pressed,
            &c,
            true,
            ModifiersState::CONTROL
        ));
    }

    #[test]
    fn monitor_specs_match_by_index_or_name() {
        assert!(matches_monitor("1", 1, Some("HDMI-1-0")));
        assert!(!matches_monitor("1", 0, Some("eDP-1")));
        assert!(matches_monitor("0", 0, None));

        assert!(matches_monitor("HDMI-1-0", 1, Some("HDMI-1-0")));
        assert!(matches_monitor("hdmi-1-0", 1, Some("HDMI-1-0")));
        assert!(!matches_monitor("HDMI-1-0", 1, Some("eDP-1")));
        assert!(!matches_monitor("HDMI-1-0", 1, None));
    }

    #[test]
    fn current_monitor_spec_is_case_insensitive() {
        assert!(is_current_monitor("current"));
        assert!(is_current_monitor("CURRENT"));
        assert!(!is_current_monitor("0"));
        assert!(!is_current_monitor("HDMI-1-0"));
    }

    #[test]
    fn present_stats_track_timings_and_displays() {
        let stats = PresentStats::default();
        assert!(stats.displays().is_empty());
        assert_eq!(stats.presents(), 0);
        assert_eq!(stats.mean_ms(), 0.0);

        stats.record_display(7, Some(1_000), 1_035);
        stats.record(500);

        let displays: Vec<DisplaySample> = stats.displays();
        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].frame_id, 7);
        assert_eq!(displays[0].capture_ts, Some(1_000));
        assert_eq!(displays[0].present_ts, 1_035);
        assert_eq!(stats.presents(), 1);
        assert_eq!(stats.total_us(), 500);
        assert_eq!(stats.max_us(), 500);
    }
}
