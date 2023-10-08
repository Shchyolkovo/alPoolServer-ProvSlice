use std::{
    collections::HashMap,
    hash::Hash,
    time::{Duration, Instant},
};

pub struct Cache<K: Eq + Hash + Clone, V: Clone> {
    duration: Duration,
    instants: HashMap<K, Instant>,
    values: HashMap<K, V>,
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    pub fn new(duration: Duration) -> Se