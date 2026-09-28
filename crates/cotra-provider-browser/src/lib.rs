use cotra_contracts::FailureCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};

pub const BROWSER_PROFILE_SCHEMA: &str = "cotra-browser-profile-v1";
pub const PROFILE_MARKER_FILE: &str = "COTRA_AUTOMATION_PROFILE";
const PROFILE_MARKER_BODY: &str =
    "cotra-browser-profile-v1\nisolated\nno-personal-data\nno-credential-import\n";

const MAX_URL_BYTES: usize = 2048;
const MAX_HOST_BYTES: usize = 253;

/// Typed denial catalog for browser actuation shapes that SG-000021
/// deliberately leaves unauthorized. Navigation, DOM actuation, downloads,
/// uploads, personal-profile access, debugging, scripting, and network egress
/// remain absent; every shape listed here must fail closed.
pub const DENIED_BROWSER_SHAPES: &[(&str, &str)] = &[
    ("browser.navigate", "navigate"),
    ("browser.snapshot", "capture"),
    ("browser.snapshot", "actuate"),
    ("browser.dom", "click"),
    ("browser.dom", "fill"),
    ("browser.dom", "type"),
    ("browser.dom", "press"),
    ("browser.dom", "select"),
    ("browser.dom", "write"),
    ("browser.download", "download"),
    ("browser.upload", "upload"),
    ("browser.profile", "use_personal"),
    ("browser.profile", "attach"),
    ("browser.profile", "launch"),
    ("browser.debug", "attach"),
    ("browser.devtools", "command"),
    ("browser.cdp", "command"),
    ("browser.script", "evaluate"),
    ("browser.launch", "launch"),
    ("browser.attach", "attach"),
    ("browser.clear", "clear_profile"),
    ("browser.external_request", "fetch"),
    ("browser.network", "fetch"),
    ("devtools", "command"),
    ("cdp", "command"),
    ("playwright", "command"),
];

/// The two typed operations SG-000021 authorizes. Everything else under a
/// browser-like capability must fail closed.
pub fn is_allowed_browser_shape(capability: &str, operation: &str) -> bool {
    matches!(
        (capability, operation),
        ("browser.profile", "status") | ("browser.destination", "validate")
    )
}

/// Returns true when a request targets browser-like authority outside the two
/// allowed SG-000021 shapes. The policy layer denies these shapes; the
/// dispatch layer treats them as unreachable defense in depth.
pub fn is_denied_browser_shape(capability: &str, operation: &str) -> bool {
    if is_allowed_browser_shape(capability, operation) {
        return false;
    }
    if capability == "browser"
        || capability.starts_with("browser.")
        || capability.starts_with("devtools.")
        || capability.starts_with("cdp.")
        || capability.starts_with("playwright.")
        || capability == "devtools"
        || capability == "cdp"
        || capability == "playwright"
    {
        return true;
    }
    DENIED_BROWSER_SHAPES.contains(&(capability, operation))
}

#[derive(Debug)]
pub struct ProviderError {
    pub code: FailureCode,
    pub message: String,
}

impl ProviderError {
    pub fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new(FailureCode::InvalidRequest, message)
    }

    fn denied(message: impl Into<String>) -> Self {
        Self::new(FailureCode::CapabilityDenied, message)
    }
}

/// Canonical bound origin: exact scheme, host, and port with a normalized
/// `scheme://host:port` identity. Origins are compared by this normalized
/// identity, never by naive string prefix logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserOrigin {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub origin: String,
}

/// A validated browser destination: exact origin binding plus the
/// deterministically pinned post-resolution address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDestination {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub origin: String,
    pub pinned_address: IpAddr,
}

impl ValidatedDestination {
    pub fn to_json(&self) -> Value {
        json!({
            "scheme": self.scheme,
            "host": self.host,
            "port": self.port,
            "origin": self.origin,
            "pinned_address": self.pinned_address.to_string(),
        })
    }
}

pub trait DnsResolver {
    fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>, ProviderError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemResolver;

impl DnsResolver for SystemResolver {
    fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>, ProviderError> {
        let mut addresses = Vec::new();
        let candidates = (host, port).to_socket_addrs().map_err(|error| {
            ProviderError::new(
                FailureCode::ProviderUnavailable,
                format!("resolve browser destination host: {error}"),
            )
        })?;
        for candidate in candidates {
            let address = candidate.ip();
            if !addresses.contains(&address) {
                addresses.push(address);
            }
        }
        if addresses.is_empty() {
            return Err(ProviderError::new(
                FailureCode::ProviderUnavailable,
                "browser destination host resolved to no addresses",
            ));
        }
        addresses.sort_by_key(|address| address.to_string());
        Ok(addresses)
    }
}

/// Parse a destination URL and bind its exact origin. This performs no network
/// activity: it validates scheme, host, and port shape only. Address policy is
/// applied after DNS resolution by [`validate_destination`].
pub fn parse_destination_url(url: &str) -> Result<BrowserOrigin, ProviderError> {
    if url.is_empty() || url.len() > MAX_URL_BYTES {
        return Err(ProviderError::invalid(
            "browser destination URL is empty or too large",
        ));
    }
    if url
        .bytes()
        .any(|byte| matches!(byte, 0 | b'\n' | b'\r' | b'\t' | b' ' | 0x0b | 0x0c | 0x7f))
    {
        return Err(ProviderError::invalid(
            "browser destination URL contains unsafe whitespace or control data",
        ));
    }
    let (raw_scheme, rest) = url.split_once("://").ok_or_else(|| {
        ProviderError::invalid("browser destination URL must contain a scheme separator")
    })?;
    let scheme = raw_scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(ProviderError::invalid(
            "browser destination URL must use the http or https scheme",
        ));
    }
    if rest.is_empty() {
        return Err(ProviderError::invalid(
            "browser destination URL is missing a host",
        ));
    }
    if rest.contains('@') {
        return Err(ProviderError::invalid(
            "browser destination URL must not contain userinfo",
        ));
    }
    if rest.contains('\\') {
        return Err(ProviderError::invalid(
            "browser destination URL must not contain a backslash",
        ));
    }
    if rest.contains('#') {
        return Err(ProviderError::invalid(
            "browser destination URL must not contain a fragment",
        ));
    }
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let path = &rest[authority_end..];
    if authority.is_empty() {
        return Err(ProviderError::invalid(
            "browser destination URL is missing a host",
        ));
    }
    if !path.is_empty() && !path.starts_with('/') {
        return Err(ProviderError::invalid(
            "browser destination URL path is invalid",
        ));
    }
    if path.contains("/..") || path.contains('\\') {
        return Err(ProviderError::invalid(
            "browser destination URL path contains an unsafe sequence",
        ));
    }
    let (host, port) = split_authority(authority, &scheme)?;
    let host = normalize_host(&host)?;
    Ok(BrowserOrigin {
        origin: format!("{scheme}://{host}:{port}"),
        scheme,
        host,
        port,
    })
}

fn split_authority(authority: &str, scheme: &str) -> Result<(String, u16), ProviderError> {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let end = bracketed.find(']').ok_or_else(|| {
            ProviderError::invalid("browser destination IPv6 host is missing a closing bracket")
        })?;
        let host = &bracketed[..end];
        let remainder = &bracketed[end + 1..];
        if host.is_empty() {
            return Err(ProviderError::invalid("browser destination host is empty"));
        }
        let port = match remainder.strip_prefix(':') {
            Some(text) => parse_port(text)?,
            None => {
                if !remainder.is_empty() {
                    return Err(ProviderError::invalid(
                        "browser destination authority is invalid",
                    ));
                }
                default_port(scheme)
            }
        };
        return Ok((host.to_owned(), port));
    }
    match authority.rfind(':') {
        Some(index) => {
            let host = &authority[..index];
            let port_text = &authority[index + 1..];
            if host.is_empty() {
                return Err(ProviderError::invalid("browser destination host is empty"));
            }
            if port_text.is_empty() || !port_text.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(ProviderError::invalid(
                    "browser destination authority has an invalid port; IPv6 literals must use brackets",
                ));
            }
            Ok((host.to_owned(), parse_port(port_text)?))
        }
        None => {
            if authority.is_empty() {
                return Err(ProviderError::invalid("browser destination host is empty"));
            }
            Ok((authority.to_owned(), default_port(scheme)))
        }
    }
}

fn default_port(scheme: &str) -> u16 {
    if scheme == "http" {
        80
    } else {
        443
    }
}

fn parse_port(text: &str) -> Result<u16, ProviderError> {
    if text.is_empty() || text.len() > 5 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ProviderError::invalid(
            "browser destination port must be numeric",
        ));
    }
    let port: u32 = text
        .parse()
        .map_err(|_| ProviderError::invalid("browser destination port is invalid"))?;
    if port == 0 || port > 65_535 {
        return Err(ProviderError::invalid(
            "browser destination port is out of range",
        ));
    }
    Ok(port as u16)
}

/// Normalize a host to its canonical lowercase identity while rejecting
/// ambiguous or unsafe representations: wildcards, userinfo remnants, zone
/// identifiers, hex/octal/integer IPv4 disguises, localhost aliases, and
/// malformed DNS names. Comparison-resistant origin identity depends on this
/// function accepting exactly one spelling per host.
fn normalize_host(raw: &str) -> Result<String, ProviderError> {
    if raw.is_empty() || raw.len() > MAX_HOST_BYTES {
        return Err(ProviderError::invalid(
            "browser destination host is empty or too large",
        ));
    }
    if raw.contains('*') {
        return Err(ProviderError::invalid(
            "browser destination host must not contain a wildcard",
        ));
    }
    if raw.contains('%') {
        return Err(ProviderError::invalid(
            "browser destination host must not contain a zone identifier",
        ));
    }
    if raw.contains('@') || raw.contains('/') || raw.contains('\\') || raw.contains('?') {
        return Err(ProviderError::invalid(
            "browser destination host contains unsafe characters",
        ));
    }
    if raw.ends_with('.') {
        return Err(ProviderError::invalid(
            "browser destination host must not use a trailing dot",
        ));
    }
    let host = raw.to_ascii_lowercase();
    reject_localhost_alias(&host)?;
    reject_alternate_numeric_host(&host)?;
    if host.parse::<IpAddr>().is_ok() {
        return Ok(host);
    }
    validate_dns_hostname(&host)?;
    Ok(host)
}

fn reject_localhost_alias(host: &str) -> Result<(), ProviderError> {
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".lan")
        || host.ends_with(".home")
    {
        return Err(ProviderError::denied(
            "browser destination localhost alias is denied",
        ));
    }
    Ok(())
}

/// Reject alternate numeric host syntaxes that naive parsers may interpret as
/// loopback or private addresses: hexadecimal (`0x7f.0.0.1`), octal
/// (`0177.0.0.1`), and bare-integer (`2130706433`) forms. Standard dotted
/// decimal literals pass through to address policy instead.
fn reject_alternate_numeric_host(host: &str) -> Result<(), ProviderError> {
    if host.contains("0x") || host.contains("0X") {
        return Err(ProviderError::invalid(
            "browser destination host uses an alternate numeric representation",
        ));
    }
    if !host.contains('.') && host.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ProviderError::invalid(
            "browser destination host uses an alternate numeric representation",
        ));
    }
    if host.contains('.')
        && host
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        let all_plain_decimal = host
            .split('.')
            .all(|part| (part.len() == 1 || !part.starts_with('0')) && part.parse::<u8>().is_ok());
        if !all_plain_decimal {
            return Err(ProviderError::invalid(
                "browser destination host uses an alternate numeric representation",
            ));
        }
    }
    Ok(())
}

fn validate_dns_hostname(host: &str) -> Result<(), ProviderError> {
    if host.starts_with('-') || host.starts_with('.') || host.ends_with('-') || host.ends_with('.')
    {
        return Err(ProviderError::invalid(
            "browser destination hostname has an unsafe leading or trailing character",
        ));
    }
    if host.contains("..") {
        return Err(ProviderError::invalid(
            "browser destination hostname contains an empty label",
        ));
    }
    if !host.contains('.') {
        return Err(ProviderError::invalid(
            "browser destination hostname must be a dotted DNS hostname",
        ));
    }
    let mut has_alpha = false;
    for label in host.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(ProviderError::invalid(
                "browser destination hostname label is empty or too large",
            ));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(ProviderError::invalid(
                "browser destination hostname label has an unsafe hyphen",
            ));
        }
        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(ProviderError::invalid(
                "browser destination hostname label uses unsafe characters",
            ));
        }
        has_alpha = has_alpha || label.bytes().any(|byte| byte.is_ascii_alphabetic());
    }
    if !has_alpha {
        return Err(ProviderError::invalid(
            "browser destination hostname must contain at least one letter",
        ));
    }
    Ok(())
}

/// Validate a destination URL end to end: bind the exact origin, re-resolve
/// DNS through the provided resolver, and apply post-resolution address
/// policy. Non-public addresses fail closed unless a future explicitly
/// trusted workspace destination allows them; SG-000021 configures no such
/// trust, so every non-public destination is denied.
pub fn validate_destination(
    url: &str,
    resolver: &impl DnsResolver,
) -> Result<ValidatedDestination, ProviderError> {
    let origin = parse_destination_url(url)?;
    let resolved = resolver.resolve(&origin.host, origin.port)?;
    let pinned = select_pinned_address(&resolved)?;
    Ok(ValidatedDestination {
        scheme: origin.scheme,
        host: origin.host,
        port: origin.port,
        origin: origin.origin,
        pinned_address: pinned,
    })
}

/// Validate a redirect target against an already validated base origin. Any
/// target whose exact origin differs from the base fails closed: a permitted
/// public origin can never silently widen into broader authority.
pub fn validate_redirect(
    base_origin: &str,
    target_url: &str,
    resolver: &impl DnsResolver,
) -> Result<ValidatedDestination, ProviderError> {
    let base = parse_destination_url(base_origin)?;
    let target = parse_destination_url(target_url)?;
    if target.origin != base.origin {
        return Err(ProviderError::denied(format!(
            "browser redirect target origin {} does not match validated origin {}; redirect widening is denied",
            target.origin, base.origin
        )));
    }
    validate_destination(target_url, resolver)
}

pub fn is_public_address(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(value) => is_public_ipv4(value),
        IpAddr::V6(value) => is_public_ipv6(value),
    }
}

fn is_public_ipv4(value: &Ipv4Addr) -> bool {
    let octets = value.octets();
    if value.is_loopback()
        || value.is_unspecified()
        || value.is_multicast()
        || value.is_link_local()
        || value.is_private()
    {
        return false;
    }
    if value.is_broadcast() || value.is_documentation() {
        return false;
    }
    if octets[0] == 0 {
        return false;
    }
    if octets[0] == 100 && (octets[1] & 0b1100_0000) == 64 {
        return false;
    }
    if octets[0] == 192 && octets[1] == 0 && (octets[2] == 0 || octets[2] == 2) {
        return false;
    }
    if octets[0] == 192 && octets[1] == 88 && octets[2] == 99 {
        return false;
    }
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return false;
    }
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return false;
    }
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return false;
    }
    if octets[0] >= 240 {
        return false;
    }
    true
}

fn is_public_ipv6(value: &Ipv6Addr) -> bool {
    if value.is_loopback() || value.is_unspecified() || value.is_multicast() {
        return false;
    }
    let segments = value.segments();
    if (segments[0] & 0xffc0) == 0xfe80 {
        return false;
    }
    if (segments[0] & 0xfe00) == 0xfc00 {
        return false;
    }
    if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        return false;
    }
    if segments[0] == 0x2001 && segments[1] == 0x0002 {
        return false;
    }
    if segments[0] == 0x2001 && segments[1] == 0x0001 {
        return false;
    }
    if segments[0] == 0x2002 {
        return false;
    }
    if segments[0] == 0x0064 && (segments[1] & 0xffc0) == 0xff00 {
        return false;
    }
    if value.octets()[0] == 0 && value.octets()[1] == 0 {
        return false;
    }
    if let Some(mapped) = value.to_ipv4_mapped() {
        return is_public_ipv4(&mapped);
    }
    if value.to_ipv4().is_some() {
        return false;
    }
    true
}

pub fn select_pinned_address(addresses: &[IpAddr]) -> Result<IpAddr, ProviderError> {
    if addresses.is_empty() {
        return Err(ProviderError::new(
            FailureCode::ProviderUnavailable,
            "browser destination host resolved to no addresses",
        ));
    }
    let mut public: Vec<IpAddr> = addresses
        .iter()
        .copied()
        .filter(is_public_address)
        .collect();
    if public.is_empty() {
        return Err(ProviderError::denied(
            "browser destination resolved only to non-public addresses; destination is denied",
        ));
    }
    public.sort_by_key(|address| address.to_string());
    public.dedup();
    Ok(public[0])
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn profile_identity(root: &Path) -> String {
    sha256_hex(format!("{BROWSER_PROFILE_SCHEMA}\n{}", root.to_string_lossy()).as_bytes())
}

/// Canonical root for the isolated Cotra automation browser profile. Tests
/// override it with `COTRA_BROWSER_STATE_DIR`; production defaults keep the
/// profile under Cotra protected local state, never inside a workspace and
/// never inside a personal browser directory.
pub fn default_profile_root() -> PathBuf {
    if let Some(path) = std::env::var_os("COTRA_BROWSER_STATE_DIR") {
        let path = PathBuf::from(path);
        if path.ends_with("browser-profile") {
            return path;
        }
        return path.join("browser-profile");
    }
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join("Cotra")
            .join("browser-profile");
    }
    std::env::temp_dir().join("cotra").join("browser-profile")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserProfile {
    pub root: PathBuf,
    pub identity: String,
    pub fresh_storage: bool,
    pub isolated: bool,
}

impl BrowserProfile {
    pub fn status_json(&self, workspace_id: &str, policy_revision: &str) -> Value {
        json!({
            "profile_root": self.root.to_string_lossy(),
            "profile_identity": self.identity,
            "fresh_storage": self.fresh_storage,
            "isolated": self.isolated,
            "personal_data": false,
            "workspace_id": workspace_id,
            "policy_revision": policy_revision,
        })
    }
}

/// Directory or file names that indicate a personal browser profile. Matching
/// is case-insensitive on whole path components, so the Cotra automation
/// profile directory itself (`browser-profile`) never matches.
fn is_personal_component(component: &str) -> bool {
    matches!(
        component,
        "user data"
            | "user-data"
            | "google"
            | "chrome"
            | "google-chrome"
            | "chromium"
            | "microsoft edge"
            | "edge"
            | "edgemicrosoft"
            | "brave"
            | "brave-browser"
            | "vivaldi"
            | "opera"
            | "firefox"
            | "mozilla"
            | "profiles"
            | "profile"
            | "login data"
            | "cookies"
            | "web data"
            | "secure preferences"
            | "places.sqlite"
            | "logins.json"
    )
}

/// Returns true when a path reaches into a personal browser profile. Used to
/// fail closed before any profile directory is created or touched. Matching
/// splits on both path separators so Windows-style personal paths are
/// recognized on every platform.
pub fn is_personal_profile_path(path: &Path) -> bool {
    for component in path.components() {
        let text = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        for piece in text.split(['/', '\\']) {
            if is_personal_component(piece.trim()) {
                return true;
            }
        }
    }
    false
}

fn reject_unsafe_profile_root(root: &Path) -> Result<(), ProviderError> {
    if is_personal_profile_path(root) {
        return Err(ProviderError::denied(
            "browser profile root reaches into a personal browser profile; personal-profile mode is denied",
        ));
    }
    if !root.is_absolute() {
        return Err(ProviderError::invalid(
            "browser profile root must be an absolute path",
        ));
    }
    let portable = root.to_string_lossy().replace('/', "\\");
    if portable.starts_with("\\\\") && !portable.starts_with("\\\\?\\") {
        return Err(ProviderError::denied(
            "browser profile root cannot use UNC or device namespaces",
        ));
    }
    for component in root.components() {
        match component {
            Component::ParentDir => {
                return Err(ProviderError::invalid(
                    "browser profile root must not contain parent components",
                ));
            }
            Component::Prefix(prefix) => match prefix.kind() {
                std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_) => {}
                _ => {
                    return Err(ProviderError::denied(
                        "browser profile root cannot use UNC or device namespaces",
                    ));
                }
            },
            Component::Normal(_) | Component::RootDir | Component::CurDir => {}
        }
    }
    Ok(())
}

/// Create or open the dedicated isolated automation profile. The profile
/// carries fresh Cotra-owned storage only: no personal cookies, passwords,
/// sessions, extensions, or history are ever imported, and caller-selected
/// profile directories are unreachable because dispatch never accepts one.
pub fn ensure_isolated_profile(root: &Path) -> Result<BrowserProfile, ProviderError> {
    reject_unsafe_profile_root(root)?;
    let existed_before = root.is_dir();
    std::fs::create_dir_all(root).map_err(|error| {
        ProviderError::new(
            FailureCode::ProviderUnavailable,
            format!("create isolated browser profile: {error}"),
        )
    })?;
    let canonical = std::fs::canonicalize(root).map_err(|error| {
        ProviderError::new(
            FailureCode::ProviderUnavailable,
            format!("resolve isolated browser profile: {error}"),
        )
    })?;
    reject_unsafe_profile_root(&canonical)?;
    let marker = canonical.join(PROFILE_MARKER_FILE);
    let mut fresh_storage = !existed_before;
    if marker.is_file() {
        let body = std::fs::read_to_string(&marker).map_err(|error| {
            ProviderError::new(
                FailureCode::ProviderUnavailable,
                format!("read browser profile marker: {error}"),
            )
        })?;
        if body != PROFILE_MARKER_BODY {
            return Err(ProviderError::new(
                FailureCode::TargetStale,
                "browser profile storage is not a Cotra automation profile; refusing to reuse it",
            ));
        }
        fresh_storage = false;
    } else {
        if marker.exists() {
            return Err(ProviderError::new(
                FailureCode::TargetStale,
                "browser profile marker path is obstructed; refusing to reuse it",
            ));
        }
        std::fs::write(&marker, PROFILE_MARKER_BODY).map_err(|error| {
            ProviderError::new(
                FailureCode::ProviderUnavailable,
                format!("initialize isolated browser profile: {error}"),
            )
        })?;
    }
    Ok(BrowserProfile {
        identity: profile_identity(&canonical),
        root: canonical,
        fresh_storage,
        isolated: true,
    })
}

/// Caller-selected profile directories are never honored. This function
/// exists so policy and dispatch share one fail-closed denial value.
pub fn reject_caller_profile_root() -> ProviderError {
    ProviderError::denied(
        "browser profile root is owned by Cotra protected state; caller-selected profile directories are denied",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct StaticResolver {
        addresses: Vec<IpAddr>,
    }

    impl DnsResolver for StaticResolver {
        fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>, ProviderError> {
            Ok(self.addresses.clone())
        }
    }

    fn public_resolver() -> StaticResolver {
        StaticResolver {
            addresses: vec!["93.184.216.34".parse().unwrap()],
        }
    }

    fn temp_profile_root(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cotra-browser-profile-{label}-{}-{suffix}",
            std::process::id()
        ))
    }

    #[test]
    fn allowed_shapes_are_exactly_profile_status_and_destination_validate() {
        assert!(is_allowed_browser_shape("browser.profile", "status"));
        assert!(is_allowed_browser_shape("browser.destination", "validate"));
        for (capability, operation) in DENIED_BROWSER_SHAPES {
            assert!(
                !is_allowed_browser_shape(capability, operation),
                "{capability}/{operation} must not be allowed"
            );
            assert!(
                is_denied_browser_shape(capability, operation),
                "{capability}/{operation} must be denied"
            );
        }
    }

    #[test]
    fn unknown_browser_operations_fail_closed() {
        for (capability, operation) in [
            ("browser.navigate", "navigate"),
            ("browser.dom", "click"),
            ("browser.dom", "fill"),
            ("browser.download", "download"),
            ("browser.upload", "upload"),
            ("browser.profile", "use_personal"),
            ("browser.debug", "attach"),
            ("browser.script", "evaluate"),
            ("browser.cdp", "command"),
            ("browser.unknown", "unknown"),
            ("devtools", "command"),
            ("cdp", "command"),
            ("playwright", "command"),
        ] {
            assert!(
                is_denied_browser_shape(capability, operation),
                "{capability}/{operation} must fail closed"
            );
        }
        assert!(!is_denied_browser_shape("fs.read", "read"));
        assert!(!is_denied_browser_shape("process.spawn", "spawn"));
    }

    #[test]
    fn origin_binding_normalizes_scheme_host_and_port() {
        let origin = parse_destination_url("https://Example.COM/docs").expect("valid");
        assert_eq!(origin.scheme, "https");
        assert_eq!(origin.host, "example.com");
        assert_eq!(origin.port, 443);
        assert_eq!(origin.origin, "https://example.com:443");

        let http = parse_destination_url("http://example.com/").expect("valid");
        assert_eq!(http.port, 80);
        assert_eq!(http.origin, "http://example.com:80");

        let explicit = parse_destination_url("https://example.com:443/a").expect("valid");
        assert_eq!(explicit.origin, "https://example.com:443");

        let custom = parse_destination_url("https://example.com:8443/a").expect("valid");
        assert_eq!(custom.port, 8443);
        assert_eq!(custom.origin, "https://example.com:8443");
        assert_ne!(custom.origin, explicit.origin);
    }

    #[test]
    fn origin_parsing_rejects_userinfo_fragment_and_malformed_hosts() {
        for bad in [
            "https://user@example.com/",
            "https://user:pass@example.com/",
            "https://example.com/page#section",
            "https://*.example.com/",
            "https://*example.com/",
            "https://exam ple.com/",
            "https://example.com\\evil",
            "https://example.com./",
            "https://[::1/",
            "https://::1/",
            "https://example.com:0/",
            "https://example.com:99999/",
            "https://example.com:/",
            "gopher://example.com/",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://",
            "https:///path",
            "",
        ] {
            assert!(
                parse_destination_url(bad).is_err(),
                "must reject destination {bad:?}"
            );
        }
    }

    #[test]
    fn alternate_numeric_hosts_are_rejected_before_address_policy() {
        for bad in [
            "http://0x7f.0.0.1/",
            "http://0X7F.0.0.1/",
            "http://0177.0.0.1/",
            "http://2130706433/",
            "http://0x7f000001/",
            "http://3232235521/",
            "http://0.0.0.0/",
        ] {
            let result = parse_destination_url(bad);
            if bad == "http://0.0.0.0/" {
                let origin = result.expect("dotted decimal parses");
                assert_eq!(origin.host, "0.0.0.0");
            } else {
                assert!(
                    result.is_err(),
                    "must reject alternate numeric host {bad:?}"
                );
            }
        }
    }

    #[test]
    fn localhost_aliases_are_denied() {
        for bad in [
            "http://localhost/",
            "http://localhost:3000/",
            "http://app.localhost/",
            "http://printer.local/",
            "http://service.internal/",
            "http://nas.lan/",
            "http://router.home/",
        ] {
            let error = parse_destination_url(bad).expect_err("localhost alias must fail");
            assert_eq!(error.code, FailureCode::CapabilityDenied, "{bad:?}");
        }
    }

    #[test]
    fn literal_public_addresses_parse_and_survive_policy() {
        let origin = parse_destination_url("http://93.184.216.34/").expect("literal parses");
        assert_eq!(origin.host, "93.184.216.34");
        let validated =
            validate_destination("http://93.184.216.34/", &public_resolver()).expect("public");
        assert_eq!(validated.pinned_address.to_string(), "93.184.216.34");

        let v6 = parse_destination_url("https://[2606:2800:220:1:248:1893:25c8:1946]/")
            .expect("ipv6 parses");
        assert_eq!(v6.host, "2606:2800:220:1:248:1893:25c8:1946");
    }

    #[test]
    fn ssrf_addresses_fail_closed_after_resolution() {
        let loopback_v4: IpAddr = "127.0.0.1".parse().unwrap();
        let loopback_v6: IpAddr = "::1".parse().unwrap();
        let private: IpAddr = "10.0.0.1".parse().unwrap();
        let link_local: IpAddr = "169.254.169.254".parse().unwrap();
        let unspecified: IpAddr = "0.0.0.0".parse().unwrap();
        let multicast: IpAddr = "224.0.0.1".parse().unwrap();
        let mapped: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        for address in [
            loopback_v4,
            loopback_v6,
            private,
            link_local,
            unspecified,
            multicast,
            mapped,
        ] {
            assert!(
                !is_public_address(&address),
                "SSRF address {address} must be non-public"
            );
            let resolver = StaticResolver {
                addresses: vec![address],
            };
            let error = validate_destination("https://example.com/", &resolver)
                .expect_err("SSRF must fail closed");
            assert_eq!(error.code, FailureCode::CapabilityDenied, "{address}");
        }
    }

    #[test]
    fn pinned_selection_is_deterministic_and_requires_public() {
        let private: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(select_pinned_address(&[]).is_err());
        assert!(select_pinned_address(&[private]).is_err());
        let first: IpAddr = "8.8.8.8".parse().unwrap();
        let second: IpAddr = "1.1.1.1".parse().unwrap();
        let pinned = select_pinned_address(&[first, second, private]).expect("pinned");
        assert_eq!(pinned, second);
        assert_eq!(select_pinned_address(&[first, second]).unwrap(), pinned);
    }

    #[test]
    fn redirect_widening_fails_closed() {
        let resolver = public_resolver();
        let same = validate_redirect(
            "https://example.com/",
            "https://example.com/other-path?q=1",
            &resolver,
        )
        .expect("same-origin redirect validates");
        assert_eq!(same.origin, "https://example.com:443");

        for target in [
            "https://evil.example.com/",
            "https://example.com.evil.com/",
            "https://example.com:8443/",
            "http://example.com/",
            "https://example.co/",
        ] {
            let error = validate_redirect("https://example.com/", target, &resolver)
                .expect_err("widening must fail");
            assert_eq!(error.code, FailureCode::CapabilityDenied, "{target}");
        }

        let prefix_trap = validate_redirect(
            "https://example.com/",
            "https://example.com.evil.com/",
            &resolver,
        );
        assert!(
            prefix_trap.is_err(),
            "prefix trap must not compare by prefix"
        );
    }

    #[test]
    fn isolated_profile_has_deterministic_identity_and_no_personal_data() {
        let root = temp_profile_root("isolated");
        let profile = ensure_isolated_profile(&root).expect("profile");
        assert!(profile.isolated);
        assert!(profile.root.is_absolute());
        assert!(marker_body(&profile.root).contains("no-personal-data"));
        let reopened = ensure_isolated_profile(&root).expect("reopen");
        assert_eq!(profile.identity, reopened.identity);
        assert!(!reopened.fresh_storage);
        let status = profile.status_json("default", "sg-000021-v1");
        assert_eq!(status["isolated"], true);
        assert_eq!(status["personal_data"], false);
        assert_eq!(status["workspace_id"], "default");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn personal_profile_paths_are_denied_before_touching_disk() {
        for candidate in [
            "C:\\Users\\Owner\\AppData\\Local\\Google\\Chrome\\User Data\\Default",
            "C:\\Users\\Owner\\AppData\\Local\\Microsoft\\Edge\\User Data",
            "/home/owner/.config/google-chrome/Default",
            "/home/owner/.mozilla/firefox/profiles/abc123",
        ] {
            let path = PathBuf::from(candidate);
            assert!(
                is_personal_profile_path(&path),
                "{candidate:?} must be recognized as personal"
            );
            let error =
                ensure_isolated_profile(&path).expect_err("personal profile must be denied");
            assert_eq!(error.code, FailureCode::CapabilityDenied, "{candidate:?}");
        }
        assert!(!is_personal_profile_path(&PathBuf::from(
            "C:\\ProgramData\\Cotra\\browser-profile"
        )));
    }

    #[test]
    fn caller_profile_roots_and_traversal_are_denied() {
        assert_eq!(
            reject_caller_profile_root().code,
            FailureCode::CapabilityDenied
        );
        let traversal = std::env::temp_dir().join("..").join("escape-profile");
        assert!(ensure_isolated_profile(&traversal).is_err());
        let relative = PathBuf::from("relative/profile");
        assert!(ensure_isolated_profile(&relative).is_err());
    }

    #[test]
    fn foreign_marker_content_is_rejected() {
        let root = temp_profile_root("foreign");
        std::fs::create_dir_all(&root).expect("seed dir");
        std::fs::write(root.join(PROFILE_MARKER_FILE), "foreign browser data")
            .expect("seed marker");
        let error = ensure_isolated_profile(&root).expect_err("foreign storage must fail");
        assert_eq!(error.code, FailureCode::TargetStale);
        let _ = std::fs::remove_dir_all(root);
    }

    fn marker_body(root: &Path) -> String {
        std::fs::read_to_string(root.join(PROFILE_MARKER_FILE)).expect("marker")
    }
}
