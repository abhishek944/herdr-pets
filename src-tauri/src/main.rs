// Prevents an extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|argument| argument == "--snapshot") {
        println!("{}", herdr_pets_lib::snapshot_json());
        return;
    }
    herdr_pets_lib::run();
}
