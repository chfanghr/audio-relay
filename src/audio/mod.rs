use std::{
    collections::BTreeMap,
    sync::{Arc, PoisonError, RwLock},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use log::{debug, info};
use pipewire::{
    channel::{Receiver, channel},
    context::ContextRc,
    main_loop::MainLoopRc,
    registry::GlobalObject,
    spa::utils::dict::DictRef,
    types::ObjectType,
};

fn run_pipewire_loop(state: Arc<RwLock<AudioGraphState>>, quit_rx: Receiver<()>) -> Result<()> {
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

    let state_for_global_handler = state.clone();
    let state_for_global_remove_handler = state.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            debug!(target: "pipewire","new global object with id {} : {:?}", global.id, global);
            if let Some(port_desc) = PortDesc::from_global_object(global) {
                info!(target: "pipewire", "port registered: {:?}", port_desc);
                state_for_global_handler
                    .write()
                    .unwrap_or_else(PoisonError::into_inner)
                    .ports
                    .insert(port_desc.id, port_desc);
            } else {
                info!(target: "pipewire", "object with id {} is not a port", global.id);
            }
        })
        .global_remove(move |id| {
            info!(target: "pipewire", "global object with id {} removed", id);
            state_for_global_remove_handler
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .ports
                .remove(&id);
        })
        .register();

    let initial_sync_seq = core.sync(0)?;
    let state_for_done_handler = state.clone();
    let _core_listener = core
        .add_listener_local()
        .done(move |_id, seq| {
            if initial_sync_seq == seq {
                info!(target: "pipewire", "initial sync done");
                state_for_done_handler
                    .write()
                    .unwrap_or_else(PoisonError::into_inner)
                    .initial_sync_done = true;
            };
        })
        .register();

    main_loop.run();

    Ok(())
}

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
    id: u32,
    direction: PortDirection,
    channel: String,
    object_path: String,
    alias: String,
}

impl PortDesc {
    fn from_global_object(global: &GlobalObject<&DictRef>) -> Option<Self> {
        let id = global.id;
        (global.type_ == ObjectType::Port).then_some(())?;
        let props = global.props?;
        let direction = props
            .get("port.direction")
            .and_then(PortDirection::from_string)?;
        let channel = props.get("audio.channel")?.to_owned();
        let object_path = props.get("object.path")?.to_owned();
        let alias = props.get("port.alias")?.to_owned();
        Some(Self {
            id,
            direction,
            channel,
            object_path,
            alias,
        })
    }
}

#[derive(Debug, Clone)]
struct AudioGraphState {
    initial_sync_done: bool,
    ports: BTreeMap<u32, PortDesc>,
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
    let (quit_tx, quit_rx) = channel::<()>();
    let state = Arc::new(RwLock::new(AudioGraphState::new()));
    let state_for_pipewire_loop = state.clone();

    ctrlc::set_handler(move || {
        info!("ctrl+c received, quiting");
        _ = quit_tx.send(());
    })
    .context("error setting up signal handler")?;

    let pipewire_loop_thread_handle =
        thread::spawn(move || run_pipewire_loop(state_for_pipewire_loop, quit_rx).unwrap());

    let _monitoring_thread_handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(5));

            let state = state.read().unwrap_or_else(PoisonError::into_inner);
            debug!(target: "monitoring", "current state: {:?}", state)
        }
    });

    pipewire_loop_thread_handle
        .join()
        .map_err(|err| anyhow!("failure upon joining the pipewire loop thread: {:?}", err))?;

    Ok(())
}
