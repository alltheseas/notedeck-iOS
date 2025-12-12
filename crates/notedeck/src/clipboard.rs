//! Platform-agnostic clipboard abstraction
//!
//! This trait allows different clipboard implementations:
//! - Desktop/Android: egui_winit::clipboard::Clipboard
//! - iOS: Uses egui-ios FFI events for UIPasteboard

/// Trait for clipboard operations
pub trait Clipboard: Send {
    /// Get text from the clipboard
    fn get(&mut self) -> Option<String>;

    /// Set text to the clipboard
    fn set_text(&mut self, text: String);
}

/// Desktop/Android clipboard implementation using egui-winit
#[cfg(not(target_os = "ios"))]
pub mod platform {
    use super::Clipboard;

    /// Wrapper around egui_winit::clipboard::Clipboard
    pub struct WinitClipboard {
        inner: egui_winit::clipboard::Clipboard,
    }

    impl WinitClipboard {
        pub fn new() -> Self {
            Self {
                inner: egui_winit::clipboard::Clipboard::new(None),
            }
        }
    }

    impl Default for WinitClipboard {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Clipboard for WinitClipboard {
        fn get(&mut self) -> Option<String> {
            self.inner.get()
        }

        fn set_text(&mut self, text: String) {
            self.inner.set_text(text);
        }
    }
}

/// iOS clipboard implementation
///
/// On iOS, clipboard operations are handled through the native UIKit layer.
/// The Swift side manages UIPasteboard directly, and clipboard events are
/// passed through the egui-ios FFI as InputEvent::Copy, Cut, Paste.
#[cfg(target_os = "ios")]
pub mod platform {
    use super::Clipboard;
    use std::sync::{Arc, Mutex};

    /// iOS clipboard that stores text locally
    ///
    /// The actual clipboard sync with UIPasteboard happens on the Swift side.
    /// This struct holds text that was received from Swift (for paste) or
    /// that should be sent to Swift (for copy).
    pub struct IosClipboard {
        /// Text received from Swift (paste content)
        paste_content: Arc<Mutex<Option<String>>>,
        /// Text to send to Swift (copy content)
        copy_content: Arc<Mutex<Option<String>>>,
    }

    impl IosClipboard {
        pub fn new() -> Self {
            Self {
                paste_content: Arc::new(Mutex::new(None)),
                copy_content: Arc::new(Mutex::new(None)),
            }
        }

        /// Called when Swift sends paste content from UIPasteboard
        pub fn receive_paste(&self, text: String) {
            if let Ok(mut content) = self.paste_content.lock() {
                *content = Some(text);
            }
        }

        /// Get copy content to send to Swift for UIPasteboard
        pub fn take_copy_content(&self) -> Option<String> {
            if let Ok(mut content) = self.copy_content.lock() {
                content.take()
            } else {
                None
            }
        }
    }

    impl Default for IosClipboard {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Clipboard for IosClipboard {
        fn get(&mut self) -> Option<String> {
            if let Ok(mut content) = self.paste_content.lock() {
                content.take()
            } else {
                None
            }
        }

        fn set_text(&mut self, text: String) {
            if let Ok(mut content) = self.copy_content.lock() {
                *content = Some(text);
            }
        }
    }
}

/// Type alias for the platform-specific clipboard
#[cfg(not(target_os = "ios"))]
pub type PlatformClipboard = platform::WinitClipboard;

#[cfg(target_os = "ios")]
pub type PlatformClipboard = platform::IosClipboard;
