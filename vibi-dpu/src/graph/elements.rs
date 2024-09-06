use std::{
    borrow::Borrow,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use serde::{Serialize, Deserialize};
// TODO, FIXME - remove all unwraps

use crate::utils::review::Review;

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
        if self.nodes.contains_key(node.function_name()) {
            log::error!(
                "[add_node] Node already exists: old - {:#?}, new - {:#?}",
                &self.nodes[node.function_name()],
                node
            );
            return;
        }
        self.nodes.insert(node.function_name().to_string(), node.to_owned());
    }

    pub fn get_node(&self, func_name: &str) -> Option<&MermaidNode> {
        self.nodes.get(func_name)
    }

    pub fn get_mut_node(&mut self, func_name: &str) -> Option<&mut MermaidNode> {
        self.nodes.get_mut(func_name)
    }

    pub fn render_subgraph(&self, review: &Review, subgraph_map: &HashMap<String, MermaidSubgraph>) -> String {
        let mut all_nodes = Vec::new();
        for (_, node) in self.nodes() {
            all_nodes.push(node.render_node(review, subgraph_map));
        }
        let subgraph_str = format!(
            "\tsubgraph {} [{}]\n{}\n\tend\n{}\n",
            self.mermaid_id,
            self.name,
            all_nodes.join("\n"),
            self.render_subgraph_style()
        );
        subgraph_str
    }

    fn render_subgraph_style(&self) -> String {
        let mut class_str = "";
        for (_, node) in self.nodes() {
            match node.color().as_str() {
                "yellow" => {
                    class_str = "modified";
                    break;
                },
                "red" => {
                    match class_str {
                        "green" | "yellow" => {
                            class_str = "modified";
                            break;
                        },
                        "" | "red" | _ => {
                            class_str = "red";
                        }
                    }
                },
                "green" => {
                    match class_str {
                        "red" | "yellow" => {
                            class_str = "modified";
                            break;
                        },
                        "" | "green" | _ => {
                            class_str = "green";
                        }
                    }
                }
                "" | _ => ()
            }
        }
        if class_str != "" {
            return format!("\tclass {} {}", self.mermaid_id(), class_str);
        }
        return "".to_string();
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct MermaidNode {
    function_name: String,
    mermaid_id: String,
    parent_id: String,
    color: String,
    def_line: usize
}

impl MermaidNode {
    // Constructor
    pub fn new(function_name: String, parent_id: String, def_line: usize) -> Self {
        let mermaid_id = generate_random_string(4);
        Self {
            mermaid_id,
            function_name,
            parent_id,
            color: "".to_string(),
            def_line
        }
    }

    pub fn color(&self) -> &String {
        &self.color
    }

    pub fn function_name(&self) -> &String {
        &self.function_name
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

    pub fn compare_and_change_color(&mut self, node_color: &str) {
        if (self.color() == "red" && node_color == "green") ||
        (self.color() == "green" && node_color == "red") {
            self.set_color("yellow");
        }
    }

    pub fn render_node(&self, review: &Review, subgraph_map: &HashMap<String, MermaidSubgraph>) -> String {
        let url_str = format!("\tclick {} href \"{}\" _blank",
            self.mermaid_id(), self.get_node_str(review, subgraph_map));
        let class_str = self.get_style_class();
        let node_str = format!("\t{}[{}]", &self.mermaid_id, &self.function_name);
        return format!("{}\n{}\n{}", &node_str, &class_str, &url_str);
    }
    
    fn get_node_str(&self, review: &Review, subgraph_map: &HashMap<String, MermaidSubgraph>) -> String {
        if let Some(subgraph) = subgraph_map.get(self.parent_id()) {
            let file_hash = sha256::digest(subgraph.name());
            return match self.color.as_str() {
                "green" | "yellow" => {
                    let diff_side_str = "R";
                    format!(
                        "https://github.com/{}/{}/pull/{}/files#diff-{}{}{}",
                        review.repo_owner(),
                        review.repo_name(),
                        review.id(),
                        &file_hash,
                        diff_side_str,
                        self.def_line
                    )
                }
                "red" => {
                    let diff_side_str = "L";
                    format!(
                        "https://github.com/{}/{}/pull/{}/files#diff-{}{}{}",
                        review.repo_owner(),
                        review.repo_name(),
                        review.id(),
                        &file_hash,
                        diff_side_str,
                        self.def_line
                    )
                }
                "" | _ => format!(
                    "https://github.com/{}/{}/blob/{}/{}#L{}",
                    review.repo_owner(),
                    review.repo_name(),
                    review.base_head_commit(),
                    subgraph.name(),
                    self.def_line
                ),
            };
        }
        return "".to_string();
    }
    
    fn get_style_class(&self) -> String {
        let class_str_prefix = format!("class {}", self.mermaid_id());
        match self.color.as_str() {
            "green"  => format!("\t{} added", &class_str_prefix),
            "red" => format!("\t{} deleted", &class_str_prefix),
            "yellow" => format!("\t{} modified", &class_str_prefix),
            "" | _ => "".to_string()
        }
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
        dest_color: &str,
        source_def_line: usize,
        dest_def_line: usize
    ) {        
        self.create_node(source_file, source_func_name, source_color, source_def_line);
        self.create_node(dest_file, dest_func_name, dest_color, dest_def_line);
        let edge = MermaidEdge::new(
            calling_line_num,
            source_func_name.to_string(),
            source_file.to_string(),
            dest_func_name.to_string(),
            dest_file.to_string(),
            edge_color.to_string());
        self.add_edge_to_edges(edge);
    }

    fn create_node(&mut self, subgraph_key: &str, node_func_name: &str, node_color: &str, def_line: usize) {
        if let Some(subgraph) = self.subgraphs.get_mut(subgraph_key) {
            if let Some(node) = subgraph.get_mut_node(node_func_name) {
                node.compare_and_change_color(node_color);
            } else {
                let mut node = MermaidNode::new(node_func_name.to_string(),
                subgraph.name().to_string(), def_line);
                node.set_color(node_color);
                subgraph.add_node(node);
            }
        } else {
            let mut subgraph = MermaidSubgraph::new(subgraph_key.to_string());
            let mut node = MermaidNode::new(node_func_name.to_string(),
            subgraph.name().to_string(), def_line);
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

    // fn render_edges(&self) -> String {
    //     let mut all_edges = Vec::<String>::new();
    //     let mut all_edges_style = Vec::<String>::new();
    //     for (idx, (_, edge)) in self.edges.iter().enumerate() {
    //         all_edges.push(edge.render_edge_definition(&self.subgraphs));
    //         all_edges_style.push(format!("\tlinkStyle {} {}", idx, edge.render_edge_style()));
    //     }
    //     let all_edges_str = format!("{}{}", all_edges.join("\n"), all_edges_style.join("\n"));
    //     all_edges_str
    // }

    fn render_subgraphs(&self, review: &Review) -> String {
        format!("{}\n{}",
            self.subgraphs
                .values()
                .map(|subgraph| subgraph.render_subgraph(review, &self.subgraphs))
                .collect::<Vec<String>>()
                .join("\n"),
            self.subgraph_style_defs())
    }

    fn subgraph_style_defs(&self) -> String {
        let modified_class_def = "\tclassDef modified stroke:black,fill:yellow";
        let added_class_def = "\tclassDef added stroke:black,fill:#b7e892,color:black";
        let deleted_class_def = "\tclassDef deleted stroke:black,fill:red";
        format!("{}\n{}\n{}", modified_class_def, added_class_def, deleted_class_def)
    }

    pub fn render_elements(&self, review: &Review) -> String {
        let all_elements_str = format!("{}\n{}", &self.render_subgraphs(review), &self.render_edges());
        all_elements_str
    }

    fn render_edges(&self) -> String {
        let mut edge_defs = Vec::<String>::new();
        let mut default_edge_styles = Vec::<String>::new();
        let mut green_edge_styles = Vec::<String>::new();
        let mut red_edge_styles = Vec::<String>::new();
        let mut yellow_edge_styles = Vec::<String>::new();
        for (_, edge) in &self.edges {
            let src_node_id = self.subgraphs[edge.src_subgraph_key()].nodes()[edge.src_func_key()].mermaid_id();
            let dest_node_id = self.subgraphs[edge.dest_subgraph_key()].nodes()[edge.dest_func_key()].mermaid_id();
            let edge_def_str = format!("\t{} ==\"Line {}\" =====>{}", src_node_id, edge.line(), dest_node_id);
            edge_defs.push(edge_def_str);
            match edge.color().as_str() {
                "red" => red_edge_styles.push((edge_defs.len() - 1).to_string()),
                "green" => green_edge_styles.push((edge_defs.len() - 1).to_string()),
                "yellow" => yellow_edge_styles.push((edge_defs.len() - 1).to_string()),
                "" | _ => default_edge_styles.push((edge_defs.len() - 1).to_string())
            }
        }
        if !edge_defs.is_empty() {
            let default_edges_str = match default_edge_styles.is_empty() {
                true => "".to_string(),
                false => format!("\tlinkStyle {} stroke-width:1", default_edge_styles.join(",")) 
            };
            let green_edges_str = match green_edge_styles.is_empty() {
                true => "".to_string(),
                false => format!("\tlinkStyle {} stroke:green,stroke-width:8", green_edge_styles.join(",")) 
            };
            let red_edges_str = match red_edge_styles.is_empty() {
                true => "".to_string(),
                false => format!("\tlinkStyle {} stroke:red,stroke-width:10", red_edge_styles.join(",")) 
            };
            let yellow_edges_str = match yellow_edge_styles.is_empty() {
                true => "".to_string(),
                false => format!("\tlinkStyle {} stroke:#ffe302,stroke-width:10", yellow_edge_styles.join(",")) 
            };
            return format!("{}\n{}\n{}\n{}\n{}",
                edge_defs.join("\n"),
                &default_edges_str,
                &green_edges_str,
                &red_edges_str,
                &yellow_edges_str
            );
        }

        return "".to_string();
    }
}
