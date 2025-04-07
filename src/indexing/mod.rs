pub mod analyzer;
pub mod extractor;
pub mod processor;

pub use analyzer::{enhance_method_names, generate_summaries, identify_relationships};
pub use processor::process_codebase_parallel;
