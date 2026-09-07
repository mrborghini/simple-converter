#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ffmpeg;
mod formats;
mod paths;
mod templates;
mod view;

use iced::{Size, window};

fn main() -> iced::Result {
    let icon = window::icon::from_file_data(include_bytes!("../assets/icon.png"), None).ok();

    iced::application(app::App::boot, app::App::update, app::App::view)
        .title("Simple Converter")
        .subscription(app::App::subscription)
        .window(window::Settings {
            size: Size::new(820.0, 720.0),
            min_size: Some(Size::new(620.0, 540.0)),
            icon,
            ..window::Settings::default()
        })
        .run()
}
