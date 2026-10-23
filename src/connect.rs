//! Connection check.
//!
//! Two parts are connected only where a stud on top of the lower part sits in
//! a socket on the underside of the upper part (same grid cell, top of one at
//! exactly the bottom of the other). Side-by-side contact holds nothing.
//!
//! The graph is rooted at the baseplates. Parts that cannot reach a baseplate
//! are *floating*. Parts that hang on the rest of the build by one single stud
//! joint (a bridge edge of weight one) are *weak*.

use crate::model::Brick;
use crate::parts::Part;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Weak,
    Floating,
}

#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub status: Vec<Status>,
    /// Neighbours of each brick with the number of stud joints between them.
    pub adj: Vec<Vec<(usize, u32)>>,
    pub joints: usize,
    pub weak: usize,
    pub floating: usize,
}

impl Analysis {
    pub fn flagged(&self) -> usize {
        self.weak + self.floating
    }
}

/// Stud joints between pairs of bricks: (lower, upper, count).
pub fn joints(bricks: &[Brick]) -> Vec<(usize, usize, u32)> {
    let mut sockets: HashMap<(i32, i32, i32), usize> = HashMap::with_capacity(bricks.len() * 6);
    for (i, b) in bricks.iter().enumerate() {
        for (x, z) in b.socket_world() {
            sockets.insert((x, z, b.y), i);
        }
    }
    let mut pairs: HashMap<(usize, usize), u32> = HashMap::new();
    for (i, b) in bricks.iter().enumerate() {
        let top = b.top();
        for (x, z) in b.stud_world() {
            if let Some(&j) = sockets.get(&(x, z, top)) {
                if j != i {
                    *pairs.entry((i, j)).or_default() += 1;
                }
            }
        }
    }
    let mut v: Vec<_> = pairs.into_iter().map(|((a, b), n)| (a, b, n)).collect();
    v.sort_unstable();
    v
}

pub fn analyze(bricks: &[Brick]) -> Analysis {
    let n = bricks.len();
    let mut adj: Vec<Vec<(usize, u32)>> = vec![Vec::new(); n];
    let mut total = 0usize;
    for (a, b, k) in joints(bricks) {
        adj[a].push((b, k));
        adj[b].push((a, k));
        total += k as usize;
    }

    // Graph with a virtual root (index n) tied firmly to every baseplate.
    let root = n;
    let mut g = adj.clone();
    g.push(Vec::new());
    for (i, b) in bricks.iter().enumerate() {
        if b.part == Part::Baseplate {
            g[root].push((i, u32::MAX));
            g[i].push((root, u32::MAX));
        }
    }

    // Iterative DFS from the root: discovery order, low-link, subtree size.
    let mut disc = vec![usize::MAX; n + 1];
    let mut low = vec![0usize; n + 1];
    let mut order: Vec<usize> = Vec::with_capacity(n + 1); // vertex by discovery time
    let mut size = vec![1usize; n + 1];
    let mut weak_roots: Vec<usize> = Vec::new();
    // frame: (vertex, parent, weight of edge from parent, next neighbour idx)
    let mut stack: Vec<(usize, usize, u32, usize)> = Vec::new();
    disc[root] = 0;
    low[root] = 0;
    order.push(root);
    stack.push((root, usize::MAX, 0, 0));
    while let Some(top) = stack.last_mut() {
        let (v, parent, _, ref mut idx) = *top;
        if *idx < g[v].len() {
            let (w, _wt) = g[v][*idx];
            let wt = g[v][*idx].1;
            *idx += 1;
            if w == parent {
                continue;
            }
            if disc[w] == usize::MAX {
                disc[w] = order.len();
                low[w] = disc[w];
                order.push(w);
                stack.push((w, v, wt, 0));
            } else {
                low[v] = low[v].min(disc[w]);
            }
        } else {
            let (v, parent, wt, _) = stack.pop().unwrap();
            if parent != usize::MAX {
                low[parent] = low[parent].min(low[v]);
                size[parent] += size[v];
                if low[v] > disc[parent] && wt == 1 {
                    weak_roots.push(v);
                }
            }
        }
    }

    let mut status = vec![Status::Floating; n];
    for &v in &order {
        if v < n {
            status[v] = Status::Ok;
        }
    }
    for r in weak_roots {
        let start = disc[r];
        for &v in &order[start..start + size[r]] {
            if v < n {
                status[v] = Status::Weak;
            }
        }
    }
    let weak = status.iter().filter(|s| **s == Status::Weak).count();
    let floating = status.iter().filter(|s| **s == Status::Floating).count();
    Analysis { status, adj, joints: total, weak, floating }
}
