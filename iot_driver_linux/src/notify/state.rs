//! Notification state management — per-key priority stacks.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use super::keymap::MATRIX_LEN;
use crate::effect::ResolvedEffect;
use monsgeek_transport::protocol::LedPos;

/// A notification posted to the daemon.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u64,
    pub source: String,
    pub effect_name: String,
    /// LED-grid cells this notification lights (row-major `row * 16 + col`).
    pub led_positions: Vec<LedPos>,
    pub resolved: ResolvedEffect,
    pub priority: i32,
    pub ttl: Option<Duration>,
    pub created: Instant,
    /// Per-key stagger delay in ms. Keys not in this map start immediately.
    pub stagger_offsets: HashMap<LedPos, f64>,
}

impl Notification {
    /// Check if this notification has expired.
    pub fn is_expired(&self) -> bool {
        match self.ttl {
            Some(ttl) => self.created.elapsed() >= ttl,
            None => false,
        }
    }
}

/// Per-key priority stack: maps priority -> notification ID.
/// Higher priority wins (BTreeMap last entry).
type PriorityStack = BTreeMap<i32, u64>;

/// Central notification store.
pub struct NotificationStore {
    /// All notifications by ID.
    notifications: BTreeMap<u64, Notification>,
    /// Per-key priority stacks (indexed by matrix position 0-95).
    key_stacks: Vec<PriorityStack>,
    /// Next notification ID.
    next_id: u64,
}

impl Default for NotificationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationStore {
    pub fn new() -> Self {
        Self {
            notifications: BTreeMap::new(),
            key_stacks: vec![BTreeMap::new(); MATRIX_LEN],
            next_id: 1,
        }
    }

    /// Add a notification. Returns its assigned ID.
    pub fn add(&mut self, mut notif: Notification) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        notif.id = id;

        for &pos in &notif.led_positions {
            if let Some(stack) = self.key_stacks.get_mut(usize::from(pos)) {
                stack.insert(notif.priority, id);
            }
        }

        self.notifications.insert(id, notif);
        id
    }

    /// Remove a notification by ID.
    pub fn remove(&mut self, id: u64) -> Option<Notification> {
        if let Some(notif) = self.notifications.remove(&id) {
            for &pos in &notif.led_positions {
                let idx = usize::from(pos);
                if self
                    .key_stacks
                    .get(idx)
                    .and_then(|s| s.get(&notif.priority))
                    == Some(&id)
                {
                    self.key_stacks[idx].remove(&notif.priority);
                }
            }
            Some(notif)
        } else {
            None
        }
    }

    /// Remove all notifications for given key indices.
    pub fn remove_by_key(&mut self, positions: &[LedPos]) -> Vec<u64> {
        let mut removed_ids = Vec::new();
        for &pos in positions {
            let Some(stack) = self.key_stacks.get(usize::from(pos)) else {
                continue;
            };
            let ids: Vec<u64> = stack.values().copied().collect();
            for id in ids {
                if self.remove(id).is_some() {
                    removed_ids.push(id);
                }
            }
        }
        removed_ids
    }

    /// Remove all notifications from a given source.
    pub fn remove_by_source(&mut self, source: &str) -> Vec<u64> {
        let ids: Vec<u64> = self
            .notifications
            .iter()
            .filter(|(_, n)| n.source == source)
            .map(|(&id, _)| id)
            .collect();
        let mut removed = Vec::new();
        for id in ids {
            if self.remove(id).is_some() {
                removed.push(id);
            }
        }
        removed
    }

    /// Clear all notifications.
    pub fn clear(&mut self) {
        self.notifications.clear();
        for stack in &mut self.key_stacks {
            stack.clear();
        }
    }

    /// Expire notifications that have exceeded their TTL.
    pub fn expire(&mut self) -> Vec<u64> {
        let expired: Vec<u64> = self
            .notifications
            .iter()
            .filter(|(_, n)| n.is_expired())
            .map(|(&id, _)| id)
            .collect();
        let mut removed = Vec::new();
        for id in expired {
            if self.remove(id).is_some() {
                removed.push(id);
            }
        }
        removed
    }

    /// Get the active (highest-priority) notification for an LED-grid cell.
    pub fn active_for_key(&self, pos: LedPos) -> Option<&Notification> {
        self.key_stacks
            .get(usize::from(pos))?
            .values()
            .next_back()
            .and_then(|id| self.notifications.get(id))
    }

    /// List all active notifications as (id, key_str, source, effect_name, priority).
    pub fn list(&self) -> Vec<(u64, String, String, String, i32)> {
        let labels = crate::effect::preview::build_labels();
        self.notifications
            .values()
            .map(|n| {
                let key_str = n
                    .led_positions
                    .iter()
                    .filter_map(|&i| {
                        labels
                            .get(usize::from(i))
                            .map(|l| l.trim())
                            .filter(|l| !l.is_empty())
                            .or(Some("?"))
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                (
                    n.id,
                    key_str,
                    n.source.clone(),
                    n.effect_name.clone(),
                    n.priority,
                )
            })
            .collect()
    }

    /// Number of active notifications.
    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }

    /// Get a notification by ID.
    pub fn get(&self, id: u64) -> Option<&Notification> {
        self.notifications.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect;

    fn make_notif(indices: Vec<LedPos>, priority: i32, source: &str) -> Notification {
        let mut vars = BTreeMap::new();
        vars.insert("color".to_string(), "red".to_string());
        let lib = effect::EffectLibrary::from_toml(effect::DEFAULT_EFFECTS_TOML).unwrap();
        let resolved = effect::resolve(lib.get("solid").unwrap(), &vars).unwrap();
        Notification {
            id: 0,
            source: source.to_string(),
            effect_name: "solid".to_string(),
            led_positions: indices,
            resolved,
            priority,
            ttl: None,
            created: Instant::now(),
            stagger_offsets: HashMap::new(),
        }
    }

    #[test]
    fn test_add_and_active() {
        let mut store = NotificationStore::new();
        let n = make_notif(vec![LedPos::new(1)], 0, "test");
        let id = store.add(n);
        assert_eq!(id, 1);
        assert!(store.active_for_key(LedPos::new(1)).is_some());
        assert_eq!(store.active_for_key(LedPos::new(1)).unwrap().id, 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut store = NotificationStore::new();
        let low = make_notif(vec![LedPos::new(5)], -10, "tmux");
        let high = make_notif(vec![LedPos::new(5)], 10, "email");
        let _low_id = store.add(low);
        let high_id = store.add(high);
        assert_eq!(store.active_for_key(LedPos::new(5)).unwrap().id, high_id);
    }

    #[test]
    fn test_remove_reveals_lower() {
        let mut store = NotificationStore::new();
        let low = make_notif(vec![LedPos::new(5)], -10, "tmux");
        let high = make_notif(vec![LedPos::new(5)], 10, "email");
        let low_id = store.add(low);
        let high_id = store.add(high);
        store.remove(high_id);
        assert_eq!(store.active_for_key(LedPos::new(5)).unwrap().id, low_id);
    }

    #[test]
    fn test_remove_by_source() {
        let mut store = NotificationStore::new();
        store.add(make_notif(vec![LedPos::new(1)], 0, "tmux"));
        store.add(make_notif(vec![LedPos::new(2)], 0, "tmux"));
        store.add(make_notif(vec![LedPos::new(3)], 0, "email"));
        let removed = store.remove_by_source("tmux");
        assert_eq!(removed.len(), 2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_ttl_expiry() {
        let mut store = NotificationStore::new();
        let mut n = make_notif(vec![LedPos::new(1)], 0, "test");
        n.ttl = Some(Duration::from_millis(0));
        n.created = Instant::now() - Duration::from_secs(1);
        store.add(n);
        let expired = store.expire();
        assert_eq!(expired.len(), 1);
        assert!(store.is_empty());
    }
}
