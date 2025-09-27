use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    ops::Deref,
    rc::Rc,
    sync::Arc,
};

use heed3::{
    Database, PutFlags, RoTxn, RwTxn, WithoutTls,
    types::{Bytes, Unit},
};

use crate::helix_engine::{
    types::VectorError,
    vector_core::{vector::HVector, vector_core::VectorCore},
};

pub struct VecTxn<'env> {
    pub txn: RwTxn<'env>,
    pub cache: HashMap<(u128, usize), HashSet<Arc<HVector>>>,
}

impl<'env> VecTxn<'env> {
    pub fn new(txn: RwTxn<'env>) -> Self {
        Self {
            txn,
            cache: HashMap::with_capacity(256),
        }
    }

    pub fn set_neighbors(
        &mut self,
        curr_vec: Arc<HVector>,
        level: usize,
        neighbors: &BinaryHeap<Arc<HVector>>,
    ) {
        // get change sets in neighbors
        let neighbors = neighbors.iter().map(Arc::clone).collect::<HashSet<_>>();

        let old_neighbors = self
            .cache
            .remove(&(curr_vec.get_id(), level))
            .unwrap_or_default();

        for old_neighbor in &old_neighbors {
            if neighbors.contains(old_neighbor) {
                continue;
            }
            if let Some(neighbor_set) = self.cache.get_mut(&(old_neighbor.get_id(), level)) {
                neighbor_set.remove(&curr_vec);
            }
        }

        for neighbor in &neighbors {
            if neighbor.get_id() == curr_vec.get_id() || old_neighbors.contains(neighbor) {
                continue;
            }
            self.cache
                .entry((neighbor.get_id(), level))
                .or_insert_with(HashSet::new)
                .insert(Arc::clone(&curr_vec));
        }

        self.cache.insert((curr_vec.get_id(), level), neighbors);
    }

    pub fn get_neighbors(&self, id: u128, level: usize) -> Option<Vec<Arc<HVector>>> {
        self.cache
            .get(&(id, level))
            .map(|x| x.iter().map(Arc::clone).collect())
    }

    pub fn insert_neighbors(&mut self, id: u128, level: usize, neighbors: &Vec<Arc<HVector>>) {
        let neighbors = neighbors.iter().map(Arc::clone).collect::<HashSet<_>>();
        self.cache.entry((id, level)).or_default().extend(neighbors);
    }

    pub fn get_rtxn(&self) -> &RoTxn<'env, WithoutTls> {
        &self.txn
    }

    pub fn get_wtxn(&mut self) -> &mut RwTxn<'env> {
        &mut self.txn
    }

    pub fn commit(mut self, db: &Database<Bytes, Unit>) -> Result<(), VectorError> {
        let txn = &mut self.txn;
        let mut vec = HashSet::with_capacity(self.cache.len() * 128);
        let mut vecs = 0;
        for (id, level) in self.cache.keys() {
            if let Some(neighbors) = self.cache.get(&(*id, *level)) {
                for neighbor in neighbors {
                    if neighbor.get_id() == *id {
                        continue;
                    }
                    let out_key = VectorCore::out_edges_key(*id, *level, Some(neighbor.get_id()));
                    let in_key = VectorCore::out_edges_key(neighbor.get_id(), *level, Some(*id));
                    vec.insert(out_key);
                    vec.insert(in_key);
                }
                vecs += 1;
            }
        }
        // vec.sort();
        for key in vec {
            // db.put_with_flags(txn, PutFlags::APPEND, &key, &())?;
            db.put(txn, &key, &())?;
        }

        self.txn.commit().map_err(VectorError::from)
    }
}

impl<'env> Deref for VecTxn<'env> {
    type Target = RoTxn<'env, WithoutTls>;

    fn deref(&self) -> &Self::Target {
        &self.txn
    }
}
