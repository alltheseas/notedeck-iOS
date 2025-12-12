//! iOS rendering layer for Notedeck
//!
//! This crate provides the wgpu/Metal rendering infrastructure for running
//! Notedeck on iOS devices. It handles:
//!
//! - wgpu setup with Metal backend
//! - egui rendering via egui_wgpu_backend
//! - Swift FFI via swift-bridge
//! - Input event translation from iOS to egui

mod ffi;
mod input;
mod output;
mod renderer;

pub use input::InputEvent;
pub use output::{CursorIcon, OutputState};
pub use renderer::NotedeckRenderer;
