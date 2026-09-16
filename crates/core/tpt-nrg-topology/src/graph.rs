//! Graph and shortest-path utilities.

use std::collections::{HashMap, VecDeque};

use tpt_nrg_core::EnergySystem;

use crate::TopologyResult;

/// Adjacency-list representation of the power-system network.
///
/// Buses are stored in a dense, 0-indexed `Vec` keyed by [`NetworkTopology::bus_id_to_index`].
#[derive(Debug, Clone)]
pub struct NetworkTopology {
    /// Bus ids in dense, 0-indexed order.
    pub bus_ids: Vec<usize>,
    /// Map from bus id to dense index.
    pub bus_id_to_index: HashMap<usize, usize>,
    /// Adjacency list: `adjacency[i] = vec![(neighbor_index, branch_id)]`.
    pub adjacency: Vec<Vec<(usize, usize)>>,
}

impl NetworkTopology {
    /// Build a topology from an [`EnergySystem`].
    #[must_use]
    pub fn from_energy_system(sys: &EnergySystem) -> Self {
        let bus_ids: Vec<usize> = sys.buses.iter().map(|b| b.id).collect();
        let bus_id_to_index: HashMap<usize, usize> =
            bus_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let mut adjacency = vec![Vec::new(); bus_ids.len()];
        for br in &sys.branches {
            if !br.in_service {
                continue;
            }
            if let (Some(&i), Some(&j)) = (
                bus_id_to_index.get(&br.from_bus),
                bus_id_to_index.get(&br.to_bus),
            ) {
                adjacency[i].push((j, br.id));
                adjacency[j].push((i, br.id));
            }
        }
        Self {
            bus_ids,
            bus_id_to_index,
            adjacency,
        }
    }

    /// Number of buses.
    #[must_use]
    pub fn n_buses(&self) -> usize {
        self.bus_ids.len()
    }

    /// Find all connected components ("islands") of the network.
    ///
    /// Each component is returned as a `Vec<usize>` of bus ids.
    #[must_use]
    pub fn find_islands(&self) -> Vec<Vec<usize>> {
        let n = self.n_buses();
        let mut visited = vec![false; n];
        let mut components: Vec<Vec<usize>> = Vec::new();
        for start in 0..n {
            if visited[start] {
                continue;
            }
            let mut queue = VecDeque::from([start]);
            visited[start] = true;
            let mut component = Vec::new();
            while let Some(node) = queue.pop_front() {
                component.push(self.bus_ids[node]);
                for &(neighbor, _) in &self.adjacency[node] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
            component.sort_unstable();
            components.push(component);
        }
        components
    }

    /// Find the shortest path between two buses using BFS (unweighted).
    ///
    /// Returns `None` if no path exists.
    #[must_use]
    pub fn find_shortest_path(&self, from_bus: usize, to_bus: usize) -> Option<PathResult> {
        let start = *self.bus_id_to_index.get(&from_bus)?;
        let goal = *self.bus_id_to_index.get(&to_bus)?;
        let n = self.n_buses();
        let mut visited = vec![false; n];
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; n];
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        while let Some(node) = queue.pop_front() {
            if node == goal {
                break;
            }
            for &(neighbor, branch) in &self.adjacency[node] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    prev[neighbor] = Some((node, branch));
                    queue.push_back(neighbor);
                }
            }
        }
        if !visited[goal] {
            return None;
        }
        let mut path_buses = Vec::new();
        let mut path_branches = Vec::new();
        let mut cur = goal;
        while cur != start {
            let (p, br) = prev[cur].unwrap();
            path_buses.push(self.bus_ids[cur]);
            path_branches.push(br);
            cur = p;
        }
        path_buses.push(self.bus_ids[start]);
        path_buses.reverse();
        path_branches.reverse();
        Some(PathResult {
            buses: path_buses,
            branches: path_branches,
        })
    }
}

/// Result of a shortest-path query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathResult {
    /// Bus ids along the path, in order.
    pub buses: Vec<usize>,
    /// Branch ids along the path (length = `buses.len() - 1`).
    pub branches: Vec<usize>,
}

/// Convenience: wrap a result in the crate-local error type.
#[allow(dead_code)]
pub(crate) fn _ensure<T>(r: TopologyResult<T>) -> TopologyResult<T> {
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Branch, Bus, BusType, Generator, GeneratorType};

    fn small_system() -> EnergySystem {
        let mut sys = EnergySystem::new("t", "T", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq)).unwrap();
        sys.add_bus(Bus::new(3, "B3", BusType::Pq)).unwrap();
        sys.add_bus(Bus::new(4, "B4", BusType::Pq)).unwrap();
        sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.05))
            .unwrap();
        sys.add_branch(Branch::new(2, "L23", 2, 3, 0.01, 0.05))
            .unwrap();
        sys.add_branch(Branch::new(3, "L34", 3, 4, 0.01, 0.05))
            .unwrap();
        sys.add_generator(Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 0.0).at_bus(1))
            .unwrap();
        sys
    }

    #[test]
    fn island_detection_single() {
        let topo = NetworkTopology::from_energy_system(&small_system());
        let islands = topo.find_islands();
        assert_eq!(islands.len(), 1);
        assert_eq!(islands[0].len(), 4);
    }

    #[test]
    fn island_detection_split() {
        let mut sys = small_system();
        // Disconnect bus 4 by taking its branch out of service.
        for br in &mut sys.branches {
            if br.id == 3 {
                br.in_service = false;
            }
        }
        let topo = NetworkTopology::from_energy_system(&sys);
        let mut islands = topo.find_islands();
        islands.sort_by_key(|c| c.len());
        assert_eq!(islands.len(), 2);
        assert_eq!(islands[0], vec![4]);
        assert_eq!(islands[1], vec![1, 2, 3]);
    }

    #[test]
    fn shortest_path() {
        let topo = NetworkTopology::from_energy_system(&small_system());
        let p = topo.find_shortest_path(1, 4).unwrap();
        assert_eq!(p.buses, vec![1, 2, 3, 4]);
        assert_eq!(p.branches, vec![1, 2, 3]);
    }

    #[test]
    fn no_path() {
        let mut sys = small_system();
        for br in &mut sys.branches {
            br.in_service = false;
        }
        let topo = NetworkTopology::from_energy_system(&sys);
        assert!(topo.find_shortest_path(1, 4).is_none());
    }
}
