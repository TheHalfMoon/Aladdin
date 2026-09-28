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

/// Typed denial catalog for browser shapes that remain unauthorized.
/// SG-000021 left navigation, DOM actuation, downloads, uploads,
/// personal-profile access, debugging, scripting, and network egress absent.
/// SG-000022 authorizes only page lifecycle and origin-bound navigation;
/// DOM observation, DOM actuation, downloads, uploads, personal-profile
/// access, debugging, scripting, and network egress remain absent and every
/// shape listed here must fail closed.
pub const DENIED_BROWSER_SHAPES: &[(&str, &str)] = &[
    ("browser.navigate", "navigate"),
    ("browser.snapshot", "capture"),
    ("browser.snapshot", "actuate"),
    ("browser.snapshot", "observe"),
    ("browser.dom", "click"),
    ("browser.dom", "fill"),
    ("browser.dom", "type"),
    ("browser.dom", "press"),
    ("browser.dom", "select"),
    ("browser.dom", "write"),
    ("browser.dom", "snapshot"),
    ("browser.dom", "observe"),
    ("browser.accessibility", "query"),
    ("browser.page", "close"),
    ("browser.navigation", "back"),
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

/// The typed operations SG-000022 authorizes: the two SG-000021 reads plus
/// page lifecycle and origin-bound navigation. Everything else under a
/// browser-like capability must fail closed.
pub fn is_allowed_browser_shape(capability: &str, operation: &str) -> bool {
    matches!(
        (capability, operation),
        ("browser.profile", "status")
            | ("browser.destination", "validate")
            | ("browser.page", "open")
            | ("browser.navigation", "preview")
            | ("browser.navigation", "navigate")
    )
}

/// Returns true when a request targets browser-like authority outside the
/// allowed SG-000022 shapes. The policy layer denies these shapes; the
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

/// Compute the deterministic identity for a canonical profile root without
/// creating any directory. Page records bind this identity so a foreign
/// profile directory can never satisfy a page opened on the isolated profile.
pub fn profile_identity_for_root(root: &Path) -> String {
    profile_identity(root)
}

// ---------------------------------------------------------------------------
// SG-000022 typed page lifecycle and origin-bound bounded navigation.
// ---------------------------------------------------------------------------

/// Schema for the page registry file stored under Cotra protected state.
pub const PAGE_REGISTRY_SCHEMA: &str = "cotra-browser-pages-v1";
/// File name for the page registry inside the isolated profile directory.
pub const PAGE_REGISTRY_FILE: &str = "pages.jsonl";
/// Bound on simultaneously allocated pages per workspace.
pub const MAX_PAGES_PER_WORKSPACE: usize = 16;
/// Bound on redirect hops validated for a single navigation.
pub const MAX_REDIRECT_HOPS: usize = 8;
/// Prefix for server-allocated page identities. Callers never choose this
/// value; only identities present in the registry are valid.
pub const PAGE_ID_PREFIX: &str = "pg-";

/// Lifecycle state for a typed page. Pages start open with no origin and
/// become active after the first successful navigation. Closed pages never
/// become valid again; their handles fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    Open,
    Active,
    Closed,
}

impl PageState {
    pub fn as_str(self) -> &'static str {
        match self {
            PageState::Open => "open",
            PageState::Active => "active",
            PageState::Closed => "closed",
        }
    }
}

/// Server-side page record bound to the isolated profile, one workspace, its
/// current origin, and a lifecycle generation. The generation increments on
/// every successful navigation so replaced pages invalidate old handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRecord {
    pub page_id: String,
    pub workspace_id: String,
    pub profile_identity: String,
    pub current_origin: String,
    pub generation: u64,
    pub state: PageState,
    pub policy_revision: String,
}

impl PageRecord {
    pub fn to_json(&self) -> Value {
        json!({
            "page_id": self.page_id,
            "workspace_id": self.workspace_id,
            "profile_identity": self.profile_identity,
            "current_origin": self.current_origin,
            "generation": self.generation,
            "state": self.state.as_str(),
            "policy_revision": self.policy_revision,
        })
    }
}

/// Read-only navigation preview material. Preview performs full origin
/// binding, re-resolution, and post-resolution policy without mutating page
/// state and without requiring approval. The caller uses the preview to
/// build a fresh navigation request with exact expected state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationPreview {
    pub page_id: String,
    pub workspace_id: String,
    pub policy_revision: String,
    pub current_origin: String,
    pub current_generation: u64,
    pub target_origin: String,
    pub pinned_address: IpAddr,
    pub redirect_count: usize,
    pub final_origin: String,
}

impl NavigationPreview {
    pub fn to_json(&self) -> Value {
        json!({
            "page_id": self.page_id,
            "workspace_id": self.workspace_id,
            "policy_revision": self.policy_revision,
            "current_origin": self.current_origin,
            "current_generation": self.current_generation,
            "target_origin": self.target_origin,
            "pinned_address": self.pinned_address.to_string(),
            "redirect_count": self.redirect_count,
            "final_origin": self.final_origin,
        })
    }
}

/// Typed bounded navigation evidence. Contains only origin, identity, and
/// approval linkage material. Cookies, credentials, tokens, headers, DOM
/// content, and raw browser internals never enter this packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationEvidence {
    pub page_id: String,
    pub workspace_id: String,
    pub policy_revision: String,
    pub profile_identity: String,
    pub prior_origin: String,
    pub prior_generation: u64,
    pub target_origin: String,
    pub pinned_address: IpAddr,
    pub redirect_count: usize,
    pub final_origin: String,
    pub new_generation: u64,
}

impl NavigationEvidence {
    pub fn to_json(&self, approval_record_id: &str) -> Value {
        json!({
            "page_id": self.page_id,
            "workspace_id": self.workspace_id,
            "policy_revision": self.policy_revision,
            "profile_identity": self.profile_identity,
            "prior_origin": self.prior_origin,
            "prior_generation": self.prior_generation,
            "target_origin": self.target_origin,
            "pinned_address": self.pinned_address.to_string(),
            "redirect_count": self.redirect_count,
            "final_origin": self.final_origin,
            "new_generation": self.new_generation,
            "approval_record_id": approval_record_id,
            "cookies": false,
            "credentials": false,
        })
    }
}

/// File-backed page registry stored under Cotra protected local state. Only
/// server-allocated identities in this registry are valid; caller-supplied
/// strings that are absent here fail closed as stale handles.
#[derive(Debug)]
pub struct PageStore {
    path: PathBuf,
    pages: std::collections::BTreeMap<String, PageRecord>,
}

impl PageStore {
    pub fn load_or_create(path: PathBuf) -> Self {
        let mut store = Self {
            path,
            pages: std::collections::BTreeMap::new(),
        };
        if store.path.is_file() {
            if let Ok(text) = std::fs::read_to_string(&store.path) {
                for line in text.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(record) = serde_json::from_str::<StoredPage>(line) {
                        if record.schema != PAGE_REGISTRY_SCHEMA {
                            break;
                        }
                        let page = PageRecord {
                            page_id: record.page_id,
                            workspace_id: record.workspace_id,
                            profile_identity: record.profile_identity,
                            current_origin: record.current_origin,
                            generation: record.generation,
                            state: match record.state.as_str() {
                                "active" => PageState::Active,
                                "closed" => PageState::Closed,
                                _ => PageState::Open,
                            },
                            policy_revision: record.policy_revision,
                        };
                        if page.page_id.is_empty() {
                            break;
                        }
                        store.pages.insert(page.page_id.clone(), page);
                    }
                }
            }
        } else if let Some(parent) = store.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        store
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self, page_id: &str) -> Option<&PageRecord> {
        self.pages.get(page_id)
    }

    pub fn count_for_workspace(&self, workspace_id: &str) -> usize {
        self.pages
            .values()
            .filter(|page| page.workspace_id == workspace_id && page.state != PageState::Closed)
            .count()
    }

    /// Allocate a fresh page bound to the isolated profile, one workspace,
    /// an empty current origin, generation zero, and the current policy
    /// revision. No network activity occurs. Caller-supplied identities are
    /// never accepted; the identity is always server-allocated.
    pub fn open_page(
        &mut self,
        workspace_id: &str,
        profile_identity: &str,
        policy_revision: &str,
    ) -> Result<PageRecord, ProviderError> {
        if workspace_id.trim().is_empty() {
            return Err(ProviderError::invalid("browser page workspace is empty"));
        }
        if profile_identity.trim().is_empty() {
            return Err(ProviderError::invalid(
                "browser page profile identity is empty",
            ));
        }
        if policy_revision.trim().is_empty() {
            return Err(ProviderError::invalid(
                "browser page policy revision is empty",
            ));
        }
        if self.count_for_workspace(workspace_id) >= MAX_PAGES_PER_WORKSPACE {
            return Err(ProviderError::new(
                FailureCode::OutputLimit,
                format!(
                    "browser page bound exceeded for workspace; at most {MAX_PAGES_PER_WORKSPACE} open pages"
                ),
            ));
        }
        let page_id = fresh_page_id(workspace_id, profile_identity);
        if self.pages.contains_key(&page_id) {
            return Err(ProviderError::new(
                FailureCode::InternalError,
                "browser page identity collision; retry page open",
            ));
        }
        let page = PageRecord {
            page_id: page_id.clone(),
            workspace_id: workspace_id.to_owned(),
            profile_identity: profile_identity.to_owned(),
            current_origin: String::new(),
            generation: 0,
            state: PageState::Open,
            policy_revision: policy_revision.to_owned(),
        };
        self.persist(&page);
        self.pages.insert(page_id, page.clone());
        Ok(page)
    }

    /// Apply a validated navigation transition. The caller must already have
    /// validated expected state, target origin, pinned address, redirect
    /// chain, and fresh approval. This function re-checks expected state
    /// against current state, bumps the generation, and persists.
    pub fn apply_navigation(
        &mut self,
        page_id: &str,
        expected_origin: &str,
        expected_generation: u64,
        final_origin: &str,
    ) -> Result<(PageRecord, String, u64), ProviderError> {
        let current = self.pages.get(page_id).cloned().ok_or_else(|| {
            ProviderError::new(
                FailureCode::TargetStale,
                "browser page handle is unknown; stale page handles fail closed",
            )
        })?;
        if current.state == PageState::Closed {
            return Err(ProviderError::new(
                FailureCode::TargetStale,
                "browser page is closed; stale page handles fail closed",
            ));
        }
        if current.current_origin != expected_origin {
            return Err(ProviderError::new(
                FailureCode::TargetStale,
                "browser page origin changed since preview; stale page handles fail closed",
            ));
        }
        if current.generation != expected_generation {
            return Err(ProviderError::new(
                FailureCode::TargetStale,
                "browser page generation changed since preview; stale page handles fail closed",
            ));
        }
        let prior_origin = current.current_origin.clone();
        let prior_generation = current.generation;
        let mut next = current;
        next.current_origin = final_origin.to_owned();
        next.generation = next.generation.saturating_add(1);
        next.state = PageState::Active;
        self.persist(&next);
        self.pages.insert(page_id.to_owned(), next.clone());
        Ok((next, prior_origin, prior_generation))
    }

    fn persist(&self, page: &PageRecord) {
        use std::io::Write as _;
        let stored = StoredPage {
            schema: PAGE_REGISTRY_SCHEMA.to_owned(),
            page_id: page.page_id.clone(),
            workspace_id: page.workspace_id.clone(),
            profile_identity: page.profile_identity.clone(),
            current_origin: page.current_origin.clone(),
            generation: page.generation,
            state: page.state.as_str().to_owned(),
            policy_revision: page.policy_revision.clone(),
        };
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = serde_json::to_writer(&mut file, &stored);
            let _ = file.write_all(b"\n");
            let _ = file.flush();
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StoredPage {
    schema: String,
    page_id: String,
    workspace_id: String,
    profile_identity: String,
    current_origin: String,
    generation: u64,
    state: String,
    policy_revision: String,
}

fn fresh_page_id(workspace_id: &str, profile_identity: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher as _};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = DefaultHasher::new();
    std::process::id().hash(&mut hasher);
    now.hash(&mut hasher);
    count.hash(&mut hasher);
    workspace_id.hash(&mut hasher);
    profile_identity.hash(&mut hasher);
    let digest = Sha256::digest(format!("{:016x}{:016x}", hasher.finish(), count).as_bytes());
    let mut hex = String::with_capacity(32);
    use std::fmt::Write as _;
    for byte in digest.iter().take(16) {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    format!("{PAGE_ID_PREFIX}{hex}")
}

/// Canonical path for the page registry file. Tests override it with
/// `COTRA_BROWSER_STATE_DIR`; production keeps it inside the isolated
/// profile directory under Cotra protected local state.
pub fn default_page_registry_path(profile_root: &Path) -> PathBuf {
    profile_root.join(PAGE_REGISTRY_FILE)
}

/// Returns true when a navigation target path triggers download handling.
/// Executable, script, and archive suffixes are denied so navigation cannot
/// become a bypass around download policy. Query strings never hide the
/// suffix because only the path component is inspected.
pub fn is_download_trigger_url(url: &str) -> bool {
    let Some((_, rest)) = url.split_once("://") else {
        return false;
    };
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let mut path = &rest[authority_end..];
    if let Some(end) = path.find(['?', '#']) {
        path = &path[..end];
    }
    let file = path.rsplit('/').next().unwrap_or("");
    let lower = file.to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    const SUFFIXES: &[&str] = &[
        ".exe", ".msi", ".msix", ".dll", ".sys", ".ps1", ".bat", ".cmd", ".vbs", ".vbe", ".js",
        ".jse", ".wsf", ".wsh", ".zip", ".7z", ".rar", ".tar", ".gz", ".cab", ".iso", ".img",
    ];
    SUFFIXES.iter().any(|suffix| lower.ends_with(suffix))
}

/// Validate a redirect chain hop by hop. Every hop must parse, must share the
/// exact validated base origin, must re-resolve to a public address, must not
/// repeat a previously seen URL, and must not exceed the hop bound. Scheme
/// downgrade, loops, widening, SSRF, and malformed targets fail closed.
pub fn validate_redirect_chain(
    base_origin: &str,
    hops: &[String],
    resolver: &impl DnsResolver,
) -> Result<(ValidatedDestination, usize, String), ProviderError> {
    let base = parse_destination_url(base_origin).map_err(|e| {
        ProviderError::new(
            FailureCode::TargetStale,
            format!("browser redirect base origin is invalid: {}", e.message),
        )
    })?;
    if hops.len() > MAX_REDIRECT_HOPS {
        return Err(ProviderError::denied(format!(
            "browser redirect chain exceeds at most {MAX_REDIRECT_HOPS} hops"
        )));
    }
    let mut seen: Vec<String> = Vec::new();
    let mut current_origin = base.origin.clone();
    let mut final_validated: Option<ValidatedDestination> = None;
    for hop in hops {
        if is_download_trigger_url(hop) {
            return Err(ProviderError::denied(
                "browser redirect target triggers download handling; downloads remain denied",
            ));
        }
        let target = parse_destination_url(hop)?;
        if target.origin != base.origin {
            return Err(ProviderError::denied(format!(
                "browser redirect target origin {} does not match validated origin {}; redirect widening is denied",
                target.origin, base.origin
            )));
        }
        if seen.iter().any(|seen_url| seen_url == hop) {
            return Err(ProviderError::denied(
                "browser redirect chain contains a loop; redirect loops are denied",
            ));
        }
        seen.push(hop.clone());
        let validated = validate_destination(hop, resolver)?;
        if validated.origin != base.origin {
            return Err(ProviderError::denied(
                "browser redirect hop origin drifted after resolution; redirect widening is denied",
            ));
        }
        current_origin = validated.origin.clone();
        final_validated = Some(validated);
    }
    match final_validated {
        Some(final_destination) => Ok((final_destination, hops.len(), current_origin)),
        None => Err(ProviderError::invalid(
            "browser redirect chain is empty; supply at least one hop or navigate directly",
        )),
    }
}

/// Build a read-only navigation preview for a known page without mutating
/// state. Validates the target URL and optional redirect chain with full
/// re-resolution and post-resolution policy. Download triggers fail closed.
pub fn preview_navigation(
    page: &PageRecord,
    target_url: &str,
    redirect_chain: &[String],
    resolver: &impl DnsResolver,
) -> Result<NavigationPreview, ProviderError> {
    if page.state == PageState::Closed {
        return Err(ProviderError::new(
            FailureCode::TargetStale,
            "browser page is closed; stale page handles fail closed",
        ));
    }
    if is_download_trigger_url(target_url) {
        return Err(ProviderError::denied(
            "browser navigation target triggers download handling; downloads remain denied",
        ));
    }
    let target = validate_destination(target_url, resolver)?;
    let (final_origin, pinned, redirect_count) = if redirect_chain.is_empty() {
        (target.origin.clone(), target.pinned_address, 0)
    } else {
        if target.origin.is_empty() {
            return Err(ProviderError::invalid(
                "browser navigation target origin is empty",
            ));
        }
        let (final_destination, count, final_name) =
            validate_redirect_chain(&target.origin, redirect_chain, resolver)?;
        if final_destination.origin != target.origin {
            return Err(ProviderError::denied(
                "browser redirect chain widened beyond the validated target origin",
            ));
        }
        (final_name, final_destination.pinned_address, count)
    };
    Ok(NavigationPreview {
        page_id: page.page_id.clone(),
        workspace_id: page.workspace_id.clone(),
        policy_revision: page.policy_revision.clone(),
        current_origin: page.current_origin.clone(),
        current_generation: page.generation,
        target_origin: target.origin,
        pinned_address: pinned,
        redirect_count,
        final_origin,
    })
}

/// Compute the SOFT approval digest for a navigation transition. The digest
/// binds page identity, expected origin and generation, target origin, pinned
/// address, redirect chain, workspace, profile identity, and policy revision.
/// Any material drift invalidates the approval.
pub fn navigation_approval_digest(
    workspace_id: &str,
    policy_revision: &str,
    profile_identity: &str,
    page_id: &str,
    expected_origin: &str,
    expected_generation: u64,
    target_origin: &str,
    pinned_address: &IpAddr,
    redirect_chain: &[String],
    final_origin: &str,
) -> String {
    let mut hasher = Sha256::new();
    digest_bytes(&mut hasher, b"COTRA_BROWSER_NAVIGATION_V1");
    digest_bytes(&mut hasher, workspace_id.as_bytes());
    digest_bytes(&mut hasher, policy_revision.as_bytes());
    digest_bytes(&mut hasher, profile_identity.as_bytes());
    digest_bytes(&mut hasher, page_id.as_bytes());
    digest_bytes(&mut hasher, expected_origin.as_bytes());
    digest_bytes(&mut hasher, expected_generation.to_string().as_bytes());
    digest_bytes(&mut hasher, target_origin.as_bytes());
    digest_bytes(&mut hasher, pinned_address.to_string().as_bytes());
    for hop in redirect_chain {
        digest_bytes(&mut hasher, hop.as_bytes());
    }
    digest_bytes(&mut hasher, final_origin.as_bytes());
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    use std::fmt::Write as _;
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn digest_bytes(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
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
    fn allowed_shapes_are_exactly_profile_status_destination_page_and_navigation() {
        assert!(is_allowed_browser_shape("browser.profile", "status"));
        assert!(is_allowed_browser_shape("browser.destination", "validate"));
        assert!(is_allowed_browser_shape("browser.page", "open"));
        assert!(is_allowed_browser_shape("browser.navigation", "preview"));
        assert!(is_allowed_browser_shape("browser.navigation", "navigate"));
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
            ("browser.dom", "snapshot"),
            ("browser.dom", "observe"),
            ("browser.accessibility", "query"),
            ("browser.snapshot", "observe"),
            ("browser.page", "close"),
            ("browser.navigation", "back"),
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

    fn temp_page_registry(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cotra-browser-pages-{label}-{}-{suffix}.jsonl",
            std::process::id()
        ))
    }

    fn open_test_page(store: &mut PageStore) -> PageRecord {
        store
            .open_page("default", "profile-identity-test", "sg-000022-v1")
            .expect("open page")
    }

    #[test]
    fn page_open_binds_workspace_profile_and_generation() {
        let path = temp_page_registry("open");
        let mut store = PageStore::load_or_create(path.clone());
        let first = open_test_page(&mut store);
        assert!(first.page_id.starts_with(PAGE_ID_PREFIX));
        assert_eq!(first.workspace_id, "default");
        assert_eq!(first.profile_identity, "profile-identity-test");
        assert_eq!(first.current_origin, "");
        assert_eq!(first.generation, 0);
        assert_eq!(first.state, PageState::Open);
        let second = open_test_page(&mut store);
        assert_ne!(first.page_id, second.page_id);
        assert_eq!(store.count_for_workspace("default"), 2);
        assert_eq!(store.count_for_workspace("other"), 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn page_open_enforces_per_workspace_bound() {
        let path = temp_page_registry("bound");
        let mut store = PageStore::load_or_create(path.clone());
        for _ in 0..MAX_PAGES_PER_WORKSPACE {
            open_test_page(&mut store);
        }
        let error = store
            .open_page("default", "profile-identity-test", "sg-000022-v1")
            .expect_err("bound must fail closed");
        assert_eq!(error.code, FailureCode::OutputLimit);
        assert!(store
            .open_page("other", "profile-identity-test", "sg-000022-v1")
            .is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn forged_page_handles_fail_closed() {
        let path = temp_page_registry("forged");
        let mut store = PageStore::load_or_create(path.clone());
        let page = open_test_page(&mut store);
        assert!(store.get(&page.page_id).is_some());
        assert!(store.get("pg-00000000000000000000000000000000").is_none());
        assert!(store.get("").is_none());
        let error = store
            .apply_navigation(
                &"pg-forged-handle".to_owned(),
                "",
                0,
                "https://example.com:443",
            )
            .expect_err("forged handle must fail");
        assert_eq!(error.code, FailureCode::TargetStale);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stale_generation_and_origin_fail_closed() {
        let path = temp_page_registry("stale");
        let mut store = PageStore::load_or_create(path.clone());
        let page = open_test_page(&mut store);
        let error = store
            .apply_navigation(
                &page.page_id,
                "https://example.com:443",
                0,
                "https://example.com:443",
            )
            .expect_err("wrong origin must fail");
        assert_eq!(error.code, FailureCode::TargetStale);
        let error = store
            .apply_navigation(&page.page_id, "", 7, "https://example.com:443")
            .expect_err("wrong generation must fail");
        assert_eq!(error.code, FailureCode::TargetStale);
        let (next, prior_origin, prior_generation) = store
            .apply_navigation(&page.page_id, "", 0, "https://example.com:443")
            .expect("first navigation");
        assert_eq!(prior_origin, "");
        assert_eq!(prior_generation, 0);
        assert_eq!(next.generation, 1);
        assert_eq!(next.current_origin, "https://example.com:443");
        assert_eq!(next.state, PageState::Active);
        let error = store
            .apply_navigation(&page.page_id, "", 0, "https://example.com:443")
            .expect_err("reused generation must fail");
        assert_eq!(error.code, FailureCode::TargetStale);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn redirect_chain_validates_hop_by_hop_with_loop_and_downgrade_denial() {
        let resolver = public_resolver();
        let base = "https://example.com/";
        let validated = validate_destination(base, &resolver).expect("base");
        let hops = vec![
            "https://example.com/step-one".to_owned(),
            "https://example.com/step-two".to_owned(),
        ];
        let (final_destination, count, final_origin) =
            validate_redirect_chain(&validated.origin, &hops, &resolver).expect("chain");
        assert_eq!(count, 2);
        assert_eq!(final_origin, "https://example.com:443");
        assert_eq!(final_destination.origin, "https://example.com:443");

        let widened = vec!["https://evil.example.com/".to_owned()];
        let error = validate_redirect_chain(&validated.origin, &widened, &resolver)
            .expect_err("widening must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);

        let downgrade = vec!["http://example.com/".to_owned()];
        let error = validate_redirect_chain(&validated.origin, &downgrade, &resolver)
            .expect_err("downgrade must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);

        let alternate_port = vec!["https://example.com:8443/".to_owned()];
        let error = validate_redirect_chain(&validated.origin, &alternate_port, &resolver)
            .expect_err("alternate port must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);

        let looped = vec![
            "https://example.com/a".to_owned(),
            "https://example.com/a".to_owned(),
        ];
        let error = validate_redirect_chain(&validated.origin, &looped, &resolver)
            .expect_err("loop must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);

        let userinfo = vec!["https://user@example.com/".to_owned()];
        assert!(validate_redirect_chain(&validated.origin, &userinfo, &resolver).is_err());

        let mut too_many = Vec::new();
        for index in 0..(MAX_REDIRECT_HOPS + 1) {
            too_many.push(format!("https://example.com/hop-{index}"));
        }
        let error = validate_redirect_chain(&validated.origin, &too_many, &resolver)
            .expect_err("too many hops must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
    }

    #[test]
    fn redirect_hops_apply_ssrf_policy_per_hop() {
        let loopback = StaticResolver {
            addresses: vec!["127.0.0.1".parse().unwrap()],
        };
        let base = "https://example.com/";
        let hops = vec!["https://example.com/private".to_owned()];
        let error =
            validate_redirect_chain(base, &hops, &loopback).expect_err("SSRF hop must fail closed");
        assert_eq!(error.code, FailureCode::CapabilityDenied);

        let metadata = StaticResolver {
            addresses: vec!["169.254.169.254".parse().unwrap()],
        };
        let error = validate_redirect_chain(base, &hops, &metadata)
            .expect_err("metadata hop must fail closed");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
    }

    #[test]
    fn download_triggers_are_denied_for_executable_script_and_archive_paths() {
        for url in [
            "https://example.com/tool.exe",
            "https://example.com/setup.msi",
            "https://example.com/run.ps1",
            "https://example.com/run.bat",
            "https://example.com/payload.js",
            "https://example.com/archive.zip",
            "https://example.com/archive.7z",
            "https://example.com/image.iso",
            "https://example.com/TOOL.EXE?download=1",
        ] {
            assert!(
                is_download_trigger_url(url),
                "{url} must be a download trigger"
            );
        }
        for url in [
            "https://example.com/docs",
            "https://example.com/page.html",
            "https://example.com/api/data.json",
            "https://example.com/",
        ] {
            assert!(!is_download_trigger_url(url), "{url} must not be a trigger");
        }
        let resolver = public_resolver();
        let path = temp_page_registry("download");
        let mut store = PageStore::load_or_create(path.clone());
        let page = open_test_page(&mut store);
        assert!(preview_navigation(&page, "https://example.com/tool.exe", &[], &resolver).is_err());
        let chain = vec!["https://example.com/archive.zip".to_owned()];
        assert!(preview_navigation(&page, "https://example.com/docs", &chain, &resolver).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn navigation_preview_binds_origin_with_ssrf_denial() {
        let resolver = public_resolver();
        let path = temp_page_registry("preview");
        let mut store = PageStore::load_or_create(path.clone());
        let page = open_test_page(&mut store);
        let preview =
            preview_navigation(&page, "https://example.com/docs", &[], &resolver).expect("preview");
        assert_eq!(preview.target_origin, "https://example.com:443");
        assert_eq!(preview.final_origin, "https://example.com:443");
        assert_eq!(preview.redirect_count, 0);
        assert_eq!(preview.pinned_address.to_string(), "93.184.216.34");

        let loopback = StaticResolver {
            addresses: vec!["10.0.0.1".parse().unwrap()],
        };
        let error = preview_navigation(&page, "https://example.com/docs", &[], &loopback)
            .expect_err("private must fail");
        assert_eq!(error.code, FailureCode::CapabilityDenied);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn navigation_digest_changes_for_material_drift() {
        let pinned: IpAddr = "93.184.216.34".parse().unwrap();
        let base = navigation_approval_digest(
            "default",
            "sg-000022-v1",
            "profile-a",
            "pg-abc",
            "",
            0,
            "https://example.com:443",
            &pinned,
            &[],
            "https://example.com:443",
        );
        let drifted_origin = navigation_approval_digest(
            "default",
            "sg-000022-v1",
            "profile-a",
            "pg-abc",
            "https://other.example:443",
            0,
            "https://example.com:443",
            &pinned,
            &[],
            "https://example.com:443",
        );
        assert_ne!(base, drifted_origin);
        let drifted_generation = navigation_approval_digest(
            "default",
            "sg-000022-v1",
            "profile-a",
            "pg-abc",
            "",
            1,
            "https://example.com:443",
            &pinned,
            &[],
            "https://example.com:443",
        );
        assert_ne!(base, drifted_generation);
        let drifted_pin: IpAddr = "1.1.1.1".parse().unwrap();
        let drifted_address = navigation_approval_digest(
            "default",
            "sg-000022-v1",
            "profile-a",
            "pg-abc",
            "",
            0,
            "https://example.com:443",
            &drifted_pin,
            &[],
            "https://example.com:443",
        );
        assert_ne!(base, drifted_address);
    }

    #[test]
    fn navigation_evidence_carries_no_secret_material() {
        let pinned: IpAddr = "93.184.216.34".parse().unwrap();
        let evidence = NavigationEvidence {
            page_id: "pg-test".to_owned(),
            workspace_id: "default".to_owned(),
            policy_revision: "sg-000022-v1".to_owned(),
            profile_identity: "profile-a".to_owned(),
            prior_origin: String::new(),
            prior_generation: 0,
            target_origin: "https://example.com:443".to_owned(),
            pinned_address: pinned,
            redirect_count: 0,
            final_origin: "https://example.com:443".to_owned(),
            new_generation: 1,
        };
        let json = evidence.to_json("apr-1");
        let text = json.to_string().to_ascii_lowercase();
        assert!(!text.contains("cookie"));
        assert!(json["cookies"] == false);
        let preview = NavigationPreview {
            page_id: "pg-test".to_owned(),
            workspace_id: "default".to_owned(),
            policy_revision: "sg-000022-v1".to_owned(),
            current_origin: String::new(),
            current_generation: 0,
            target_origin: "https://example.com:443".to_owned(),
            pinned_address: pinned,
            redirect_count: 0,
            final_origin: "https://example.com:443".to_owned(),
        };
        assert_eq!(
            preview.to_json()["target_origin"],
            "https://example.com:443"
        );
    }

    fn marker_body(root: &Path) -> String {
        std::fs::read_to_string(root.join(PROFILE_MARKER_FILE)).expect("marker")
    }
}
