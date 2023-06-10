use std::collections::VecDeque;
use std::fmt::Debug;

use serde::Serialize;
use serde_json::{json, Value};

pub trait Node {
    fn is_root(&self, pid: i64) -> bool;
    fn get_pid(&self) -> i64;
    fn get_id(&self) -> i64;
    fn get_data(&self) -> Value;
}

pub trait NodeTrait<T: Clone + Node> {
    fn build_tree(data: &mut [T], pid: i64) -> Vec<Value> {
        let mut tree: Vec<Value> = Vec::new();
        if data.len() == 0 {
            return tree;
        }

        let mut stack: VecDeque<(T, usize)> = VecDeque::new();
        stack.push_back((data[0].clone(), 0));

        while let Some((node, index)) = stack.pop_back() {
            if node.is_root(pid) {
                let children = Self::build_tree(&mut data[index + 1..], node.get_id());
                let mut item = node.get_data();
                item["children"] = json!(children);
                tree.push(item);
            }

            if index + 1 < data.len() {
                stack.push_back((data[index + 1].clone(), index + 1));
            }
        }

        tree
    }
}
