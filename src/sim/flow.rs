//! Steady blood-flow solve on a vessel graph.
//!
//! Formulation (nodal):
//! - Unknowns: pressures at `PressureBc::Free` nodes.
//! - Edge flow: Q = (P_from − P_to) / R with Hagen–Poiseuille
//!   R = 8 η L / (π r⁴).
//! - Kirchhoff current law at each free node: Σ Q_in − Σ Q_out = 0.
//! - Fixed-pressure nodes supply whatever flow is needed (heart / ground).
//!
//! Small graphs use a dense LU via nalgebra; assembly also builds a
//! `nalgebra_sparse::CsrMatrix` so the sparse path is exercised.

use crate::sim::graph::{EdgeId, NodeId, PressureBc, VesselGraph};
use nalgebra::{DMatrix, DVector};
use nalgebra_sparse::{CooMatrix, CsrMatrix};

/// Blood viscosity placeholder (Pa·s). Whole blood ~3–4×10⁻³; tune later.
pub const ETA_BLOOD: f64 = 3.5e-3;

#[derive(Debug, Clone)]
pub struct FlowSolution {
    /// Pressure at every node (fixed BCs copied through).
    pub pressure: Vec<f64>,
    /// Flow on every edge, positive in the edge's from→to direction.
    pub flow: Vec<f64>,
}

pub fn poiseuille_resistance(radius: f64, length: f64, eta: f64) -> f64 {
    debug_assert!(radius > 0.0 && length > 0.0 && eta > 0.0);
    8.0 * eta * length / (std::f64::consts::PI * radius.powi(4))
}

fn edge_resistance(graph: &VesselGraph, edge: &crate::sim::graph::Edge) -> f64 {
    let _ = graph;
    poiseuille_resistance(edge.radius, edge.length, ETA_BLOOD)
}

/// Map free-node NodeId → row/col index in the reduced system.
fn free_index_map(graph: &VesselGraph) -> Vec<Option<usize>> {
    let mut map = vec![None; graph.nodes.len()];
    let mut k = 0usize;
    for (i, node) in graph.nodes.iter().enumerate() {
        if matches!(node.pressure_bc, PressureBc::Free) {
            map[i] = Some(k);
            k += 1;
        }
    }
    map
}

fn fixed_pressure(graph: &VesselGraph, id: NodeId) -> Option<f64> {
    match graph.nodes[id.0].pressure_bc {
        PressureBc::Fixed(p) => Some(p),
        PressureBc::Free => None,
    }
}

/// Assemble reduced dense system A p = b for free-node pressures.
fn assemble_dense(graph: &VesselGraph) -> (DMatrix<f64>, DVector<f64>, Vec<Option<usize>>) {
    let map = free_index_map(graph);
    let n = map.iter().filter(|m| m.is_some()).count();
    let mut a = DMatrix::<f64>::zeros(n, n);
    let mut b = DVector::<f64>::zeros(n);

    for edge in &graph.edges {
        let r = edge_resistance(graph, edge);
        let gcond = 1.0 / r;
        let i = edge.from.0;
        let j = edge.to.0;

        // Stamp conductance like a resistor between i and j.
        // For free node i: ... + g (P_i - P_j) toward leaving i equals 0 contribution pattern.
        match (map[i], map[j]) {
            (Some(ii), Some(jj)) => {
                a[(ii, ii)] += gcond;
                a[(jj, jj)] += gcond;
                a[(ii, jj)] -= gcond;
                a[(jj, ii)] -= gcond;
            }
            (Some(ii), None) => {
                let pj = fixed_pressure(graph, edge.to).expect("fixed");
                a[(ii, ii)] += gcond;
                b[ii] += gcond * pj;
            }
            (None, Some(jj)) => {
                let pi = fixed_pressure(graph, edge.from).expect("fixed");
                a[(jj, jj)] += gcond;
                b[jj] += gcond * pi;
            }
            (None, None) => {
                // Both Dirichlet: no unknown; flow determined after solve.
            }
        }
    }

    (a, b, map)
}

/// Same stamping into a COO/CSR matrix (sparse path for larger graphs later).
pub fn assemble_sparse_csr(graph: &VesselGraph) -> (CsrMatrix<f64>, DVector<f64>, Vec<Option<usize>>) {
    let map = free_index_map(graph);
    let n = map.iter().filter(|m| m.is_some()).count();
    let mut coo = CooMatrix::<f64>::new(n, n);
    let mut b = DVector::<f64>::zeros(n);

    // Dense accumulation then push — fine for Phase 0; keeps CSR path real.
    let mut dense = vec![0.0; n * n];
    let idx = |r: usize, c: usize| r * n + c;

    for edge in &graph.edges {
        let r = edge_resistance(graph, edge);
        let gcond = 1.0 / r;
        let i = edge.from.0;
        let j = edge.to.0;
        match (map[i], map[j]) {
            (Some(ii), Some(jj)) => {
                dense[idx(ii, ii)] += gcond;
                dense[idx(jj, jj)] += gcond;
                dense[idx(ii, jj)] -= gcond;
                dense[idx(jj, ii)] -= gcond;
            }
            (Some(ii), None) => {
                let pj = fixed_pressure(graph, edge.to).expect("fixed");
                dense[idx(ii, ii)] += gcond;
                b[ii] += gcond * pj;
            }
            (None, Some(jj)) => {
                let pi = fixed_pressure(graph, edge.from).expect("fixed");
                dense[idx(jj, jj)] += gcond;
                b[jj] += gcond * pi;
            }
            (None, None) => {}
        }
    }

    for r in 0..n {
        for c in 0..n {
            let v = dense[idx(r, c)];
            if v != 0.0 {
                coo.push(r, c, v);
            }
        }
    }

    (CsrMatrix::from(&coo), b, map)
}

fn pressures_from_reduced(
    graph: &VesselGraph,
    map: &[Option<usize>],
    p_free: &DVector<f64>,
) -> Vec<f64> {
    let mut pressure = vec![0.0; graph.nodes.len()];
    for (i, node) in graph.nodes.iter().enumerate() {
        match node.pressure_bc {
            PressureBc::Fixed(p) => pressure[i] = p,
            PressureBc::Free => {
                let k = map[i].expect("free node mapped");
                pressure[i] = p_free[k];
            }
        }
    }
    pressure
}

fn edge_flows(graph: &VesselGraph, pressure: &[f64]) -> Vec<f64> {
    graph
        .edges
        .iter()
        .map(|e| {
            let r = edge_resistance(graph, e);
            (pressure[e.from.0] - pressure[e.to.0]) / r
        })
        .collect()
}

/// Solve steady flows. Uses dense LU for the reduced system (exact for Phase 0 sizes).
pub fn solve_flows(graph: &VesselGraph) -> Result<FlowSolution, String> {
    let n_free = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.pressure_bc, PressureBc::Free))
        .count();
    if n_free == 0 {
        let pressure: Vec<f64> = graph
            .nodes
            .iter()
            .map(|n| match n.pressure_bc {
                PressureBc::Fixed(p) => p,
                PressureBc::Free => 0.0,
            })
            .collect();
        let flow = edge_flows(graph, &pressure);
        return Ok(FlowSolution { pressure, flow });
    }

    // Exercise sparse assembly even when we solve densely.
    let (csr, _b_sparse, _map_s) = assemble_sparse_csr(graph);
    assert_eq!(csr.nrows(), n_free);

    let (a, b, map) = assemble_dense(graph);
    let lu = a.lu();
    let p_free = lu.solve(&b).ok_or_else(|| "flow solve failed (singular A)".to_string())?;
    let pressure = pressures_from_reduced(graph, &map, &p_free);
    let flow = edge_flows(graph, &pressure);
    Ok(FlowSolution { pressure, flow })
}

/// Max |Σ flows| at free nodes (should be ~0).
pub fn max_mass_imbalance(graph: &VesselGraph, sol: &FlowSolution) -> f64 {
    let mut imb = vec![0.0; graph.nodes.len()];
    for (e_idx, edge) in graph.edges.iter().enumerate() {
        let q = sol.flow[e_idx];
        imb[edge.from.0] -= q;
        imb[edge.to.0] += q;
    }
    graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n.pressure_bc, PressureBc::Free))
        .map(|(i, _)| imb[i].abs())
        .fold(0.0_f64, f64::max)
}

pub fn edge_flow(sol: &FlowSolution, id: EdgeId) -> f64 {
    sol.flow[id.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::graph::VesselGraph;

    #[test]
    fn closed_loop_3_mass_balance_and_radius_effect() {
        let (mut g, _h, _c, _v, e_art, _e_ven, _e_ret) = VesselGraph::closed_loop_3();
        let sol = solve_flows(&g).expect("solve");
        assert!(
            max_mass_imbalance(&g, &sol) < 1e-9,
            "mass imbalance too large: {}",
            max_mass_imbalance(&g, &sol)
        );
        for q in &sol.flow {
            assert!(q.is_finite());
        }

        let q_before = edge_flow(&sol, e_art).abs();

        // Larger radius → lower R → higher |flow| on that edge (series-dominated path).
        g.edges[e_art.0].radius *= 1.5;
        let sol2 = solve_flows(&g).expect("solve2");
        let q_after = edge_flow(&sol2, e_art).abs();
        assert!(
            q_after > q_before * 1.05,
            "expected larger flow after radius↑: before={q_before} after={q_after}"
        );
        assert!(max_mass_imbalance(&g, &sol2) < 1e-9);
    }

    #[test]
    fn poiseuille_scales_as_r_to_minus_4() {
        let r1 = poiseuille_resistance(0.002, 0.1, ETA_BLOOD);
        let r2 = poiseuille_resistance(0.004, 0.1, ETA_BLOOD);
        // Doubling r → R /= 16.
        assert!((r1 / r2 - 16.0).abs() < 1e-9);
    }

    #[test]
    fn sparse_assembly_matches_dense_dims() {
        let (g, _, _, _, _, _, _) = VesselGraph::closed_loop_3();
        let (csr, b, map) = assemble_sparse_csr(&g);
        let n_free = map.iter().filter(|m| m.is_some()).count();
        assert_eq!(csr.nrows(), n_free);
        assert_eq!(csr.ncols(), n_free);
        assert_eq!(b.len(), n_free);
        assert!(n_free >= 1);
    }
}
