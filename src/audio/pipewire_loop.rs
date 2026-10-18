use std::{
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
};

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

fn forward_mpsc_to_pipewire_channel<T: 'static + Send>(
    rx: mpsc::Receiver<T>,
    tx: pipewire::channel::Sender<T>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        rx.iter().for_each(|x| {
            _ = tx.send(x);
        });
    })
}

pub fn run() -> Result<(
    mpsc::Receiver<PipeWireGraphEvent>,
    mpsc::Sender<PipewireLinkEvent>,
    impl FnOnce() -> (),
    impl Fn() -> (),
)> {
    let (quit_tx, quit_rx) = pipewire::channel::channel::<()>();
    let (link_tx, link_rx) = pipewire::channel::channel::<PipewireLinkEvent>();

    let (link_sender, link_receiver) = mpsc::channel();
    let (update_sender, update_receiver) = mpsc::channel();

    let handle = thread::spawn(move || start(update_sender, link_rx, quit_rx).unwrap());
    let fwd_handle = forward_mpsc_to_pipewire_channel(link_receiver, link_tx);

    let quit = move || {
        _ = quit_tx.send(());
    };
    let join = move || {
        _ = handle.join();
        _ = fwd_handle.join();
    };

    Ok((update_receiver, link_sender, join, quit))
}

fn start(
    update_sender: mpsc::Sender<PipeWireGraphEvent>,
    _link_receiver: pipewire::channel::Receiver<PipewireLinkEvent>,
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
                    .send(PipeWireGraphEvent::AddPort(port_desc))
                    .unwrap();
            } else {
                info!(target: "pipewire", "object with id {} is not a port", global.id);
            }
        })
        .global_remove(move |id| {
            info!(target: "pipewire", "remove global object with id {}", id);
            update_sender_for_global_remove_handler
                .send(PipeWireGraphEvent::RemovePort(ObjectId(id)))
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
                    .send(PipeWireGraphEvent::InitialSyncCompleted)
                    .unwrap();
            };
        })
        .register();

    main_loop.run();

    Ok(())
}
