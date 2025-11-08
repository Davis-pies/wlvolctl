use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shellexpand;

use ini::Ini;

use gtk4::glib::{ControlFlow, timeout_add_local};
use gtk4::gdk::Key;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Image, Label, Orientation, Scale, Separator, ToggleButton, Window,
    EventControllerFocus, EventControllerKey,
};
use gtk4::gdk_pixbuf::Pixbuf;

use crate::audio::{AudioBackend, Stream};
use crate::pulseaudio_cli::PulseAudioCli;

pub fn run_popup_ui() {
    // Strip --popup so GTK doesn't see unknown args
    let mut gtk_args: Vec<String> = std::env::args().collect();
    gtk_args.retain(|a| a != "--popup");

    let app = Application::new(Some("com.example.wlvolctl.popup"), Default::default());

    app.connect_activate(|app| {
        // Invisible transient parent to allow proper modality/focus
        let parent = ApplicationWindow::new(app);
        parent.hide();

        let popup = Window::builder()
            .transient_for(&parent)
            .decorated(false)
            .resizable(false)
            .build();

        let backend: Arc<Mutex<PulseAudioCli>> = Arc::new(Mutex::new(PulseAudioCli));
        let icon_cache = Arc::new(load_icon_cache());

        let hbox = GtkBox::new(Orientation::Horizontal, 12);
        popup.set_child(Some(&hbox));

        // Refresh closure
        let hbox_clone = hbox.clone();
        let backend_clone = Arc::clone(&backend);
        let icons_clone = Arc::clone(&icon_cache);

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
            ControlFlow::Continue
        };

        // Run once immediately, then every 4 seconds
        update_ui();
        timeout_add_local(Duration::from_secs(4), update_ui);

        // Auto-close on focus loss (GTK4 controllers, no Inhibit)
        // let focus = EventControllerFocus::new();
        // {
        //     let popup = popup.clone();
        //     focus.connect_leave(move |_| {
        //         popup.close();
        //     });
        // }
        // popup.add_controller(focus);

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

    app.run_with_args(&gtk_args);
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
            ControlFlow::Continue
        };

        update_ui();
        timeout_add_local(Duration::from_secs(4), update_ui);

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
                            if let (Some(name), Some(icon)) = (section.get("Name"), section.get("Icon")) {
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

