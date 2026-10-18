use std::{sync::mpsc, thread::JoinHandle};

use anyhow::Result;
use log::{debug, info};

use super::*;

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

pub fn run() -> Result<(
    mpsc::Receiver<PipeWireGraphUpdate>,
    JoinHandle<()>,
    impl Fn() -> (),
)> {
    let (quit_tx, quit_rx) = pipewire::channel::channel::<()>();
    let (update_sender, update_receiver) = mpsc::channel();

    let handle = thread::spawn(move || start(update_sender, quit_rx).unwrap());

    Ok((update_receiver, handle, move || {
        _ = quit_tx.send(());
    }))
}

fn start(
    update_sender: mpsc::Sender<PipeWireGraphUpdate>,
    quit_rx: pipewire::channel::Receiver<()>,
) -> Result<()> {
    let main_loop = pipewire::main_loop::MainLoopRc::new(None)?;

    let main_loop_weak = main_loop.downgrade();
    let _quit = quit_rx.attach(main_loop.loop_(), move |_| {
        if let Some(main_loop) = main_loop_weak.upgrade() {
            info!(target: "pipewire", "stopping the pipewire loop");
            main_loop.quit();
        }
    });

    let context = pipewire::context::ContextRc::new(&main_loop, None)?;
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
                    .send(PipeWireGraphUpdate::AddPort(port_desc))
                    .unwrap();
            } else {
                info!(target: "pipewire", "object with id {} is not a port", global.id);
            }
        })
        .global_remove(move |id| {
            info!(target: "pipewire", "remove global object with id {}", id);
            update_sender_for_global_remove_handler
                .send(PipeWireGraphUpdate::RemovePort(ObjectId(id)))
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
                    .send(PipeWireGraphUpdate::InitialSyncCompleted)
                    .unwrap();
            };
        })
        .register();

    main_loop.run();

    Ok(())
}
