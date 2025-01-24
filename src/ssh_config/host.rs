use regex::Regex;
use std::collections::HashMap;

use super::EntryType;

pub(crate) type Entry = (EntryType, String);

#[derive(Debug, Clone)]
pub struct Host {
    patterns: Vec<String>,
    entries: HashMap<EntryType, String>,
}

impl Host {
    #[must_use]
    pub fn new(patterns: Vec<String>) -> Host {
        Host {
            patterns,
            entries: HashMap::new(),
        }
    }

    pub fn update(&mut self, entry: Entry) {
        self.entries.insert(entry.0, entry.1);
    }

    pub(crate) fn extend_patterns(&mut self, host: &Host) {
        self.patterns.extend(host.patterns.clone());
    }

    pub(crate) fn extend_entries(&mut self, host: &Host) {
        self.entries.extend(host.entries.clone());
    }

    pub(crate) fn extend_if_not_contained(&mut self, host: &Host) {
        for (key, value) in &host.entries {
            if !self.entries.contains_key(key) {
                self.entries.insert(key.clone(), value.clone());
            }
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn get_patterns(&self) -> &Vec<String> {
        &self.patterns
    }

    /// # Panics
    ///
    /// Will panic if the regex cannot be compiled.
    #[allow(clippy::must_use_candidate)]
    pub fn matching_pattern_regexes(&self) -> Vec<(Regex, bool)> {
        if self.patterns.is_empty() {
            return Vec::new();
        }

        self.patterns
            .iter()
            .filter_map(|pattern| {
                let contains_wildcard =
                    pattern.contains('*') || pattern.contains('?') || pattern.contains('!');
                if !contains_wildcard {
                    return None;
                }

                let mut pattern = pattern
                    .replace('.', r"\.")
                    .replace('*', ".*")
                    .replace('?', ".");

                let is_negated = pattern.starts_with('!');
                if is_negated {
                    pattern.remove(0);
                }

                pattern = format!("^{pattern}$");
                Some((Regex::new(&pattern).unwrap(), is_negated))
            })
            .collect()
    }

    #[allow(clippy::must_use_candidate)]
    pub fn get(&self, entry: &EntryType) -> Option<String> {
        self.entries.get(entry).cloned()
    }

    #[allow(clippy::must_use_candidate)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait HostVecExt {
    /// Apply the name entry to the hostname entry if the hostname entry is empty.
    #[must_use]
    fn apply_name_to_empty_hostname(&self) -> Self;

    /// Merges the hosts with the same entries into one host.
    #[must_use]
    fn merge_same_hosts(&self) -> Self;

    /// Spreads the hosts with multiple patterns into multiple hosts with one pattern.
    #[must_use]
    fn spread(&self) -> Self;

    /// Apply patterns entries to non-pattern hosts and remove the pattern hosts.
    #[must_use]
    fn apply_patterns(&self) -> Self;
}

impl HostVecExt for Vec<Host> {
    fn apply_name_to_empty_hostname(&self) -> Self {
        let mut hosts = self.clone();

        for host in &mut hosts {
            if host.get(&EntryType::Hostname).is_none() {
                let name = host.patterns.first().unwrap().clone();
                host.update((EntryType::Hostname, name));
            }
        }

        hosts
    }

    fn merge_same_hosts(&self) -> Self {
        let mut hosts = self.clone();

        for i in (0..hosts.len()).rev() {
            let (left, right) = hosts.split_at_mut(i); // Split into left and right parts

            let current_host = &right[0];

            for j in (0..i).rev() {
                let target_host = &mut left[j];

                if current_host.entries != target_host.entries {
                    continue;
                }

                target_host.extend_patterns(current_host);
                target_host.extend_entries(current_host);
                hosts.remove(i);
                break;
            }
        }

        hosts
    }

    fn spread(&self) -> Vec<Host> {
        let mut hosts = Vec::new();

        for host in self {
            let patterns = host.get_patterns();
            if patterns.is_empty() {
                hosts.push(host.clone());
                continue;
            }

            for pattern in patterns {
                let mut new_host = host.clone();
                new_host.patterns = vec![pattern.clone()];
                hosts.push(new_host);
            }
        }

        hosts
    }

    /// Apply patterns entries to non-pattern hosts and remove the pattern hosts.
    ///
    /// You might want to call [`HostVecExt::merge_same_hosts`] after this.
    fn apply_patterns(&self) -> Self {
        let mut hosts = self.spread();
        let mut pattern_indexes = Vec::new();

        for i in 0..hosts.len() {
            let matching_pattern_regexes = hosts[i].matching_pattern_regexes();
            if matching_pattern_regexes.is_empty() {
                continue;
            }

            pattern_indexes.push(i);

            for j in 0..hosts.len() {
                if i == j {
                    continue;
                }

                if !hosts[j].matching_pattern_regexes().is_empty() {
                    continue;
                }

                for (regex, is_negated) in &matching_pattern_regexes {
                    if regex.is_match(&hosts[j].patterns[0]) == *is_negated {
                        continue;
                    }

                    let host = hosts[i].clone();
                    hosts[j].extend_if_not_contained(&host);
                    break;
                }
            }
        }

        for i in pattern_indexes.into_iter().rev() {
            hosts.remove(i);
        }

        hosts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_patterns() {
        let mut hosts = Vec::new();

        let mut host = Host::new(vec!["*".to_string()]);
        host.update((EntryType::Hostname, "example.com".to_string()));
        hosts.push(host);

        let mut host = Host::new(vec!["!example.com".to_string()]);
        host.update((EntryType::User, "hello".to_string()));
        hosts.push(host);

        let mut host = Host::new(vec!["example.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.push(host);

        let mut host = Host::new(vec!["hello.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.push(host);

        let hosts = hosts.apply_patterns();

        assert_eq!(hosts.len(), 2);

        assert_eq!(hosts[0].patterns.len(), 1);
        assert_eq!(hosts[0].patterns[0], "example.com");
        assert_eq!(hosts[0].entries.len(), 2);
        assert_eq!(hosts[0].entries[&EntryType::Hostname], "example.com");
        assert_eq!(hosts[0].entries[&EntryType::Port], "22");

        assert_eq!(hosts[1].patterns.len(), 1);
        assert_eq!(hosts[1].patterns[0], "hello.com");
        assert_eq!(hosts[1].entries.len(), 3);
        assert_eq!(hosts[1].entries[&EntryType::Hostname], "example.com");
        assert_eq!(hosts[1].entries[&EntryType::User], "hello");
        assert_eq!(hosts[1].entries[&EntryType::Port], "22");
    }

    #[test]
    fn test_spread() {
        let mut hosts = Vec::new();

        let mut host = Host::new(vec!["example.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.push(host);

        let mut host = Host::new(vec!["hello.com".to_string(), "world.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.push(host);

        let hosts = hosts.spread();

        assert_eq!(hosts.len(), 3);

        assert_eq!(hosts[0].patterns.len(), 1);
        assert_eq!(hosts[0].patterns[0], "example.com");
        assert_eq!(hosts[0].entries.len(), 1);
        assert_eq!(hosts[0].entries[&EntryType::Port], "22");

        assert_eq!(hosts[1].patterns.len(), 1);
        assert_eq!(hosts[1].patterns[0], "hello.com");
        assert_eq!(hosts[1].entries.len(), 1);
        assert_eq!(hosts[1].entries[&EntryType::Port], "22");

        assert_eq!(hosts[2].patterns.len(), 1);
        assert_eq!(hosts[2].patterns[0], "world.com");
        assert_eq!(hosts[2].entries.len(), 1);
        assert_eq!(hosts[2].entries[&EntryType::Port], "22");
    }

    #[test]
    fn test_merge_same_hosts() {
        let mut hosts = Vec::new();

        let mut host = Host::new(vec!["same1.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.push(host);

        let mut host = Host::new(vec!["same2.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.push(host);

        let hosts = hosts.merge_same_hosts();

        assert_eq!(hosts.len(), 3);

        assert_eq!(hosts[0].patterns.len(), 2);
        assert_eq!(hosts[0].patterns[0], "same1.com");
        assert_eq!(hosts[0].entries.len(), 1);
        assert_eq!(hosts[0].entries[&EntryType::Port], "22");
    }
}
