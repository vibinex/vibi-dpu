use std::{
    borrow::Borrow,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use serde::{Serialize, Deserialize};
// TODO, FIXME - remove all unwraps

use super::utils::generate_random_string;

#[derive(Debug, Default, Clone)]
pub struct MermaidSubgraph {
    name: String,
    nodes: HashMap<String, MermaidNode>,
    mermaid_id: String,
    color: String
}

impl MermaidSubgraph {
    // Constructor
    pub fn new(name: String) -> Self {
        let mermaid_id = generate_random_string(4);
        Self {
            name,
            nodes: HashMap::new(),
            mermaid_id,
            color: "".to_string()
        }
    }

    // Getter for nodes
    pub fn nodes(&self) -> &HashMap<String, MermaidNode> {
        &self.nodes
    }

    pub fn mermaid_id(&self) -> &String {
        &self.mermaid_id
    }

    pub fn set_color(&mut self, color: &str) {
        self.color = color.to_string();
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    // Setter for nodes
    pub fn set_nodes(&mut self, nodes: HashMap<String, MermaidNode>) {
        self.nodes = nodes;
    }

    pub fn add_node(&mut self, node: MermaidNode) {
        if self.nodes.contains_key(node.mermaid_id()) {
            log::error!(
                "[add_node] Node already exists: old - {:#?}, new - {:#?}",
                &self.nodes[node.mermaid_id()],
                node
            );
            return;
        }
        self.nodes.insert(node.mermaid_id().to_string(), node.to_owned());
    }

    pub fn get_node(&self, func_name: &str) -> Option<&MermaidNode> {
        self.nodes.get(func_name)
    }

    pub fn get_mut_node(&mut self, func_name: &str) -> Option<&mut MermaidNode> {
        self.nodes.get_mut(func_name)
    }

    pub fn render_subgraph(&self) -> String {
        let mut all_nodes = Vec::new();
        for (_, node) in self.nodes() {
            all_nodes.push(node.render_node());
        }
        let subgraph_str = format!(
            "\tsubgraph {} [{}]\n{}\nend\n",
            self.mermaid_id,
            self.name,
            all_nodes.join("\n")
        );
        subgraph_str
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct MermaidNode {
    function_name: String,
    mermaid_id: String,
    parent_id: String,
    color: String
}

impl MermaidNode {
    // Constructor
    pub fn new(function_name: String, parent_id: String) -> Self {
        let mermaid_id = generate_random_string(4);
        Self {
            mermaid_id,
            function_name,
            parent_id,
            color: "".to_string()
        }
    }

    pub fn color(&self) -> &String {
        &self.color
    }

    // Getter for mermaid_id
    pub fn mermaid_id(&self) -> &String {
        &self.mermaid_id
    }

    pub fn parent_id(&self) -> &String {
        &self.parent_id
    }

    pub fn set_color(&mut self, color: &str) {
        self.color = color.to_string()
    }

    // Setter for function_name
    pub fn set_function_name(&mut self, function_name: String) {
        self.function_name = function_name;
    }

    pub fn compare_and_change_color(&mut self, node_color: &str) {
        if (self.color() == "red" && node_color == "green") ||
        (self.color() == "green" && node_color == "red") {
            self.set_color("yellow");
        }
    }

    pub fn render_node(&self) -> String {
        // TODO FIXME - get line num or funcdef obj
        let node_str = format!("\t{}[{}]", &self.mermaid_id, &self.function_name);
        node_str
    }
}

#[derive(Debug, Default, Clone)]
pub struct MermaidEdge {
    line: usize,
    src_func_key: String,
    src_subgraph_key: String,
    dest_func_key: String,
    dest_subgraph_key: String,
    color: String
}

impl MermaidEdge {
    // Constructor
    pub fn new(
        line: usize,
        src_func_key: String,
        src_subgraph_key: String,
        dest_func_key: String,
        dest_subgraph_key: String,
        color: String,
    ) -> Self {
        Self {
            line,
            src_func_key,
            src_subgraph_key,
            dest_func_key,
            dest_subgraph_key,
            color,
        }
    }

    // Getter for line
    pub fn line(&self) -> usize {
        self.line
    }

    // Getter for color
    pub fn color(&self) -> &String {
        &self.color
    }

    // Getter for src_func_key
    pub fn src_func_key(&self) -> &String {
        &self.src_func_key
    }

    // Getter for src_subgraph_key
    pub fn src_subgraph_key(&self) -> &String {
        &self.src_subgraph_key
    }

    // Getter for dest_func_key
    pub fn dest_func_key(&self) -> &String {
        &self.dest_func_key
    }

    // Getter for dest_subgraph_key
    pub fn dest_subgraph_key(&self) -> &String {
        &self.dest_subgraph_key
    }

    // Setter for color
    pub fn set_color(&mut self, color: &str) {
        self.color = color.to_string();
    }

    pub fn compare_and_set_color(&mut self, edge_color: &str) {
       if (self.color() == "green" && edge_color == "red")  || 
        (self.color() == "red" && edge_color == "green") {
            self.set_color("yellow");
        }
    }

    pub fn add_edge_and_nodes(&mut self) {
        // add edge and source and destination nodes
    }

    pub fn get_edge_key(&self) -> String {
        let edge_key = format!(
            "{}/{}/{}/{}/{}", self.src_subgraph_key(), self.src_func_key(),
            self.line(),
            self.dest_subgraph_key(), self.dest_func_key()
        );
        return edge_key;
    }

    pub fn render_edge_definition(&self, subgraph_map: &HashMap<String, MermaidSubgraph>) -> String {
        let src_subgraph_opt = subgraph_map.get(&self.src_subgraph_key);
        if src_subgraph_opt.is_none() {
            log::debug!("[render_edge_definition] Unable to get subgraph: {}", &self.src_subgraph_key);
            return "".to_string();
        }
        let src_node_opt = src_subgraph_opt.expect("Empty src_subgraph_opt").nodes().get(&self.src_func_key);
        if src_node_opt.is_none() {
            log::debug!("[render_edge_definition] Unable to get node: {} in subgraph: {}", &self.src_func_key, &self.src_subgraph_key);
            return "".to_string();
        }
        let src_node = src_node_opt.expect("Empty src_node_opt");

        let dest_subgraph_opt = subgraph_map.get(&self.dest_subgraph_key);
        if dest_subgraph_opt.is_none() {
            log::debug!("[render_edge_definition] Unable to get subgraph: {}", &self.dest_subgraph_key);
            return "".to_string();
        }
        let dest_node_opt = dest_subgraph_opt.expect("Empty src_subgraph_opt").nodes().get(&self.dest_func_key);
        if dest_node_opt.is_none() {
            log::debug!("[render_edge_definition] Unable to get node: {} in subgraph: {}", &self.dest_func_key, &self.dest_subgraph_key);
            return "".to_string();
        }
        let dest_node = dest_node_opt.expect("Empty src_node_opt");
        let edge_str = format!("\t{} -- Line {} --> {}\n", src_node.mermaid_id(), self.line, dest_node.mermaid_id());
        edge_str
    }

    pub fn render_edge_style(&self) -> String {
        let style_str = format!("stroke:{},stroke-width:4px;", self.color());
        style_str
    }
}

#[derive(Debug, Default, Clone)]
pub struct MermaidGraphElements {
    edges: HashMap<String, MermaidEdge>,
    subgraphs: HashMap<String, MermaidSubgraph>,
}

impl MermaidGraphElements {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            subgraphs: HashMap::new(),
        }
    }

    pub fn add_edge(
        &mut self,
        edge_color: &str,
        calling_line_num: usize,
        source_func_name: &str,
        dest_func_name: &str,
        source_file: &str,
        dest_file: &str,
        source_color: &str,
        dest_color: &str
    ) {        
        self.create_node(source_file, source_func_name, source_color);
        self.create_node(dest_file, dest_func_name, dest_color);
        let edge = MermaidEdge::new(
            calling_line_num,
            source_func_name.to_string(),
            source_file.to_string(),
            dest_func_name.to_string(),
            dest_file.to_string(),
            edge_color.to_string());
        self.add_edge_to_edges(edge);
    }

    fn create_node(&mut self, subgraph_key: &str, node_func_name: &str, node_color: &str) {
        if let Some(subgraph) = self.subgraphs.get_mut(subgraph_key) {
            if let Some(node) = subgraph.get_mut_node(node_func_name) {
                node.compare_and_change_color(node_color);
            } else {
                let mut node = MermaidNode::new(node_func_name.to_string(),
                subgraph.mermaid_id().to_string());
                node.set_color(node_color);
                subgraph.add_node(node);
            }
        } else {
            let mut subgraph = MermaidSubgraph::new(subgraph_key.to_string());
            let mut node = MermaidNode::new(node_func_name.to_string(),
            subgraph.mermaid_id().to_string());
            node.set_color(node_color);
            subgraph.add_node(node);
            self.add_subgraph(subgraph);
        }
    }

    fn add_subgraph(&mut self, subgraph: MermaidSubgraph) {
        if !self.subgraphs.contains_key(subgraph.name()) {
            self.subgraphs.insert(subgraph.name().to_string(), subgraph);
        }
    }

    fn add_edge_to_edges(&mut self, edge: MermaidEdge) {
        let edge_key = edge.get_edge_key();
        if let Some(edge_mut) = self.edges.get_mut(&edge_key) {
            edge_mut.compare_and_set_color(edge.color());
            return;
        }
        self.edges.insert(edge_key, edge);
    }

    fn render_edges(&self) -> String {
        let mut all_edges = Vec::<String>::new();
        let mut all_edges_style = Vec::<String>::new();
        for (idx, (_, edge)) in self.edges.iter().enumerate() {
            all_edges.push(edge.render_edge_definition(&self.subgraphs));
            all_edges_style.push(format!("\tlinkStyle {} {}", idx, edge.render_edge_style()));
        }
        let all_edges_str = format!("{}{}", all_edges.join("\n"), all_edges_style.join("\n"));
        all_edges_str
    }

    fn render_subgraphs(&self) -> String {
        self.subgraphs
            .values()
            .map(|subgraph| subgraph.render_subgraph())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn render_elements(&self) -> String {
        let all_elements_str = format!("{}\n{}", &self.render_subgraphs(), &self.render_edges());
        all_elements_str
    }
}
