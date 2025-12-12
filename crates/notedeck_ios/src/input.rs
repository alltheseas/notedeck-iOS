//! Input event handling for iOS

/// Input events from iOS (touch, keyboard, etc.)
pub enum InputEvent {
    /// Pointer/touch moved to position
    PointerMoved(f32, f32),
    /// Mouse wheel scroll (for trackpad)
    MouseWheel(f32, f32),
    /// Left mouse/touch down at position
    LeftMouseDown(f32, f32, bool),
    /// Right mouse down (long press)
    RightMouseDown(f32, f32, bool),
    /// Window focus changed
    WindowFocused(bool),
    /// Scene phase changed (0=background, 1=inactive, 2=active)
    ScenePhaseChanged(u8),
    /// Text committed from keyboard
    TextCommit(String),
    /// IME preedit text
    ImePreedit(String),
    /// Keyboard visibility changed
    KeyboardVisibility(bool),
    /// Virtual key press (backspace=0, enter=1, tab=2, escape=3, arrows=4-7)
    VirtualKey(u8, bool),
    /// Copy command
    Copy,
    /// Cut command
    Cut,
    /// Paste with text
    Paste(String),
}

impl InputEvent {
    /// Convert to egui event
    pub fn into_egui_event(self) -> Option<egui::Event> {
        match self {
            InputEvent::PointerMoved(x, y) => Some(egui::Event::PointerMoved(egui::pos2(x, y))),

            InputEvent::MouseWheel(x, y) => Some(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(x, y),
                modifiers: egui::Modifiers::NONE,
            }),

            InputEvent::LeftMouseDown(x, y, pressed) => Some(egui::Event::PointerButton {
                pos: egui::pos2(x, y),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }),

            InputEvent::RightMouseDown(x, y, pressed) => Some(egui::Event::PointerButton {
                pos: egui::pos2(x, y),
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }),

            InputEvent::WindowFocused(focused) => Some(egui::Event::WindowFocused(focused)),

            InputEvent::ScenePhaseChanged(_phase) => {
                // Could map to WindowFocused based on phase
                None
            }

            InputEvent::TextCommit(text) => Some(egui::Event::Text(text)),

            InputEvent::ImePreedit(text) => Some(egui::Event::Ime(egui::ImeEvent::Preedit(text))),

            InputEvent::KeyboardVisibility(_visible) => {
                // This is handled separately, not as an egui event
                None
            }

            InputEvent::VirtualKey(key_code, pressed) => {
                let key = match key_code {
                    0 => egui::Key::Backspace,
                    1 => egui::Key::Enter,
                    2 => egui::Key::Tab,
                    3 => egui::Key::Escape,
                    4 => egui::Key::ArrowUp,
                    5 => egui::Key::ArrowDown,
                    6 => egui::Key::ArrowLeft,
                    7 => egui::Key::ArrowRight,
                    _ => return None,
                };

                Some(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
            }

            InputEvent::Copy => Some(egui::Event::Copy),
            InputEvent::Cut => Some(egui::Event::Cut),
            InputEvent::Paste(text) => Some(egui::Event::Paste(text)),
        }
    }

    // FFI constructors for swift-bridge
    pub fn from_pointer_moved(x: f32, y: f32) -> Self {
        Self::PointerMoved(x, y)
    }

    pub fn from_mouse_wheel(x: f32, y: f32) -> Self {
        Self::MouseWheel(x, y)
    }

    pub fn from_left_mouse_down(x: f32, y: f32, pressed: bool) -> Self {
        Self::LeftMouseDown(x, y, pressed)
    }

    pub fn from_right_mouse_down(x: f32, y: f32, pressed: bool) -> Self {
        Self::RightMouseDown(x, y, pressed)
    }

    pub fn from_window_focused(focused: bool) -> Self {
        Self::WindowFocused(focused)
    }

    pub fn from_scene_phase_changed(phase: u8) -> Self {
        Self::ScenePhaseChanged(phase)
    }

    pub fn from_text_commit(text: String) -> Self {
        Self::TextCommit(text)
    }

    pub fn from_ime_preedit(text: String) -> Self {
        Self::ImePreedit(text)
    }

    pub fn from_keyboard_visibility(visible: bool) -> Self {
        Self::KeyboardVisibility(visible)
    }

    pub fn from_virtual_key(key_code: u8, pressed: bool) -> Self {
        Self::VirtualKey(key_code, pressed)
    }

    pub fn from_copy() -> Self {
        Self::Copy
    }

    pub fn from_cut() -> Self {
        Self::Cut
    }

    pub fn from_paste(text: String) -> Self {
        Self::Paste(text)
    }
}
