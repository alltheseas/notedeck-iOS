# Notedeck iOS

Native iOS support for Notedeck using wgpu with Metal backend and swift-bridge for Rust/Swift FFI.

## Architecture Overview

Unlike desktop (eframe) or Android (winit), iOS uses a different architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                      SwiftUI Layer                          │
│  ┌─────────────────┐  ┌──────────────────────────────────┐  │
│  │ NotedeckMobileApp│  │         NotedeckView             │  │
│  │   (App Entry)   │  │  (GeometryReader + UIViewRep)    │  │
│  └─────────────────┘  └──────────────────────────────────┘  │
│                              │                              │
│                              ▼                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              NotedeckUIView (UIView)                 │   │
│  │  - CAMetalLayer for rendering                        │   │
│  │  - Touch event handling                              │   │
│  │  - Keyboard input                                    │   │
│  └──────────────────────────────────────────────────────┘   │
│                              │                              │
│                              ▼                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         NotedeckRendererController                   │   │
│  │  - CADisplayLink for 120fps render loop              │   │
│  │  - Safe area inset handling                          │   │
│  │  - Event queue management                            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ swift-bridge FFI
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       Rust Layer                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              NotedeckRenderer (renderer.rs)          │   │
│  │  - wgpu Device/Queue/Surface                         │   │
│  │  - egui Context + Renderer                           │   │
│  │  - Notedeck + Chrome                                 │   │
│  └──────────────────────────────────────────────────────┘   │
│                              │                              │
│                              ▼                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                    Notedeck Core                     │   │
│  │  - nostrdb (LMDB with iOS-specific 1GiB map size)    │   │
│  │  - Relay connections                                 │   │
│  │  - Account management                                │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Key Differences from Desktop/Android

| Aspect | Desktop (eframe) | Android | iOS |
|--------|------------------|---------|-----|
| Event Loop | eframe/winit | winit via android-activity | SwiftUI/CADisplayLink |
| Rendering | wgpu/glow | wgpu/glow | wgpu (Metal only) |
| Surface | Window handle | ANativeWindow | CAMetalLayer |
| FFI | N/A | JNI | swift-bridge |
| Safe Area | N/A | Window insets | UIWindow.safeAreaInsets |

## Building

### Prerequisites

1. Rust with iOS targets:
   ```bash
   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
   ```

2. swift-bridge-cli:
   ```bash
   cargo install swift-bridge-cli
   ```

3. Xcode with iOS SDK (14.0+)

4. xcodegen (optional, for regenerating project):
   ```bash
   brew install xcodegen
   ```

### Build Steps

1. Build the Rust libraries and generate Swift package:
   ```bash
   ./crates/notedeck_ios/scripts/build_release.sh
   ```

   This script:
   - Builds `libnotedeck_ios.a` for `aarch64-apple-ios` (device)
   - Builds `libnotedeck_ios.a` for `aarch64-apple-ios-sim` (simulator)
   - Runs `swift-bridge-cli` to generate the Swift package at `crates/notedeck_ios/NotedeckMobile/`

2. Open the Xcode project:
   ```bash
   open ios-app/NotedeckMobile/NotedeckMobile.xcodeproj
   ```

3. Build and run on device/simulator from Xcode.

### Regenerating Xcode Project

If you need to modify the project structure:

```bash
cd ios-app/NotedeckMobile
xcodegen generate
```

## Crate Structure

### `notedeck_ios` (this crate)

Main iOS renderer crate with:
- `renderer.rs` - wgpu/Metal rendering, egui integration, safe area handling
- `input.rs` - iOS touch/keyboard events to egui events
- `output.rs` - egui output to iOS (cursor, keyboard requests, clipboard)
- `ffi.rs` - swift-bridge FFI definitions

### `egui-ios`

Shared egui iOS types (used by both `notedeck_ios` and potentially other iOS egui apps):
- `InputEvent` - Touch, keyboard, clipboard events
- `OutputState` - Cursor icon, keyboard visibility, IME rect
- FFI bindings for swift-bridge

## iOS-Specific Considerations

### Memory Limits

iOS has stricter virtual memory limits than desktop. Key adjustments:

- **LMDB map size**: Reduced from 1 TiB (desktop) to 1 GiB (iOS)
  - Location: `crates/notedeck/src/app.rs`
  - iOS kills apps that try to map too much virtual memory

### Safe Area Insets

iOS devices have notches, Dynamic Island, and home indicators that require content insets:

```rust
// Safe area is passed from Swift to Rust each frame
renderer.set_safe_area(top, right, bottom, left);

// Applied as egui::Frame margin in render()
let frame = egui::Frame::NONE.inner_margin(egui::Margin {
    top: safe_area.top.round() as i8,
    // ...
});
```

### Mobile UI Detection

The app uses `is_narrow()` to detect mobile screens:
- Screen width < 550 points triggers mobile UI (`render_damus_mobile`)
- Screen width >= 550 points triggers desktop UI (`render_damus_desktop`)

iPhone 13 reports ~390 points width, correctly triggering mobile UI.

### Debug Mode

Desktop notedeck has a debug mode panic for non-debug builds. This is bypassed on iOS:
- Location: `crates/notedeck_chrome/src/chrome.rs` in `stop_debug_mode()`
- Reason: iOS developers building from source know what they're doing

## Swift/Rust FFI

The FFI uses `swift-bridge` which generates:
- Rust structs/methods exposed to Swift
- Swift wrappers in `NotedeckMobile.swift`

Key FFI types:
```rust
// From Rust to Swift
type NotedeckRenderer;  // Main renderer
type OutputState;       // Frame output (cursor, keyboard)
type CursorIcon;        // Cursor type

// From Swift to Rust
type InputEvent;        // Touch, keyboard events
```

## Troubleshooting

### "mdb_env_open failed, error 12"
LMDB memory map too large. Ensure iOS uses 1 GiB map size.

### Black screen on launch
Check Xcode console for panic messages. Common causes:
- Missing data directory
- wgpu/Metal initialization failure

### UI overlaps status bar
Ensure `updateSafeArea()` is being called from Swift and `set_safe_area()` is wired up in Rust.

### Desktop UI showing on iPhone
Check console for "Screen dimensions" log. If `is_narrow=false`, there's a coordinate issue.

## Files Reference

```
notedeck/
├── crates/
│   ├── egui-ios/                    # Shared iOS egui types
│   │   ├── src/
│   │   │   ├── input.rs             # InputEvent enum
│   │   │   ├── output.rs            # OutputState struct
│   │   │   └── ffi.rs               # swift-bridge definitions
│   │   └── Cargo.toml
│   │
│   ├── notedeck_ios/                # Main iOS renderer
│   │   ├── src/
│   │   │   ├── renderer.rs          # wgpu/egui rendering
│   │   │   ├── input.rs             # Re-export from egui-ios
│   │   │   ├── output.rs            # Re-export + extensions
│   │   │   └── ffi.rs               # NotedeckRenderer FFI
│   │   ├── scripts/
│   │   │   ├── build_debug.sh
│   │   │   └── build_release.sh
│   │   ├── generated/               # swift-bridge generated
│   │   └── NotedeckMobile/          # Generated Swift package
│   │
│   ├── notedeck/
│   │   └── src/
│   │       ├── app.rs               # iOS LMDB map size
│   │       └── clipboard.rs         # iOS clipboard impl
│   │
│   └── notedeck_chrome/
│       └── src/
│           ├── ios.rs               # iOS entry point (alternative)
│           └── chrome.rs            # Debug mode skip for iOS
│
└── ios-app/
    └── NotedeckMobile/
        ├── project.yml              # xcodegen project definition
        ├── NotedeckMobile.xcodeproj # Generated Xcode project
        └── NotedeckMobile/
            ├── NotedeckMobileApp.swift
            └── Views/
                ├── NotedeckView.swift
                ├── NotedeckUIView.swift
                └── NotedeckRendererController.swift
```
