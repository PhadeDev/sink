//! Phase 3 mic engine: captures the selected (or default) microphone,
//! runs the native DSP chain (gate → gain → compressor → limiter), and
//! plays the processed signal into a `Audio/Source/Virtual` node — a
//! virtual microphone that Discord/OBS can capture.
//!
//! Topology:  hw mic ──capture stream──▶ DSP ──ring──▶ playback stream ──▶ sink_mic (virtual source)

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use pipewire as pw;
use pw::spa;
use spa::pod::Pod;

use crate::audio::pw_native::dsp::{DspChain, DspSettings};
use crate::audio::pw_native::levels::LevelStore;
use crate::audio::pw_native::ring::Ring;
use crate::audio::types::MicConfig;
use crate::error::SinkError;

/// node.name of the virtual microphone.
pub const MIC_NODE: &str = "sink_mic";
/// Internal stream names (excluded from app/stream listings).
pub const MIC_CAPTURE_NAME: &str = "sink-internal-mic-capture";
pub const MIC_PLAYBACK_NAME: &str = "sink-internal-mic-playback";

/// Live-tunable DSP parameters, shared with the RT capture callback.
pub struct MicParams {
    gain_bits: AtomicU32,
    gate: AtomicBool,
    comp: AtomicBool,
    limiter: AtomicBool,
    muted: AtomicBool,
}

impl MicParams {
    pub fn from_config(config: &MicConfig) -> Self {
        let p = Self {
            gain_bits: AtomicU32::new(1.0f32.to_bits()),
            gate: AtomicBool::new(true),
            comp: AtomicBool::new(true),
            limiter: AtomicBool::new(true),
            muted: AtomicBool::new(false),
        };
        p.apply(config);
        p
    }

    pub fn apply(&self, config: &MicConfig) {
        let gain = f32::from(config.gain_percent) / 100.0;
        self.gain_bits.store(gain.to_bits(), Ordering::Relaxed);
        self.gate.store(config.gate_enabled, Ordering::Relaxed);
        self.comp.store(config.comp_enabled, Ordering::Relaxed);
        self.limiter.store(config.limiter_enabled, Ordering::Relaxed);
        self.muted.store(config.muted, Ordering::Relaxed);
    }

    fn settings(&self) -> DspSettings {
        DspSettings {
            gate_enabled: self.gate.load(Ordering::Relaxed),
            comp_enabled: self.comp.load(Ordering::Relaxed),
            limiter_enabled: self.limiter.load(Ordering::Relaxed),
            gain: f32::from_bits(self.gain_bits.load(Ordering::Relaxed)),
            muted: self.muted.load(Ordering::Relaxed),
        }
    }
}

struct CaptureCtx {
    chain: DspChain,
    params: Arc<MicParams>,
    ring: Arc<Ring>,
    levels: Arc<LevelStore>,
    level_slot: usize,
    scratch: Vec<f32>,
}

struct PlaybackCtx {
    ring: Arc<Ring>,
}

pub struct MicStreams {
    _capture: pw::stream::StreamRc,
    _capture_listener: pw::stream::StreamListener<CaptureCtx>,
    playback: pw::stream::StreamRc,
    _playback_listener: pw::stream::StreamListener<PlaybackCtx>,
    pub params: Arc<MicParams>,
}

impl MicStreams {
    /// Node id of the playback stream — the loop links its output ports to
    /// the virtual mic itself (WirePlumber 0.5 does not reliably honor
    /// target.object for playback→virtual-source routing).
    pub fn playback_node_id(&self) -> u32 {
        self.playback.node_id()
    }
}

/// Mono F32 format pod for stream negotiation.
fn mono_f32_format() -> Result<Vec<u8>, SinkError> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_channels(1);
    let object = spa::pod::Object {
        type_: spa::sys::SPA_TYPE_OBJECT_Format,
        id: spa::sys::SPA_PARAM_EnumFormat,
        properties: info.into(),
    };
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(object),
    )
    .map(|(c, _)| c.into_inner())
    .map_err(|e| SinkError::Config(format!("mic format pod: {e:?}")))
}

impl MicStreams {
    /// Build both streams. `mic_target` is the node.name of the hardware
    /// mic to capture (None = system default source). Targets are set via
    /// the `target.object` property — the connect-id parameter is
    /// deprecated and WirePlumber 0.5 ignores it.
    pub fn new(
        core: &pw::core::CoreRc,
        config: &MicConfig,
        mic_target: Option<&str>,
        levels: Arc<LevelStore>,
    ) -> Result<Self, SinkError> {
        let err = |stage: &str, e: pw::Error| SinkError::Config(format!("mic {stage}: {e}"));
        let params = Arc::new(MicParams::from_config(config));
        let level_slot = levels
            .slot_for(MIC_NODE)
            .ok_or_else(|| SinkError::Config("meter budget exhausted for mic".into()))?;
        // ~85 ms of headroom at 48 kHz; actual added latency is one quantum.
        let ring = Arc::new(Ring::new(4096));

        // ---- capture: hardware mic -> DSP -> ring ----
        let mut capture_props = pw::properties::properties! {
            "media.type" => "Audio",
            "media.category" => "Capture",
            "node.name" => MIC_CAPTURE_NAME,
            "node.passive" => "true",
        };
        if let Some(target) = mic_target {
            capture_props.insert("target.object", target);
        }
        let capture = pw::stream::StreamRc::new(core.clone(), MIC_CAPTURE_NAME, capture_props)
            .map_err(|e| err("capture stream", e))?;

        let capture_listener = capture
            .add_local_listener_with_user_data(CaptureCtx {
                chain: DspChain::new(48000.0),
                params: params.clone(),
                ring: ring.clone(),
                levels,
                level_slot,
                scratch: Vec::with_capacity(4096),
            })
            .param_changed(|_, ctx, id, param| {
                // Track the negotiated rate so DSP time constants are right.
                if id != spa::param::ParamType::Format.as_raw() {
                    return;
                }
                let Some(param) = param else { return };
                let mut info = spa::param::audio::AudioInfoRaw::new();
                if info.parse(param).is_ok() && info.rate() > 0 {
                    ctx.chain = DspChain::new(info.rate() as f32);
                }
            })
            .process(|stream, ctx| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let datas = buffer.datas_mut();
                let Some(data) = datas.first_mut() else { return };
                let valid = data.chunk().size() as usize;
                let Some(bytes) = data.data() else { return };

                let n = (valid.min(bytes.len())) / 4;
                ctx.scratch.clear();
                ctx.scratch.extend(
                    bytes[..n * 4]
                        .chunks_exact(4)
                        .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]])),
                );

                let settings = ctx.params.settings();
                ctx.chain.process(&mut ctx.scratch, &settings);

                // Post-DSP level for the UI (mono → both meter channels).
                let peak = ctx.scratch.iter().fold(0.0f32, |a, s| a.max(s.abs()));
                ctx.levels.raise(ctx.level_slot, 0, peak);
                ctx.levels.raise(ctx.level_slot, 1, peak);

                ctx.ring.push(&ctx.scratch);
            })
            .register()
            .map_err(|e| err("capture listener", e))?;

        let format = mono_f32_format()?;
        let mut capture_params = [Pod::from_bytes(&format)
            .ok_or_else(|| SinkError::Config("mic capture format pod invalid".into()))?];
        capture
            .connect(
                spa::utils::Direction::Input,
                None,
                pw::stream::StreamFlags::AUTOCONNECT
                    | pw::stream::StreamFlags::MAP_BUFFERS
                    | pw::stream::StreamFlags::RT_PROCESS,
                &mut capture_params,
            )
            .map_err(|e| err("capture connect", e))?;

        // ---- playback: ring -> virtual source ----
        // node.autoconnect=false keeps WirePlumber's hands off this stream
        // (it routes playback streams to the default *sink*, i.e. the
        // speakers — observed live); the loop links it to sink_mic itself.
        let playback = pw::stream::StreamRc::new(
            core.clone(),
            MIC_PLAYBACK_NAME,
            pw::properties::properties! {
                "media.type" => "Audio",
                "media.category" => "Playback",
                "node.name" => MIC_PLAYBACK_NAME,
                "node.passive" => "true",
                "node.autoconnect" => "false",
                "node.dont-reconnect" => "true",
            },
        )
        .map_err(|e| err("playback stream", e))?;

        let playback_listener = playback
            .add_local_listener_with_user_data(PlaybackCtx { ring })
            .process(|stream, ctx| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                // Fill only what the graph asked for this cycle — filling
                // the whole mmap'd buffer (8k+ frames vs ~1k produced per
                // quantum) starves the ring and chops the audio.
                let requested = buffer.requested() as usize;
                let datas = buffer.datas_mut();
                let Some(data) = datas.first_mut() else { return };
                let max_bytes = data.data().map(|d| d.len()).unwrap_or(0);
                let max_frames = max_bytes / 4;
                let n = if requested > 0 {
                    requested.min(max_frames)
                } else {
                    max_frames.min(1024)
                };
                if n == 0 {
                    return;
                }
                {
                    let bytes = data.data().expect("checked above");
                    // Pop straight into the buffer as f32 ne bytes.
                    let mut frame = [0.0f32; 1024];
                    let mut written = 0;
                    while written < n {
                        let take = (n - written).min(frame.len());
                        ctx.ring.pop(&mut frame[..take]);
                        for (i, s) in frame[..take].iter().enumerate() {
                            let off = (written + i) * 4;
                            bytes[off..off + 4].copy_from_slice(&s.to_ne_bytes());
                        }
                        written += take;
                    }
                }
                let chunk = data.chunk_mut();
                *chunk.offset_mut() = 0;
                *chunk.stride_mut() = 4;
                *chunk.size_mut() = (n * 4) as u32;
            })
            .register()
            .map_err(|e| err("playback listener", e))?;

        let mut playback_params = [Pod::from_bytes(&format)
            .ok_or_else(|| SinkError::Config("mic playback format pod invalid".into()))?];
        playback
            .connect(
                spa::utils::Direction::Output,
                None,
                // No AUTOCONNECT: the loop creates the links to sink_mic.
                pw::stream::StreamFlags::MAP_BUFFERS | pw::stream::StreamFlags::RT_PROCESS,
                &mut playback_params,
            )
            .map_err(|e| err("playback connect", e))?;

        Ok(Self {
            _capture: capture,
            _capture_listener: capture_listener,
            playback,
            _playback_listener: playback_listener,
            params,
        })
    }
}
