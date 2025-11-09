// src/ui.rs
use std::collections::HashMap;
use std::fs;
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shellexpand;

use ini::Ini;

use gtk4::gdk::Key; 
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::*; // This pulls in most required traits like WidgetExt
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, EventControllerKey, Image,
    Label, Orientation, Scale, Separator, ToggleButton,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::audio::{AudioBackend, Stream};
use crate::pulseaudio_cli::PulseAudioCli;


pub fn run_popup_ui(
    module_x: Option<i32>,
    module_y: Option<i32>,
    module_w: Option<i32>,
    module_h: Option<i32>,
) {
    println!("[DEBUG_UI]: 1. Entered run_popup_ui");

    let mut gtk_args: Vec<String> = std::env::args().collect();
    // Retain logic to strip all custom args (x, y, w, h)
    gtk_args.retain(|a| {
        !a.starts_with("--popup")
            && !a.starts_with("--x=")
            && !a.starts_with("--y=")
            && !a.starts_with("--w=")
            && !a.starts_with("--h=")
    });

    let app = Application::new(Some("com.example.wlvolctl.popup.v2"), Default::default());
    println!("[DEBUG_UI]: 2. Created GTK Application");

    app.connect_activate(move |app| {
        println!(
            "[DEBUG_UI]: 4. [connect_activate]: Fired! module_x={:?}, module_y={:?}",
            module_x, module_y
        );

        let popup = ApplicationWindow::new(app);

        // Initialize layer shell
        popup.init_layer_shell();

        // Set as overlay layer (appears above normal windows)
        popup.set_layer(Layer::Overlay);

        // Remove window decorations
        popup.set_decorated(false);

        let backend: Arc<Mutex<PulseAudioCli>> = Arc::new(Mutex::new(PulseAudioCli));
        let icon_cache = Arc::new(load_icon_cache());

        let hbox = GtkBox::new(Orientation::Horizontal, 12);
        popup.set_child(Some(&hbox));

        // --- POSITIONING LOGIC ---

        // ================== THE ANCHOR FIX ==================
        // We MUST anchor to the top and left. This tells layer-shell
        // to use the margins as x/y coordinates from the top-left.
        popup.set_anchor(Edge::Left, true);
        popup.set_anchor(Edge::Top, true);
        popup.set_anchor(Edge::Right, false);
        popup.set_anchor(Edge::Bottom, false);
        // ================== END OF ANCHOR FIX ==================

        // This flag ensures we only position the window ONCE
        let is_positioned = Arc::new(Cell::new(false));

        // Position the window
        if let (Some(x), Some(y), Some(w), Some(h)) = (module_x, module_y, module_w, module_h) {
            println!(
                "[DEBUG_UI]: Position: Using provided coordinates (x={}, y={}, w={}, h={})",
                x, y, w, h
            );

            // ================== Map + Defer 50ms ==================
            popup.connect_map(move |popup_widget| {
                println!("[DEBUG_UI]: 'map' signal Fired!");

                // Only run this logic *once*
                if is_positioned.get() {
                    println!("[DEBUG_UI]: 'map': Already positioned, skipping.");
                    return;
                }

                // We need to move our values into the timeout closure
                let popup_clone = popup_widget.clone().downcast::<ApplicationWindow>().unwrap();
                let is_positioned_clone = is_positioned.clone();

                // DEFER the logic with a 50ms delay
                gtk4::glib::timeout_add_local_once(Duration::from_millis(50), move || {
                    let width = popup_clone.allocated_width();
                    let height = popup_clone.allocated_height();
                    println!("[DEBUG_UI]: 'timeout_add_local_once': Fired! w={}, h={}", width, height);

                    if !is_positioned_clone.get() && height > 1 {
                        println!("[DEBUG_UI]: 'timeout': Running position logic...");
                        if let Some(surface) = popup_clone.surface() {
                            let display = surface.display();
                            
                            // ================== MONITOR FIX ==================
                            // Manually find the monitor at point (x, y)
                            let monitors = display.monitors();
                            
                            // FIX: Use gtk4::gdk::Monitor
                            let mut target_monitor: Option<gtk4::gdk::Monitor> = None;
                            
                            for i in 0..monitors.n_items() {
                                if let Some(obj) = monitors.item(i) {
                                    // FIX: Use gtk4::gdk::Monitor
                                    if let Ok(monitor) = obj.downcast::<gtk4::gdk::Monitor>() {
                                        let geom = monitor.geometry();
                                        // Check if (x, y) is inside this monitor's geometry
                                        if x >= geom.x() && x < (geom.x() + geom.width()) &&
                                           y >= geom.y() && y < (geom.y() + geom.height()) {
                                            target_monitor = Some(monitor);
                                            break;
                                        }
                                    }
                                }
                            }

                            if let Some(monitor) = target_monitor {
                            // ================== END MONITOR FIX ==================
                                let monitor_geom = monitor.geometry();
                                let monitor_center_y =
                                    monitor_geom.y() + (monitor_geom.height() / 2);

                                // Inference: Determine if the bar is on top or bottom
                                let final_y = if y < monitor_center_y {
                                    // Bar is on TOP, position window BELOW module
                                    y + h
                                } else {
                                    // Bar is on BOTTOM, position window ABOVE module
                                    y - height
                                };

                                // Center the popup horizontally over the module
                                let mut final_x = x + (w / 2) - (width / 2);

                                // Clamp X to stay on-screen (relative to THIS monitor)
                                let monitor_x_start = monitor_geom.x();
                                let monitor_x_end = monitor_geom.x() + monitor_geom.width();
                                let max_x = monitor_x_end - width;

                                // Clamp ensures final_x is within [monitor_x_start, max_x]
                                final_x =
                                    final_x.clamp(monitor_x_start, max_x.max(monitor_x_start));

                                println!(
                                    "[DEBUG_UI]: 'timeout': Calculated final_x={}, final_y={}",
                                    final_x, final_y
                                );

                                // Because we anchored Left/Top, these margins are now
                                // (x, y) coordinates relative to the top-left corner.
                                popup_clone.set_margin(Edge::Left, final_x);
                                popup_clone.set_margin(Edge::Top, final_y);

                                is_positioned_clone.set(true); // Mark as positioned
                                println!("[DEBUG_UI]: 'timeout': Position set!");
                            } else {
                                println!("[DEBUG_UI]: 'timeout': FAILED to find monitor at (x,y) = ({}, {})", x, y);
                            }
                        } else {
                            println!("[DEBUG_UI]: 'timeout': FAILED to get surface");
                        }
                    } else {
                        if !is_positioned_clone.get() {
                            println!("[DEBUG_UI]: 'timeout': height was 0 or 1, not positioning.");
                        }
                    }
                });
            });

        } else {
            println!("[DEBUG_UI]: Position: Using fallback (top-right). Coords were None.");

            // Fallback if coordinates are missing: top-right corner
            // We must set the *correct* anchors for this to work
            popup.set_anchor(Edge::Right, true);
            popup.set_anchor(Edge::Top, true);
            popup.set_anchor(Edge::Left, false); // Make sure this is false
            popup.set_anchor(Edge::Bottom, false); // Make sure this is false

            popup.set_margin(Edge::Right, 10);
            popup.set_margin(Edge::Top, 40);
        }

        // Don't steal keyboard focus
        popup.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        // Refresh closure
        let hbox_clone = hbox.clone();
        let backend_clone = Arc::clone(&backend);
        let icons_clone = Arc::new(load_icon_cache());

        let update_ui = move || {
            let streams: Vec<Stream> = {
                let b = backend_clone.lock().unwrap();
                b.list_streams().unwrap_or_default()
            };

            // Clear existing children (GTK4: iterate via first_child/next_sibling)
            while let Some(child) = hbox_clone.first_child() {
                hbox_clone.remove(&child);
            }

            if streams.is_empty() {
                let empty = Label::new(Some("No active streams"));
                hbox_clone.append(&empty);
            } else {
                for s in &streams {
                    let col = build_column(&backend_clone, &icons_clone, s.clone());
                    hbox_clone.append(&col);
                }
            }

            hbox_clone.show();
            gtk4::glib::ControlFlow::Continue
        };

        // Run once immediately, then every 4 seconds
        update_ui();
        gtk4::glib::timeout_add_local(Duration::from_secs(4), update_ui);

        // Close on Esc key
        let key = EventControllerKey::new();
        {
            let popup = popup.clone();
            key.connect_key_pressed(move |_, keyval, _, _| {
                if keyval == Key::Escape {
                    popup.close();
                    true.into()
                } else {
                    false.into()
                }
            });
        }
        popup.add_controller(key);

        popup.present();
    });

    println!("[DEBUG_UI]: 3. Calling app.run_with_args()...");
    app.run_with_args(&gtk_args);
    println!("[DEBUG_UI]: 5. app.run_with_args() has exited.");
}

pub fn run_full_ui() {
    let app = Application::new(Some("org.wlvolctl.ui"), Default::default());
    app.connect_activate(|app| {
        let backend = Arc::new(Mutex::new(PulseAudioCli));
        let icon_cache = Arc::new(load_icon_cache());

        let window = ApplicationWindow::new(app);
        window.set_title(Some("wlvolctl POC"));
        window.set_default_size(600, 300);

        let vbox = GtkBox::new(Orientation::Vertical, 6);
        window.set_child(Some(&vbox));

        let header = Label::new(Some("Per-application volumes"));
        vbox.append(&header);
        vbox.append(&Separator::new(Orientation::Horizontal));

        // Horizontal container for stream columns
        let streams_box = GtkBox::new(Orientation::Horizontal, 12);
        vbox.append(&streams_box);

        // Refresh loop
        let streams_box_clone = streams_box.clone();
        let backend_clone = Arc::clone(&backend);
        let icons_clone = Arc::clone(&icon_cache);

        let update_ui = move || {
            let streams = backend_clone
                .lock()
                .unwrap()
                .list_streams()
                .unwrap_or_default();

            // Clear existing children
            while let Some(child) = streams_box_clone.first_child() {
                streams_box_clone.remove(&child);
            }

            if streams.is_empty() {
                let empty = Label::new(Some("No active streams"));
                streams_box_clone.append(&empty);
            } else {
                for s in &streams {
                    let col = build_column(&backend_clone, &icons_clone, s.clone());
                    streams_box_clone.append(&col);
                }
            }

            streams_box_clone.show();
            gtk4::glib::ControlFlow::Continue // <-- FIX 3
        };

        update_ui();
        gtk4::glib::timeout_add_local(Duration::from_secs(4), update_ui); // <-- FIX 4

        window.show();
    });

    app.run();
}

fn load_icon_cache() -> HashMap<String, String> {
    let mut map = HashMap::new();

    // Desktop files
    let desktop_dirs = vec!["/usr/share/applications", "~/.local/share/applications"];
    for dir in desktop_dirs {
        let expanded = shellexpand::tilde(dir).to_string();
        if let Ok(entries) = std::fs::read_dir(&expanded) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(conf) = Ini::load_from_file(entry.path()) {
                        if let Some(section) = conf.section(Some("Desktop Entry")) {
                            if let (Some(name), Some(icon)) =
                                (section.get("Name"), section.get("Icon"))
                            {
                                map.insert(name.to_lowercase(), icon.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Flatpak scalable SVGs
    let flatpak_dirs = vec![
        "~/.local/share/flatpak/exports/share/icons/hicolor/scalable/apps",
        "/var/lib/flatpak/exports/share/icons/hicolor/scalable/apps",
    ];
    for dir in flatpak_dirs {
        let expanded = shellexpand::tilde(dir).to_string();
        if let Ok(entries) = std::fs::read_dir(&expanded) {
            for entry in entries.flatten() {
                if let Some(fname) = entry.file_name().to_str() {
                    if fname.ends_with(".svg") {
                        let app_id = fname.trim_end_matches(".svg");
                        let path = entry.path().to_string_lossy().to_string();
                        map.insert(app_id.to_lowercase(), path.clone());
                        if let Some(stripped) = app_id.split('.').last() {
                            map.insert(stripped.to_lowercase(), path.clone());
                        }
                    }
                }
            }
        }
    }

    println!("Icon cache loaded: {} entries", map.len());
    map
}

fn build_column(
    backend: &Arc<Mutex<PulseAudioCli>>,
    icons: &Arc<HashMap<String, String>>,
    s: Stream,
) -> GtkBox {
    let v = GtkBox::new(Orientation::Vertical, 6);
    let app_name = s.name.to_lowercase();

    // Icon widget
    let icon_widget = if let Some(path_or_name) = icons.get(&app_name) {
        if std::path::Path::new(path_or_name.as_str()).exists() {
            // File path → load and scale
            match Pixbuf::from_file_at_size(path_or_name, 48, 48) {
                Ok(pixbuf) => {
                    let img = Image::from_pixbuf(Some(&pixbuf));
                    img.set_pixel_size(48);
                    img.set_size_request(48, 48);
                    img
                }
                Err(_) => {
                    let img = Image::from_icon_name("applications-multimedia");
                    img.set_pixel_size(48);
                    img.set_size_request(48, 48);
                    img
                }
            }
        } else {
            // Theme icon name
            let img = Image::from_icon_name(path_or_name);
            img.set_pixel_size(48);
            img.set_size_request(48, 48);
            img
        }
    } else {
        let img = Image::from_icon_name("applications-multimedia");
        img.set_pixel_size(48);
        img.set_size_request(48, 48);
        img
    };

    // Label, slider, mute toggle
    let label = Label::new(Some(&s.name));
    label.set_xalign(0.5);

    let scale = Scale::with_range(Orientation::Vertical, 0.0, 1.0, 0.01);
    scale.set_inverted(true);
    scale.set_draw_value(false);
    scale.set_size_request(60, 160);
    scale.set_value(s.volume_01 as f64);

    let mute = ToggleButton::with_label("Mute");
    mute.set_active(s.mute);

    // Slider binding
    let id = s.id;
    let backend1: Arc<Mutex<PulseAudioCli>> = Arc::clone(backend);
    scale.connect_value_changed(move |sc| {
        let val = sc.value() as f32;
        if let Ok(b) = backend1.lock() {
            let _ = b.set_volume(id, val);
            println!("Set volume for {} to {}", id, val);
        }
    });

    // Mute binding
    let id2 = s.id;
    let backend2: Arc<Mutex<PulseAudioCli>> = Arc::clone(backend);
    mute.connect_toggled(move |btn| {
        let active = btn.is_active();
        if let Ok(b) = backend2.lock() {
            let _ = b.set_mute(id2, active);
            println!("Mute for {} set to {}", id2, active);
        }
    });

    // Append children (GTK4)
    v.append(&icon_widget);
    v.append(&label);
    v.append(&scale);
    v.append(&mute);

    v
}

// Optional: theme fallback mapping helper
fn icon_for_app(name: &str) -> &str {
    match name.to_lowercase().as_str() {
        "firefox" => "firefox",
        "vlc" => "vlc",
        "spotify" => "spotify",
        "stremio" => "video-player", // fallback if no specific icon
        _ => "applications-multimedia",
    }
}

fn load_desktop_icons() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let dirs = vec![
        "/usr/share/applications",
        "~/.local/share/applications",
        "~/.local/share/flatpak/exports/share/applications",
    ];

    for dir in dirs {
        let expanded = shellexpand::tilde(dir).to_string();
        if let Ok(entries) = fs::read_dir(&expanded) {
            println!("Scanning directory: {}", expanded);
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("desktop") {
                    let path = entry.path();
                    if let Ok(conf) = Ini::load_from_file(&path) {
                        if let Some(section) = conf.section(Some("Desktop Entry")) {
                            if let (Some(name), Some(icon)) =
                                (section.get("Name"), section.get("Icon"))
                            {
                                println!("Loaded desktop entry: {} -> {}", name, icon);
                                map.insert(name.to_lowercase(), icon.to_string());
                            }
                        }
                    } else {
                        println!("Failed to parse: {:?}", path);
                    }
                }
            }
        } else {
            println!("Directory not found: {}", expanded);
        }
    }

    println!("Total icons loaded: {}", map.len());
    map
}

fn load_flatpak_icons() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let dirs = vec![
        "~/.local/share/flatpak/exports/share/icons/hicolor/128x128/apps",
        "/var/lib/flatpak/exports/share/icons/hicolor/128x128/apps",
    ];

    for dir in dirs {
        let expanded = shellexpand::tilde(dir).to_string();
        if let Ok(entries) = std::fs::read_dir(expanded) {
            for entry in entries.flatten() {
                if let Some(fname) = entry.file_name().to_str() {
                    if fname.ends_with(".png") {
                        let app_id = fname.trim_end_matches(".png");
                        // store both full ID and stripped name
                        map.insert(
                            app_id.to_lowercase(),
                            entry.path().to_string_lossy().to_string(),
                        );
                        if let Some(stripped) = app_id.split('.').last() {
                            map.insert(
                                stripped.to_lowercase(),
                                entry.path().to_string_lossy().to_string(),
                            );
                        }
                    }
                }
            }
        }
    }
    map
}
