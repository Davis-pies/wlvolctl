use wlvolctl::pulseaudio_cli::PulseAudioCli;
use wlvolctl::audio::AudioBackend;

#[test]
fn test_list_streams_pulseaudio() {
    if !PulseAudioCli::available() {
        eprintln!("pactl not available, skipping PulseAudio test");
        return;
    }

    let backend = PulseAudioCli;
    let streams = backend.list_streams().unwrap();
    println!("Streams: {:?}", streams);

    assert!(streams.len() >= 0);
}

#[test]
fn test_control_streams_pulseaudio() {
    if !PulseAudioCli::available() {
        eprintln!("pactl not available, skipping PulseAudio test");
        return;
    }

    let backend = PulseAudioCli;
    let streams = backend.list_streams().unwrap();

    if streams.is_empty() {
        eprintln!("No active streams to test");
        return;
    }

    let s = &streams[0];
    println!("Testing stream: {:?}", s);

    // Lower volume to 50%
    backend.set_volume(s.id, 0.5).unwrap();
    println!("Set volume of {} to 50%", s.name);

    // Mute
    backend.set_mute(s.id, true).unwrap();
    println!("Muted {}", s.name);

    // Unmute
    backend.set_mute(s.id, false).unwrap();
    println!("Unmuted {}", s.name);

    // Raise volume back to 100%
    backend.set_volume(s.id, 1.0).unwrap();
    println!("Restored volume of {} to 100%", s.name);
}

#[test]
fn test_volume_and_mute_state_updates() {
    if !PulseAudioCli::available() {
        eprintln!("pactl not available, skipping PulseAudio test");
        return;
    }

    let backend = PulseAudioCli;
    let mut streams = backend.list_streams().unwrap();
    if streams.is_empty() {
        eprintln!("No active streams to test");
        return;
    }

    let s = &streams[0];
    println!("Initial stream: {:?}", s);

    // Save original state
    let original_vol = s.volume_01;
    let original_mute = s.mute;

    // --- Test volume change ---
    backend.set_volume(s.id, 0.5).unwrap();
    let updated = backend.list_streams().unwrap()
        .into_iter().find(|st| st.id == s.id).unwrap();
    println!("After set_volume: {:?}", updated);
    assert!((updated.volume_01 - 0.5).abs() < 0.05, "Volume not updated");

    // --- Test mute toggle ---
    backend.set_mute(s.id, true).unwrap();
    let muted = backend.list_streams().unwrap()
        .into_iter().find(|st| st.id == s.id).unwrap();
    println!("After mute: {:?}", muted);
    assert!(muted.mute, "Mute state not updated");

    backend.set_mute(s.id, false).unwrap();
    let unmuted = backend.list_streams().unwrap()
        .into_iter().find(|st| st.id == s.id).unwrap();
    println!("After unmute: {:?}", unmuted);
    assert!(!unmuted.mute, "Unmute state not updated");

    // --- Restore original state ---
    backend.set_volume(s.id, original_vol).unwrap();
    backend.set_mute(s.id, original_mute).unwrap();
    println!("Restored original state for {}", s.name);
}

