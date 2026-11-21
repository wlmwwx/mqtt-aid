//! Entry point of the MQTT Assistant desktop application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mqtt_app::MqttApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("MQTT Assistant")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MQTT Assistant",
        options,
        Box::new(|_cc| Ok(Box::new(MqttApp::new()))),
    )
}
