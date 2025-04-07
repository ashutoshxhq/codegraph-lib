pub mod code_graph;
pub mod indexing;
pub mod parsers;
pub mod utils;

use log::{debug, info};
use std::io;
use std::path::Path;

pub fn process_codebase(root_path: &Path, num_threads: usize) -> io::Result<code_graph::CodeGraph> {
    info!(
        "Processing codebase at: {:?} with {} threads",
        root_path, num_threads
    );
    let mut graph = indexing::processor::process_codebase_parallel(root_path, num_threads)?;

    // Identify relationships between nodes
    info!(
        "Building relationships between {} nodes...",
        graph.node_count()
    );
    indexing::analyzer::identify_relationships(&mut graph);

    // Enhance method names with their parent class/struct
    indexing::analyzer::enhance_method_names(&mut graph);

    info!(
        "Code graph built with {} nodes and {} relationships",
        graph.node_count(),
        graph.relationship_count()
    );

    Ok(graph)
}

pub fn analyze_codebase(
    root_path: &Path,
    output_path: &Path,
    num_threads: usize,
) -> io::Result<()> {
    info!("Starting codebase analysis");
    debug!("Root path: {:?}, Output path: {:?}", root_path, output_path);

    let mut graph = process_codebase(root_path, num_threads)?;

    info!("Generating summaries for {} nodes", graph.node_count());
    indexing::analyzer::generate_summaries(&mut graph);

    info!("Exporting graph to JSON at {:?}", output_path);
    utils::io::export_graph_to_json(&graph, output_path)?;

    info!(
        "Analysis complete: {} nodes and {} relationships",
        graph.node_count(),
        graph.relationship_count()
    );

    Ok(())
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
