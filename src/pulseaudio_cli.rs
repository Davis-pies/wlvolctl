use std::process::Command;
use regex::Regex;

use crate::audio::{AudioBackend, AudioError, BackendTag, Stream};

pub struct PulseAudioCli;

impl PulseAudioCli {
    pub fn available() -> bool {
        Command::new("which")
            .arg("pactl")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl AudioBackend for PulseAudioCli {
    fn list_streams(&self) -> Result<Vec<Stream>, AudioError> {
        let out = Command::new("pactl")
            .args(["list", "sink-inputs"])
            .output()
            .map_err(|e| AudioError::CommandFailed(e.to_string()))?;

        if !out.status.success() {
            return Err(AudioError::CommandFailed("pactl list failed".into()));
        }

        let text = String::from_utf8_lossy(&out.stdout);
        let re_id = Regex::new(r"^Sink Input #(\d+)").unwrap();
        let re_name = Regex::new(r#"application\.name\s*=\s*"([^"]+)""#).unwrap();
        let re_vol = Regex::new(r"(\d+)%").unwrap();
        let re_mute = Regex::new(r"Mute:\s*(yes|no)").unwrap();

        let mut streams = Vec::new();
        let mut cur_id: Option<u32> = None;
        let mut cur_name: Option<String> = None;
        let mut cur_vol: Option<f32> = None;
        let mut cur_mute: bool = false;

        for line in text.lines() {
            if let Some(c) = re_id.captures(line) {
                // flush previous
                if let (Some(id), Some(name), Some(vol)) = (cur_id, cur_name.clone(), cur_vol) {
                    streams.push(Stream {
                        id,
                        name,
                        icon_name: None,
                        volume_01: vol,
                        mute: cur_mute,
                        backend_tag: BackendTag::PulseAudio,
                    });
                }
                cur_id = c[1].parse().ok();
                cur_name = None;
                cur_vol = None;
                cur_mute = false;
                continue;
            }
            if let Some(c) = re_name.captures(line) {
                cur_name = Some(c[1].to_string());
                continue;
            }
            if let Some(c) = re_vol.captures(line) {
                let pct: f32 = c[1].parse().unwrap_or(0.0);
                cur_vol = Some((pct / 100.0).clamp(0.0, 1.0));
                continue;
            }
            if let Some(c) = re_mute.captures(line) {
                cur_mute = &c[1] == "yes";
                continue;
            }
        }
        if let (Some(id), Some(name), Some(vol)) = (cur_id, cur_name, cur_vol) {
            streams.push(Stream {
                id,
                name,
                icon_name: None,
                volume_01: vol,
                mute: cur_mute,
                backend_tag: BackendTag::PulseAudio,
            });
        }

        Ok(streams)
    }

    fn set_volume(&self, stream_id: u32, vol_01: f32) -> Result<(), AudioError> {
        let pct = (vol_01.clamp(0.0, 1.0) * 100.0).round() as i32;
        let status = Command::new("pactl")
            .args(["set-sink-input-volume", &stream_id.to_string(), &format!("{}%", pct)])
            .status()
            .map_err(|e| AudioError::CommandFailed(e.to_string()))?;
        if status.success() { Ok(()) } else { Err(AudioError::CommandFailed("pactl set-volume failed".into())) }
    }

    fn set_mute(&self, stream_id: u32, mute: bool) -> Result<(), AudioError> {
        let status = Command::new("pactl")
            .args(["set-sink-input-mute", &stream_id.to_string(), if mute { "1" } else { "0" }])
            .status()
            .map_err(|e| AudioError::CommandFailed(e.to_string()))?;
        if status.success() { Ok(()) } else { Err(AudioError::CommandFailed("pactl set-mute failed".into())) }
    }
}

