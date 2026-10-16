pub mod adapter_install;
pub mod detect;
pub mod discovery;
pub mod driver;
pub mod supervisor;

pub use detect::{
    Detected, DetectionEnv, DetectionSpec, NodeEntry, detect_agents, find_executable, node_entry,
};
pub use discovery::{
    AdapterLaunch, AdapterStatus, DiscoveryReport, ExecutableSearch, ProbeOptions, ResolveError,
    probe_adapter, probe_adapter_on, resolve_executable,
};
pub use driver::{
    ACP_PROTOCOL_VERSION, AcpAuthoritativeCompletion, AcpDriver, AcpInitializeRequest,
    AcpInitializeResponse, AcpPermissionRequest, AcpPermissionResponse, AcpPromptChunk,
    AcpSessionRequest, AcpSessionResponse, AgentDriver, AgentEvent, AgentEventKind, AgentFailure,
    AgentWire, DriverCapabilities, DriverConfig, DriverError, DriverLimits, DriverMode,
    DriverOutcome, DriverStatus, FailureKind, McpConfigError, McpStdioServer, McpStdioSupport,
    OwnershipSource, PromptOutcome, ServerInfo, SteeringMode, redact_sensitive_string,
    start_driver,
};
pub use supervisor::{AgentSupervisor, QualificationStatus, SupervisorError};

pub mod session;
pub use session::{
    AcpSession, AdapterConfig, SessionControl, SessionProgress, copy_draft, source_revision,
};
