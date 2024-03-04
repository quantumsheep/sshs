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
    fn apply_name_to_empty_hostname(&mut self) -> &mut Self;

    /// Merges the hosts with the same entries into one host.
    fn merge_same_hosts(&mut self) -> &mut Self;

    /// Spreads the hosts with multiple patterns into multiple hosts with one pattern.
    fn spread(&mut self) -> &mut Self;

    /// Apply patterns entries to non-pattern hosts and remove the pattern hosts.
    fn apply_patterns(&mut self) -> &mut Self;
}

impl HostVecExt for Vec<Host> {
    fn apply_name_to_empty_hostname(&mut self) -> &mut Self {
        for host in self.iter_mut() {
            if host.get(&EntryType::Hostname).is_none() {
                let name = host.patterns.first().unwrap().clone();
                host.update((EntryType::Hostname, name.clone()));
            }
        }

        self
    }

    fn merge_same_hosts(&mut self) -> &mut Self {
        for i in (0..self.len()).rev() {
            for j in (0..i).rev() {
                if self[i].entries != self[j].entries {
                    continue;
                }

                let host = self[i].clone();
                self[j].extend_patterns(&host);
                self[j].extend_entries(&host);
                self.remove(i);
                break;
            }
        }

        self
    }

    fn spread(&mut self) -> &mut Self {
        let mut hosts = Vec::new();

        for host in self.iter_mut() {
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

        self
    }

    /// Apply patterns entries to non-pattern hosts and remove the pattern hosts.
    ///
    /// You might want to call [`HostVecExt::merge_same_hosts`] after this.
    fn apply_patterns(&mut self) -> &mut Self {
        let hosts = self.spread();
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

        assert_eq!(hosts[0].patterns[0], "example.com");
        assert_eq!(hosts[0].entries.len(), 2);
        assert_eq!(hosts[0].entries[&EntryType::Hostname], "example.com");
        assert_eq!(hosts[0].entries[&EntryType::Port], "22");

        assert_eq!(hosts[1].patterns[0], "hello.com");
        assert_eq!(hosts[1].entries.len(), 3);
        assert_eq!(hosts[1].entries[&EntryType::Hostname], "example.com");
        assert_eq!(hosts[1].entries[&EntryType::User], "hello");
        assert_eq!(hosts[1].entries[&EntryType::Port], "22");
    }
}
