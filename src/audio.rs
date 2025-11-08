use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Stream {
    pub id: u32,
    pub name: String,
    pub icon_name: Option<String>,
    pub volume_01: f32,
    pub mute: bool,
    pub backend_tag: BackendTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendTag {
    PipeWire,
    PulseAudio,
}

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("backend not available")]
    NotAvailable,
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("parse error: {0}")]
    ParseError(String),
}

pub trait AudioBackend {
    fn list_streams(&self) -> Result<Vec<Stream>, AudioError>;
    fn set_volume(&self, stream_id: u32, vol_01: f32) -> Result<(), AudioError>;
    fn set_mute(&self, stream_id: u32, mute: bool) -> Result<(), AudioError>;
}

