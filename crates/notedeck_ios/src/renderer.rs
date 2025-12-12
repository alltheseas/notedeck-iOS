//! wgpu/Metal renderer for Notedeck on iOS
//!
//! This module provides the core rendering infrastructure for iOS:
//! - wgpu with Metal backend for GPU rendering
//! - egui integration for immediate-mode UI
//! - Safe area inset handling for notch/Dynamic Island
//! - CAMetalLayer surface management
//!
//! The renderer is created from Swift via FFI and receives:
//! - A CAMetalLayer pointer for rendering surface
//! - Screen dimensions and scale factor
//! - Input events each frame (touch, keyboard, etc.)
//!
//! Unlike eframe on desktop, there's no event loop here - Swift's
//! CADisplayLink drives the render loop and calls render() each frame.

use std::path::PathBuf;
use std::sync::Arc;

use futures::executor;

use notedeck::{App, Notedeck};
use notedeck_chrome::Chrome;

use crate::input::InputEvent;
use crate::output::{CursorIcon, OutputState};

/// Safe area insets from iOS (in points, not pixels).
///
/// These represent the areas of the screen obscured by hardware features:
/// - `top`: Status bar, notch, or Dynamic Island (~47pt on iPhone with notch)
/// - `bottom`: Home indicator on Face ID devices (~34pt)
/// - `left`/`right`: Usually 0 except in landscape or with side features
///
/// The renderer applies these as egui::Frame margins so UI content doesn't
/// underlap the notch or home indicator.
#[derive(Clone, Copy, Default)]
pub struct SafeAreaInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// The main wgpu/Metal renderer for Notedeck on iOS.
///
/// This struct owns all rendering state and is created once when the app starts.
/// Swift holds a reference to this via the FFI and calls render() each frame.
pub struct NotedeckRenderer {
    // wgpu
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,

    // egui
    context: egui::Context,
    raw_input: egui::RawInput,
    egui_renderer: egui_wgpu::Renderer,

    // Notedeck
    notedeck: Notedeck,
    chrome: Option<Chrome>,

    // iOS safe area
    safe_area: SafeAreaInsets,
    display_scale: f32,
}

impl NotedeckRenderer {
    /// Create a new renderer with the given Metal layer
    ///
    /// # Arguments
    /// * `layer_ptr` - Pointer to CAMetalLayer
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    /// * `display_scale` - Display scale factor (e.g., 2.0 for Retina)
    /// * `data_path` - Path to app data directory
    pub fn new(
        layer_ptr: *mut std::ffi::c_void,
        width: u32,
        height: u32,
        display_scale: f32,
        data_path: String,
    ) -> Self {
        // Initialize tracing
        Self::init_tracing();

        tracing::info!(
            "NotedeckRenderer::new width={} height={} scale={}",
            width,
            height,
            display_scale
        );

        // Setup wgpu with Metal backend
        let descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..Default::default()
        };
        let instance = wgpu::Instance::new(&descriptor);
        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer_ptr))
                .expect("Failed to create surface")
        };

        let adapter = executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("Failed to find an appropriate adapter");

        let (device, queue) = executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                ..Default::default()
            },
            None,
        ))
        .expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let tex_format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let mut config = surface
            .get_default_config(&adapter, width, height)
            .expect("Failed to create config");
        config.format = tex_format;
        config.view_formats = vec![tex_format];

        surface.configure(&device, &config);

        // Setup egui
        let context = egui::Context::default();
        context.set_pixels_per_point(display_scale);

        let raw_input = egui::RawInput {
            viewport_id: egui::ViewportId::ROOT,
            viewports: std::iter::once((
                egui::ViewportId::ROOT,
                egui::ViewportInfo {
                    native_pixels_per_point: Some(display_scale),
                    focused: Some(true),
                    ..Default::default()
                },
            ))
            .collect(),
            predicted_dt: 1.0 / 120.0,
            focused: true,
            system_theme: None,
            max_texture_side: Some(wgpu::Limits::default().max_texture_dimension_2d as usize),
            ..Default::default()
        };

        // Create egui renderer
        let egui_renderer = egui_wgpu::Renderer::new(&device, tex_format, None, 1, false);

        // Initialize Notedeck
        let path = PathBuf::from(data_path);
        let app_args = vec!["notedeck".to_string()];

        let mut notedeck = Notedeck::new(&context, path, &app_args);
        notedeck.setup(&context);

        // Create Chrome
        let chrome = match Chrome::new_with_apps_ios(&app_args, &mut notedeck) {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::error!("Failed to create Chrome: {:?}", e);
                None
            }
        };

        Self {
            device,
            queue,
            surface,
            config,
            context,
            raw_input,
            egui_renderer,
            notedeck,
            chrome,
            safe_area: SafeAreaInsets::default(),
            display_scale,
        }
    }

    /// Set safe area insets (in points, not pixels)
    pub fn set_safe_area(&mut self, top: f32, right: f32, bottom: f32, left: f32) {
        self.safe_area = SafeAreaInsets {
            top,
            right,
            bottom,
            left,
        };
        tracing::debug!(
            "Safe area set: top={}, right={}, bottom={}, left={}",
            top,
            right,
            bottom,
            left
        );
    }

    fn init_tracing() {
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
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Render a frame
    ///
    /// # Arguments
    /// * `time` - Current time in seconds
    /// * `input_events` - Input events from iOS
    ///
    /// # Returns
    /// Output state for Swift (cursor, keyboard, etc.)
    pub fn render(&mut self, time: f64, input_events: Vec<InputEvent>) -> OutputState {
        let ctx = &self.context;

        self.raw_input.time = Some(time);

        // Set screen rect
        let width_points = self.config.width as f32 / ctx.pixels_per_point();
        let height_points = self.config.height as f32 / ctx.pixels_per_point();
        let rect =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width_points, height_points));
        self.raw_input.screen_rect = Some(rect);

        // Log screen dimensions for debugging mobile vs desktop UI detection
        static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            tracing::info!(
                "Screen dimensions: {}x{} pixels, {}x{} points (ppp={}), is_narrow={}",
                self.config.width,
                self.config.height,
                width_points,
                height_points,
                ctx.pixels_per_point(),
                width_points < 550.0
            );
        }

        // Convert input events
        self.raw_input.events = input_events
            .into_iter()
            .filter_map(|e| e.into_egui_event())
            .collect();

        // Run egui frame with safe area handling
        let safe_area = self.safe_area;
        let full_output = ctx.run(self.raw_input.take(), |ctx| {
            // Create a frame with margin for safe area insets
            // Safe area values are typically 0-50 points which fits in i8 (-128 to 127)
            let frame = egui::Frame::NONE.inner_margin(egui::Margin {
                top: safe_area.top.round() as i8,
                right: safe_area.right.round() as i8,
                bottom: safe_area.bottom.round() as i8,
                left: safe_area.left.round() as i8,
            });

            egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
                if let Some(chrome) = &mut self.chrome {
                    let mut app_ctx = self.notedeck.app_context();
                    let _ = chrome.update(&mut app_ctx, ui);
                }
            });
        });

        // Extract output state
        let wants_keyboard = ctx.wants_keyboard_input();
        let ime_rect = full_output.platform_output.ime.as_ref().map(|ime| ime.rect);
        let copied_text = full_output.platform_output.copied_text.clone();

        // Tessellate shapes
        let paint_jobs = ctx.tessellate(full_output.shapes, ctx.pixels_per_point());

        // Get current frame
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Failed to get current texture: {:?}", e);
                return OutputState::new(CursorIcon::Default);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: ctx.pixels_per_point(),
        };

        let tdelta = full_output.textures_delta;

        // Update textures
        for (id, image_delta) in &tdelta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // Update buffers
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // Execute render pass
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Use forget_lifetime() to satisfy the 'static requirement in the damus egui fork
            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &paint_jobs,
                &screen_descriptor,
            );
        }

        // Submit commands
        self.queue.submit(Some(encoder.finish()));

        // Present frame
        frame.present();

        // Free old textures
        for id in &tdelta.free {
            self.egui_renderer.free_texture(id);
        }

        OutputState::with_full_state(
            full_output.platform_output.cursor_icon.into(),
            wants_keyboard,
            ime_rect,
            copied_text,
        )
    }
}
