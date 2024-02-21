use regex::Regex;
use std::str::FromStr;
use std::{collections::HashMap, error::Error, io::BufRead};
use strum_macros;

/// List from <https://man7.org/linux/man-pages/man5/ssh_config.5.html>
#[derive(Debug, strum_macros::Display, strum_macros::EnumString, Eq, PartialEq, Hash, Clone)]
#[strum(ascii_case_insensitive)]
pub enum EntryType {
    #[strum(disabled)]
    Unknown(String),
    Host,
    Match,
    AddKeysToAgent,
    AddressFamily,
    BatchMode,
    BindAddress,
    BindInterface,
    CanonicalDomains,
    CanonicalizeFallbackLocal,
    CanonicalizeHostname,
    CanonicalizeMaxDots,
    CanonicalizePermittedCNAMEs,
    CASignatureAlgorithms,
    CertificateFile,
    ChannelTimeout,
    CheckHostIP,
    Ciphers,
    ClearAllForwardings,
    Compression,
    ConnectionAttempts,
    ConnectTimeout,
    ControlMaster,
    ControlPath,
    ControlPersist,
    DynamicForward,
    EnableEscapeCommandline,
    EnableSSHKeysign,
    EscapeChar,
    ExitOnForwardFailure,
    FingerprintHash,
    ForkAfterAuthentication,
    ForwardAgent,
    ForwardX11,
    ForwardX11Timeout,
    ForwardX11Trusted,
    GatewayPorts,
    GlobalKnownHostsFile,
    GSSAPIAuthentication,
    GSSAPIDelegateCredentials,
    HashKnownHosts,
    HostbasedAcceptedAlgorithms,
    HostbasedAuthentication,
    HostKeyAlgorithms,
    HostKeyAlias,
    Hostname,
    IdentitiesOnly,
    IdentityAgent,
    IdentityFile,
    IgnoreUnknown,
    Include,
    IPQoS,
    KbdInteractiveAuthentication,
    KbdInteractiveDevices,
    KexAlgorithms,
    KnownHostsCommand,
    LocalCommand,
    LocalForward,
    LogLevel,
    LogVerbose,
    MACs,
    NoHostAuthenticationForLocalhost,
    NumberOfPasswordPrompts,
    ObscureKeystrokeTiming,
    PasswordAuthentication,
    PermitLocalCommand,
    PermitRemoteOpen,
    PKCS11Provider,
    Port,
    PreferredAuthentications,
    ProxyCommand,
    ProxyJump,
    ProxyUseFdpass,
    PubkeyAcceptedAlgorithms,
    PubkeyAuthentication,
    RekeyLimit,
    RemoteCommand,
    RemoteForward,
    RequestTTY,
    RequiredRSASize,
    RevokedHostKeys,
    SecurityKeyProvider,
    SendEnv,
    ServerAliveCountMax,
    ServerAliveInterval,
    SessionType,
    SetEnv,
    StdinNull,
    StreamLocalBindMask,
    StreamLocalBindUnlink,
    StrictHostKeyChecking,
    SyslogFacility,
    TCPKeepAlive,
    Tag,
    Tunnel,
    TunnelDevice,
    UpdateHostKeys,
    User,
    UserKnownHostsFile,
    VerifyHostKeyDNS,
    VisualHostKey,
    XAuthLocation,
}

type Entry = (EntryType, String);

#[derive(Debug, Clone)]
pub struct Host {
    patterns: Vec<String>,
    entries: HashMap<EntryType, String>,
}

impl Host {
    fn new(patterns: Vec<String>) -> Host {
        Host {
            patterns,
            entries: HashMap::new(),
        }
    }

    fn update(&mut self, entry: Entry) {
        self.entries.insert(entry.0, entry.1);
    }

    fn extend(&mut self, host: &Host) {
        self.patterns.extend(host.patterns.clone());
        self.entries.extend(host.entries.clone());
    }

    fn extend_if_not_contained(&mut self, host: &Host) {
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

#[derive(Debug, Clone)]
pub struct Hosts(Vec<Host>);

impl Hosts {
    fn new() -> Hosts {
        Hosts(Vec::new())
    }

    fn add(&mut self, host: Host) {
        self.0.push(host);
    }

    fn last_mut(&mut self) -> &mut Host {
        self.0.last_mut().unwrap()
    }

    #[allow(clippy::must_use_candidate)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Host> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Host> {
        self.0.iter_mut()
    }

    pub fn merge_same_hosts(&mut self) -> &mut Self {
        for i in (0..self.0.len()).rev() {
            for j in (0..i).rev() {
                if self.0[i].entries != self.0[j].entries {
                    continue;
                }

                let host = self.0[i].clone();
                self.0[j].extend(&host);
                self.0.remove(i);
                break;
            }
        }

        self
    }

    pub fn spread(&mut self) -> &mut Self {
        let mut hosts = Vec::new();

        for host in &self.0 {
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

    pub fn apply_patterns(&mut self) -> &mut Self {
        let hosts = self.spread();
        let mut pattern_indexes = Vec::new();

        for i in 0..hosts.0.len() {
            let matching_pattern_regexes = hosts.0[i].matching_pattern_regexes();
            if matching_pattern_regexes.is_empty() {
                continue;
            }

            pattern_indexes.push(i);

            for j in (i + 1)..hosts.0.len() {
                if !hosts.0[j].matching_pattern_regexes().is_empty() {
                    continue;
                }

                for (regex, is_negated) in &matching_pattern_regexes {
                    if regex.is_match(&hosts.0[j].patterns[0]) == *is_negated {
                        continue;
                    }

                    let host = hosts.0[i].clone();
                    hosts.0[j].extend_if_not_contained(&host);
                    break;
                }
            }
        }

        for i in pattern_indexes.into_iter().rev() {
            hosts.0.remove(i);
        }

        hosts
    }
}

#[derive(Debug)]
pub struct Parser {
    ignore_unknown_entries: bool,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    #[must_use]
    pub fn new() -> Parser {
        Parser {
            ignore_unknown_entries: true,
        }
    }

    /// # Errors
    ///
    /// Will return `Err` if the SSH configuration cannot be parsed.
    pub fn parse(&self, reader: &mut impl BufRead) -> Result<Hosts, Box<dyn Error>> {
        let mut global_host = Host::new(Vec::new());
        let mut hosts = Hosts::new();

        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            let entry = parse_line(&line)?;
            line.clear();

            match entry.0 {
                EntryType::Unknown(_) => {
                    if !self.ignore_unknown_entries {
                        return Err(format!("Unknown entry: {line}").into());
                    }
                }
                EntryType::Host => {
                    let patterns = parse_patterns(&entry.1);
                    hosts.add(Host::new(patterns));

                    continue;
                }
                _ => {}
            }

            if hosts.is_empty() {
                global_host.update(entry);
            } else {
                hosts.last_mut().update(entry);
            }
        }

        if !global_host.is_empty() {
            for host in hosts.iter_mut() {
                host.extend_if_not_contained(&global_host);
            }
        }

        Ok(hosts)
    }
}

fn parse_line(line: &str) -> Result<Entry, Box<dyn Error>> {
    let (key, value) = line
        .trim()
        .split_once(' ')
        .map(|(k, v)| (k.trim_end(), v.trim_start()))
        .ok_or(format!("Invalid line: {line}"))?;

    Ok((
        EntryType::from_str(key).unwrap_or(EntryType::Unknown(key.to_string())),
        value.to_string(),
    ))
}

fn parse_patterns(entry_value: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    let mut pattern = String::new();
    let mut in_double_quotes = false;

    for c in entry_value.chars() {
        if c == '"' {
            if in_double_quotes {
                patterns.push(pattern.trim().to_string());
                pattern.clear();

                in_double_quotes = false;
            } else {
                in_double_quotes = true;
            }
        } else if c.is_whitespace() {
            if in_double_quotes {
                pattern.push(c);
            } else if !pattern.is_empty() {
                patterns.push(pattern.trim().to_string());
                pattern.clear();
            }
        } else {
            pattern.push(c);
        }
    }

    if !pattern.is_empty() {
        patterns.push(pattern.trim().to_string());
    }

    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_patterns() {
        let mut hosts = Hosts::new();

        let mut host = Host::new(vec!["*".to_string()]);
        host.update((EntryType::Hostname, "example.com".to_string()));
        hosts.add(host);

        let mut host = Host::new(vec!["!example.com".to_string()]);
        host.update((EntryType::User, "hello".to_string()));
        hosts.add(host);

        let mut host = Host::new(vec!["example.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.add(host);

        let mut host = Host::new(vec!["hello.com".to_string()]);
        host.update((EntryType::Port, "22".to_string()));
        hosts.add(host);

        let hosts = hosts.apply_patterns();

        assert_eq!(hosts.0.len(), 2);

        assert_eq!(hosts.0[0].patterns[0], "example.com");
        assert_eq!(hosts.0[0].entries.len(), 2);
        assert_eq!(hosts.0[0].entries[&EntryType::Hostname], "example.com");
        assert_eq!(hosts.0[0].entries[&EntryType::Port], "22");

        assert_eq!(hosts.0[1].patterns[0], "hello.com");
        assert_eq!(hosts.0[1].entries.len(), 3);
        assert_eq!(hosts.0[1].entries[&EntryType::Hostname], "example.com");
        assert_eq!(hosts.0[1].entries[&EntryType::User], "hello");
        assert_eq!(hosts.0[1].entries[&EntryType::Port], "22");
    }
}
