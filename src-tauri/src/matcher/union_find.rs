use std::collections::HashMap;

pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]); // path compression
        }
        self.parent[x]
    }

    pub fn union(&mut self, x: usize, y: usize) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        // Union by rank
        match self.rank[rx].cmp(&self.rank[ry]) {
            std::cmp::Ordering::Less => self.parent[rx] = ry,
            std::cmp::Ordering::Greater => self.parent[ry] = rx,
            std::cmp::Ordering::Equal => {
                self.parent[ry] = rx;
                self.rank[rx] += 1;
            }
        }
    }

    pub fn groups(&mut self) -> HashMap<usize, Vec<usize>> {
        let n = self.parent.len();
        let mut map: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..n {
            let root = self.find(i);
            map.entry(root).or_default().push(i);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_singleton_groups() {
        let mut uf = UnionFind::new(3);
        let groups = uf.groups();
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_union_merges_groups() {
        let mut uf = UnionFind::new(4);
        uf.union(0, 1);
        uf.union(2, 3);
        let groups = uf.groups();
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_transitive_closure() {
        let mut uf = UnionFind::new(3);
        uf.union(0, 1);
        uf.union(1, 2);
        let groups = uf.groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.values().next().unwrap().len(), 3);
    }
}
