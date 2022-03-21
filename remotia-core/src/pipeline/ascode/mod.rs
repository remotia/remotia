use log::info;
use tokio::{sync::mpsc::{self, UnboundedSender}, task::JoinHandle};

use crate::types::FrameData;

use self::{component::Component, feeder::AscodePipelineFeeder};

pub mod component;
pub mod feeder;

pub struct AscodePipeline {
    components: Vec<Component>,
    feeding_sender: Option<UnboundedSender<FrameData>>,

    tag: String,

    bound: bool
}

impl AscodePipeline {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            feeding_sender: None,

            tag: "".to_string(),

            bound: false
        }
    }

    pub fn link(mut self, component: Component) -> Self {
        self.components.push(component);
        self
    }

    pub fn get_feeder(&self) -> AscodePipelineFeeder {
        let sender = self.feeding_sender.as_ref().unwrap().clone();
        AscodePipelineFeeder::new(sender)
    }

    pub fn run(self) -> Vec<JoinHandle<()>> {
        info!("[{}] Launching threads...", self.tag);
        if !self.bound {
            panic!("[{}] Called 'run' before binding the pipeline", self.tag);
        }

        let mut handles = Vec::new();

        for component in self.components {
            let handle = component.launch();
            handles.push(handle);
        }

        handles
    }

    pub fn bind(mut self) -> Self {
        info!("[{}] Binding channels...", self.tag);

        for i in 0..self.components.len()-1 {
            let (sender, receiver) = mpsc::unbounded_channel::<FrameData>();

            let src_component = self.components.get_mut(i).unwrap();
            src_component.set_sender(sender);

            let dst_component = self.components.get_mut(i + 1).unwrap();
            dst_component.set_receiver(receiver);
        }

        self.bound = true;

        self
    }

    pub fn feedable(mut self) -> Self {
        let head = self.components.get_mut(0).unwrap();

        let (sender, receiver) = mpsc::unbounded_channel::<FrameData>();
        self.feeding_sender = Some(sender);

        head.set_receiver(receiver);

        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tag = tag.to_string();
        self
    }
}

impl Default for AscodePipeline {
    fn default() -> Self {
        Self::new()
    }
}