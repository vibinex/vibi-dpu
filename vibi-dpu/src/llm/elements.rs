use std::{borrow::BorrowMut, collections::HashMap};
use serde::{Serialize, Deserialize};

use super::utils::generate_random_string;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct MermaidSubgraph {
    name: String,
    nodes: HashMap<String, MermaidNode>,
    mermaid_id: String
}

impl MermaidSubgraph {
    // Constructor
    pub fn new(name: String, nodes: HashMap<String, MermaidNode>) -> Self {
        let mermaid_id = generate_random_string(4);
        Self { name, nodes, mermaid_id }
    }

    // Getter for nodes
    pub fn nodes(&self) -> &HashMap<String, MermaidNode> {
        &self.nodes
    }

    // Setter for nodes
    pub fn set_nodes(&mut self, nodes: HashMap<String, MermaidNode>) {
        self.nodes = nodes;
    }

    pub fn add_node(&mut self, node: MermaidNode) {
        if self.nodes.contains_key(node.function_name()) {
            log::error!(
                "[add_node] Node already exists: old - {:#?}, new - {:#?}",
                &self.nodes[node.function_name()], &node);
            return;
        }
        self.nodes.insert(node.function_name.to_string(), node);
    }

    pub fn render_subgraph(&self) -> String{
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
        // self.set_subgraph_str(Some(subgraph_str));
        return subgraph_str;
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct MermaidNode {
    function_name: String,
    mermaid_id: String,
}

impl MermaidNode {
    // Constructor
    pub fn new( function_name: String) -> Self {
        let mermaid_id = generate_random_string(4);
        Self { mermaid_id, function_name }
    }

    // Getter for function_name
    pub fn function_name(&self) -> &String {
        &self.function_name
    }

    // Getter for mermaid_id
    pub fn mermaid_id(&self) -> &String {
        &self.mermaid_id
    }

    // Setter for function_name
    pub fn set_function_name(&mut self, function_name: String) {
        self.function_name = function_name;
    }

    pub fn render_node(&self) -> String {
        let node_str = format!("\t{}[{}]", &self.mermaid_id, &self.function_name);
        // self.set_node_str(Some(node_str.clone()));
        return node_str;
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct MermaidEdge {
    line: usize,
    caller_function: MermaidNode,
    called_function: MermaidNode,
    color: String,
}

impl MermaidEdge {
    // Constructor
    pub fn new(line: usize, caller_function: MermaidNode, called_function: MermaidNode, color: String) -> Self {
        Self { line, caller_function, called_function, color }
    }

    // Getter for edge_str
    pub fn line(&self) -> usize {
        self.line
    }

    // Getter for caller_function
    pub fn caller_function(&self) -> &MermaidNode {
        &self.caller_function
    }

    // Setter for caller_function
    pub fn set_caller_function(&mut self, caller_function: MermaidNode) {
        self.caller_function = caller_function;
    }

    // Getter for called_function
    pub fn called_function(&self) -> &MermaidNode {
        &self.called_function
    }

    // Setter for called_function
    pub fn set_called_function(&mut self, called_function: MermaidNode) {
        self.called_function = called_function;
    }

    // Getter for color
    pub fn color(&self) -> &String {
        &self.color
    }

    // Setter for color
    pub fn set_color(&mut self, color: String) {
        self.color = color;
    }

    pub fn render_edge_definition(&self) -> String {
        let edge_str = format!(
            "\t{} -- Line {} --> {}\n",
            self.caller_function().mermaid_id(),
            self.line,
            self.called_function().mermaid_id(),
        );
        return edge_str;
    }

    pub fn render_edge_style(&self) -> String {
        let style_str = format!(
            "stroke:{},stroke-width:4px;",
            self.color()
        );
        return style_str;
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct MermaidEdges {
    all_edges: Vec<MermaidEdge>,
}

impl MermaidEdges {
    pub fn new(all_edges: Vec<MermaidEdge>) -> Self {
        MermaidEdges {all_edges }
    }

    pub fn all_edges(&self) -> &Vec<MermaidEdge> {
        return &self.all_edges;
    }

    pub fn add_edge(&mut self, edge: MermaidEdge) {
        self.all_edges.push(edge);
    }

    pub fn render_edges(&self) -> String {
        let mut all_edges = Vec::<String>::new();
        let mut all_edges_style = Vec::<String>::new();
        for (idx, edge) in self.all_edges().iter().enumerate() {
            all_edges.push(edge.render_edge_definition());
            all_edges_style.push(
                format!("\tlinkStyle {} {}", idx, edge.render_edge_style())
            );
        }
        let all_edges_str = format!(
            "{}{}",
            all_edges.join("\n"),
            all_edges_style.join("\n")
        );
        return all_edges_str;
    }
}