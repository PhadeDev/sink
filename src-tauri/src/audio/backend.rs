use crate::audio::types::{AppStream, OutputDevice};
use crate::error::SinkError;

/// Abstraction over the underlying audio system.
///
/// Phase 1 implements this with `PactlBackend` (subprocess calls to pactl).
/// Phase 2 will swap in a native `PipeWireBackend` without touching the UI
/// or the Tauri command layer — commands must only ever talk to this trait.
pub trait AudioBackend: Send + Sync {
    fn create_virtual_sink(&self, name: &str) -> Result<(), SinkError>;
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
}
