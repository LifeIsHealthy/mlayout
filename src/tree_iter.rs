use std;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
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

pub fn combine(iter: BoxIter) -> BoxIter {
    let mut vec: Vec<Node> = Vec::new();
    let mut concat = String::new();
    let mut add_concat = false;
    for node in iter {
        if node.is_empty() {
            concat.push_str(&node.name);
            add_concat = true;
        } else {
            if add_concat {
                let concat_node = Node {
                    name: concat.clone(),
                    children: vec![],
                };
                concat.clear();
                vec.push(concat_node);
                add_concat = false;
            }
            vec.push(node)
        }
    }
    if add_concat {
        let concat_node = Node {
            name: concat,
            children: vec![],
        };
        vec.push(concat_node);
    }
    Box::new(vec.into_iter())
}

pub fn map_tree(root: Node, func: Rc<Fn(BoxIter) -> BoxIter>) -> Node {
    let Node { name, children } = root;
    let node_iter = NodeIterator {
        children: children.into_iter(),
        func: func.clone(),
    };
    let node_iter_box = Box::new(node_iter);
    let node_vec: Vec<_> = func(node_iter_box).collect();
    Node {
        name: name,
        children: node_vec,
    }
}

pub struct NodeIterator<F> {
    children: std::vec::IntoIter<Node>,
    func: F,
}

impl Iterator for NodeIterator<Rc<Fn(BoxIter) -> BoxIter>> {
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        match self.children.next() {
            None => None,
            Some(node) => Some(map_tree(node, self.func.clone())),
        }
    }
}
