use std::collections::HashMap;

pub struct KvStore {
    kvs: HashMap<String, String>,
}

impl KvStore {
    pub fn new() -> Self {
        KvStore {
            kvs: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.kvs.insert(key, value);
    }

    pub fn get(&self, key: String) -> Option<String>{
        if let Some(v) = self.kvs.get(&key) {
            return Some(v.clone());
        }
        None
    }

    pub fn remove(&mut self, key: String) {
        self.kvs.remove(&key);
    }
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}
