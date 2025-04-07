use log::{error, info, warn};
use relik_codegraph::{analyze_codebase, version};
use std::path::Path;
use std::time::Instant;

fn main() -> std::io::Result<()> {
    // Initialize logger
    if std::env::var_os("RUST_LOG").is_none() {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        error!("Not enough arguments provided");
        eprintln!(
            "Usage: {} <codebase_path> [output_path] [num_threads] [format]",
            args[0]
        );
        eprintln!("Version: {}", version());
        return Ok(());
    }

    let codebase_path = Path::new(&args[1]);
    let output_path = if args.len() >= 3 {
        Path::new(&args[2])
    } else {
        Path::new("code_graph.json")
    };

    let num_threads = if args.len() >= 4 {
        args[3].parse().unwrap_or_else(|_| {
            let cpu_count = num_cpus::get();
            warn!(
                "Invalid thread count provided, defaulting to {} CPUs",
                cpu_count
            );
            cpu_count
        })
    } else {
        let cpu_count = num_cpus::get();
        info!("Using default thread count: {}", cpu_count);
        cpu_count
    };

    let format = if args.len() >= 5 { &args[4] } else { "json" };

    info!("Relik Indexor v{}", version());
    info!("Processing codebase at: {:?}", codebase_path);
    info!("Using {} threads", num_threads);
    info!("Output format: {}", format);
    info!("Parser: Tree-sitter");

    let start_time = Instant::now();

    match format {
        "json" => {
            info!("Starting indexing with JSON output");
            analyze_codebase(codebase_path, output_path, num_threads)?;
        }
        _ => {
            warn!("Unsupported format: {}. Using JSON instead.", format);
            analyze_codebase(codebase_path, output_path, num_threads)?;
        }
    }

    let elapsed = start_time.elapsed();
    info!("Indexing completed in {:.2?}", elapsed);
    info!("Output saved to: {:?}", output_path);

    Ok(())
}
