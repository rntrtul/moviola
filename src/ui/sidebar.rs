use crate::video::metadata::{AudioCodec, ContainerFormat, VideoCodec};

mod crop;
mod output;
pub(crate) mod sidebar;

// fixme: too similar to videoContainerInfo
#[derive(Debug, Clone, Copy)]
pub struct OutputContainerSettings {
    pub(crate) no_audio: bool,
    pub(crate) audio_stream_idx: u32,
    pub(crate) audio_codec: AudioCodec,
    pub(crate) audio_bitrate: u32,
    pub(crate) container: ContainerFormat,
    pub(crate) video_codec: VideoCodec,
    pub(crate) video_bitrate: u32,
}

pub struct ControlsExportSettings {
    pub container: OutputContainerSettings,
    pub container_is_default: bool,
}