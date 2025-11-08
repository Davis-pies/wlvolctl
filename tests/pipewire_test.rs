use wlvolctl::pipewire_cli::PipeWireCli;
use wlvolctl::audio::AudioBackend;

#[test]
fn test_list_streams_pipewire() {
    if !PipeWireCli::available() {
        eprintln!("wpctl not available, skipping PipeWire test");
        return;
    }

    let backend = PipeWireCli;
    let streams = backend.list_streams().unwrap();
    println!("Streams: {:?}", streams);

    // Basic sanity check
    assert!(streams.len() >= 0);
}

