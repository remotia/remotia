use std::{cell::RefCell, collections::HashMap, fmt::Debug, hash::Hash};

use tokio::sync::mpsc::UnboundedReceiver;

use super::{PipelineHandle, Pipeline};

pub struct PipelineRegistry<F, K> {
    pipelines: HashMap<K, Pipeline<F>>,
    pending_handles: RefCell<HashMap<K, Vec<UnboundedReceiver<()>>>>,
}

impl<F, K> PipelineRegistry<F, K> where K: Eq + Hash {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
            pending_handles: RefCell::new(HashMap::new()),
        }
    }

    pub fn lazy_handle(&self, id: K) -> PipelineHandle
    where
        K: Clone,
    {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        self.pending_handles.borrow_mut().entry(id).or_default().push(rx);
        PipelineHandle { shutdown_tx: tx }
    }

    pub fn register_empty(&mut self, id: K)
    where
        F: Default + Debug + Send + 'static,
    {
        self.pipelines.insert(id, Pipeline::<F>::new());
    }

    pub fn register(&mut self, id: K, pipeline: Pipeline<F>) {
        self.pipelines.insert(id, pipeline);
    }

    pub fn get_mut(&mut self, id: &K) -> &mut Pipeline<F> {
        self.pipelines.get_mut(id).unwrap()
    }

    pub fn get(&self, id: &K) -> &Pipeline<F> {
        self.pipelines.get(id).unwrap()
    }

    pub async fn run(mut self)
    where
        F: Default + Debug + Send + 'static,
    {
        for (key, receivers) in self.pending_handles.borrow_mut().drain() {
            if let Some(pipeline) = self.pipelines.get(&key) {
                let signal = pipeline.shutdown_signal();
                for mut rx in receivers {
                    let signal = signal.clone();
                    tokio::spawn(async move {
                        if rx.recv().await.is_some() {
                            signal.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    });
                }
            }
        }

        let mut handles = Vec::new();
        for (_, pipeline) in self.pipelines.drain() {
            handles.extend(pipeline.run());
        }

        for handle in handles {
            handle.await.unwrap()
        }
    }
}

#[macro_export]
macro_rules! register {
    ($registry:ident, $id:expr, $pipeline:expr) => {{
        let _pipe = $pipeline;
        $registry.register($id, _pipe);
    }};
}
