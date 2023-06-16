use std::fmt::Debug;

use log::info;
use tokio::{sync::mpsc::{self, UnboundedSender}, task::JoinHandle};

use self::{component::Component, feeder::PipelineFeeder};

pub mod component;
pub mod feeder;
pub mod registry;

pub struct Pipeline<F> {
    components: Vec<Component<F>>,
    feeding_sender: Option<UnboundedSender<F>>,

    tag: String,

    bound: bool,

    to_be_feedable: bool,
}

impl<F: Debug + Default + Send + 'static> Pipeline<F> {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            feeding_sender: None,

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

    pub fn run(mut self) -> Vec<JoinHandle<()>> {
        info!("[{}] Launching threads...", self.tag);

        if !self.bound {
            self.bind();
        }

        if self.to_be_feedable {
            self.make_feedable();
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