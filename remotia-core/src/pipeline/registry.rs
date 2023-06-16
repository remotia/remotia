use std::{collections::HashMap, fmt::Debug, hash::Hash};

use super::Pipeline;

pub struct PipelineRegistry<F, K> {
    pipelines: HashMap<K, Pipeline<F>>,
}

impl<F, K> PipelineRegistry<F, K> where K: Eq + Hash {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
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
