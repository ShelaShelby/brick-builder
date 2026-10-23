//! Step-by-step instruction generation.
//!
//! Bricks are placed bottom to top. A brick only becomes available once it
//! has a stud joint with something already placed, so every step can really
//! be built. Bricks are then grouped into steps whose size grows with the
//! size of the build.

use crate::connect;
use crate::model::Brick;
use crate::parts::Part;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Debug, Default)]
pub struct Instructions {
    /// Each step lists indices into the model's brick list, in placement order.
    pub steps: Vec<Vec<usize>>,
}

impl Instructions {
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Step index in which each brick is placed.
    pub fn step_of(&self, n: usize) -> Vec<usize> {
        let mut v = vec![usize::MAX; n];
        for (s, step) in self.steps.iter().enumerate() {
            for &i in step {
                v[i] = s;
            }
        }
        v
    }
}

/// Bricks per step for a build of `n` bricks.
pub fn step_size(n: usize) -> usize {
    (((n as f32).sqrt() * 0.5).round() as usize).clamp(2, 30)
}

pub fn generate(bricks: &[Brick]) -> Instructions {
    let n = bricks.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, b, _) in connect::joints(bricks) {
        adj[a].push(b);
        adj[b].push(a);
    }
    // Sort key: section, layer, then a coarse block serpentine so a step stays local.
    let key = |i: usize| {
        let b = &bricks[i];
        let bz = b.z.div_euclid(8);
        let bx = b.x.div_euclid(8);
        let bx = if bz % 2 == 0 { bx } else { -bx };
        (b.group, b.y, bz, bx, b.z, b.x, i)
    };

    let mut placed = vec![false; n];
    let mut queued = vec![false; n];
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut heap = BinaryHeap::new();

    let mut steps: Vec<Vec<usize>> = Vec::new();
    let baseplates: Vec<usize> = (0..n).filter(|&i| bricks[i].part == Part::Baseplate).collect();
    for &i in &baseplates {
        placed[i] = true;
        queued[i] = true;
    }
    if !baseplates.is_empty() {
        steps.push(baseplates.clone());
    }
    for &i in &baseplates {
        for &j in &adj[i] {
            if !queued[j] {
                queued[j] = true;
                heap.push(Reverse(key(j)));
            }
        }
    }
    let mut remaining: Vec<usize> = (0..n).filter(|&i| !placed[i]).collect();
    remaining.sort_by_key(|&i| key(i));
    let mut rem_pos = 0;
    loop {
        let next = if let Some(Reverse(k)) = heap.pop() {
            k.6
        } else {
            // Nothing connected is left: continue with the lowest loose brick.
            while rem_pos < remaining.len() && placed[remaining[rem_pos]] {
                rem_pos += 1;
            }
            if rem_pos >= remaining.len() {
                break;
            }
            let i = remaining[rem_pos];
            queued[i] = true;
            i
        };
        if placed[next] {
            continue;
        }
        placed[next] = true;
        order.push(next);
        for &j in &adj[next] {
            if !queued[j] {
                queued[j] = true;
                heap.push(Reverse(key(j)));
            }
        }
    }

    let t = step_size(order.len());
    let mut cur: Vec<usize> = Vec::new();
    for i in order {
        if let Some(&last) = cur.last() {
            let lb: &Brick = &bricks[last];
            let b = &bricks[i];
            let new_section = lb.group != b.group;
            let new_layer = b.y != lb.y && cur.len() * 3 >= t * 2;
            if cur.len() >= t || new_section || new_layer {
                steps.push(std::mem::take(&mut cur));
            }
        }
        cur.push(i);
    }
    if !cur.is_empty() {
        steps.push(cur);
    }
    Instructions { steps }
}

/// Check that every brick appears exactly once and is attached to something
/// already built at the moment it is placed.
pub fn validate(bricks: &[Brick], ins: &Instructions) -> Result<(), String> {
    let n = bricks.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, b, _) in connect::joints(bricks) {
        adj[a].push(b);
        adj[b].push(a);
    }
    let mut placed = vec![false; n];
    for (s, step) in ins.steps.iter().enumerate() {
        if step.is_empty() {
            return Err(format!("step {} is empty", s + 1));
        }
        for &i in step {
            if i >= n || placed[i] {
                return Err(format!("brick {i} placed twice or out of range"));
            }
            let ok = bricks[i].part == Part::Baseplate || adj[i].iter().any(|&j| placed[j]);
            if !ok {
                return Err(format!("step {}: brick {} has no support", s + 1, i));
            }
            placed[i] = true;
        }
    }
    if let Some(i) = placed.iter().position(|p| !p) {
        return Err(format!("brick {i} never placed"));
    }
    Ok(())
}
