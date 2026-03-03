use im_rc::{HashMap, Vector};

pub trait ExtNodeDataTrait {}

pub trait EdgeDataTrait<ExtNodeData>: Clone {
    fn update(&mut self, other: &Self) -> bool;
    fn expand(self, hole: &ExtNodeData, ind: TypeNodeInd) -> Self;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeNodeInd(pub usize);

#[derive(Clone, Debug)]
struct ReachabilityNode<ExtNodeData, ExtEdgeData> {
    data: ExtNodeData,
    flows_from: HashMap<TypeNodeInd, ExtEdgeData>,
    flows_to: HashMap<TypeNodeInd, ExtEdgeData>,
}

#[derive(Clone)]
pub struct Reachability<ExtNodeData, ExtEdgeData> {
    nodes: Vector<ReachabilityNode<ExtNodeData, ExtEdgeData>>,
}
impl<ExtNodeData: ExtNodeDataTrait + Clone, ExtEdgeData: EdgeDataTrait<ExtNodeData>>
    Reachability<ExtNodeData, ExtEdgeData>
{
    pub fn new() -> Self {
        Self {
            nodes: Vector::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, i: TypeNodeInd) -> Option<&ExtNodeData> {
        self.nodes.get(i.0).map(|rn| &rn.data)
    }

    pub fn get_mut(&mut self, i: TypeNodeInd) -> Option<&mut ExtNodeData> {
        self.nodes.get_mut(i.0).map(|rn| &mut rn.data)
    }

    pub fn get_edge(&self, lhs: TypeNodeInd, rhs: TypeNodeInd) -> Option<&ExtEdgeData> {
        self.nodes.get(lhs.0).and_then(|rn| rn.flows_to.get(&rhs))
    }

    pub fn add_node(&mut self, data: ExtNodeData) -> TypeNodeInd {
        let i = self.len();
        let n = ReachabilityNode {
            data,
            flows_from: HashMap::new(),
            flows_to: HashMap::new(),
        };
        self.nodes.push_back(n);
        TypeNodeInd(i)
    }

    fn update_edge_value(
        &mut self,
        lhs: TypeNodeInd,
        rhs: TypeNodeInd,
        val: ExtEdgeData,
    ) {
        // Update flows_to on lhs node
        let lhs_node = self.nodes.get_mut(lhs.0).unwrap();
        lhs_node.flows_to.insert(rhs, val.clone());

        // Update flows_from on rhs node
        let rhs_node = self.nodes.get_mut(rhs.0).unwrap();
        rhs_node.flows_from.insert(lhs, val);
    }

    pub fn add_edge(
        &mut self,
        lhs: TypeNodeInd,
        rhs: TypeNodeInd,
        edge_val: ExtEdgeData,
        out: &mut Vec<(TypeNodeInd, TypeNodeInd, ExtEdgeData)>,
    ) {
        let mut work = vec![(lhs, rhs, edge_val)];

        while let Some((lhs, rhs, mut edge_val)) = work.pop() {
            let old_edge = self.nodes.get(lhs.0).unwrap().flows_to.get(&rhs).cloned();
            match old_edge {
                Some(mut old) => {
                    if old.update(&edge_val) {
                        edge_val = old;
                    } else {
                        continue;
                    }
                }
                None => {}
            };
            self.update_edge_value(lhs, rhs, edge_val.clone());

            // Collect ancestors and descendants before mutating
            // Sort to ensure deterministic iteration order (HashMap iteration is unordered)
            let mut lhs_ancestors: Vec<TypeNodeInd> = self.nodes.get(lhs.0).unwrap()
                .flows_from.keys().copied().collect();
            lhs_ancestors.sort();
            let mut rhs_descendants: Vec<TypeNodeInd> = self.nodes.get(rhs.0).unwrap()
                .flows_to.keys().copied().collect();
            rhs_descendants.sort();

            let temp = edge_val.clone().expand(&self.nodes.get(lhs.0).unwrap().data, lhs);
            for lhs2 in lhs_ancestors {
                work.push((lhs2, rhs, temp.clone()));
            }

            let temp = edge_val.clone().expand(&self.nodes.get(rhs.0).unwrap().data, rhs);
            for rhs2 in rhs_descendants {
                work.push((lhs, rhs2, temp.clone()));
            }

            out.push((lhs, rhs, edge_val));
        }
    }
}
