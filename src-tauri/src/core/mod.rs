//! Rust Core architecture boundary.
//!
//! Phase 02 only defines module ownership. Business behavior, transport
//! backends and Node replacement are intentionally deferred to later phases.

pub mod can;
pub mod config;
pub mod error;
pub mod event_bus;
pub mod logging;
pub mod runtime;
pub mod task;
pub mod device;
pub mod discovery;
pub mod doip;
pub mod isotp;
pub mod lin;
pub mod plugin;
pub mod replay;
pub mod serial;
pub mod someip;
pub mod transport;
pub mod uds;
pub mod xcp;

/// Compile-only marker for the Phase 02 skeleton.
#[derive(Clone, Copy, Debug, Default)]
pub struct CoreSkeleton;

impl CoreSkeleton {
    pub const PHASE: &'static str = "phase-02-core-skeleton";
}

#[cfg(test)]
mod tests {
    use super::CoreSkeleton;

    #[test]
    fn phase_two_core_boundary_is_compilable() {
        assert_eq!(CoreSkeleton::PHASE, "phase-02-core-skeleton");
    }
}
