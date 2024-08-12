async fn graph_edges() {
    let incoming_edges = incoming_edges().await;
    let outgoing_edges = outgoing_edges().await;
    let graph = edge_nodes().await;
}

async fn incoming_edges() {
    // find incoming edges from full_graph to diff_graph
    // find incoming green edges from diff_graph to diff_graph
}

async fn outgoing_edges() {
    // find outgoing edges from diff_graph to full_graph
    // find outgoing edges from diff_graph to diff_graph
}

async fn edge_nodes() {
    // render all edges and their nodes
}