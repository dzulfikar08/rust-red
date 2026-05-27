use crate::protocol::packets::QoS;
use bytes::Bytes;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RetainedMessage {
    pub payload: Bytes,
    pub qos: QoS,
}

pub struct RetainedStore {
    messages: HashMap<String, RetainedMessage>,
}

impl RetainedStore {
    pub fn new() -> Self {
        Self { messages: HashMap::new() }
    }

    pub fn store(&mut self, topic: String, payload: Bytes, qos: QoS) {
        self.messages.insert(topic, RetainedMessage { payload, qos });
    }

    pub fn remove(&mut self, topic: &str) {
        self.messages.remove(topic);
    }

    pub fn match_retained(&self, filter: &str) -> Vec<(String, RetainedMessage)> {
        self.messages
            .iter()
            .filter(|(topic, _)| topic_matches_filter(filter, topic))
            .map(|(t, m)| (t.clone(), m.clone()))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }
}

pub fn topic_matches_filter(filter: &str, topic: &str) -> bool {
    let fp: Vec<&str> = filter.split('/').collect();
    let tp: Vec<&str> = topic.split('/').collect();
    let (mut fi, mut ti) = (0, 0);
    while fi < fp.len() && ti < tp.len() {
        if fp[fi] == "#" {
            return true;
        }
        if fp[fi] == "+" || fp[fi] == tp[ti] {
            fi += 1;
            ti += 1;
        } else {
            return false;
        }
    }
    (fi == fp.len() && ti == tp.len()) || (fi < fp.len() && fp[fi] == "#")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::packets::QoS;

    // --- RetainedStore tests ---

    #[test]
    fn retained_store_and_retrieve() {
        let mut store = RetainedStore::new();
        store.store("test/topic".into(), Bytes::from("hello"), QoS::AtMostOnce);
        assert_eq!(store.len(), 1);

        let matches = store.match_retained("test/topic");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "test/topic");
        assert_eq!(&matches[0].1.payload[..], b"hello");
    }

    #[test]
    fn retained_store_overwrite() {
        let mut store = RetainedStore::new();
        store.store("test/topic".into(), Bytes::from("v1"), QoS::AtMostOnce);
        store.store("test/topic".into(), Bytes::from("v2"), QoS::AtLeastOnce);
        assert_eq!(store.len(), 1);

        let matches = store.match_retained("test/topic");
        assert_eq!(&matches[0].1.payload[..], b"v2");
        assert_eq!(matches[0].1.qos, QoS::AtLeastOnce);
    }

    #[test]
    fn retained_store_remove() {
        let mut store = RetainedStore::new();
        store.store("a".into(), Bytes::from("1"), QoS::AtMostOnce);
        store.store("b".into(), Bytes::from("2"), QoS::AtMostOnce);
        store.remove("a");
        assert_eq!(store.len(), 1);
        assert!(store.match_retained("a").is_empty());
        assert_eq!(store.match_retained("b").len(), 1);
    }

    #[test]
    fn retained_store_remove_nonexistent() {
        let mut store = RetainedStore::new();
        store.remove("nope"); // should not panic
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn retained_match_wildcard_hash() {
        let mut store = RetainedStore::new();
        store.store("sensors/temp".into(), Bytes::from("22"), QoS::AtMostOnce);
        store.store("sensors/humidity".into(), Bytes::from("55"), QoS::AtMostOnce);
        store.store("other/data".into(), Bytes::from("x"), QoS::AtMostOnce);

        let matches = store.match_retained("sensors/#");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn retained_match_single_wildcard_plus() {
        let mut store = RetainedStore::new();
        store.store("sensors/living/temp".into(), Bytes::from("22"), QoS::AtMostOnce);
        store.store("sensors/bedroom/temp".into(), Bytes::from("20"), QoS::AtMostOnce);
        store.store("sensors/living/humidity".into(), Bytes::from("55"), QoS::AtMostOnce);

        let matches = store.match_retained("sensors/+/temp");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn retained_match_root_wildcard() {
        let mut store = RetainedStore::new();
        store.store("a/b/c".into(), Bytes::from("1"), QoS::AtMostOnce);
        store.store("x/y".into(), Bytes::from("2"), QoS::AtMostOnce);

        let matches = store.match_retained("#");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn retained_no_match() {
        let mut store = RetainedStore::new();
        store.store("a/b".into(), Bytes::from("1"), QoS::AtMostOnce);
        assert!(store.match_retained("x/y").is_empty());
        assert!(store.match_retained("a/b/c").is_empty());
    }

    // --- topic_matches_filter tests ---

    #[test]
    fn filter_exact_match() {
        assert!(topic_matches_filter("sensors/temp", "sensors/temp"));
        assert!(!topic_matches_filter("sensors/temp", "sensors/humidity"));
    }

    #[test]
    fn filter_single_level_wildcard() {
        assert!(topic_matches_filter("sensors/+/temp", "sensors/living/temp"));
        assert!(topic_matches_filter("sensors/+/temp", "sensors/bedroom/temp"));
        assert!(!topic_matches_filter("sensors/+/temp", "sensors/temp"));
        assert!(!topic_matches_filter("sensors/+/temp", "sensors/a/b/temp"));
    }

    #[test]
    fn filter_multi_level_wildcard() {
        assert!(topic_matches_filter("sensors/#", "sensors/temp"));
        assert!(topic_matches_filter("sensors/#", "sensors/a/b/c"));
        assert!(!topic_matches_filter("sensors/#", "actuators/temp"));
    }

    #[test]
    fn filter_root_wildcard() {
        assert!(topic_matches_filter("#", "anything"));
        assert!(topic_matches_filter("#", "a/b/c/d"));
    }

    #[test]
    fn filter_plus_at_end() {
        assert!(topic_matches_filter("sensors/+", "sensors/temp"));
        assert!(!topic_matches_filter("sensors/+", "sensors/a/b"));
    }
}
