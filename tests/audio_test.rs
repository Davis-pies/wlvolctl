use wlvolctl::audio::{AudioBackend, Stream, AudioError, BackendTag};
use log::{info, debug};
use env_logger;

struct DummyBackend;

impl AudioBackend for DummyBackend {
    fn list_streams(&self) -> Result<Vec<Stream>, AudioError> {
        info!("Listing streams from DummyBackend");
        Ok(vec![
            Stream {
                id: 1,
                name: "Firefox".to_string(),
                icon_name: Some("firefox".to_string()),
                volume_01: 0.5,
                mute: false,
                backend_tag: BackendTag::PipeWire,
            }
        ])
    }

    fn set_volume(&self, stream_id: u32, vol_01: f32) -> Result<(), AudioError> {
        info!("Setting volume for stream {} to {}", stream_id, vol_01);
        Ok(())
    }

    fn set_mute(&self, stream_id: u32, mute: bool) -> Result<(), AudioError> {
        info!("Setting mute for stream {} to {}", stream_id, mute);
        Ok(())
    }
}

#[test]
fn test_dummy_backend() {
    // Initialize logger for test output
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info) // ensure INFO logs are shown
        .try_init();

    let backend = DummyBackend;

    info!("Starting DummyBackend test");

    let streams = backend.list_streams().unwrap();
    info!("Streams returned: {:?}", streams);

    assert_eq!(streams[0].name, "Firefox");

    backend.set_volume(1, 0.75).unwrap();
    backend.set_mute(1, true).unwrap();

    info!("Finished DummyBackend test successfully");
}

