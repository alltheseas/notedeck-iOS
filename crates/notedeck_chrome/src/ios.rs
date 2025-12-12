//! iOS entry point for Notedeck
//!
//! Unlike Android where eframe controls the event loop, on iOS:
//! - SwiftUI controls the event loop
//! - Swift calls Rust functions each frame
//! - Rust processes input, runs egui, returns output
//!
//! This module provides a simple FFI interface that works with egui-ios types.

use crate::chrome::Chrome;
use egui::{Context, RawInput};
use egui_ios::{InputEvent, OutputState};
use notedeck::{App, Notedeck};
use std::path::PathBuf;

/// iOS app state - holds the Notedeck and Chrome instances between frames
pub struct NotedeckIos {
    ctx: Context,
    notedeck: Notedeck,
    chrome: Option<Chrome>,
}

impl NotedeckIos {
    /// Create a new Notedeck iOS instance
    ///
    /// # Arguments
    /// * `data_path` - Path to the app's data directory (from Swift)
    /// * `scale_factor` - Screen scale factor (e.g., 2.0 for Retina)
    pub fn new(data_path: String, scale_factor: f32) -> Self {
        // Initialize tracing for iOS
        use tracing_subscriber::{prelude::*, EnvFilter};

        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_level(true)
            .with_target(false)
            .without_time();

        let filter_layer = EnvFilter::try_new("info,notedeck=debug").unwrap();

        let _ = tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .try_init();

        tracing::info!("NotedeckIos::new called with data_path: {}", data_path);

        // Create egui context
        let ctx = Context::default();
        ctx.set_pixels_per_point(scale_factor);

        // Create Notedeck with default args
        // Include --debug to skip the debug mode check (iOS builds are typically debug during development)
        let path = PathBuf::from(&data_path);

        // Ensure data directory exists
        if let Err(e) = std::fs::create_dir_all(&path) {
            tracing::error!("Failed to create data directory: {:?}", e);
        }

        let app_args = vec!["notedeck".to_string(), "--debug".to_string()];

        let mut notedeck = Notedeck::new(&ctx, path, &app_args);
        notedeck.setup(&ctx);

        // Create Chrome UI container for iOS
        let chrome = match Chrome::new_with_apps_ios(&app_args, &mut notedeck) {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::error!("Failed to create Chrome: {:?}", e);
                None
            }
        };

        Self {
            ctx,
            notedeck,
            chrome,
        }
    }

    /// Process a frame with the given input events
    ///
    /// # Arguments
    /// * `events` - Input events from Swift (touches, keyboard, etc.)
    /// * `screen_width` - Current screen width in points
    /// * `screen_height` - Current screen height in points
    ///
    /// # Returns
    /// * `OutputState` - Contains cursor icon, keyboard requests, etc.
    pub fn frame(
        &mut self,
        events: Vec<InputEvent>,
        screen_width: f32,
        screen_height: f32,
    ) -> OutputState {
        // Build RawInput from iOS events
        let mut raw_input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(screen_width, screen_height),
            )),
            ..Default::default()
        };

        // Convert iOS events to egui events
        for event in events {
            if let Some(egui_event) = event.into_egui_event() {
                raw_input.events.push(egui_event);
            }
        }

        // Run egui frame
        let full_output = self.ctx.run(raw_input, |ctx| {
            // Create a temporary UI for the chrome
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    if let Some(chrome) = &mut self.chrome {
                        let mut app_ctx = self.notedeck.app_context();
                        let _ = chrome.update(&mut app_ctx, ui);
                    }
                });
        });

        // Convert output to iOS format
        let cursor = full_output.platform_output.cursor_icon;
        let wants_kb = self.ctx.wants_keyboard_input();
        let ime_rect = full_output.platform_output.ime.as_ref().map(|ime| ime.rect);

        OutputState::with_keyboard_state(cursor.into(), wants_kb, ime_rect)
    }

    /// Get the current egui context for rendering
    pub fn context(&self) -> &Context {
        &self.ctx
    }
}

// Global instance for FFI (iOS apps typically have a single instance)
static mut NOTEDECK_IOS: Option<NotedeckIos> = None;

/// Initialize the Notedeck iOS instance
///
/// # Safety
/// Must be called from the main thread before any other FFI functions.
#[no_mangle]
pub unsafe extern "C" fn notedeck_ios_init(
    data_path: *const std::ffi::c_char,
    scale_factor: f32,
) {
    let data_path = if data_path.is_null() {
        String::new()
    } else {
        std::ffi::CStr::from_ptr(data_path)
            .to_string_lossy()
            .into_owned()
    };

    NOTEDECK_IOS = Some(NotedeckIos::new(data_path, scale_factor));
}

/// Check if Notedeck iOS is initialized
#[no_mangle]
pub extern "C" fn notedeck_ios_is_initialized() -> bool {
    unsafe { NOTEDECK_IOS.is_some() }
}

/// Cleanup the Notedeck iOS instance
///
/// # Safety
/// Must be called from the main thread.
#[no_mangle]
pub unsafe extern "C" fn notedeck_ios_cleanup() {
    NOTEDECK_IOS = None;
}
