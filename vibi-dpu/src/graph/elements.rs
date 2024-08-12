use std::{borrow::Borrow, cell::{Ref, RefCell}, collections::HashMap, rc::Rc};
use serde::{Serialize, Deserialize};

use super::utils::generate_random_string;

#[derive(Debug, Default, Clone)]
pub struct MermaidSubgraph {
    name: String,
    nodes: HashMap<String, Rc<RefCell<MermaidNode>>>,
    mermaid_id: String
}

impl MermaidSubgraph {
    // Constructor
    pub fn new(name: String) -> Self {
        let mermaid_id = generate_random_string(4);
        Self { name, nodes: HashMap::new(), mermaid_id }
    }

    // Getter for nodes
    pub fn nodes(&self) -> &HashMap<String, Rc<RefCell<MermaidNode>>> {
        self.nodes.borrow()
    }

    pub fn mermaid_id(&self) -> &String {
        &self.mermaid_id
    }

    // Setter for nodes
    pub fn set_nodes(&mut self, nodes: HashMap<String, Rc<RefCell<MermaidNode>>>) {
        self.nodes = nodes;
    }

    pub fn add_node(&mut self, node: Rc<RefCell<MermaidNode>>) {
        let function_name = {
            let node_borrowed: Ref<MermaidNode> = RefCell::borrow(&*node);
            node_borrowed.function_name().to_string()
        };
        if self.nodes.contains_key(&function_name) {
            log::error!(
                "[add_node] Node already exists: old - {:#?}, new - {:#?}",
                &self.nodes[&function_name], node);
            return;
        }
        self.nodes.insert(function_name, node);
    }

    pub fn render_subgraph(&self) -> String{
        let mut all_nodes = Vec::new();
        for (_, node) in self.nodes() {
            let node_borrowed = RefCell::borrow(&*node);
            all_nodes.push(node_borrowed.render_node());
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

#[derive(Debug, Default, Clone)]
pub struct MermaidEdge {
    line: usize,
    caller_function: Rc<RefCell<MermaidNode>>,
    called_function: Rc<RefCell<MermaidNode>>,
    color: String,
}

impl MermaidEdge {
    // Constructor
    pub fn new(line: usize, caller_function: &Rc<RefCell<MermaidNode>>, called_function: &Rc<RefCell<MermaidNode>>, color: String) -> Self {
        Self {
            line,
            caller_function: Rc::clone(caller_function),
            called_function: Rc::clone(called_function),
            color }
    }

    // Getter for edge_str
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
            let caller_borrowed: Ref<MermaidNode> = RefCell::borrow(&*self.caller_function);
            let called_borrowed: Ref<MermaidNode> = RefCell::borrow(&*self.called_function);
            (caller_borrowed.function_name().to_string(), called_borrowed.function_name().to_string())
        };
        let edge_str = format!(
            "\t{} -- Line {} --> {}\n",
            caller_str,
            self.line,
            called_str,
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

    pub fn add_edge(&mut self, edge: MermaidEdge, from_subgraph: &MermaidSubgraph, to_subgraph: &MermaidSubgraph) {
        self.edges.push(edge);
        self.add_subgraph(from_subgraph);
        self.add_subgraph(to_subgraph);
    }

    fn add_subgraph(&mut self, subgraph: &MermaidSubgraph) {
        if !self.subgraphs.contains_key(subgraph.mermaid_id()) {
            self.subgraphs.insert(subgraph.mermaid_id().to_string(),
                subgraph.to_owned());
        }
    }

    fn render_edges(&self) -> String {
        let mut all_edges = Vec::<String>::new();
        let mut all_edges_style = Vec::<String>::new();
        for (idx, edge) in self.edges.iter().enumerate() {
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

    fn render_subgraphs(&self) -> String {
        return self.subgraphs.values().map(
            |subgraph| subgraph.render_subgraph()
        ).collect::<Vec<String>>().join("\n");
    }

    pub fn render_elements(&self) -> String {
        let all_elements_str = format!("{}\n{}",
        &self.render_subgraphs(), &self.render_edges());
        return all_elements_str;
    }
}