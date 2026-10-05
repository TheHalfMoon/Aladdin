//! SG-000074 public surface: discovery, profile, argv, protocol,
//! supervision, and errors. No MCP tools, no network, no execution.

mod actuation;
mod argv;
mod discovery;
mod error;
mod host;
mod navigation;
mod observation;
mod profile;
mod proto;
mod supervise;

pub use actuation::{
    coordinate_fallback, dispatch, retry_after_dispatch, role_supports_action, target_digest,
    verify_postcondition, verify_target, ActuationTarget, ApprovalToken, DispatchOutcome,
    SupportedAction,
};
pub use argv::{assert_argv_clean, build_argv};
pub use discovery::{discover_engine, Engine, EngineKind, SearchConfig};
pub use error::{HostError, UnavailableReason};
pub use host::{HostIdentity, LiveHost, Supervisor, HANDSHAKE_TIMEOUT};
pub use navigation::{
    check_rebinding_consistent, download_trigger_allowed, external_handler_allowed, mediate_frame,
    mediate_popup, mediate_service_worker, mediate_subresource, mediate_worker,
    parse_navigation_url, permission_allowed, validate_navigation, validate_redirect, DnsResolver,
    NavigationTarget, RedirectChain, SubresourceKind, MAX_REDIRECT_HOPS, MAX_URL_BYTES,
};
pub use observation::{
    cdp_command, check_freshness, devtools_open, evaluate_javascript, observe_snapshot,
    read_credentials, redact_value, select_by_caller_selector, short_digest,
    validate_identity_field, DocumentIdentity, InputNode, NodeIdentity, Observation,
    ObservationGate, ObservationScope, ObservedNode, PageIdentity, MAX_CONCURRENT_OBSERVATIONS,
    MAX_IDENTITY_BYTES, MAX_OBSERVATION_DEPTH, MAX_OBSERVATION_MS, MAX_OBSERVED_NODES,
    MAX_SNAPSHOT_BYTES, MAX_VALUE_BYTES,
};
pub use profile::{adopt_profile, setup_ephemeral_profile, AutomationProfile};
pub use proto::{
    decode_frame, encode_frame, write_frame, HostReply, HostRequest, MAX_FRAME_BYTES,
    PROTOCOL_GENERATION,
};
pub use supervise::{
    spawn_engine_null, spawn_host_piped, HostPipes, SupervisedChild, JOB_ACTIVE_PROCESS_LIMIT,
    LAUNCH_GRACE, SHUTDOWN_TIMEOUT,
};
