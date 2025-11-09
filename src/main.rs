// src/main.rs
mod audio;
mod pipewire_cli;
mod pulseaudio_cli;
mod ui;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut popup_mode = false;
    let mut x_coord: Option<i32> = None;
    let mut y_coord: Option<i32> = None;
    let mut w_coord: Option<i32> = None;
    let mut h_coord: Option<i32> = None;

    for arg in &args[1..] {
        if arg == "--popup" {
            popup_mode = true;
        } else if let Some(val) = arg.strip_prefix("--x=") {
            x_coord = val.parse().ok();
        } else if let Some(val) = arg.strip_prefix("--y=") {
            y_coord = val.parse().ok();
        } else if let Some(val) = arg.strip_prefix("--w=") {
            w_coord = val.parse().ok();
        } else if let Some(val) = arg.strip_prefix("--h=") {
            h_coord = val.parse().ok();
        }
    }

    if popup_mode {
        println!(
            "DEBUG: entering popup mode at (x,y,w,h): ({:?}, {:?}, {:?}, {:?})",
            x_coord, y_coord, w_coord, h_coord
        );
        ui::run_popup_ui(x_coord, y_coord, w_coord, h_coord);
    } else {
        println!("DEBUG: entering full mode");
        ui::run_full_ui();
    }
}
