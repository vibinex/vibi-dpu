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
    nodes: HashMap<String, Arc<Mutex<MermaidNode>>>,
    mermaid_id: String,
}

impl MermaidSubgraph {
    // Constructor
    pub fn new(name: String) -> Self {
        let mermaid_id = generate_random_string(4);
        Self {
            name,
            nodes: HashMap::new(),
            mermaid_id,
        }
    }

    // Getter for nodes
    pub fn nodes(&self) -> &HashMap<String, Arc<Mutex<MermaidNode>>> {
        &self.nodes
    }

    pub fn mermaid_id(&self) -> &String {
        &self.mermaid_id
    }

    // Setter for nodes
    pub fn set_nodes(&mut self, nodes: HashMap<String, Arc<Mutex<MermaidNode>>>) {
        self.nodes = nodes;
    }

    pub fn add_node(&mut self, node: &Arc<Mutex<MermaidNode>>) {
        let node_owned = Arc::clone(node);
        let function_name = {
            let node_borrowed = node_owned.lock().unwrap();
            node_borrowed.function_name().to_string()
        };
        if self.nodes.contains_key(&function_name) {
            log::error!(
                "[add_node] Node already exists: old - {:#?}, new - {:#?}",
                &self.nodes[&function_name],
                node
            );
            return;
        }
        self.nodes.insert(function_name, node_owned);
    }

    pub fn get_node(&self, func_name: &str) -> Option<&Arc<Mutex<MermaidNode>>> {
        self.nodes.get(func_name)
    }

    pub fn render_subgraph(&self) -> String {
        let mut all_nodes = Vec::new();
        for (_, node) in self.nodes() {
            let node_borrowed = node.lock().unwrap();
            all_nodes.push(node_borrowed.render_node());
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
}

impl MermaidNode {
    // Constructor
    pub fn new(function_name: String) -> Self {
        let mermaid_id = generate_random_string(4);
        Self {
            mermaid_id,
            function_name,
        }
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
        node_str
    }
}

#[derive(Debug, Default, Clone)]
pub struct MermaidEdge {
    line: usize,
    caller_function: Arc<Mutex<MermaidNode>>,
    called_function: Arc<Mutex<MermaidNode>>,
    color: String,
}

impl MermaidEdge {
    // Constructor
    pub fn new(
        line: usize,
        caller_function: &Arc<Mutex<MermaidNode>>,
        called_function: &Arc<Mutex<MermaidNode>>,
        color: String,
    ) -> Self {
        Self {
            line,
            caller_function: Arc::clone(caller_function),
            called_function: Arc::clone(called_function),
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

    // Setter for color
    pub fn set_color(&mut self, color: String) {
        self.color = color;
    }

    pub fn add_edge_and_nodes(&mut self) {
        // add edge and source and destination nodes
    }

    pub fn render_edge_definition(&self) -> String {
        let (caller_str, called_str) = {
            let caller_borrowed = self.caller_function.lock().unwrap();
            let called_borrowed = self.called_function.lock().unwrap();
            (
                caller_borrowed.function_name().to_string(),
                called_borrowed.function_name().to_string(),
            )
        };
        let edge_str = format!("\t{} -- Line {} --> {}\n", caller_str, self.line, called_str);
        edge_str
    }

    pub fn render_edge_style(&self) -> String {
        let style_str = format!("stroke:{},stroke-width:4px;", self.color());
        style_str
    }
}

#[derive(Debug, Default, Clone)]
pub struct MermaidGraphElements {
    edges: Vec<MermaidEdge>,
    subgraphs: HashMap<String, MermaidSubgraph>,
}

impl MermaidGraphElements {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            subgraphs: HashMap::new(),
        }
    }

    pub fn subgraph_for_file(&self, file: &str) -> Option<&MermaidSubgraph> {
        self.subgraphs.get(file)
    }

    pub fn add_edge(
        &mut self,
        edge_color: &str,
        line: usize,
        source_func_name: &str,
        dest_func_name: &str,
        source_file: &str,
        dest_file: &str,
    ) {
        let source_node: Arc<Mutex<MermaidNode>>;
        let dest_node: Arc<Mutex<MermaidNode>>;

        if let Some(subgraph) = self.subgraphs.get_mut(source_file) {
            if let Some(node) = subgraph.get_node(source_func_name) {
                source_node = Arc::clone(node);
            } else {
                let node = MermaidNode::new(source_func_name.to_string());
                source_node = Arc::new(Mutex::new(node));
                subgraph.add_node(&source_node);
            }
        } else {
            let node = MermaidNode::new(source_func_name.to_string());
            source_node = Arc::new(Mutex::new(node));
            let mut subgraph = MermaidSubgraph::new(source_file.to_string());
            subgraph.add_node(&source_node);
            self.add_subgraph(subgraph);
        }

        if let Some(subgraph) = self.subgraphs.get_mut(dest_file) {
            if let Some(node) = subgraph.get_node(dest_func_name) {
                dest_node = Arc::clone(node);
            } else {
                let node = MermaidNode::new(dest_func_name.to_string());
                dest_node = Arc::new(Mutex::new(node));
                subgraph.add_node(&dest_node);
            }
        } else {
            let node = MermaidNode::new(dest_func_name.to_string());
            dest_node = Arc::new(Mutex::new(node));
            let mut subgraph = MermaidSubgraph::new(dest_file.to_string());
            subgraph.add_node(&dest_node);
            self.add_subgraph(subgraph);
        }

        let edge = MermaidEdge::new(line, &source_node, &dest_node, edge_color.to_string());
        self.edges.push(edge);
    }

    fn add_subgraph(&mut self, subgraph: MermaidSubgraph) {
        if !self.subgraphs.contains_key(subgraph.mermaid_id()) {
            self.subgraphs.insert(subgraph.mermaid_id().to_string(), subgraph);
        }
    }

    fn render_edges(&self) -> String {
        let mut all_edges = Vec::<String>::new();
        let mut all_edges_style = Vec::<String>::new();
        for (idx, edge) in self.edges.iter().enumerate() {
            all_edges.push(edge.render_edge_definition());
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
