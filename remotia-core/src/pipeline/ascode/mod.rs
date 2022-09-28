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

    bound: bool,

    to_be_feedable: bool,
}

impl AscodePipeline {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            feeding_sender: None,

            tag: "".to_string(),

            bound: false,

            to_be_feedable: false,
        }
    }

    pub fn singleton(component: Component) -> Self {
        Self::new().link(component)
    }

    pub fn link(mut self, component: Component) -> Self {
        self.components.push(component);
        self
    }

    pub fn get_feeder(&mut self) -> AscodePipelineFeeder {
        if self.to_be_feedable {
            self.make_feedable();
        }

        let sender = self.feeding_sender.as_ref().unwrap().clone();
        AscodePipelineFeeder::new(sender)
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
            let (sender, receiver) = mpsc::unbounded_channel::<FrameData>();

            let src_component = self.components.get_mut(i).unwrap();
            src_component.set_sender(sender);

            let dst_component = self.components.get_mut(i + 1).unwrap();
            dst_component.set_receiver(receiver);
        }

        self.bound = true;
    }

    fn make_feedable(&mut self) {
        let head = self.components.get_mut(0).unwrap();

        let (sender, receiver) = mpsc::unbounded_channel::<FrameData>();
        self.feeding_sender = Some(sender);

        head.set_receiver(receiver);
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