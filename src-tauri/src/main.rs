#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::io::Result<()> {
    bitchord_lib::run();
    Ok(())
}