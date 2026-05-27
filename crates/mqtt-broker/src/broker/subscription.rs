use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Subscription {
    pub client_id: String,
    pub qos: crate::protocol::packets::QoS,
}

#[derive(Debug, Default)]
struct TrieNode {
    children: BTreeMap<String, TrieNode>,
    subscriptions: Vec<Subscription>,
    /// Subscriptions using the # multi-level wildcard at this node.
    multi_subscriptions: Vec<Subscription>,
}

pub struct TopicTrie {
    root: TrieNode,
    /// Subscriptions containing + single-level wildcards, matched linearly.
    wildcard_subscriptions: Vec<(String, Subscription)>,
}

impl TopicTrie {
    pub fn new() -> Self {
        Self { root: TrieNode::default(), wildcard_subscriptions: Vec::new() }
    }

    pub fn subscribe(&mut self, client_id: String, topic_filter: String, qos: crate::protocol::packets::QoS) {
        let levels: Vec<&str> = topic_filter.split('/').collect();
        if topic_filter.ends_with('#') && !topic_filter.ends_with("/#") && levels.len() == 1 && levels[0] == "#" {
            // Root-level "#" matches everything
            self.root.multi_subscriptions.retain(|s| s.client_id != client_id);
            self.root.multi_subscriptions.push(Subscription { client_id, qos });
        } else if topic_filter.ends_with('#') {
            let parent_levels = &levels[..levels.len() - 1];
            let node = self.find_or_create_node(parent_levels);
            node.multi_subscriptions.retain(|s| s.client_id != client_id);
            node.multi_subscriptions.push(Subscription { client_id, qos });
        } else if levels.contains(&"+") {
            // Single-level wildcard subscription — match linearly
            self.wildcard_subscriptions.retain(|(f, s)| s.client_id != client_id || f != &topic_filter);
            self.wildcard_subscriptions.push((topic_filter, Subscription { client_id, qos }));
        } else {
            let node = self.find_or_create_node(&levels);
            node.subscriptions.retain(|s| s.client_id != client_id);
            node.subscriptions.push(Subscription { client_id, qos });
        }
    }

    pub fn unsubscribe(&mut self, client_id: &str, topic_filter: &str) {
        let levels: Vec<&str> = topic_filter.split('/').collect();
        if topic_filter.ends_with('#') {
            let parent_levels =
                if levels.len() == 1 && levels[0] == "#" { &[][..] } else { &levels[..levels.len() - 1] };
            if let Some(node) = self.find_node_mut(parent_levels) {
                node.multi_subscriptions.retain(|s| s.client_id != client_id);
            }
        } else if levels.contains(&"+") {
            self.wildcard_subscriptions.retain(|(f, s)| s.client_id != client_id || f != topic_filter);
        } else {
            if let Some(node) = self.find_node_mut(&levels) {
                node.subscriptions.retain(|s| s.client_id != client_id);
            }
        }
    }

    /// Remove all subscriptions for a given client across the entire trie.
    pub fn remove_subscriptions_for_client(&mut self, client_id: &str) {
        Self::remove_recursive_inner(&mut self.root, client_id);
        self.wildcard_subscriptions.retain(|(_, s)| s.client_id != client_id);
    }

    fn remove_recursive_inner(node: &mut TrieNode, client_id: &str) {
        node.subscriptions.retain(|s| s.client_id != client_id);
        node.multi_subscriptions.retain(|s| s.client_id != client_id);
        for child in node.children.values_mut() {
            Self::remove_recursive_inner(child, client_id);
        }
    }

    /// Find all subscribers matching a concrete topic.
    pub fn match_topic(&self, topic: &str) -> Vec<Subscription> {
        let levels: Vec<&str> = topic.split('/').collect();
        let mut results = Vec::new();
        self.match_recursive(&self.root, &levels, 0, &mut results);

        // Also match + wildcard subscriptions linearly
        for (filter, sub) in &self.wildcard_subscriptions {
            if topic_matches_single_wildcard(filter, &levels) {
                results.push(sub.clone());
            }
        }

        results
    }

    fn match_recursive(&self, node: &TrieNode, levels: &[&str], depth: usize, results: &mut Vec<Subscription>) {
        // Multi-level wildcard (#) at this level: all subscriptions match remaining
        for sub in &node.multi_subscriptions {
            results.push(sub.clone());
        }

        if depth >= levels.len() {
            // Exact match at this level
            for sub in &node.subscriptions {
                results.push(sub.clone());
            }
            return;
        }

        // Try matching exact level name
        if let Some(child) = node.children.get(levels[depth]) {
            self.match_recursive(child, levels, depth + 1, results);
        }
    }

    pub fn dump_subscriptions(&self) -> Vec<super::SubscriptionInfo> {
        let mut result = Vec::new();
        self.dump_recursive(&self.root, String::new(), &mut result);
        for (filter, sub) in &self.wildcard_subscriptions {
            result.push(super::SubscriptionInfo {
                topic_filter: filter.clone(),
                client_id: sub.client_id.clone(),
                qos: sub.qos as u8,
            });
        }
        result
    }

    fn dump_recursive(&self, node: &TrieNode, prefix: String, result: &mut Vec<super::SubscriptionInfo>) {
        for sub in &node.subscriptions {
            result.push(super::SubscriptionInfo {
                topic_filter: if prefix.is_empty() { "/".to_string() } else { prefix.clone() },
                client_id: sub.client_id.clone(),
                qos: sub.qos as u8,
            });
        }
        for sub in &node.multi_subscriptions {
            result.push(super::SubscriptionInfo {
                topic_filter: format!("{}#", if prefix.is_empty() { "".to_string() } else { format!("{}/", prefix) }),
                client_id: sub.client_id.clone(),
                qos: sub.qos as u8,
            });
        }
        for (name, child) in &node.children {
            let child_prefix = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
            self.dump_recursive(child, child_prefix, result);
        }
    }

    fn find_or_create_node<'a>(&'a mut self, levels: &[&str]) -> &'a mut TrieNode {
        let mut node = &mut self.root;
        for level in levels {
            if *level == "+" {
                continue;
            }
            if *level == "#" {
                continue;
            }
            node = node.children.entry(level.to_string()).or_default();
        }
        node
    }

    fn find_node_mut<'a>(&'a mut self, levels: &[&str]) -> Option<&'a mut TrieNode> {
        let mut node = &mut self.root;
        for level in levels {
            if *level == "+" {
                continue;
            }
            if *level == "#" {
                continue;
            }
            match node.children.get_mut(*level) {
                Some(child) => node = child,
                None => return None,
            }
        }
        Some(node)
    }
}

/// Check if a topic filter containing + wildcards matches a concrete topic.
/// e.g., "sensors/+/temperature" matches "sensors/living/temperature"
fn topic_matches_single_wildcard(filter: &str, topic_levels: &[&str]) -> bool {
    let filter_levels: Vec<&str> = filter.split('/').collect();
    if filter_levels.len() != topic_levels.len() {
        return false;
    }
    for (f, t) in filter_levels.iter().zip(topic_levels.iter()) {
        if *f != "+" && *f != *t {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::packets::QoS;

    #[test]
    fn test_exact_match() {
        let mut trie = TopicTrie::new();
        trie.subscribe("client1".into(), "sensors/temperature".into(), QoS::AtMostOnce);

        let matches = trie.match_topic("sensors/temperature");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].client_id, "client1");

        let no_matches = trie.match_topic("sensors/humidity");
        assert!(no_matches.is_empty());
    }

    #[test]
    fn test_multi_level_wildcard() {
        let mut trie = TopicTrie::new();
        trie.subscribe("client1".into(), "sensors/#".into(), QoS::AtLeastOnce);

        let matches = trie.match_topic("sensors/temperature");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].client_id, "client1");

        let matches_deep = trie.match_topic("sensors/living/temperature");
        assert_eq!(matches_deep.len(), 1);
    }

    #[test]
    fn test_root_wildcard() {
        let mut trie = TopicTrie::new();
        trie.subscribe("client1".into(), "#".into(), QoS::AtMostOnce);

        let matches = trie.match_topic("any/topic");
        assert_eq!(matches.len(), 1);

        let matches_root = trie.match_topic("root");
        assert_eq!(matches_root.len(), 1);
    }

    #[test]
    fn test_single_level_wildcard() {
        let mut trie = TopicTrie::new();
        trie.subscribe("client1".into(), "sensors/+/temperature".into(), QoS::AtMostOnce);

        let matches = trie.match_topic("sensors/living/temperature");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].client_id, "client1");

        let matches2 = trie.match_topic("sensors/bedroom/temperature");
        assert_eq!(matches2.len(), 1);

        // Different depth — no match
        let no_matches = trie.match_topic("sensors/temperature");
        assert!(no_matches.is_empty());

        // Different last level — no match
        let no_matches2 = trie.match_topic("sensors/living/humidity");
        assert!(no_matches2.is_empty());
    }

    #[test]
    fn test_single_wildcard_multiple() {
        let mut trie = TopicTrie::new();
        trie.subscribe("client1".into(), "+/+/status".into(), QoS::AtMostOnce);

        let matches = trie.match_topic("device/room1/status");
        assert_eq!(matches.len(), 1);

        let no_matches = trie.match_topic("device/status");
        assert!(no_matches.is_empty());
    }

    #[test]
    fn test_remove_subscriptions_for_client() {
        let mut trie = TopicTrie::new();
        trie.subscribe("c1".into(), "a/b".into(), QoS::AtMostOnce);
        trie.subscribe("c2".into(), "a/b".into(), QoS::AtLeastOnce);
        trie.subscribe("c1".into(), "c/#".into(), QoS::AtMostOnce);
        trie.subscribe("c1".into(), "sensors/+/temp".into(), QoS::AtMostOnce);

        trie.remove_subscriptions_for_client("c1");

        let matches = trie.match_topic("a/b");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].client_id, "c2");

        let matches_c = trie.match_topic("c/d");
        assert!(matches_c.is_empty());

        let matches_wild = trie.match_topic("sensors/living/temp");
        assert!(matches_wild.is_empty());
    }
}
