use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use sha2::{Digest, Sha256};

use super::splash_host::TemplateHandle;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub app_type: String,
    pub app_version: u32,
    pub template_id: String,
    pub template_hash: u64,
    pub manifest_version: u32,
    pub host_version: u32,
}

#[derive(Debug, Default)]
pub struct TemplateCache {
    entries: RwLock<HashMap<CacheKey, Arc<TemplateHandle>>>,
}

impl TemplateCache {
    pub fn get(&self, key: &CacheKey) -> Option<Arc<TemplateHandle>> {
        self.entries
            .read()
            .ok()
            .and_then(|entries| entries.get(key).cloned())
    }

    pub fn insert(&self, key: CacheKey, handle: Arc<TemplateHandle>) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(key, handle);
        }
    }
}

pub fn template_hash(source: &str) -> u64 {
    let digest = Sha256::digest(source.as_bytes());
    let mut truncated = [0u8; 8];
    truncated.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(truncated)
}

#[cfg(test)]
mod tests {
    use super::{template_hash, CacheKey, TemplateCache};
    use crate::home::app_registry::splash_host::TemplateHandle;
    use std::sync::Arc;

    #[test]
    fn cache_key_equality_tracks_all_six_dimensions() {
        let key = CacheKey {
            app_type: "weather".into(),
            app_version: 2,
            template_id: "card_standard".into(),
            template_hash: 11,
            manifest_version: 1,
            host_version: 1,
        };

        assert_eq!(key.clone(), key);
        assert_ne!(
            key.clone(),
            CacheKey {
                template_hash: 12,
                ..key.clone()
            }
        );
        assert_ne!(
            key.clone(),
            CacheKey {
                manifest_version: 2,
                ..key.clone()
            }
        );
        assert_ne!(
            key.clone(),
            CacheKey {
                host_version: 2,
                ..key.clone()
            }
        );
    }

    #[test]
    fn template_hash_changes_when_source_changes() {
        assert_ne!(
            template_hash("RoundedView {}"),
            template_hash("RoundedView { Label {} }")
        );
    }

    #[test]
    fn cache_lookup_misses_when_manifest_or_host_version_changes() {
        let cache = TemplateCache::default();
        let key = CacheKey {
            app_type: "weather".into(),
            app_version: 2,
            template_id: "card_standard".into(),
            template_hash: 42,
            manifest_version: 1,
            host_version: 1,
        };
        let handle = Arc::new(TemplateHandle::new_for_test(
            "weather_guidance",
            "card_standard",
            "RoundedView {}",
        ));

        cache.insert(key.clone(), handle);

        assert!(cache.get(&key).is_some());
        assert!(cache
            .get(&CacheKey {
                manifest_version: 2,
                ..key.clone()
            })
            .is_none());
        assert!(cache
            .get(&CacheKey {
                host_version: 2,
                ..key
            })
            .is_none());
    }
}
