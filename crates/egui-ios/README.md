# egui-ios

iOS FFI bindings for [egui](https://github.com/emilk/egui) via [swift-bridge](https://github.com/chinedufn/swift-bridge).

This crate provides Swift-compatible types for embedding egui in iOS apps:

- `InputEvent` - Touch, keyboard, and lifecycle events from Swift to egui
- `OutputState` - Cursor, keyboard, and IME state from egui to Swift
- `CursorIcon` - Cursor icons mapped to iOS equivalents

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
egui-ios = "0.1"
```

In your Rust code:

```rust
use egui_ios::{InputEvent, OutputState, CursorIcon};

// Convert input events to egui events
let egui_events: Vec<egui::Event> = input_events
    .into_iter()
    .filter_map(|e| e.into_egui_event())
    .collect();

// Run egui frame
let full_output = ctx.run(raw_input, |ctx| {
    // Your UI code
});

// Create output state for Swift
let output = OutputState::with_keyboard_state(
    full_output.platform_output.cursor_icon.into(),
    ctx.wants_keyboard_input(),
    full_output.platform_output.ime.as_ref().map(|ime| ime.rect),
);
```

## Swift Integration

The build generates Swift bindings in `generated/egui-ios/`. Include these in your Xcode project.

See the [SwiftUI Embedding Guide](../../docs/SWIFTUI_EMBEDDING.md) for complete integration examples including:
- Touch event handling
- Native keyboard relay with autocomplete/autocorrect
- Scene lifecycle callbacks
- IME/CJK input support

## Input Events

| Event | Description |
|-------|-------------|
| `from_pointer_moved(x, y)` | Touch/pointer position |
| `from_left_mouse_down(x, y, pressed)` | Primary touch |
| `from_mouse_wheel(x, y)` | Scroll gesture |
| `from_text_commit(text)` | Committed text after autocomplete |
| `from_ime_preedit(text)` | IME composition text |
| `from_virtual_key(code, pressed)` | Special keys (backspace, enter, etc.) |
| `from_scene_phase_changed(phase)` | iOS scene lifecycle |

## Virtual Key Codes

| Code | Key |
|------|-----|
| 0 | Backspace |
| 1 | Enter |
| 2 | Tab |
| 3 | Escape |
| 4-7 | Arrow keys (up, down, left, right) |

## Output State

Check `OutputState` after each frame:

- `wants_keyboard()` - Show/hide iOS keyboard
- `has_ime_rect()` / `get_ime_rect_*()` - Keyboard positioning
- `get_cursor_icon()` - Cursor for pointer interactions
