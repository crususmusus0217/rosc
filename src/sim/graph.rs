//! Circulation graph: nodes (junctions / organs) and edges (vessels).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub usize);

/// How pressure is constrained at a node.
#[derive(Debug, Clone, Copy)]
pub enum PressureBc {
    /// Unknown pressure (solved for).
    Free,
    /// Dirichlet pressure [same units as the heart stub, arbitrary for now].
    Fixed(f64),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub pressure_bc: PressureBc,
}

/// Directed vessel segment from `from` → `to`.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    /// Lumen radius (m). Resistance ∝ 1/r⁴ (Hagen–Poiseuille).
    pub radius: f64,
    /// Length (m).
    pub length: f64,
}

#[derive(Debug, Clone, Default)]
pub struct VesselGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl VesselGraph {
    pub fn add_node(&mut self, name: impl Into<String>, pressure_bc: PressureBc) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            name: name.into(),
            pressure_bc,
        });
        id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, radius: f64, length: f64) -> EdgeId {
        let id = EdgeId(self.edges.len());
        self.edges.push(Edge {
            from,
            to,
            radius,
            length,
        });
        id
    }

    /// Three-node closed loop used by Phase 0 tests:
    /// heart (fixed P) → capillary (free) → vein (P=0) → heart.
    pub fn closed_loop_3() -> (Self, NodeId, NodeId, NodeId, EdgeId, EdgeId, EdgeId) {
        let mut g = Self::default();
        let heart = g.add_node("heart_out", PressureBc::Fixed(100.0));
        let cap = g.add_node("capillary", PressureBc::Free);
        let vein = g.add_node("vein_ref", PressureBc::Fixed(0.0));
        let e_art = g.add_edge(heart, cap, 0.002, 0.10);
        let e_ven = g.add_edge(cap, vein, 0.002, 0.10);
        let e_return = g.add_edge(vein, heart, 0.003, 0.05);
        (g, heart, cap, vein, e_art, e_ven, e_return)
    }
}
