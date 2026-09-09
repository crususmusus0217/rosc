//! Live closed-loop circuit state for the Phase 1 vertical slice.

use crate::sim::flow::{edge_flow, max_mass_imbalance, solve_flows, FlowSolution};
use crate::sim::graph::{EdgeId, NodeId, VesselGraph};

/// Myocardial / circulating pools (Phase 1 stubs — not full biochemistry).
#[derive(Debug, Clone)]
pub struct MetabolicPools {
    /// Arterial-ish O₂ saturation proxy in \[0, 1\].
    pub o2: f64,
    /// Heart ATP proxy in arbitrary units.
    pub atp: f64,
    pub atp_max: f64,
    /// ATP spent per circulation tick while pumping.
    pub atp_per_beat: f64,
    /// O₂ consumed per tick while pumping.
    pub o2_per_beat: f64,
    /// O₂ recovered per tick from the "lung stub" while pumping.
    pub o2_recharge: f64,
}

impl Default for MetabolicPools {
    fn default() -> Self {
        Self {
            o2: 0.95,
            atp: 100.0,
            atp_max: 100.0,
            atp_per_beat: 2.5,
            o2_per_beat: 0.035,
            o2_recharge: 0.03,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitStatus {
    Pumping,
    Failed,
}

/// Authoritative sim state for the demo loop.
#[derive(Debug, Clone)]
pub struct LiveCircuit {
    pub graph: VesselGraph,
    pub heart: NodeId,
    pub capillary: NodeId,
    pub vein: NodeId,
    pub edge_art: EdgeId,
    pub edge_ven: EdgeId,
    pub edge_return: EdgeId,
    pub solution: Option<FlowSolution>,
    pub pools: MetabolicPools,
    pub status: CircuitStatus,
    /// World-space layout for the three nodes (x, y) in pixels-ish units.
    pub layout: [(f32, f32); 3],
}

impl LiveCircuit {
    pub fn closed_loop_demo() -> Self {
        let (graph, heart, capillary, vein, edge_art, edge_ven, edge_return) =
            VesselGraph::closed_loop_3();
        let mut circuit = Self {
            graph,
            heart,
            capillary,
            vein,
            edge_art,
            edge_ven,
            edge_return,
            solution: None,
            pools: MetabolicPools::default(),
            status: CircuitStatus::Pumping,
            // Triangle layout centered near origin.
            layout: [
                (0.0, 120.0),   // heart
                (-160.0, -80.0), // capillary
                (160.0, -80.0),  // vein
            ],
        };
        circuit.resync_flow();
        circuit
    }

    pub fn node_pos(&self, id: NodeId) -> (f32, f32) {
        self.layout[id.0]
    }

    pub fn resync_flow(&mut self) {
        match solve_flows(&self.graph) {
            Ok(sol) => self.solution = Some(sol),
            Err(e) => {
                eprintln!("flow solve failed: {e}");
                self.solution = None;
            }
        }
    }

    /// One circulation tick: solve → metabolism → maybe fail.
    pub fn circulation_step(&mut self) {
        if self.status == CircuitStatus::Failed {
            // Residual pressures may still exist; keep last solution for drawing.
            return;
        }

        self.resync_flow();

        // Pumping costs ATP and O₂; lung stub partially recharges O₂.
        self.pools.atp = (self.pools.atp - self.pools.atp_per_beat).max(0.0);
        self.pools.o2 = (self.pools.o2 - self.pools.o2_per_beat + self.pools.o2_recharge)
            .clamp(0.0, 1.0);

        // Hypoxia accelerates ATP drain slightly.
        if self.pools.o2 < 0.4 {
            self.pools.atp = (self.pools.atp - self.pools.atp_per_beat * 0.5).max(0.0);
        }

        if self.pools.atp <= 0.0 {
            self.status = CircuitStatus::Failed;
            // Collapse heart outflow pressure to model pump failure.
            if let crate::sim::graph::PressureBc::Fixed(p) =
                &mut self.graph.nodes[self.heart.0].pressure_bc
            {
                *p = 0.0;
            }
            self.resync_flow();
        }
    }

    pub fn refill_atp(&mut self) {
        self.pools.atp = self.pools.atp_max;
        self.pools.o2 = 0.95;
        if self.status == CircuitStatus::Failed {
            self.status = CircuitStatus::Pumping;
            if let crate::sim::graph::PressureBc::Fixed(p) =
                &mut self.graph.nodes[self.heart.0].pressure_bc
            {
                *p = 100.0;
            }
            self.resync_flow();
        }
    }

    pub fn art_flow_abs(&self) -> f64 {
        self.solution
            .as_ref()
            .map(|s| edge_flow(s, self.edge_art).abs())
            .unwrap_or(0.0)
    }

    pub fn mass_imbalance(&self) -> f64 {
        match &self.solution {
            Some(s) => max_mass_imbalance(&self.graph, s),
            None => f64::NAN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_loop_solves() {
        let c = LiveCircuit::closed_loop_demo();
        assert!(c.solution.is_some());
        assert!(c.art_flow_abs() > 0.0);
        assert!(c.mass_imbalance() < 1e-8);
    }

    #[test]
    fn atp_depletion_fails_pump() {
        let mut c = LiveCircuit::closed_loop_demo();
        c.pools.atp = 3.0;
        c.pools.atp_per_beat = 2.0;
        c.circulation_step();
        assert_eq!(c.status, CircuitStatus::Pumping);
        c.circulation_step();
        assert_eq!(c.status, CircuitStatus::Failed);
        assert!(c.pools.atp <= 0.0);
    }
}
