use std;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub children: Vec<Node>,
}
impl Node {
    fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

pub type BoxIter = Box<Iterator<Item = Node>>;

pub fn combine(iter: BoxIter) -> Node {
    let mut concat = String::new();
    for node in iter {
        concat.push_str(&node.name);
    }
    Node {
        name: concat,
        children: vec![],
    }
}

pub fn map_tree(root: Node, func: Rc<Fn(BoxIter) -> Node>) -> Node {
    let node_iter = NodeIterator {
        children: root.children.into_iter(),
        func: func.clone(),
    };
    let node_iter_box = Box::new(node_iter);
    func(node_iter_box)
}

pub struct NodeIterator<F> {
    children: std::vec::IntoIter<Node>,
    func: F,
}

impl Iterator for NodeIterator<Rc<Fn(BoxIter) -> Node>> {
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        let cur_node = self.children.next();
        let cur_node = match cur_node {
            None => return None,
            Some(node) => node,
        };
        if cur_node.is_empty() {
            Some(cur_node)
        } else {
            Some(map_tree(cur_node, self.func.clone()))
        }
    }
}
