use std::fmt::Debug;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use log::info;
use tokio::{sync::mpsc::{self, UnboundedReceiver, UnboundedSender}, task::JoinHandle};

use self::{component::Component, feeder::PipelineFeeder};

pub mod component;
pub mod feeder;
pub mod registry;

pub struct PipelineHandle {
    shutdown_tx: UnboundedSender<()>,
}

impl PipelineHandle {
    pub fn request_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Clone for PipelineHandle {
    fn clone(&self) -> Self {
        Self {
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

pub struct LazyPipelineHandle {
    shutdown_tx: UnboundedSender<()>,
}

impl LazyPipelineHandle {
    pub fn request_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Clone for LazyPipelineHandle {
    fn clone(&self) -> Self {
        Self {
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

pub struct Pipeline<F> {
    components: Vec<Component<F>>,
    feeding_sender: Option<UnboundedSender<F>>,

    shutdown_tx: Option<UnboundedSender<()>>,
    shutdown_signal: Arc<AtomicBool>,

    tag: String,

    bound: bool,

    to_be_feedable: bool,
}

impl<F: Debug + Default + Send + 'static> Pipeline<F> {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            feeding_sender: None,

            shutdown_tx: None,
            shutdown_signal: Arc::new(AtomicBool::new(false)),

            tag: "".to_string(),

            bound: false,

            to_be_feedable: false,
        }
    }

    pub fn singleton(component: Component<F>) -> Self {
        Self::new().link(component)
    }

    pub fn link(mut self, component: Component<F>) -> Self {
        self.components.push(component);
        self
    }

    pub fn get_feeder(&mut self) -> PipelineFeeder<F> {
        if self.to_be_feedable {
            self.make_feedable();
        }

        let sender = self.feeding_sender.as_ref().unwrap().clone();
        PipelineFeeder::new(sender)
    }

    pub fn shutdown_signal(&self) -> Arc<AtomicBool> {
        self.shutdown_signal.clone()
    }

    pub fn get_handle(&mut self) -> PipelineHandle {
        if self.shutdown_tx.is_none() {
            let (tx, mut rx) = mpsc::unbounded_channel::<()>();
            self.shutdown_tx = Some(tx);

            let signal = self.shutdown_signal.clone();
            tokio::spawn(async move {
                if rx.recv().await.is_some() {
                    signal.store(true, Ordering::Relaxed);
                }
            });
        }

        PipelineHandle {
            shutdown_tx: self.shutdown_tx.as_ref().unwrap().clone(),
        }
    }

    pub fn run(mut self) -> Vec<JoinHandle<()>> {
        info!("[{}] Launching threads...", self.tag);

        if !self.bound {
            self.bind();
        }

        if self.to_be_feedable {
            self.make_feedable();
        }

        for component in &mut self.components {
            component.set_shutdown_signal(self.shutdown_signal.clone());
        }

        let mut handles = Vec::new();

        for component in self.components {
            let handle = component.launch();
            handles.push(handle);
        }

        handles
    }

    fn bind(&mut self) {
        info!("[{}] Binding channels...", self.tag);

        for i in 0..self.components.len()-1 {
            let (sender, receiver) = mpsc::unbounded_channel::<F>();

            let src_component = self.components.get_mut(i).unwrap();
            src_component.set_sender(sender);

            let dst_component = self.components.get_mut(i + 1).unwrap();
            dst_component.set_receiver(receiver);
        }

        self.bound = true;
    }

    fn make_feedable(&mut self) {
        let head = self.components.get_mut(0).unwrap();

        let (sender, receiver) = mpsc::unbounded_channel::<F>();
        self.feeding_sender = Some(sender);

        head.set_receiver(receiver);

        self.to_be_feedable = false;
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tag = tag.to_string();
        self
    }

    pub fn feedable(mut self) -> Self {
        self.to_be_feedable = true;
        self
    }
}

impl<F: Default + Debug + Send + 'static> Default for Pipeline<F> {
    fn default() -> Self {
        Self::new()
    }
}