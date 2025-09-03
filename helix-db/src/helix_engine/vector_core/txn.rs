use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    ops::Deref,
};

use heed3::{RoTxn, RwTxn, WithoutTls};

use crate::helix_engine::vector_core::vector::HVector;

pub struct VecTxn<'scope, 'env> {
    pub txn: &'scope mut RwTxn<'env>,
    pub cache: HashMap<(u128, usize), HashSet<&'scope HVector>>,
}

impl<'scope, 'env> VecTxn<'scope, 'env> {
    pub fn new(txn: &'scope mut RwTxn<'env>) -> Self {
        Self {
            txn,
            cache: HashMap::with_capacity(4096),
        }
    }

    pub fn set_neighbors(&mut self, id: u128, level: usize, neighbors: &'scope BinaryHeap<HVector>) {
        // get change sets in neighbors
        let neighbors = neighbors.iter().map(|x| x).collect::<HashSet<_>>();

        let old_neighbors_to_delete = self
            .cache
            .get(&(id, level))
            .unwrap()
            .difference(&neighbors)
            .map(|x| *x)
            .collect::<HashSet<_>>();

        for neighbor in old_neighbors_to_delete {
            if let Some(neighbor_set) = self
                .cache
                .get_mut(&(neighbor.get_id(), neighbor.get_level()))
            {
                neighbor_set.remove(&neighbor);
            }
        }

        self.cache.insert((id, level), neighbors);
    }

    pub fn get_neighbors(&self, id: u128, level: usize) -> Option<Vec<HVector>> {
        self.cache
            .get(&(id, level))
            .map(|x| x.iter().map(|x| *x).cloned().collect())
    }

    pub fn get_rtxn(&self) -> &RoTxn<'env, WithoutTls> {
        self.txn
    }

    pub fn get_wtxn(&mut self) -> &mut RwTxn<'env> {
        self.txn
    }
}

impl<'scope, 'env> Deref for VecTxn<'scope, 'env> {
    type Target = RoTxn<'env, WithoutTls>;

    fn deref(&self) -> &Self::Target {
        &self.txn
    }
}
