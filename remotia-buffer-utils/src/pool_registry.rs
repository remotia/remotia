use std::collections::{hash_map::Keys, HashMap};

use remotia_core::processors::containers::sequential::Sequential;

use crate::pool::BuffersPool;

pub struct PoolRegistry {
    pools: HashMap<String, BuffersPool>,
}

impl PoolRegistry {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    pub async fn register(&mut self, slot_id: &str, pool_size: usize, buffer_size: usize) {
        let pool = BuffersPool::new(slot_id, pool_size, buffer_size).await;
        self.pools.insert(slot_id.to_string(), pool);
    }

    pub fn get(&self, slot_id: &str) -> &BuffersPool {
        self.pools.get(slot_id).expect(&format!(
            "No pool with ID {} found in the registry",
            slot_id
        ))
    }

    pub fn mass_redeemer(&self, soft: bool) -> Sequential {
        let mut sequential = Sequential::new();
        for (_, pool) in &self.pools {
            let redeemer = if soft {
                pool.redeemer().soft()
            } else {
                pool.redeemer()
            };
            sequential = sequential.append(redeemer)
        }

        sequential
    }

    pub fn mass_borrower(&self) -> Sequential {
        let mut sequential = Sequential::new();
        for (_, pool) in &self.pools {
            sequential = sequential.append(pool.borrower())
        }

        sequential
    }

    pub fn get_buffer_ids(&self) -> Keys<String, BuffersPool> {
        self.pools.keys()
    }
}
