//! Character pose cache — LRU cache for rendered character poses.
//!
//! Avoids re-rendering the same character pose across multiple frames.
//! Cache key: (CharacterDesc, CharacterState key fields) → rendered Pixmap

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use tiny_skia::Pixmap;

use crate::procedural::{CharacterDesc, CharacterState};

/// A cache key for a character pose.
///
/// We only hash the fields that affect the visual output, not transient
/// state like secondary motion values.
#[derive(Debug, Clone)]
pub struct PoseCacheKey {
    pub character_name: String,
    pub body_angle: f64,
    pub line_of_action: f64,
    pub torso_bend: f64,
    pub torso_squash: f64,
    pub shoulder_left: f64,
    pub shoulder_right: f64,
    pub arm_left_angle: f64,
    pub arm_right_angle: f64,
    pub elbow_left_bend: f64,
    pub elbow_right_bend: f64,
    pub leg_left_angle: f64,
    pub leg_right_angle: f64,
    pub knee_left_bend: f64,
    pub knee_right_bend: f64,
    pub head_tilt: f64,
    pub head_nod: f64,
    pub y_offset: f64,
    // Expression fields
    pub eyebrow_left: f64,
    pub eyebrow_right: f64,
    pub eye_open_left: f64,
    pub eye_open_right: f64,
    pub eye_direction: f64,
    pub mouth_smile: f64,
    pub mouth_open: f64,
}

impl PoseCacheKey {
    /// Create a cache key from a character description and state.
    pub fn from_state(name: &str, desc: &CharacterDesc, state: &CharacterState) -> Self {
        Self {
            character_name: name.to_string(),
            body_angle: state.body_angle,
            line_of_action: state.line_of_action,
            torso_bend: state.torso_bend,
            torso_squash: state.torso_squash,
            shoulder_left: state.shoulder_left,
            shoulder_right: state.shoulder_right,
            arm_left_angle: state.arm_left_angle,
            arm_right_angle: state.arm_right_angle,
            elbow_left_bend: state.elbow_left_bend,
            elbow_right_bend: state.elbow_right_bend,
            leg_left_angle: state.leg_left_angle,
            leg_right_angle: state.leg_right_angle,
            knee_left_bend: state.knee_left_bend,
            knee_right_bend: state.knee_right_bend,
            head_tilt: state.head_tilt,
            head_nod: state.head_nod,
            y_offset: state.y_offset,
            eyebrow_left: state.expression.eyebrow_left,
            eyebrow_right: state.expression.eyebrow_right,
            eye_open_left: state.expression.eye_open_left,
            eye_open_right: state.expression.eye_open_right,
            eye_direction: state.expression.eye_direction,
            mouth_smile: state.expression.mouth_smile,
            mouth_open: state.expression.mouth_open,
        }
    }

    /// Quantize floating point values for cache key hashing.
    /// This prevents tiny floating point differences from causing cache misses.
    fn quantize(v: f64) -> i64 {
        (v * 100.0).round() as i64
    }

    /// Check if two cache keys are equivalent with tolerance.
    pub fn equiv(&self, other: &Self) -> bool {
        self.character_name == other.character_name
            && Self::quantize(self.body_angle) == Self::quantize(other.body_angle)
            && Self::quantize(self.line_of_action) == Self::quantize(other.line_of_action)
            && Self::quantize(self.torso_bend) == Self::quantize(other.torso_bend)
            && Self::quantize(self.torso_squash) == Self::quantize(other.torso_squash)
            && Self::quantize(self.shoulder_left) == Self::quantize(other.shoulder_left)
            && Self::quantize(self.shoulder_right) == Self::quantize(other.shoulder_right)
            && Self::quantize(self.arm_left_angle) == Self::quantize(other.arm_left_angle)
            && Self::quantize(self.arm_right_angle) == Self::quantize(other.arm_right_angle)
            && Self::quantize(self.elbow_left_bend) == Self::quantize(other.elbow_left_bend)
            && Self::quantize(self.elbow_right_bend) == Self::quantize(other.elbow_right_bend)
            && Self::quantize(self.leg_left_angle) == Self::quantize(other.leg_left_angle)
            && Self::quantize(self.leg_right_angle) == Self::quantize(other.leg_right_angle)
            && Self::quantize(self.knee_left_bend) == Self::quantize(other.knee_left_bend)
            && Self::quantize(self.knee_right_bend) == Self::quantize(other.knee_right_bend)
            && Self::quantize(self.head_tilt) == Self::quantize(other.head_tilt)
            && Self::quantize(self.head_nod) == Self::quantize(other.head_nod)
            && Self::quantize(self.y_offset) == Self::quantize(other.y_offset)
            && Self::quantize(self.eyebrow_left) == Self::quantize(other.eyebrow_left)
            && Self::quantize(self.eyebrow_right) == Self::quantize(other.eyebrow_right)
            && Self::quantize(self.eye_open_left) == Self::quantize(other.eye_open_left)
            && Self::quantize(self.eye_open_right) == Self::quantize(other.eye_open_right)
            && Self::quantize(self.eye_direction) == Self::quantize(other.eye_direction)
            && Self::quantize(self.mouth_smile) == Self::quantize(other.mouth_smile)
            && Self::quantize(self.mouth_open) == Self::quantize(other.mouth_open)
    }
}

impl Hash for PoseCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.character_name.hash(state);
        Self::quantize(self.body_angle).hash(state);
        Self::quantize(self.line_of_action).hash(state);
        Self::quantize(self.torso_bend).hash(state);
        Self::quantize(self.torso_squash).hash(state);
        Self::quantize(self.shoulder_left).hash(state);
        Self::quantize(self.shoulder_right).hash(state);
        Self::quantize(self.arm_left_angle).hash(state);
        Self::quantize(self.arm_right_angle).hash(state);
        Self::quantize(self.elbow_left_bend).hash(state);
        Self::quantize(self.elbow_right_bend).hash(state);
        Self::quantize(self.leg_left_angle).hash(state);
        Self::quantize(self.leg_right_angle).hash(state);
        Self::quantize(self.knee_left_bend).hash(state);
        Self::quantize(self.knee_right_bend).hash(state);
        Self::quantize(self.head_tilt).hash(state);
        Self::quantize(self.head_nod).hash(state);
        Self::quantize(self.y_offset).hash(state);
        Self::quantize(self.eyebrow_left).hash(state);
        Self::quantize(self.eyebrow_right).hash(state);
        Self::quantize(self.eye_open_left).hash(state);
        Self::quantize(self.eye_open_right).hash(state);
        Self::quantize(self.eye_direction).hash(state);
        Self::quantize(self.mouth_smile).hash(state);
        Self::quantize(self.mouth_open).hash(state);
    }
}

impl PartialEq for PoseCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.equiv(other)
    }
}

impl Eq for PoseCacheKey {}

/// LRU cache for rendered character poses.
///
/// Stores rendered Pixmaps keyed by pose parameters.
/// When the cache exceeds `max_size`, the least recently used entry is evicted.
pub struct PoseCache {
    map: HashMap<PoseCacheKey, CachedEntry>,
    lru: Vec<PoseCacheKey>,
    max_size: usize,
    hits: u64,
    misses: u64,
}

struct CachedEntry {
    pixmap: Pixmap,
    width: u32,
    height: u32,
}

impl PoseCache {
    /// Create a new cache with the given maximum size.
    ///
    /// Default max size is 512 entries.
    pub fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::new(),
            lru: Vec::with_capacity(max_size),
            max_size,
            hits: 0,
            misses: 0,
        }
    }

    /// Get a cached pixmap for the given key, if present.
    ///
    /// Returns `Some((pixmap, width, height))` on cache hit.
    pub fn get(&mut self, key: &PoseCacheKey) -> Option<(&Pixmap, u32, u32)> {
        if let Some(entry) = self.map.get_mut(key) {
            // Move to front of LRU.
            if let Some(pos) = self.lru.iter().position(|k| k == key) {
                self.lru.remove(pos);
                self.lru.push(key.clone());
            }
            self.hits += 1;
            Some((&entry.pixmap, entry.width, entry.height))
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a pixmap into the cache.
    ///
    /// Returns the evicted key if any (when cache was full).
    pub fn insert(
        &mut self,
        key: PoseCacheKey,
        pixmap: Pixmap,
        width: u32,
        height: u32,
    ) -> Option<PoseCacheKey> {
        // If the key already exists, update it.
        if self.map.contains_key(&key) {
            if let Some(pos) = self.lru.iter().position(|k| *k == key) {
                self.lru.remove(pos);
            }
        }

        // Evict the oldest entry if we're at capacity.
        let mut evicted = None;
        if self.map.len() >= self.max_size {
            if let Some(oldest) = self.lru.first().cloned() {
                self.map.remove(&oldest);
                self.lru.remove(0);
                evicted = Some(oldest);
            }
        }

        // Insert the new entry.
        self.map.insert(
            key.clone(),
            CachedEntry {
                pixmap,
                width,
                height,
            },
        );
        self.lru.push(key);

        evicted
    }

    /// Get cache statistics.
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    /// Get the current cache size.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.map.clear();
        self.lru.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

impl Default for PoseCache {
    fn default() -> Self {
        Self::new(512)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procedural::{Expression, HairStyle};

    fn test_desc() -> CharacterDesc {
        CharacterDesc {
            name: "test".to_string(),
            body: crate::procedural::BodyDesc {
                height: 0.5,
                build: 0.5,
                skin_color: [200, 180, 160],
            },
            face: crate::procedural::FaceDesc {
                shape: 0.5,
                eye_size: 1.0,
                eye_color: [80, 100, 120],
                eyebrow_thickness: 0.5,
                nose_size: 0.5,
                lip_fullness: 0.5,
            },
            hair: crate::procedural::HairDesc {
                color: [60, 40, 30],
                style: HairStyle::Straight,
                length: 0.5,
            },
            outfit: crate::procedural::OutfitDesc {
                top: crate::procedural::ClothingItem {
                    kind: crate::procedural::ClothingKind::TShirt,
                    color: [100, 120, 140],
                    secondary_color: None,
                },
                bottom: crate::procedural::ClothingItem {
                    kind: crate::procedural::ClothingKind::Pants,
                    color: [60, 60, 70],
                    secondary_color: None,
                },
                shoes: crate::procedural::ShoeDesc {
                    color: [40, 40, 40],
                    kind: crate::procedural::ShoeKind::Sneakers,
                },
                accessories: vec![],
            },
        }
    }

    #[test]
    fn test_cache_hit_miss() {
        let mut cache = PoseCache::new(10);
        let desc = test_desc();
        let mut state = crate::procedural::CharacterState::default();
        state.body_angle = 45.0;

        let key = PoseCacheKey::from_state("test", &desc, &state);

        // Should be a miss.
        assert!(cache.get(&key).is_none());

        // Create a dummy pixmap.
        let pixmap = Pixmap::new(100, 100).unwrap();
        cache.insert(key.clone(), pixmap, 100, 100);

        // Should be a hit.
        assert!(cache.get(&key).is_some());

        let (hits, misses) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = PoseCache::new(3);
        let desc = test_desc();

        // Insert 3 items.
        for i in 0..3 {
            let mut state = crate::procedural::CharacterState::default();
            state.body_angle = i as f64 * 30.0;
            let key = PoseCacheKey::from_state("test", &desc, &state);
            let pixmap = Pixmap::new(100, 100).unwrap();
            cache.insert(key, pixmap, 100, 100);
        }

        assert_eq!(cache.len(), 3);

        // Insert a 4th item — should evict the oldest.
        let mut state = crate::procedural::CharacterState::default();
        state.body_angle = 90.0;
        let key = PoseCacheKey::from_state("test", &desc, &state);
        let pixmap = Pixmap::new(100, 100).unwrap();
        let evicted = cache.insert(key, pixmap, 100, 100);

        assert_eq!(cache.len(), 3);
        assert!(evicted.is_some());
    }
}