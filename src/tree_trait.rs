// use std;
// use std::rc::Rc;
// use std::iter;
//
// trait Tree<I>: Sized where I: Iterator<Item=Self> {
//     fn children(self) -> I;
//     fn is_leaf(&self) -> bool;
// }
//
// #[derive(Debug, PartialEq, Eq)]
// pub struct Node {
//     pub name: String,
//     pub children: Vec<Node>,
// }
// impl Node {
//     fn is_empty(&self) -> bool {
//         self.children.is_empty()
//     }
// }
//
// impl Tree<std::vec::IntoIter<Node>> for Node {
//     fn children(self) -> std::vec::IntoIter<Node> {
//         self.children.into_iter()
//     }
//     fn is_leaf(&self) -> bool {
//         self.is_empty()
//     }
// }
//
// pub type BoxIter = Box<Iterator<Item=Node>>;
//
// pub fn combine(iter: BoxIter) -> BoxIter {
//     let mut concat = String::new();
//     for node in iter {
//         if node.is_empty() {
//             concat.push_str(&node.name);
//         }
//     }
//     let concat_node = Node{ name: concat, children: vec![] };
//     let concat_node_iter = iter::once(concat_node);
//     Box::new(concat_node_iter)
// }
//
// pub fn map_tree<I, N>(root: N, func: Rc<Fn(Box<Iterator<Item=N>>) -> Box<Iterator<Item=N>>>) -> N where I: Iterator<Item=N>, N: Tree<I> {
//     let node_iter = NodeIterator{ children: root.children(), func: func.clone() };
//     let node_iter_box = Box::new(node_iter);
//     let node_vec: Vec<_> = func(node_iter_box).collect();
//     Node { name: name, children: node_vec }
// }
//
// pub struct NodeIterator<F> {
//     children: std::vec::IntoIter<Node>,
//     func: F,
// }
//
// impl Iterator for NodeIterator<Rc<Fn(BoxIter) -> BoxIter>> {
//     type Item = Node;
//
//     fn next(&mut self) -> Option<Node> {
//         let cur_node = self.children.next();
//         let cur_node = match cur_node { None => return None, Some(node) => node };
//         if cur_node.is_empty() {
//             Some(cur_node)
//         } else {
//             Some(map_tree(cur_node, self.func.clone()))
//         }
//     }
// }
