use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

fn compute_hash<T>(value: &T) -> u64
where
    T: Hash + ?Sized,
{
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone)]
pub struct Hc<T: Hash + Eq> {
    value: Arc<T>,
    hash: u64,
}

impl<T: Hash + Eq> Hc<T> {
    pub fn hash_value(&self) -> u64 {
        self.hash
    }
}

impl<T: Hash + Eq + std::fmt::Debug> std::fmt::Debug for Hc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hc")
            .field("value", &self.value)
            .field("hash", &self.hash)
            .finish()
    }
}

impl<T: Hash + Eq> Hash for Hc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl<T: Hash + Eq> PartialEq for Hc<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.hash != other.hash {
            return false;
        }
        if Arc::ptr_eq(&self.value, &other.value) {
            return true;
        }
        self.value.as_ref() == other.value.as_ref()
    }
}

impl<T: Hash + Eq> Eq for Hc<T> {}

impl<T: Hash + Eq> Deref for Hc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref()
    }
}

#[derive(Default)]
pub struct InternTable<T: Hash + Eq> {
    entries: HashMap<u64, Vec<Arc<T>>>,
}

impl<T: Hash + Eq> InternTable<T> {
    pub fn intern(&mut self, value: T) -> Hc<T> {
        let hash = compute_hash(&value);
        if let Some(bucket) = self.entries.get(&hash) {
            if let Some(existing) = bucket.iter().find(|existing| existing.as_ref() == &value) {
                return Hc {
                    value: Arc::clone(existing),
                    hash,
                };
            }
        }

        let value = Arc::new(value);
        self.entries
            .entry(hash)
            .or_default()
            .push(Arc::clone(&value));
        Hc { value, hash }
    }
}

#[derive(Clone)]
pub struct HcSlice<T: Hash + Eq> {
    values: Arc<[T]>,
    hash: u64,
}

impl<T: Hash + Eq> HcSlice<T> {
    pub fn hash_value(&self) -> u64 {
        self.hash
    }
}

impl<T: Hash + Eq + std::fmt::Debug> std::fmt::Debug for HcSlice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HcSlice")
            .field("values", &self.values)
            .field("hash", &self.hash)
            .finish()
    }
}

impl<T: Hash + Eq> Hash for HcSlice<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl<T: Hash + Eq> PartialEq for HcSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.hash != other.hash {
            return false;
        }
        if Arc::ptr_eq(&self.values, &other.values) {
            return true;
        }
        self.values.as_ref() == other.values.as_ref()
    }
}

impl<T: Hash + Eq> Eq for HcSlice<T> {}

impl<T: Hash + Eq> Deref for HcSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.values.as_ref()
    }
}

#[derive(Default)]
pub struct SliceInternTable<T: Hash + Eq> {
    entries: HashMap<u64, Vec<Arc<[T]>>>,
}

impl<T: Hash + Eq> SliceInternTable<T> {
    pub fn intern_slice(&mut self, values: Vec<T>) -> HcSlice<T> {
        let hash = compute_hash(values.as_slice());
        if let Some(bucket) = self.entries.get(&hash) {
            if let Some(existing) = bucket
                .iter()
                .find(|existing| existing.as_ref() == values.as_slice())
            {
                return HcSlice {
                    values: Arc::clone(existing),
                    hash,
                };
            }
        }

        let values: Arc<[T]> = values.into();
        self.entries
            .entry(hash)
            .or_default()
            .push(Arc::clone(&values));
        HcSlice { values, hash }
    }
}
