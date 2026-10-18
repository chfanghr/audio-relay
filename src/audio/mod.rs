mod pipewire_loop;

use std::{collections::BTreeMap, thread};

use anyhow::{Context, Result, anyhow};
use log::{debug, info};
use pipewire::{registry::GlobalObject, spa::utils::dict::DictRef, types::ObjectType};

// from pipwire loop
enum PipeWireGraphEvent {
    AddPort(PortDesc),
    RemovePort(ObjectId),
    InitialSyncCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
struct ObjectId(u32);

#[derive(Debug, Clone)]
enum PortDirection {
    In,
    Out,
}

impl PortDirection {
    fn from_string(s: &str) -> Option<Self> {
        match s {
            "in" => Some(Self::In),
            "out" => Some(Self::Out),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct PortDesc {
    id: ObjectId,
    direction: PortDirection,
    channel: String,
    object_path: String,
    alias: String,
    node_id: ObjectId,
}

#[derive(Debug, Clone)]
struct LinkDesc {
    id: ObjectId,
    input_node_id: ObjectId,
    input_port_id: ObjectId,
    output_node_id: ObjectId,
    output_port_id: ObjectId,
}

#[derive(Debug, Clone)]
struct PipewireGraphState {
    initial_sync_done: bool,
    ports: BTreeMap<ObjectId, PortDesc>,
}

impl PipewireGraphState {
    fn new() -> Self {
        Self {
            initial_sync_done: false,
            ports: BTreeMap::new(),
        }
    }
}

// to pipwire loop
enum PipewireLinkEvent {
    CreateLink {
        input_node_id: ObjectId,
        input_port_id: ObjectId,
        output_node_id: ObjectId,
        output_port_id: ObjectId,
    },
    DestroyLink {
        link_id: ObjectId,
    },
}

pub fn demo_main() -> Result<()> {
    let (update_receiver, _link_sender, join_loop, stop_loop_thread) = pipewire_loop::run()?;

    ctrlc::set_handler(move || {
        info!(target: "signal", "interrupted, quitting");
        stop_loop_thread();
    })
    .context("error setting up signal handler")?;

    _ = thread::spawn(move || {
        _ = update_receiver
            .iter()
            .fold(PipewireGraphState::new(), |mut acc, update| {
                match update {
                    PipeWireGraphEvent::AddPort(port_desc) => {
                        acc.ports.insert(port_desc.id, port_desc);
                    }
                    PipeWireGraphEvent::RemovePort(object_id) => {
                        acc.ports.remove(&object_id);
                    }
                    PipeWireGraphEvent::InitialSyncCompleted => {
                        acc.initial_sync_done = true;
                    }
                };
                debug!(target: "monitoring", "current state: {:?}", acc);
                acc
            });
    });

    join_loop();

    Ok(())
}
