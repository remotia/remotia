use std::{collections::HashMap, fmt::Display};

use bytes::{Bytes, BytesMut};
use serde::Serialize;

use crate::error::DropReason;

#[derive(Default, Clone, Debug)]
pub struct FrameData {
    readonly_buffers: HashMap<String, Bytes>,
    writable_buffers: HashMap<String, BytesMut>,

    stats: HashMap<String, u128>,

    drop_reason: Option<DropReason>,
}

impl FrameData {
    //*******//
    // Stats //
    //*******//

    pub fn set(&mut self, key: &str, value: u128) {
        self.stats.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> u128 {
        *self
            .stats
            .get(key)
            .unwrap_or_else(|| panic!("{}", missing_key_msg(key)))
    }

    pub fn has(&self, key: &str) -> bool {
        self.stats.contains_key(key)
    }

    pub fn get_stats(&self) -> &HashMap<String, u128> {
        &self.stats
    }

    pub fn merge_stats(&mut self, other_stats: HashMap<String, u128>) {
        self.stats.extend(other_stats);
    }

    //*********//
    // Buffers //
    //*********//

    pub fn insert_readonly_buffer(&mut self, key: &str, buffer: Bytes) {
        self.readonly_buffers.insert(key.to_string(), buffer);
    }

    pub fn extract_readonly_buffer(&mut self, key: &str) -> Option<Bytes> {
        self.readonly_buffers.remove(key)
    }

    pub fn has_readonly_buffer(&self, key: &str) -> bool {
        self.readonly_buffers.contains_key(key)
    }

    pub fn get_readonly_buffer_ref(&mut self, key: &str) -> &Bytes {
        self.readonly_buffers
            .get(key)
            .unwrap_or_else(|| panic!("{}", missing_key_msg(key)))
    }

    pub fn insert_writable_buffer(&mut self, key: &str, buffer: BytesMut) {
        self.writable_buffers.insert(key.to_string(), buffer);
    }

    pub fn extract_writable_buffer(&mut self, key: &str) -> Option<BytesMut> {
        self.writable_buffers.remove(key)
    }

    pub fn get_writable_buffer_ref(&mut self, key: &str) -> Option<&mut BytesMut> {
        self.writable_buffers.get_mut(key)
    }

    pub fn has_writable_buffer(&self, key: &str) -> bool {
        self.writable_buffers.contains_key(key)
    }

    //*************//
    // Drop reason //
    //*************//

    pub fn set_drop_reason(&mut self, error: Option<DropReason>) {
        self.drop_reason = error;
    }

    pub fn get_drop_reason(&self) -> Option<DropReason> {
        self.drop_reason
    }

    //*******//
    // Other //
    //*******//

    pub fn clone_without_buffers(&self) -> Self {
        Self {
            stats: self.stats.clone(),
            drop_reason: self.drop_reason,

            ..Default::default()
        }
    }
}

fn missing_key_msg(key: &str) -> String {
    format!("Missing key '{}'", key)
}

impl Display for FrameData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ Read-only buffers: {:?}, Writable buffers: {:?}, Stats: {:?}, Drop reason: {:?} }}",
            self.readonly_buffers.keys(),
            self.writable_buffers.keys(),
            self.stats,
            self.drop_reason
        )
    }
}
