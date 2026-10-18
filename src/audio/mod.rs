use std::{collections::BTreeMap, sync::mpsc, thread};

use anyhow::{Context, Result, anyhow};
use log::{debug, info};
use pipewire::{
    context::ContextRc, main_loop::MainLoopRc, registry::GlobalObject, spa::utils::dict::DictRef,
    types::ObjectType,
};

enum Update {
    AddPort(PortDesc),
    RemovePort(ObjectId),
    InitialSyncCompleted,
}

fn run_pipewire_loop(
    update_sender: mpsc::Sender<Update>,
    quit_rx: pipewire::channel::Receiver<()>,
) -> Result<()> {
    let main_loop = MainLoopRc::new(None)?;

    let main_loop_weak = main_loop.downgrade();
    let _quit = quit_rx.attach(main_loop.loop_(), move |_| {
        if let Some(main_loop) = main_loop_weak.upgrade() {
            info!(target: "pipewire", "stopping the pipewire loop");
            main_loop.quit();
        }
    });

    let context = ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;

    let update_sender_for_global_handler = update_sender.clone();
    let update_sender_for_global_remove_handler = update_sender.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            debug!(target: "pipewire","new global object with id {} : {:?}", global.id, global);
            if let Some(port_desc) = PortDesc::from_global_object(global) {
                info!(target: "pipewire", "register port: {:?}", port_desc);
                update_sender_for_global_handler
                    .send(Update::AddPort(port_desc))
                    .unwrap();
            } else {
                info!(target: "pipewire", "object with id {} is not a port", global.id);
            }
        })
        .global_remove(move |id| {
            info!(target: "pipewire", "remove global object with id {}", id);
            update_sender_for_global_remove_handler
                .send(Update::RemovePort(ObjectId(id)))
                .unwrap();
        })
        .register();

    let initial_sync_seq = core.sync(0)?;
    let update_sender_for_done_handler = update_sender.clone();
    let _core_listener = core
        .add_listener_local()
        .done(move |_id, seq| {
            if initial_sync_seq == seq {
                info!(target: "pipewire", "initial sync done");
                update_sender_for_done_handler
                    .send(Update::InitialSyncCompleted)
                    .unwrap();
            };
        })
        .register();

    main_loop.run();

    Ok(())
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

impl PortDesc {
    fn from_global_object(global: &GlobalObject<&DictRef>) -> Option<Self> {
        let id = ObjectId(global.id);
        (global.type_ == ObjectType::Port).then_some(())?;
        let props = global.props?;
        let direction = props
            .get("port.direction")
            .and_then(PortDirection::from_string)?;
        let channel = props.get("audio.channel")?.to_owned();
        let object_path = props.get("object.path")?.to_owned();
        let alias = props.get("port.alias")?.to_owned();
        let node_id = ObjectId(props.get("node.id")?.parse().ok()?);
        Some(Self {
            id,
            direction,
            channel,
            object_path,
            alias,
            node_id,
        })
    }
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
    let (quit_tx, quit_rx) = pipewire::channel::channel::<()>();
    let (update_sender, update_receiver) = mpsc::channel();

    ctrlc::set_handler(move || {
        info!(target: "signal", "interrupted, quiting");
        _ = quit_tx.send(());
    })
    .context("error setting up signal handler")?;

    let pipewire_loop_thread_handle =
        thread::spawn(move || run_pipewire_loop(update_sender, quit_rx).unwrap());

    let _monitoring_thread_handle = thread::spawn(move || {
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

    pipewire_loop_thread_handle
        .join()
        .map_err(|err| anyhow!("failure upon joining the pipewire loop thread: {:?}", err))?;

    Ok(())
}
