//! The PipeWire main-loop thread. All PipeWire objects live here (they are
//! not Send); the `PipeWireBackend` facade talks to this thread through a
//! pipewire channel, and each command carries an mpsc reply sender.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;

use pipewire as pw;
use pw::core::CoreRc;
use pw::metadata::{Metadata, MetadataListener};
use pw::node::{Node, NodeListener};
use pw::registry::{GlobalObject, RegistryRc};
use pw::spa::utils::dict::DictRef;
use pw::types::ObjectType;

use crate::audio::pw_native::levels::LevelStore;
use crate::audio::pw_native::meter::MeterHandle;
use crate::audio::pw_native::mic::{MicStreams, MIC_NODE};
use crate::audio::pw_native::pods;
use crate::audio::types::{is_virtual_sink, AppStream, MicConfig, OutputDevice};
use crate::error::SinkError;

const STREAM_CLASS: &str = "Stream/Output/Audio";
const SINK_CLASS: &str = "Audio/Sink";
const SOURCE_CLASS: &str = "Audio/Source";
const VIRTUAL_SOURCE_CLASS: &str = "Audio/Source/Virtual";
/// node.name prefix of all our internal helper streams (meters, mic chain) —
/// excluded from stream listings and node tracking.
pub const INTERNAL_PREFIX: &str = "sink-internal-";
/// node.name prefix of our meter capture streams.
pub const METER_PREFIX: &str = "sink-internal-meter-";
/// node.name of the Stream Mix virtual source (Phase 5): receives a copy of
/// every channel so OBS can capture the whole mix as one source.
pub const STREAM_MIX_NODE: &str = "sink_stream";

type Reply<T> = mpsc::Sender<Result<T, SinkError>>;

pub enum Cmd {
    CreateSink { name: String, label: String, reply: Reply<()> },
    DestroySink { name: String, reply: Reply<()> },
    ListStreams { reply: Reply<Vec<AppStream>> },
    ListOutputs { reply: Reply<Vec<OutputDevice>> },
    SetNodeVolumeByName { name: String, percent: u8, reply: Reply<()> },
    SetNodeMuteByName { name: String, muted: bool, reply: Reply<()> },
    SetNodeVolumeById { id: u32, percent: u8, reply: Reply<()> },
    MoveStream { id: u32, sink_name: String, reply: Reply<()> },
    /// Route a channel's monitor to an output device (None = follow default).
    SetChannelOutput { sink_name: String, output_name: Option<String>, reply: Reply<()> },
    /// Apply mic chain configuration (create/destroy/re-tune as needed).
    SetMicConfig { config: MicConfig, reply: Reply<()> },
    /// Hardware capture devices (microphones).
    ListInputs { reply: Reply<Vec<OutputDevice>> },
}

struct PortEntry {
    id: u32,
    node_id: u32,
    /// "in" (playback/sink input port) or "out" (source/monitor port).
    direction: String,
    /// e.g. "FL", "FR", "MONO".
    channel: Option<String>,
}

struct NodeEntry {
    id: u32,
    serial: Option<u64>,
    media_class: String,
    props: HashMap<String, String>,
    proxy: Node,
    _listener: NodeListener,
    volume_percent: u8,
    channels: usize,
    muted: bool,
    /// True while the node is in the Running state (actively streaming).
    active: bool,
}

#[derive(Default)]
struct State {
    nodes: HashMap<u32, NodeEntry>,
    /// link global id -> (output node id, input node id)
    links: HashMap<u32, (u32, u32)>,
    metadata: Option<Metadata>,
    _metadata_listener: Option<MetadataListener>,
    default_sink_name: Option<String>,
    /// Virtual sinks we created: name -> created-object proxy (kept alive;
    /// destroyed explicitly on teardown).
    owned_sinks: HashMap<String, Node>,
    /// Sinks that existed before us (e.g. leftover pactl modules): name -> global id.
    adopted_sinks: HashMap<String, u32>,
    /// Create requests waiting for the sink's global to appear.
    pending_creates: HashMap<String, Vec<Reply<()>>>,
    /// Live meter capture streams per virtual sink name.
    meters: HashMap<String, MeterHandle>,
    /// All known ports, for monitor→output linking.
    ports: HashMap<u32, PortEntry>,
    /// Channel sink name -> chosen output node.name (None = follow default).
    channel_outputs: HashMap<String, Option<String>>,
    /// Channel sink name -> live loopback links: (monitor port, input port, proxy).
    channel_links: HashMap<String, Vec<(u32, u32, pw::link::Link)>>,
    /// Phase 3 mic chain.
    mic_config: MicConfig,
    /// Proxy for the sink_mic virtual source (kept alive while enabled).
    mic_source: Option<Node>,
    mic_streams: Option<MicStreams>,
    levels: Option<Arc<LevelStore>>,
    /// Phase 5 Stream Mix virtual source proxy.
    stream_mix_source: Option<Node>,
    /// Channel sink name -> links into the Stream Mix source.
    stream_links: HashMap<String, Vec<(u32, u32, pw::link::Link)>>,
    /// Links from the mic playback stream into the virtual mic.
    mic_links: Vec<(u32, u32, pw::link::Link)>,
}

impl State {
    /// Live node id of the mic playback stream. Resolved lazily — the id
    /// is only valid once the server has created the stream's node.
    fn mic_playback_node(&self) -> Option<u32> {
        self.mic_streams
            .as_ref()
            .map(|m| m.playback_node_id())
            .filter(|id| *id != u32::MAX)
    }
}

impl State {
    fn node_by_name(&self, name: &str) -> Option<&NodeEntry> {
        self.nodes
            .values()
            .find(|n| n.props.get("node.name").map(String::as_str) == Some(name))
    }

    /// The sink a stream is currently connected to, resolved through links.
    fn sink_of_stream(&self, stream_id: u32) -> Option<&NodeEntry> {
        self.links
            .values()
            .find(|(out, _)| *out == stream_id)
            .and_then(|(_, input)| self.nodes.get(input))
    }
}

// The CoreRc is needed by the command handler (object creation/destruction);
// this thread owns all PipeWire objects, so a thread-local is the simplest
// way to share it across the listener closures.
thread_local! {
    static CORE: RefCell<Option<CoreRc>> = const { RefCell::new(None) };
}

/// Entry point: runs the PipeWire loop until the channel closes.
/// `init_tx` reports startup success/failure exactly once.
pub fn run(
    receiver: pw::channel::Receiver<Cmd>,
    init_tx: mpsc::Sender<Result<(), SinkError>>,
    levels: Arc<LevelStore>,
) {
    if let Err(e) = setup_and_run(receiver, &init_tx, levels) {
        let _ = init_tx.send(Err(e));
    }
}

fn setup_and_run(
    receiver: pw::channel::Receiver<Cmd>,
    init_tx: &mpsc::Sender<Result<(), SinkError>>,
    levels: Arc<LevelStore>,
) -> Result<(), SinkError> {
    pw::init();
    let err = |stage: &str, e: pw::Error| SinkError::Config(format!("pipewire {stage}: {e}"));

    let mainloop = pw::main_loop::MainLoopRc::new(None).map_err(|e| err("mainloop", e))?;
    let context = pw::context::ContextRc::new(&mainloop, None).map_err(|e| err("context", e))?;
    let core = context.connect_rc(None).map_err(|e| err("connect", e))?;
    let registry = core.get_registry_rc().map_err(|e| err("registry", e))?;

    CORE.with(|c| *c.borrow_mut() = Some(core.clone()));

    let state = Rc::new(RefCell::new(State {
        levels: Some(levels.clone()),
        ..State::default()
    }));

    // ---- registry listeners ----
    let state_g = state.clone();
    let registry_g = registry.clone();
    let core_g = core.clone();
    let levels_g = levels.clone();
    let _reg_listener = registry
        .add_listener_local()
        .global(move |global| {
            on_global(&state_g, &registry_g, &core_g, &levels_g, global);
        })
        .global_remove({
            let state = state.clone();
            move |id| {
                let removed_sink = {
                    let mut s = state.borrow_mut();
                    s.links.remove(&id);
                    s.ports.remove(&id);
                    match s.nodes.remove(&id) {
                        Some(node) if node.media_class == SINK_CLASS => {
                            if let Some(name) = node.props.get("node.name") {
                                let name = name.clone();
                                s.meters.remove(&name);
                                s.adopted_sinks.remove(&name);
                            }
                            true
                        }
                        _ => false,
                    }
                };
                // An output device vanished (or one of our sinks died):
                // relink so affected channels fail over to the default.
                if removed_sink {
                    ensure_all_links(&state);
                }
            }
        })
        .register();

    // ---- command channel ----
    let state_c = state.clone();
    let registry_c = registry.clone();
    let _recv = receiver.attach(mainloop.loop_(), move |cmd| {
        handle_cmd(&state_c, &registry_c, cmd);
    });

    init_tx
        .send(Ok(()))
        .map_err(|_| SinkError::Config("backend owner vanished during init".into()))?;

    mainloop.run();
    Ok(())
}

fn on_global(
    state: &Rc<RefCell<State>>,
    registry: &RegistryRc,
    core: &CoreRc,
    levels: &Arc<LevelStore>,
    global: &GlobalObject<&DictRef>,
) {
    match global.type_ {
        ObjectType::Node => on_node(state, registry, core, levels, global),
        ObjectType::Port => {
            let Some(props) = global.props else { return };
            let Some(node_id) = props.get("node.id").and_then(|v| v.parse().ok()) else {
                return;
            };
            let entry = PortEntry {
                id: global.id,
                node_id,
                direction: props.get("port.direction").unwrap_or_default().to_string(),
                channel: props.get("audio.channel").map(str::to_string),
            };
            let relevant_links = {
                let mut s = state.borrow_mut();
                s.ports.insert(global.id, entry);
                s.nodes
                    .get(&node_id)
                    .is_some_and(|n| n.media_class == SINK_CLASS)
            };
            if relevant_links {
                ensure_all_links(state);
            }
            // Mic wiring depends on ports of untracked stream nodes, so
            // reconcile on every port event (no-op until both ends exist).
            ensure_mic_links(state);
        }
        ObjectType::Link => {
            let Some(props) = global.props else { return };
            let out = props.get("link.output.node").and_then(|v| v.parse().ok());
            let inp = props.get("link.input.node").and_then(|v| v.parse().ok());
            if let (Some(out), Some(inp)) = (out, inp) {
                let police = {
                    let mut s = state.borrow_mut();
                    s.links.insert(global.id, (out, inp));
                    // Police the mic playback stream: if anything (e.g. a
                    // session-manager fallback) links it somewhere other
                    // than the virtual mic, destroy that link — mic audio
                    // must never leak into the speakers.
                    match (s.mic_playback_node(), s.node_by_name(MIC_NODE)) {
                        (Some(playback), mic) if out == playback => {
                            mic.map(|n| n.id) != Some(inp)
                        }
                        _ => false,
                    }
                };
                if police {
                    let _ = registry.destroy_global(global.id);
                }
            }
        }
        ObjectType::Metadata => {
            let Some(props) = global.props else { return };
            if props.get("metadata.name") != Some("default") {
                return;
            }
            let Ok(metadata) = registry.bind::<Metadata, _>(global) else {
                return;
            };
            let state_m = state.clone();
            let listener = metadata
                .add_listener_local()
                .property(move |_subject, key, _type, value| {
                    if key == Some("default.audio.sink") {
                        // value is JSON like {"name":"alsa_output...."}
                        let name = value.and_then(|v| {
                            serde_json::from_str::<serde_json::Value>(v)
                                .ok()?
                                .get("name")?
                                .as_str()
                                .map(str::to_string)
                        });
                        let changed = {
                            let mut s = state_m.borrow_mut();
                            let changed = s.default_sink_name != name;
                            s.default_sink_name = name;
                            changed
                        };
                        // Channels following the default must relink
                        // (Sonar-style automatic device failover).
                        if changed {
                            ensure_all_links(&state_m);
                        }
                    }
                    0
                })
                .register();
            let mut s = state.borrow_mut();
            s.metadata = Some(metadata);
            s._metadata_listener = Some(listener);
        }
        _ => {}
    }
}

fn on_node(
    state: &Rc<RefCell<State>>,
    registry: &RegistryRc,
    core: &CoreRc,
    levels: &Arc<LevelStore>,
    global: &GlobalObject<&DictRef>,
) {
    let Some(dict) = global.props else { return };
    let media_class = dict.get("media.class").unwrap_or_default().to_string();
    if media_class != STREAM_CLASS
        && media_class != SINK_CLASS
        && media_class != SOURCE_CLASS
        && media_class != VIRTUAL_SOURCE_CLASS
    {
        return;
    }
    let props: HashMap<String, String> = dict
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let node_name = props.get("node.name").cloned().unwrap_or_default();
    // Never track our own internal helper streams (meters, mic chain).
    if node_name.starts_with(INTERNAL_PREFIX) {
        return;
    }

    let Ok(proxy) = registry.bind::<Node, _>(global) else {
        return;
    };

    // Track volume/mute through Props param events, and the running state
    // through info events (drives the app-list activity indicator).
    let state_p = state.clone();
    let state_i = state.clone();
    let node_id = global.id;
    let listener = proxy
        .add_listener_local()
        .info(move |info| {
            let running = matches!(info.state(), pw::node::NodeState::Running);
            let mut s = state_i.borrow_mut();
            if let Some(entry) = s.nodes.get_mut(&node_id) {
                entry.active = running;
                // Registry globals only carry an abbreviated prop set; the
                // info event has the full dict (e.g. application.process.
                // binary, needed to name Discord's "WEBRTC VoiceEngine").
                if let Some(props) = info.props() {
                    for (k, v) in props.iter() {
                        entry.props.insert(k.to_string(), v.to_string());
                    }
                }
            }
        })
        .param(move |_seq, id, _index, _next, param| {
            if id != pw::spa::param::ParamType::Props {
                return;
            }
            let Some(pod) = param else { return };
            let parsed = pods::parse_props(pod);
            let mut s = state_p.borrow_mut();
            if let Some(entry) = s.nodes.get_mut(&node_id) {
                if let Some(linear) = parsed.volume_linear {
                    entry.volume_percent = pods::linear_to_percent(linear);
                }
                if let Some(channels) = parsed.channels {
                    entry.channels = channels;
                }
                if let Some(muted) = parsed.muted {
                    entry.muted = muted;
                }
            }
        })
        .register();
    proxy.subscribe_params(&[pw::spa::param::ParamType::Props]);

    let entry = NodeEntry {
        id: global.id,
        serial: props.get("object.serial").and_then(|v| v.parse().ok()),
        media_class: media_class.clone(),
        props,
        proxy,
        _listener: listener,
        volume_percent: 100,
        channels: 2,
        muted: false,
        active: false,
    };

    let mut s = state.borrow_mut();
    s.nodes.insert(global.id, entry);

    if media_class == SINK_CLASS && is_virtual_sink(&node_name) {
        // A virtual sink came up: resolve pending create requests, remember
        // it for teardown if we didn't create it, and attach a level meter.
        if let Some(waiters) = s.pending_creates.remove(&node_name) {
            for reply in waiters {
                let _ = reply.send(Ok(()));
            }
        }
        if !s.owned_sinks.contains_key(&node_name) {
            s.adopted_sinks.insert(node_name.clone(), global.id);
        }
        if !s.meters.contains_key(&node_name) {
            match MeterHandle::new(core, &node_name, global.id, levels.clone()) {
                Ok(meter) => {
                    s.meters.insert(node_name.clone(), meter);
                }
                Err(e) => eprintln!("sink: meter for {node_name} failed: {e}"),
            }
        }
        drop(s);
        ensure_all_links(state);
        return;
    }

    // The virtual mic source came up: attach the DSP streams.
    if media_class == VIRTUAL_SOURCE_CLASS && node_name == MIC_NODE {
        drop(s);
        build_mic_streams(state, global.id);
        return;
    }

    drop(s);
    // A new hardware sink may be the (returning) target of a channel.
    if media_class == SINK_CLASS {
        ensure_all_links(state);
    }
}

/// (Re)build the mic capture/DSP/playback streams. The virtual source is
/// addressed by name via target.object, so no id is needed.
fn build_mic_streams(state: &Rc<RefCell<State>>, _source_id: u32) {
    let Some(core) = CORE.with(|c| c.borrow().clone()) else {
        return;
    };
    let mut s = state.borrow_mut();
    if !s.mic_config.enabled {
        return;
    }
    let mic_target = s.mic_config.input_device.clone();
    let Some(levels) = s.levels.clone() else { return };
    match MicStreams::new(&core, &s.mic_config, mic_target.as_deref(), levels) {
        Ok(streams) => {
            s.mic_links.clear();
            s.mic_streams = Some(streams);
        }
        Err(e) => eprintln!("sink: mic chain failed: {e}"),
    }
    drop(s);
    ensure_mic_links(state);
}

/// Link the mic playback stream's output ports into the virtual mic.
/// Called whenever ports appear; idempotent.
fn ensure_mic_links(state: &Rc<RefCell<State>>) {
    let Some(core) = CORE.with(|c| c.borrow().clone()) else {
        return;
    };
    let mut s = state.borrow_mut();
    let (Some(playback_id), Some(mic_node)) = (
        s.mic_playback_node(),
        s.node_by_name(MIC_NODE).map(|n| n.id),
    ) else {
        return;
    };
    let pairs = desired_pairs(&s, playback_id, mic_node);
    let current: Vec<(u32, u32)> = s.mic_links.iter().map(|(o, i, _)| (*o, *i)).collect();
    if current == pairs || pairs.is_empty() {
        return;
    }
    s.mic_links.clear();
    s.mic_links = create_links(&core, "mic", playback_id, mic_node, &pairs);
}

/// Compute monitor→input port pairs from `channel_id`'s output ports to
/// `target_id`'s input ports. Pairs by audio.channel where possible, with
/// an index-wrap fallback for mono/odd channel maps.
fn desired_pairs(s: &State, channel_id: u32, target_id: u32) -> Vec<(u32, u32)> {
    if channel_id == target_id {
        return Vec::new();
    }
    let mut monitors: Vec<&PortEntry> = s
        .ports
        .values()
        .filter(|p| p.node_id == channel_id && p.direction == "out")
        .collect();
    let mut inputs: Vec<&PortEntry> = s
        .ports
        .values()
        .filter(|p| p.node_id == target_id && p.direction == "in")
        .collect();
    monitors.sort_by_key(|p| p.id);
    inputs.sort_by_key(|p| p.id);
    if monitors.is_empty() || inputs.is_empty() {
        return Vec::new();
    }
    monitors
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let by_channel = m.channel.as_ref().and_then(|ch| {
                inputs
                    .iter()
                    .find(|p| p.channel.as_ref() == Some(ch))
                    .copied()
            });
            let input = by_channel.or_else(|| inputs.get(i % inputs.len()).copied())?;
            Some((m.id, input.id))
        })
        .collect()
}

/// Create link objects for `pairs` between two nodes; returns the proxies.
fn create_links(
    core: &CoreRc,
    sink_name: &str,
    out_node: u32,
    in_node: u32,
    pairs: &[(u32, u32)],
) -> Vec<(u32, u32, pw::link::Link)> {
    let mut created = Vec::new();
    for (monitor_port, input_port) in pairs {
        match core.create_object::<pw::link::Link>(
            "link-factory",
            &pw::properties::properties! {
                "link.output.node" => out_node.to_string(),
                "link.output.port" => monitor_port.to_string(),
                "link.input.node" => in_node.to_string(),
                "link.input.port" => input_port.to_string(),
            },
        ) {
            Ok(link) => created.push((*monitor_port, *input_port, link)),
            Err(e) => eprintln!("sink: link {sink_name} failed: {e}"),
        }
    }
    created
}

/// Reconcile loopback links for every virtual channel:
/// - monitor → chosen output device (or the system default when unset /
///   the chosen device is gone — automatic failover)
/// - monitor → Stream Mix source (Phase 5, for OBS capture)
///
/// Idempotent — existing correct links are left untouched.
fn ensure_all_links(state: &Rc<RefCell<State>>) {
    let Some(core) = CORE.with(|c| c.borrow().clone()) else {
        return;
    };
    let mut s = state.borrow_mut();
    let stream_mix_id = s.node_by_name(STREAM_MIX_NODE).map(|n| n.id);

    // Live channel set: every virtual sink we created or adopted.
    let channel_names: Vec<String> = s
        .owned_sinks
        .keys()
        .chain(s.adopted_sinks.keys())
        .cloned()
        .collect();

    for sink_name in &channel_names {
        let sink_name = sink_name.as_str();
        let channel_id = match s.node_by_name(sink_name) {
            Some(n) => n.id,
            None => continue,
        };

        // ---- output device links ----
        let explicit = s.channel_outputs.get(sink_name).cloned().flatten();
        let target_id = match explicit {
            Some(name) if s.node_by_name(&name).is_some() => {
                s.node_by_name(&name).map(|n| n.id)
            }
            _ => s
                .default_sink_name
                .clone()
                .and_then(|name| s.node_by_name(&name))
                .map(|n| n.id),
        };
        let pairs = target_id
            .map(|t| desired_pairs(&s, channel_id, t))
            .unwrap_or_default();
        let current: Vec<(u32, u32)> = s
            .channel_links
            .get(sink_name)
            .map(|links| links.iter().map(|(o, i, _)| (*o, *i)).collect())
            .unwrap_or_default();
        if current != pairs {
            s.channel_links.remove(sink_name);
            if let Some(in_node) = pairs
                .first()
                .and_then(|(_, input)| s.ports.get(input).map(|p| p.node_id))
            {
                let created = create_links(&core, sink_name, channel_id, in_node, &pairs);
                if !created.is_empty() {
                    s.channel_links.insert(sink_name.to_string(), created);
                }
            }
        }

        // ---- stream mix links ----
        let mix_pairs = stream_mix_id
            .map(|t| desired_pairs(&s, channel_id, t))
            .unwrap_or_default();
        let mix_current: Vec<(u32, u32)> = s
            .stream_links
            .get(sink_name)
            .map(|links| links.iter().map(|(o, i, _)| (*o, *i)).collect())
            .unwrap_or_default();
        if mix_current != mix_pairs {
            s.stream_links.remove(sink_name);
            if let Some(mix_id) = stream_mix_id {
                let created = create_links(&core, sink_name, channel_id, mix_id, &mix_pairs);
                if !created.is_empty() {
                    s.stream_links.insert(sink_name.to_string(), created);
                }
            }
        }
    }
}

fn handle_cmd(state: &Rc<RefCell<State>>, registry: &RegistryRc, cmd: Cmd) {
    match cmd {
        Cmd::CreateSink { name, label, reply } => {
            let mut s = state.borrow_mut();
            if s.node_by_name(&name).is_some() {
                // Already exists (e.g. leftover from a previous run) — the
                // registry handler has adopted it.
                let _ = reply.send(Ok(()));
                return;
            }
            if !is_virtual_sink(&name) {
                let _ = reply.send(Err(SinkError::UnknownSink(name)));
                return;
            }
            let Some(core) = CORE.with(|c| c.borrow().clone()) else {
                let _ = reply.send(Err(SinkError::Config(
                    "sink creation requires a live core".into(),
                )));
                return;
            };
            match core.create_object::<Node>(
                "adapter",
                &pw::properties::properties! {
                    "factory.name" => "support.null-audio-sink",
                    "node.name" => name.as_str(),
                    "node.description" => label.as_str(),
                    "media.class" => SINK_CLASS,
                    "audio.position" => "[ FL FR ]",
                    "monitor.channel-volumes" => "true",
                },
            ) {
                // The created proxy must be kept alive until teardown. The
                // reply fires when the global appears in the registry.
                Ok(proxy) => {
                    s.owned_sinks.insert(name.clone(), proxy);
                    s.pending_creates.entry(name).or_default().push(reply);
                }
                Err(e) => {
                    let _ = reply.send(Err(SinkError::Config(format!("create sink: {e}"))));
                    return;
                }
            }

            // Phase 5: the Stream Mix source rides along with the channels.
            if s.stream_mix_source.is_none() && s.node_by_name(STREAM_MIX_NODE).is_none() {
                match core.create_object::<Node>(
                    "adapter",
                    &pw::properties::properties! {
                        "factory.name" => "support.null-audio-sink",
                        "node.name" => STREAM_MIX_NODE,
                        "node.description" => "Sink Stream Mix",
                        "media.class" => VIRTUAL_SOURCE_CLASS,
                        "audio.position" => "[ FL FR ]",
                    },
                ) {
                    Ok(proxy) => s.stream_mix_source = Some(proxy),
                    Err(e) => eprintln!("sink: stream mix source failed: {e}"),
                }
            }
        }
        Cmd::DestroySink { name, reply } => {
            let mut s = state.borrow_mut();
            s.meters.remove(&name);
            s.channel_links.remove(&name);
            s.stream_links.remove(&name);
            s.channel_outputs.remove(&name);
            if let Some(levels) = &s.levels {
                levels.release(&name);
            }
            if let Some(proxy) = s.owned_sinks.remove(&name) {
                match CORE.with(|c| c.borrow().clone()) {
                    Some(core) => {
                        let _ = core.destroy_object(proxy);
                        let _ = reply.send(Ok(()));
                    }
                    None => {
                        let _ = reply.send(Err(SinkError::Config("core is gone".into())));
                    }
                }
            } else if let Some(id) = s.adopted_sinks.remove(&name) {
                let _ = registry.destroy_global(id);
                let _ = reply.send(Ok(()));
            } else {
                // Nothing to destroy — idempotent success.
                let _ = reply.send(Ok(()));
            }
        }
        Cmd::ListStreams { reply } => {
            let s = state.borrow();
            let streams = s
                .nodes
                .values()
                .filter(|n| n.media_class == STREAM_CLASS)
                .map(|n| {
                    let (app_name, match_prop, match_value) =
                        crate::audio::types::resolve_identity(|key| n.props.get(key).cloned());
                    AppStream {
                        index: n.id,
                        app_name,
                        match_prop,
                        match_value,
                        alias: None,
                        icon_name: n.props.get("application.icon-name").cloned(),
                        assigned_sink: s
                            .sink_of_stream(n.id)
                            .and_then(|sink| sink.props.get("node.name"))
                            .filter(|name| is_virtual_sink(name))
                            .cloned(),
                        volume_percent: n.volume_percent,
                        muted: n.muted,
                        active: n.active,
                    }
                })
                .collect();
            let _ = reply.send(Ok(streams));
        }
        Cmd::ListOutputs { reply } => {
            let s = state.borrow();
            let outputs = s
                .nodes
                .values()
                .filter(|n| {
                    n.media_class == SINK_CLASS
                        && !n
                            .props
                            .get("node.name")
                            .is_some_and(|name| is_virtual_sink(name))
                })
                .map(|n| OutputDevice {
                    index: n.id,
                    name: n.props.get("node.name").cloned().unwrap_or_default(),
                    description: n
                        .props
                        .get("node.description")
                        .or_else(|| n.props.get("node.nick"))
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect();
            let _ = reply.send(Ok(outputs));
        }
        Cmd::SetNodeVolumeByName { name, percent, reply } => {
            let s = state.borrow();
            let _ = reply.send(set_props(s.node_by_name(&name), Some(percent), None));
        }
        Cmd::SetNodeMuteByName { name, muted, reply } => {
            let s = state.borrow();
            let _ = reply.send(set_props(s.node_by_name(&name), None, Some(muted)));
        }
        Cmd::SetNodeVolumeById { id, percent, reply } => {
            let s = state.borrow();
            let _ = reply.send(set_props(s.nodes.get(&id), Some(percent), None));
        }
        Cmd::SetMicConfig { config, reply } => {
            let (needs_create, needs_destroy, needs_rebuild, source_id) = {
                let mut s = state.borrow_mut();
                let prev = s.mic_config.clone();
                s.mic_config = config.clone();

                // Live-tunable params apply without a rebuild.
                if let Some(streams) = &s.mic_streams {
                    streams.params.apply(&config);
                }

                let source_id = s.node_by_name(MIC_NODE).map(|n| n.id);
                let needs_create = config.enabled && s.mic_source.is_none();
                let needs_destroy = !config.enabled && s.mic_source.is_some();
                let needs_rebuild = config.enabled
                    && s.mic_streams.is_some()
                    && prev.input_device != config.input_device;
                (needs_create, needs_destroy, needs_rebuild, source_id)
            };

            if needs_destroy {
                let mut s = state.borrow_mut();
                s.mic_streams = None;
                s.mic_links.clear();
                if let Some(proxy) = s.mic_source.take() {
                    if let Some(core) = CORE.with(|c| c.borrow().clone()) {
                        let _ = core.destroy_object(proxy);
                    }
                }
                let _ = reply.send(Ok(()));
                return;
            }

            if needs_create {
                let Some(core) = CORE.with(|c| c.borrow().clone()) else {
                    let _ = reply.send(Err(SinkError::Config("core is gone".into())));
                    return;
                };
                match core.create_object::<Node>(
                    "adapter",
                    &pw::properties::properties! {
                        "factory.name" => "support.null-audio-sink",
                        "node.name" => MIC_NODE,
                        "node.description" => "Sink Mic",
                        "media.class" => VIRTUAL_SOURCE_CLASS,
                        "audio.position" => "[ MONO ]",
                    },
                ) {
                    Ok(proxy) => {
                        state.borrow_mut().mic_source = Some(proxy);
                        // Streams attach when the global appears (on_node).
                    }
                    Err(e) => {
                        let _ =
                            reply.send(Err(SinkError::Config(format!("create mic source: {e}"))));
                        return;
                    }
                }
            } else if needs_rebuild {
                state.borrow_mut().mic_streams = None;
                if let Some(id) = source_id {
                    build_mic_streams(state, id);
                }
            } else if config.enabled && source_id.is_some() {
                // Source exists but streams may be missing (earlier failure
                // or config re-applied at startup) — attach if needed.
                let missing = state.borrow().mic_streams.is_none();
                if missing {
                    if let Some(id) = source_id {
                        build_mic_streams(state, id);
                    }
                }
            }
            let _ = reply.send(Ok(()));
        }
        Cmd::ListInputs { reply } => {
            let s = state.borrow();
            let inputs = s
                .nodes
                .values()
                .filter(|n| {
                    let name = n.props.get("node.name").map(String::as_str);
                    n.media_class == SOURCE_CLASS
                        || (n.media_class == VIRTUAL_SOURCE_CLASS
                            && name != Some(MIC_NODE)
                            && name != Some(STREAM_MIX_NODE))
                })
                .map(|n| OutputDevice {
                    index: n.id,
                    name: n.props.get("node.name").cloned().unwrap_or_default(),
                    description: n
                        .props
                        .get("node.description")
                        .or_else(|| n.props.get("node.nick"))
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect();
            let _ = reply.send(Ok(inputs));
        }
        Cmd::SetChannelOutput { sink_name, output_name, reply } => {
            if !is_virtual_sink(&sink_name) {
                let _ = reply.send(Err(SinkError::UnknownSink(sink_name)));
                return;
            }
            state
                .borrow_mut()
                .channel_outputs
                .insert(sink_name, output_name);
            ensure_all_links(state);
            let _ = reply.send(Ok(()));
        }
        Cmd::MoveStream { id, sink_name, reply } => {
            let s = state.borrow();
            let Some(metadata) = s.metadata.as_ref() else {
                let _ = reply.send(Err(SinkError::Config(
                    "no default metadata object (is WirePlumber running?)".into(),
                )));
                return;
            };
            // Empty sink name = back to the default device.
            let target = if sink_name.is_empty() {
                s.default_sink_name.clone()
            } else {
                Some(sink_name.clone())
            };
            let serial = target
                .as_deref()
                .and_then(|name| s.node_by_name(name))
                .and_then(|n| n.serial);
            match serial {
                Some(serial) => {
                    metadata.set_property(
                        id,
                        "target.object",
                        Some("Spa:Id"),
                        Some(&serial.to_string()),
                    );
                    // Clear any stale low-level target left by other tools.
                    metadata.set_property(id, "target.node", None, None);
                    let _ = reply.send(Ok(()));
                }
                None => {
                    let _ = reply.send(Err(SinkError::UnknownSink(
                        target.unwrap_or_else(|| "<default>".into()),
                    )));
                }
            }
        }
    }
}

fn set_props(
    entry: Option<&NodeEntry>,
    volume_percent: Option<u8>,
    mute: Option<bool>,
) -> Result<(), SinkError> {
    let Some(entry) = entry else {
        return Err(SinkError::UnknownSink("node not found".into()));
    };
    let volume = volume_percent.map(|p| (pods::percent_to_linear(p), entry.channels));
    let bytes = pods::props_pod_bytes(volume, mute)?;
    let pod = pw::spa::pod::Pod::from_bytes(&bytes)
        .ok_or_else(|| SinkError::Config("constructed an invalid pod".into()))?;
    entry
        .proxy
        .set_param(pw::spa::param::ParamType::Props, 0, pod);
    Ok(())
}
