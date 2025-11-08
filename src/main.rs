// src/main.rs

mod audio;
mod pulseaudio_cli;
mod ui;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("DEBUG: raw args = {:?}", args);

    if args.len() > 1 {
        println!("DEBUG: first argument = {}", args[1]);
        match args[1].as_str() {
            "--popup" => {
                println!("DEBUG: entering popup mode");
                ui::run_popup_ui();
            }
            _ => {
                eprintln!("Unknown option {}", args[1]);
                std::process::exit(1);
            }
        }
    } else {
        println!("DEBUG: entering full mode");
        ui::run_full_ui();
    }
}

