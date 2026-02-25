use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelInfo {
    pub name: String,
    pub plugin_name: Option<String>,
    pub channel_type: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlpMetadata {
    pub bpm: Option<f32>,
    pub time_sig_num: Option<u8>,
    pub time_sig_den: Option<u8>,
    pub channel_count: u16,
    pub pattern_count: u16,
    pub mixer_track_count: u16,
    pub generators: Vec<ChannelInfo>,
    pub effects: Vec<String>,
    pub fl_version: Option<String>,
    pub warnings: Vec<String>,
}
