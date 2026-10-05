//! SG-000074 public surface: discovery, profile, argv, protocol,
//! supervision, and errors. No MCP tools, no network, no execution.

mod argv;
mod discovery;
mod error;
mod host;
mod profile;
mod proto;
mod supervise;

pub use argv::{assert_argv_clean, build_argv};
pub use discovery::{discover_engine, Engine, EngineKind, SearchConfig};
pub use error::{HostError, UnavailableReason};
pub use host::{HostIdentity, LiveHost, Supervisor, HANDSHAKE_TIMEOUT};
pub use profile::{adopt_profile, setup_ephemeral_profile, AutomationProfile};
pub use proto::{
    decode_frame, encode_frame, write_frame, HostReply, HostRequest, MAX_FRAME_BYTES,
    PROTOCOL_GENERATION,
};
pub use supervise::{
    spawn_engine_null, spawn_host_piped, HostPipes, SupervisedChild, JOB_ACTIVE_PROCESS_LIMIT,
    LAUNCH_GRACE, SHUTDOWN_TIMEOUT,
};
