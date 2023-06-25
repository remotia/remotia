#![allow(unused_imports)]
//! remotia is a pure Rust framework to design remote rendering and general video streaming pipelines as code
//! in a modular and intuitive way that makes it simple to alter and reuse components.
//! The project is in early stage and documentation is in progress, but it is already usable for experimentations
//! and to develop prototypes of cloud gaming software without relying on specific APIs.

pub mod common;

pub mod pipeline;

pub mod traits;
// pub mod types;
// pub mod error;

pub mod processors;
