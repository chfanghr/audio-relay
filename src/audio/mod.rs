mod pipewire_loop;

use std::{collections::BTreeMap, thread};

use anyhow::{Context, Result, anyhow};
use log::{debug, info};
use pipewire::{registry::GlobalObject, spa::utils::dict::DictRef, types::ObjectType};

// from pipwire loop
enum GraphEvent {
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
struct GraphState {
    initial_sync_done: bool,
    ports: BTreeMap<ObjectId, PortDesc>,
}

impl GraphState {
    fn new() -> Self {
        Self {
            initial_sync_done: false,
            ports: BTreeMap::new(),
        }
    }
}

// to pipwire loop
#[derive(Debug, Clone)]
struct CreateLink {
    input_node_id: ObjectId,
    input_port_id: ObjectId,
    output_node_id: ObjectId,
    output_port_id: ObjectId,
}

#[derive(Debug, Clone)]
struct DestroyLink {
    link_id: ObjectId,
}

#[derive(Debug, Clone)]
enum LinkEvent {
    CreateLink(CreateLink),
    DestroyLink(DestroyLink),
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
            .fold(GraphState::new(), |mut acc, update| {
                match update {
                    GraphEvent::AddPort(port_desc) => {
                        acc.ports.insert(port_desc.id, port_desc);
                    }
                    GraphEvent::RemovePort(object_id) => {
                        acc.ports.remove(&object_id);
                    }
                    GraphEvent::InitialSyncCompleted => {
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
