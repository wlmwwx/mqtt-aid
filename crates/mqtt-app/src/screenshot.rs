//! Optional screenshot support, used to verify the rendered window.
//!
//! Enable the `screenshot` feature and set `MQTT_ASSISTANT_SCREENSHOT` to a
//! file name: the application renders a few frames, writes a PNG of the window
//! and closes.
//!
//! `MQTT_ASSISTANT_OPEN_MENU` opens one of the menu bar menus before the
//! picture is taken, so that the dropdowns can be checked as well.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use egui::Context;

/// The screenshot helper of the application.
///
/// The work is split in two: `tick` counts the frames and says when the
/// application should close, and `request` asks for the picture. The two are
/// separate because egui only paints a window that has a surface: a picture
/// asked for outside a painted frame is refused.
pub struct Screenshot {
    /// How many frames have been drawn.
    frame: u32,
    /// Whether the picture has been asked for.
    requested: bool,
    /// Whether the picture has been written.
    written: Arc<AtomicBool>,
    /// Where the picture is written.
    path: Option<String>,
}

impl Screenshot {
    /// Read the environment and prepare the helper.
    pub fn new() -> Screenshot {
        Screenshot {
            frame: 0,
            requested: false,
            written: Arc::new(AtomicBool::new(false)),
            path: std::env::var("MQTT_ASSISTANT_SCREENSHOT").ok(),
        }
    }

    /// Count a frame.
    ///
    /// This is called from `App::logic`, which egui runs whether the window is
    /// shown or hidden.
    ///
    /// Returns `true` when the application should close.
    pub fn tick(&mut self) -> bool {
        if self.path.is_none() {
            return false;
        }
        self.frame += 1;

        // Give the renderer a couple of frames to deliver the pixels.
        self.frame > 90
    }

    /// Ask for the picture, if it is time.
    ///
    /// This is called from `App::ui`, which only runs when the window is
    /// actually painted. A picture asked for outside a painted frame is
    /// refused, because there are no pixels to take.
    pub fn request(&mut self, ctx: &Context) {
        let Some(path) = self.path.clone() else {
            return;
        };

        if self.frame < 20 || self.requested {
            return;
        }
        self.requested = true;

        tracing::info!("requesting a screenshot");
        let written = self.written.clone();
        ctx.request_screenshot(move |image| {
            if written.swap(true, Ordering::Relaxed) {
                return;
            }
            if let Err(error) = save(&image, &path) {
                tracing::error!("could not write the screenshot: {error}");
            } else {
                tracing::info!("screenshot written to {path}");
            }
        });
    }
}

impl Default for Screenshot {
    fn default() -> Self {
        Self::new()
    }
}

/// The menu that should be open in the picture, if any.
pub fn open_menu() -> Option<String> {
    std::env::var("MQTT_ASSISTANT_OPEN_MENU").ok()
}

/// Write an egui colour image as a PNG file.
fn save(image: &egui::ColorImage, path: &str) -> Result<(), String> {
    let size = image.size;
    let raw = image.as_raw();

    let mut buffer = Vec::with_capacity(raw.len());
    for pixel in raw.chunks_exact(4) {
        let alpha = pixel[3];
        let (r, g, b) = if alpha == 0 {
            (0, 0, 0)
        } else {
            (
                (pixel[0] as u32 * 255 / alpha as u32) as u8,
                (pixel[1] as u32 * 255 / alpha as u32) as u8,
                (pixel[2] as u32 * 255 / alpha as u32) as u8,
            )
        };
        buffer.extend_from_slice(&[r, g, b, pixel[3]]);
    }

    let picture =
        image::RgbaImage::from_raw(size[0] as u32, size[1] as u32, buffer).ok_or("bad size")?;
    picture.save(path).map_err(|error| error.to_string())
}
