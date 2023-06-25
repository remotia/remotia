use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    iter::Sum,
    ops::Div,
    time::{Duration, Instant},
};

use remotia_core::traits::{FrameProcessor, FrameProperties};

use async_trait::async_trait;
use log::info;

pub struct ConsoleAverageStatsLogger<K, V> {
    header: Option<String>,
    round_duration: Duration,

    current_round_start: Instant,

    logged_stats: HashMap<K, Vec<V>>,
}

impl<K, V> Default for ConsoleAverageStatsLogger<K, V> {
    fn default() -> Self {
        Self {
            header: None,
            round_duration: Duration::from_secs(1),
            current_round_start: Instant::now(),
            logged_stats: HashMap::new(),
        }
    }
}

impl<K, V> ConsoleAverageStatsLogger<K, V>
where
    K: Debug + Eq + Hash,
    V: Debug + Copy + TryFrom<usize> + Sum<V> + Div<Output = V>,
{
    pub fn new() -> Self {
        Self::default()
    }

    // Building functions
    pub fn header(mut self, header: &str) -> Self {
        self.header = Some(header.to_string());
        self
    }

    pub fn log(mut self, key: K) -> Self {
        self.logged_stats.insert(key, Vec::new());
        self
    }

    // Logging functions
    fn print_round_stats(&self) {
        if self.header.is_some() {
            info!("{}", self.header.as_ref().unwrap());
        }

        self.logged_stats.iter().for_each(|(key, values)| {
            match values.len().try_into() {
                Ok(logged_values_count) => {
                    let avg = values
                        .iter()
                        .map(|v| *v)
                        .sum::<V>()
                        .div(logged_values_count);

                    info!("Average {:?}: {:?}", key, avg);
                },
                Err(_) => {
                    log::warn!("Unable to print '{:?}' values due to failed logged frames count conversion", key);
                }
            };

        });
    }

    fn reset_round(&mut self) {
        self.logged_stats.values_mut().for_each(Vec::clear);
        self.current_round_start = Instant::now();
    }
}

#[async_trait]
impl<F, K, V> FrameProcessor<F> for ConsoleAverageStatsLogger<K, V>
where
    K: Copy + Eq + Hash + Send + Debug,
    V: Copy + Debug + Send + TryFrom<usize> + Sum + Div<Output = V>,
    F: FrameProperties<K, V> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        // self.logged_stats.insert(frame_data.clone_without_buffers());
        for (key, logged_values) in self.logged_stats.iter_mut() {
            match frame_data.get(key) {
                Some(value) => logged_values.push(value),
                None => (),
            };
        }

        if self.current_round_start.elapsed().gt(&self.round_duration) {
            self.print_round_stats();
            self.reset_round();
        }
        Some(frame_data)
    }
}
