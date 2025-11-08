use std::process::Command;
use regex::Regex;

use crate::audio::{AudioBackend, AudioError, BackendTag, Stream};

pub struct PipeWireCli;

impl PipeWireCli {
    pub fn available() -> bool {
        Command::new("which")
            .arg("wpctl")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl AudioBackend for PipeWireCli {
    fn list_streams(&self) -> Result<Vec<Stream>, AudioError> {
        let out = Command::new("wpctl")
            .arg("status")
            .output()
            .map_err(|e| AudioError::CommandFailed(e.to_string()))?;

        if !out.status.success() {
            return Err(AudioError::CommandFailed("wpctl status failed".into()));
        }

        let text = String::from_utf8_lossy(&out.stdout);
        let mut streams = Vec::new();
        let mut in_section = false;

        let re = Regex::new(r#"^\s*(\d+)\.\s+(.+?)\s+\(sink:\s*\d+\)\s+

\[vol:\s*([0-9.]+)\]

"#)
            .map_err(|e| AudioError::ParseError(e.to_string()))?;

        for line in text.lines() {
            let trimmed = line.trim_end();
            if trimmed.starts_with("Sink Inputs:") {
                in_section = true;
                continue;
            }
            if in_section && trimmed.is_empty() {
                in_section = false;
                continue;
            }
            if in_section {
                if let Some(caps) = re.captures(trimmed) {
                    let id: u32 = caps[1].parse().unwrap_or(0);
                    let name = caps[2].to_string();
                    let vol: f32 = caps[3].parse().unwrap_or(0.0);
                    streams.push(Stream {
                        id,
                        name,
                        icon_name: None,
                        volume_01: vol,
                        mute: false,
                        backend_tag: BackendTag::PipeWire,
                    });
                }
            }
        }

        Ok(streams)
    }

    fn set_volume(&self, stream_id: u32, vol_01: f32) -> Result<(), AudioError> {
        let v = vol_01.clamp(0.0, 1.0);
        let status = Command::new("wpctl")
            .args(["set-volume", &stream_id.to_string(), &format!("{:.3}", v)])
            .status()
            .map_err(|e| AudioError::CommandFailed(e.to_string()))?;

        if status.success() {
            Ok(())
        } else {
            Err(AudioError::CommandFailed("wpctl set-volume failed".into()))
        }
    }

    fn set_mute(&self, stream_id: u32, mute: bool) -> Result<(), AudioError> {
        let status = Command::new("wpctl")
            .args(["set-mute", &stream_id.to_string(), if mute { "1" } else { "0" }])
            .status()
            .map_err(|e| AudioError::CommandFailed(e.to_string()))?;

        if status.success() {
            Ok(())
        } else {
            Err(AudioError::CommandFailed("wpctl set-mute failed".into()))
        }
    }
}

