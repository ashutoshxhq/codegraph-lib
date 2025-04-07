use crate::code_graph::CodeGraph;
use crate::indexing::extractor::extract_code_units;
use log::{debug, error, info, trace, warn};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub fn process_codebase_parallel(root_path: &Path, num_threads: usize) -> io::Result<CodeGraph> {
    info!(
        "Starting parallel codebase processing with {} threads",
        num_threads
    );

    let graph = Arc::new(Mutex::new(CodeGraph::new()));

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    let visited_files = Arc::new(Mutex::new(HashSet::new()));
    let supported_extensions = get_supported_extensions();

    info!("Scanning directory for supported files...");
    let files_to_process =
        collect_files_to_process(root_path, &supported_extensions, &visited_files)?;
    info!("Found {} files to process", files_to_process.len());

    files_to_process.par_iter().for_each(|path| {
        debug!("Processing file: {:?}", path);
        match extract_code_units(path) {
            Ok(units) => {
                debug!("Extracted {} code units from {:?}", units.len(), path);
                let mut graph = graph.lock().unwrap();
                for unit in units {
                    trace!("Adding node: {} ({:?})", unit.name, unit.node_type);
                    graph.add_node(unit);
                }
            }
            Err(e) => {
                error!("Error processing file {:?}: {}", path, e);
            }
        }
    });

    info!("File processing complete");
    let final_graph = Arc::try_unwrap(graph)
        .expect("Failed to unwrap Arc")
        .into_inner()
        .expect("Failed to unwrap Mutex");

    let mut node_type_counts = std::collections::HashMap::new();
    for node in final_graph.all_nodes() {
        *node_type_counts.entry(node.node_type.clone()).or_insert(0) += 1;
    }

    info!("Extracted node type counts:");
    for (node_type, count) in node_type_counts {
        info!("  {:?}: {}", node_type, count);
    }

    Ok(final_graph)
}

fn collect_files_to_process(
    root_path: &Path,
    supported_extensions: &HashSet<&'static str>,
    visited_files: &Arc<Mutex<HashSet<PathBuf>>>,
) -> io::Result<Vec<PathBuf>> {
    let mut files_to_process = Vec::new();

    for entry in WalkDir::new(root_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if !supported_extensions.contains(ext) {
                trace!("Skipping unsupported file: {:?}", path);
                continue;
            }
        } else {
            trace!("Skipping file without extension: {:?}", path);
            continue;
        }

        let canonical_path = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to canonicalize path {:?}: {}", path, e);
                continue;
            }
        };

        if visited_files.lock().unwrap().contains(&canonical_path) {
            trace!("Skipping already visited file: {:?}", path);
            continue;
        }

        visited_files.lock().unwrap().insert(canonical_path);
        files_to_process.push(path.to_path_buf());
    }

    Ok(files_to_process)
}

fn get_supported_extensions() -> HashSet<&'static str> {
    let mut extensions = HashSet::new();

    // Add all language extensions
    for ext in crate::parsers::common::get_supported_extensions() {
        extensions.insert(ext);
    }

    extensions
}
