use std::path::{Path, PathBuf};

use clap::Parser;

mod compilations;
mod graph;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    project: PathBuf,
    #[clap(short, long, default_value = "compile_commands.json")]
    compilations: PathBuf,
}

#[tokio::main]
async fn main() {
    let Args {
        project,
        compilations,
    } = Args::parse();

    let project = project.canonicalize().unwrap();

    println!("parse compile commands: {compilations:?}");
    let (sources, includes_dirs) =
        compilations::collect(&compilations, |f| f.starts_with(&project))
            .await
            .unwrap();

    println!(
        "found {} sources, {} includes",
        sources.len(),
        includes_dirs.len()
    );

    println!("\ngenerate graph");
    let graph = graph::generate(&sources, includes_dirs, match_file)
        .await
        .unwrap();
    println!("{} nodes", graph.len());

    let outfile = compilations.parent().unwrap().join("dependencies.json");

    println!("serialize to: {outfile:?}");
    let out = std::fs::File::create(outfile).unwrap();
    serde_json::to_writer_pretty(out, &graph).unwrap();
}

fn match_file(path: &Path) -> bool {
    path.is_absolute() && path.file_name().map_or(false, |n| n.is_ascii())
}
