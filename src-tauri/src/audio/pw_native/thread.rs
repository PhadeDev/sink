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
use crate::audio::pw_native::pods;
use crate::audio::types::{is_virtual_sink, label_for, AppStream, OutputDevice};
use crate::error::SinkError;

const STREAM_CLASS: &str = "Stream/Output/Audio";
const SINK_CLASS: &str = "Audio/Sink";
/// node.name prefix of our own meter capture streams (excluded everywhere).
pub const METER_PREFIX: &str = "sink-meter-";

type Reply<T> = mpsc::Sender<Result<T, SinkError>>;

pub enum Cmd {
    CreateSink { name: String, reply: Reply<()> },
    DestroySink { name: String, reply: Reply<()> },
    ListStreams { reply: Reply<Vec<AppStream>> },
    ListOutputs { reply: Reply<Vec<OutputDevice>> },
    SetNodeVolumeByName { name: String, percent: u8, reply: Reply<()> },
    SetNodeMuteByName { name: String, muted: bool, reply: Reply<()> },
    SetNodeVolumeById { id: u32, percent: u8, reply: Reply<()> },
    MoveStream { id: u32, sink_name: String, reply: Reply<()> },
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

    let state = Rc::new(RefCell::new(State::default()));

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
                let mut s = state.borrow_mut();
                s.links.remove(&id);
                if let Some(node) = s.nodes.remove(&id) {
                    if let Some(name) = node.props.get("node.name") {
                        let name = name.clone();
                        s.meters.remove(&name);
                        s.adopted_sinks.remove(&name);
                    }
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
        ObjectType::Link => {
            let Some(props) = global.props else { return };
            let out = props.get("link.output.node").and_then(|v| v.parse().ok());
            let inp = props.get("link.input.node").and_then(|v| v.parse().ok());
            if let (Some(out), Some(inp)) = (out, inp) {
                state.borrow_mut().links.insert(global.id, (out, inp));
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
                        state_m.borrow_mut().default_sink_name = name;
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
    if media_class != STREAM_CLASS && media_class != SINK_CLASS {
        return;
    }
    let props: HashMap<String, String> = dict
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let node_name = props.get("node.name").cloned().unwrap_or_default();
    // Never track our own meter capture streams.
    if node_name.starts_with(METER_PREFIX) {
        return;
    }

    let Ok(proxy) = registry.bind::<Node, _>(global) else {
        return;
    };

    // Track volume/mute through Props param events.
    let state_p = state.clone();
    let node_id = global.id;
    let listener = proxy
        .add_listener_local()
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
    }
}

fn handle_cmd(state: &Rc<RefCell<State>>, registry: &RegistryRc, cmd: Cmd) {
    match cmd {
        Cmd::CreateSink { name, reply } => {
            let mut s = state.borrow_mut();
            if s.node_by_name(&name).is_some() {
                // Already exists (e.g. leftover from a previous run) — the
                // registry handler has adopted it.
                let _ = reply.send(Ok(()));
                return;
            }
            let Some(label) = label_for(&name) else {
                let _ = reply.send(Err(SinkError::UnknownSink(name)));
                return;
            };
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
                    "node.description" => label,
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
                }
            }
        }
        Cmd::DestroySink { name, reply } => {
            let mut s = state.borrow_mut();
            s.meters.remove(&name);
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
                    let (app_name, match_prop) = [
                        "application.name",
                        "application.process.binary",
                        "media.name",
                        "node.name",
                    ]
                    .iter()
                    .find_map(|key| n.props.get(*key).map(|v| (v.clone(), (*key).to_string())))
                    .unwrap_or_else(|| ("Unknown".to_string(), "application.name".to_string()));
                    AppStream {
                        index: n.id,
                        app_name,
                        match_prop,
                        alias: None,
                        icon_name: n.props.get("application.icon-name").cloned(),
                        assigned_sink: s
                            .sink_of_stream(n.id)
                            .and_then(|sink| sink.props.get("node.name"))
                            .filter(|name| is_virtual_sink(name))
                            .cloned(),
                        volume_percent: n.volume_percent,
                        muted: n.muted,
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
