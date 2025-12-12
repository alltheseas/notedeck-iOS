// setup module only available on non-iOS (requires eframe)
#[cfg(not(target_os = "ios"))]
pub mod setup;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
pub mod ios;

mod app;
mod chrome;
mod options;

pub use app::NotedeckApp;
pub use chrome::Chrome;
pub use options::ChromeOptions;
