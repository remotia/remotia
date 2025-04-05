use std::{collections::{hash_map::Keys, HashMap}, hash::Hash, fmt::Debug};

use crate::pool::BuffersPool;

pub struct PoolRegistry<K: Copy + PartialEq + Eq + Hash> {
    pools: HashMap<K, BuffersPool<K>>,
}

impl<K: Copy + PartialEq + Eq + Hash + Debug> PoolRegistry<K> {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    pub async fn register(&mut self, slot_id: K, pool_size: usize, buffer_size: usize) {
        let pool = BuffersPool::new(slot_id, pool_size, buffer_size).await;
        self.pools.insert(slot_id, pool);
    }

    pub fn get(&self, slot_id: K) -> &BuffersPool<K> {
        self.pools.get(&slot_id).expect(&format!(
            "No pool with ID {:?} found in the registry",
            slot_id
        ))
    }

    /*pub fn mass_redeemer(&self, soft: bool) -> Sequential<F> {
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

    pub fn mass_borrower(&self) -> Sequential<F> {
        let mut sequential = Sequential::new();
        for (_, pool) in &self.pools {
            sequential = sequential.append(pool.borrower())
        }

        sequential
    }*/

    pub fn get_buffer_ids(&self) -> Keys<K, BuffersPool<K>> {
        self.pools.keys()
    }
}
