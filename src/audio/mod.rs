mod pipewire_loop;

use std::{collections::BTreeMap, thread};

use anyhow::{Context, Result, anyhow};
use log::{debug, info};
use pipewire::{registry::GlobalObject, spa::utils::dict::DictRef, types::ObjectType};

pub enum Update {
    AddPort(PortDesc),
    RemovePort(ObjectId),
    InitialSyncCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub struct ObjectId(u32);

#[derive(Debug, Clone)]
pub enum PortDirection {
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
pub struct PortDesc {
    id: ObjectId,
    direction: PortDirection,
    channel: String,
    object_path: String,
    alias: String,
    node_id: ObjectId,
}

#[derive(Debug, Clone)]
struct AudioGraphState {
    initial_sync_done: bool,
    ports: BTreeMap<ObjectId, PortDesc>,
}

impl AudioGraphState {
    fn new() -> Self {
        Self {
            initial_sync_done: false,
            ports: BTreeMap::new(),
        }
    }
}

pub fn demo_main() -> Result<()> {
    let (update_receiver, loop_thread_handle, stop_loop_thread) = pipewire_loop::run()?;

    ctrlc::set_handler(move || {
        info!(target: "signal", "interrupted, quitting");
        stop_loop_thread();
    })
    .context("error setting up signal handler")?;

    _ = thread::spawn(move || {
        _ = update_receiver
            .iter()
            .fold(AudioGraphState::new(), |mut acc, update| {
                match update {
                    Update::AddPort(port_desc) => {
                        acc.ports.insert(port_desc.id, port_desc);
                    }
                    Update::RemovePort(object_id) => {
                        acc.ports.remove(&object_id);
                    }
                    Update::InitialSyncCompleted => {
                        acc.initial_sync_done = true;
                    }
                };
                debug!(target: "monitoring", "current state: {:?}", acc);
                acc
            });
    });

    loop_thread_handle
        .join()
        .map_err(|err| anyhow!("failure upon joining the pipewire loop thread: {:?}", err))?;

    Ok(())
}
