use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use fframes_studio_protocol::PreviewIdentity;

pub const MAX_THUMBNAIL_ENTRIES: usize = 64;
pub const MAX_THUMBNAIL_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ThumbnailKey {
    pub identity: PreviewIdentity,
    pub backend: String,
    pub scale_bits: u64,
    pub frame_index: usize,
    pub media_hash: String,
}

impl ThumbnailKey {
    pub fn new(identity: PreviewIdentity, scale: f64, frame_index: usize) -> Self {
        let media_hash = identity.source_revision.clone();
        Self {
            identity,
            backend: "cpu".into(),
            scale_bits: scale.to_bits(),
            frame_index,
            media_hash,
        }
    }
}

struct CacheEntry<I> {
    image: Arc<I>,
    decoded_bytes: usize,
}

/// A bounded LRU of decoded thumbnails; `I` is the image type the UI draws.
pub struct ThumbnailCache<I> {
    entries: HashMap<ThumbnailKey, CacheEntry<I>>,
    lru: VecDeque<ThumbnailKey>,
    bytes: usize,
    high_water_entries: usize,
    high_water_bytes: usize,
}

impl<I> Default for ThumbnailCache<I> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            lru: VecDeque::new(),
            bytes: 0,
            high_water_entries: 0,
            high_water_bytes: 0,
        }
    }
}

impl<I> ThumbnailCache<I> {
    pub fn get(&mut self, key: &ThumbnailKey) -> Option<Arc<I>> {
        let image = self.entries.get(key)?.image.clone();
        self.touch(key);
        Some(image)
    }

    pub fn contains(&self, key: &ThumbnailKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn insert(
        &mut self,
        key: ThumbnailKey,
        image: Arc<I>,
        decoded_bytes: usize,
    ) -> Vec<Arc<I>> {
        if decoded_bytes > MAX_THUMBNAIL_BYTES {
            return vec![image];
        }

        let mut retired = Vec::new();
        if let Some(replaced) = self.entries.remove(&key) {
            self.bytes -= replaced.decoded_bytes;
            self.remove_lru(&key);
            retired.push(replaced.image);
        }

        while self.entries.len() >= MAX_THUMBNAIL_ENTRIES
            || self.bytes + decoded_bytes > MAX_THUMBNAIL_BYTES
        {
            let oldest = self
                .lru
                .pop_front()
                .expect("non-empty cache has an LRU key");
            if let Some(evicted) = self.entries.remove(&oldest) {
                self.bytes -= evicted.decoded_bytes;
                retired.push(evicted.image);
            }
        }

        self.bytes += decoded_bytes;
        self.lru.push_back(key.clone());
        self.entries.insert(
            key,
            CacheEntry {
                image,
                decoded_bytes,
            },
        );
        self.high_water_entries = self.high_water_entries.max(self.entries.len());
        self.high_water_bytes = self.high_water_bytes.max(self.bytes);
        retired
    }

    pub fn clear(&mut self) -> Vec<Arc<I>> {
        self.lru.clear();
        self.bytes = 0;
        self.entries.drain().map(|(_, entry)| entry.image).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn high_water_entries(&self) -> usize {
        self.high_water_entries
    }

    pub fn high_water_bytes(&self) -> usize {
        self.high_water_bytes
    }

    fn touch(&mut self, key: &ThumbnailKey) {
        self.remove_lru(key);
        self.lru.push_back(key.clone());
    }

    fn remove_lru(&mut self, key: &ThumbnailKey) {
        if let Some(position) = self.lru.iter().position(|candidate| candidate == key) {
            self.lru.remove(position);
        }
    }
}
