//! Substance advection along edges after flow is solved.
//! Phase 1+: O₂ / glucose / etc. Stub for Phase 0.

#![allow(dead_code)]

/// Placeholder for per-edge concentrations once transport is implemented.
#[derive(Debug, Clone, Default)]
pub struct EdgeConcentrations {
    pub o2: f64,
    pub glucose: f64,
}
