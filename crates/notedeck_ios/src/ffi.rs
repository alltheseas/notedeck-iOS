//! Swift FFI bindings via swift-bridge

use crate::input::InputEvent;
use crate::output::{CursorIcon, OutputState};
use crate::renderer::NotedeckRenderer;

use std::ffi::c_void;

#[swift_bridge::bridge]
pub mod ffi {
    extern "Rust" {
        type InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_pointer_moved(x: f32, y: f32) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_mouse_wheel(x: f32, y: f32) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_left_mouse_down(x: f32, y: f32, pressed: bool) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_right_mouse_down(x: f32, y: f32, pressed: bool) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_window_focused(focused: bool) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_scene_phase_changed(phase: u8) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_text_commit(text: String) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_ime_preedit(text: String) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_keyboard_visibility(visible: bool) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_virtual_key(key_code: u8, pressed: bool) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_copy() -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_cut() -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_paste(text: String) -> InputEvent;
    }

    extern "Rust" {
        type OutputState;

        fn get_cursor_icon(&self) -> &CursorIcon;

        fn wants_keyboard(&self) -> bool;

        fn has_ime_rect(&self) -> bool;
        fn get_ime_rect_x(&self) -> f32;
        fn get_ime_rect_y(&self) -> f32;
        fn get_ime_rect_width(&self) -> f32;
        fn get_ime_rect_height(&self) -> f32;

        fn get_copied_text(&self) -> &str;
    }

    extern "Rust" {
        type CursorIcon;

        fn is_default(&self) -> bool;
        fn is_pointing_hand(&self) -> bool;
        fn is_resize_horizontal(&self) -> bool;
        fn is_resize_vertical(&self) -> bool;
        fn is_text(&self) -> bool;
    }

    extern "Rust" {
        type NotedeckRenderer;

        #[swift_bridge(init)]
        fn new(
            view_ptr: *mut c_void,
            width: u32,
            height: u32,
            display_scale: f32,
            data_path: String,
        ) -> NotedeckRenderer;

        fn resize(&mut self, width: u32, height: u32);

        fn set_safe_area(&mut self, top: f32, right: f32, bottom: f32, left: f32);

        fn render(&mut self, time: f64, input_events: Vec<InputEvent>) -> OutputState;
    }
}
