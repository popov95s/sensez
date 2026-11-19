pub(super) const MAX_TREE_DEPTH: usize = 512;

pub(super) fn tree_depth(root: tree_sitter::Node, limit: usize) -> usize {
    let mut cursor = root.walk();
    let mut depth = TreeDepth::default();
    loop {
        if cursor.goto_first_child() {
            if depth.descend() > limit {
                return depth.max();
            }
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return depth.max();
            }
            depth.ascend();
        }
    }
}

#[derive(Default)]
struct TreeDepth {
    current: usize,
    max: usize,
}

impl TreeDepth {
    fn descend(&mut self) -> usize {
        self.current += 1;
        self.max = self.max.max(self.current);
        self.max
    }

    fn ascend(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    fn max(&self) -> usize {
        self.max
    }
}
