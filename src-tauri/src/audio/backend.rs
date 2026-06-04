use crate::audio::types::{AppStream, MicConfig, OutputDevice};
use crate::error::SinkError;

/// Abstraction over the underlying audio system.
///
/// `PipeWireBackend` (native, pipewire-rs) is the default; `PactlBackend`
/// (pactl subprocess calls) is the automatic fallback. Commands must only
/// ever talk to this trait — never to a concrete backend.
pub trait AudioBackend: Send + Sync {
    /// `label` is the human-readable device description shown by system
    /// mixers (channels are user-defined since the dynamic-channels work).
    fn create_virtual_sink(&self, name: &str, label: &str) -> Result<(), SinkError>;
    fn destroy_virtual_sink(&self, name: &str) -> Result<(), SinkError>;
    fn list_app_streams(&self) -> Result<Vec<AppStream>, SinkError>;
    fn list_output_devices(&self) -> Result<Vec<OutputDevice>, SinkError>;
    fn set_sink_volume(&self, sink_name: &str, volume_percent: u8) -> Result<(), SinkError>;
    fn set_sink_mute(&self, sink_name: &str, muted: bool) -> Result<(), SinkError>;
    /// Move an app stream to a sink. An empty `sink_name` means "unassign":
    /// the stream is returned to the system default sink.
    fn move_stream_to_sink(&self, stream_index: u32, sink_name: &str) -> Result<(), SinkError>;
    /// Set the volume of a single app stream (sink input).
    /// Not in the original trait sketch, but required by the `set_app_volume`
    /// command — commands are forbidden from calling pactl directly.
    fn set_app_volume(&self, stream_index: u32, volume_percent: u8) -> Result<(), SinkError>;

    /// Route a channel's audio to a physical output device (Phase 4).
    /// `None` means "follow the system default output" (which also gives
    /// automatic failover when the device disappears). The native backend
    /// creates passive in-graph links; the pactl fallback uses
    /// module-loopback.
    fn set_channel_output(
        &self,
        sink_name: &str,
        output_name: Option<&str>,
    ) -> Result<(), SinkError>;

    /// Hardware capture devices (microphones) for the Phase 3 mic chain.
    fn list_input_devices(&self) -> Result<Vec<OutputDevice>, SinkError>;

    /// Current system defaults: (output sink name, input source name).
    fn get_default_devices(&self) -> Result<(Option<String>, Option<String>), SinkError>;

    /// Set the system default output device. Channels following the
    /// default relink automatically.
    fn set_default_output(&self, name: &str) -> Result<(), SinkError>;

    /// Set the system default input device (what the mic chain captures
    /// when no explicit input is chosen).
    fn set_default_input(&self, name: &str) -> Result<(), SinkError>;

    /// Apply the Phase 3 mic chain configuration. Native-backend only; the
    /// pactl fallback reports it as unsupported.
    fn set_mic_config(&self, config: &MicConfig) -> Result<(), SinkError>;
}
