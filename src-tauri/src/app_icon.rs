//! Swaps the running app's icon between the dark and the light symbol: the
//! Dock icon on macOS, the window and taskbar icon on Windows and Linux.
//! The installed icon (Finder, Start menu, launcher) stays the dark one.

use crate::error::{CmdError, CmdResult};
use tauri::{image::Image, AppHandle, Manager, Runtime};

const DARK: &[u8] = include_bytes!("../icons/app/dark.png");
const LIGHT: &[u8] = include_bytes!("../icons/app/light.png");

fn png(variant: &str) -> CmdResult<&'static [u8]> {
    match variant {
        "dark" => Ok(DARK),
        "light" => Ok(LIGHT),
        other => Err(CmdError::invalid(format!("unknown app icon {other}"))),
    }
}

#[tauri::command]
pub fn set_app_icon<R: Runtime>(app: AppHandle<R>, variant: String) -> CmdResult<()> {
    let bytes = png(&variant)?;
    let image = Image::from_bytes(bytes).map_err(CmdError::internal)?;
    for window in app.webview_windows().values() {
        // Has no effect on macOS, where windows have no icon of their own.
        if let Err(e) = window.set_icon(image.clone()) {
            log::warn!("could not set the icon of {}: {e}", window.label());
        }
    }
    #[cfg(target_os = "macos")]
    app.run_on_main_thread(move || macos::set_dock_icon(bytes))
        .map_err(CmdError::internal)?;
    log::info!("app icon set to {variant}");
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    /// Same calls Tauri makes for the development icon; lasts until the app quits.
    pub fn set_dock_icon(png: &'static [u8]) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let data = NSData::with_bytes(png);
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
            log::warn!("could not decode the Dock icon");
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        // SAFETY: called on the main thread with a valid image.
        unsafe { app.setApplicationIconImage(Some(&image)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_icons_are_square_pngs_and_unknown_names_are_refused() {
        for v in ["dark", "light"] {
            let img = Image::from_bytes(png(v).unwrap()).unwrap();
            assert_eq!((img.width(), img.height()), (512, 512), "{v}");
        }
        assert_eq!(png("purple").unwrap_err().code, "invalidInput");
    }
}
