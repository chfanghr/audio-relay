use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{self},
    thread::JoinHandle,
};

use anyhow::Result;
use log::{debug, info, warn};

use super::*;

impl PortDesc {
    fn from_global_object(
        global: &pipewire::registry::GlobalObject<&pipewire::spa::utils::dict::DictRef>,
    ) -> Option<Self> {
        let id = ObjectId(global.id);
        (global.type_ == pipewire::types::ObjectType::Port).then_some(())?;
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

impl CreateLink {
    fn to_properties(&self) -> pipewire::properties::PropertiesBox {
        pipewire::properties::properties! {
          "link.input.port" => self.input_port_id.0.to_string(),
          "link.output.port" => self.output_port_id.0.to_string(),
          "link.input.node" => self.input_node_id.0.to_string(),
          "link.output.node" => self.output_node_id.0.to_string(),
        }
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
    mpsc::Receiver<GraphEvent>,
    mpsc::Sender<LinkEvent>,
    impl FnOnce() -> (),
    impl Fn() -> (),
)> {
    let (quit_tx, quit_rx) = pipewire::channel::channel::<()>();
    let (link_tx, link_rx) = pipewire::channel::channel::<LinkEvent>();

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

struct LoopState {
    objects:
        BTreeMap<ObjectId, pipewire::registry::GlobalObject<pipewire::properties::PropertiesBox>>,
}

impl LoopState {
    fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
        }
    }
}

fn start(
    update_sender: mpsc::Sender<GraphEvent>,
    link_rx: pipewire::channel::Receiver<LinkEvent>,
    quit_rx: pipewire::channel::Receiver<()>,
) -> Result<()> {
    let loop_state = Rc::new(RefCell::new(LoopState::new()));

    let main_loop = pipewire::main_loop::MainLoopRc::new(None)?;

    let main_loop_for_quit_handler_weak = main_loop.downgrade();
    let _quit = quit_rx.attach(main_loop.loop_(), move |_| {
        if let Some(main_loop) = main_loop_for_quit_handler_weak.upgrade() {
            info!(target: "pipewire", "stopping the pipewire loop");
            main_loop.quit();
        }
    });

    let context = pipewire::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;

    let registry_for_link_handler_weak = registry.downgrade();
    let core_for_link_handler_weak = core.downgrade();
    let loop_state_for_link_handler = loop_state.clone();
    let _link = link_rx.attach(main_loop.loop_(), move |e| {
        if let Some(core) = core_for_link_handler_weak.upgrade()
            && let Some(registry) = registry_for_link_handler_weak.upgrade()
        {
            let res: Result<()> = (|| match e {
                LinkEvent::CreateLink(create_link) => {
                    let _link = core.create_object::<pipewire::link::Link>(
                        "link-factory",
                        &create_link.to_properties(),
                    )?;
                    Ok(())
                }
                LinkEvent::DestroyLink(destroy_link) => {
                    let objects = loop_state_for_link_handler.borrow();
                    let obj = objects.objects.get(&destroy_link.link_id).ok_or(anyhow!(
                        "cannot find link with object id {:?}",
                        destroy_link.link_id
                    ))?;
                    let link: pipewire::link::Link = registry.bind(obj)?;
                    core.destroy_object(link)
                        .context(format!("destroy link with id {:?}", destroy_link.link_id))?;
                    Ok(())
                }
            })();
            if let Err(err) = res {
                warn!(target: "pipewire", "couldn't handle link event: {}", err)
            }
        }
    });

    let update_sender_for_global_handler = update_sender.clone();
    let update_sender_for_global_remove_handler = update_sender.clone();
    let loop_state_for_global_handler = loop_state.clone();
    let loop_state_for_global_remove_handler = loop_state.clone();
    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            debug!(target: "pipewire","new global object with id {} : {:?}", global.id, global);
            loop_state_for_global_handler
                .borrow_mut()
                .objects
                .insert(ObjectId(global.id), global.to_owned());
            if let Some(port_desc) = PortDesc::from_global_object(global) {
                info!(target: "pipewire", "register port: {:?}", port_desc);
                update_sender_for_global_handler
                    .send(GraphEvent::AddPort(port_desc))
                    .unwrap();
            } else {
                info!(target: "pipewire", "object with id {} is not a port", global.id);
            }
        })
        .global_remove(move |id| {
            info!(target: "pipewire", "remove global object with id {}", id);
            loop_state_for_global_remove_handler
                .borrow_mut()
                .objects
                .remove(&ObjectId(id));
            update_sender_for_global_remove_handler
                .send(GraphEvent::RemovePort(ObjectId(id)))
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
                    .send(GraphEvent::InitialSyncCompleted)
                    .unwrap();
            };
        })
        .register();

    main_loop.run();

    Ok(())
}
