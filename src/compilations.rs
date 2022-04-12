use std::{
    collections::HashSet,
    io,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

static CMD_INCLUD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("(^| )-I ?([\\w\\-/\\.]+)( |$)").unwrap());

#[derive(Deserialize)]
struct Command {
    file: PathBuf,
    command: String,
}

pub async fn collect<M>(
    compilations: &Path,
    matcher: M,
) -> io::Result<(Vec<PathBuf>, HashSet<PathBuf>)>
where
    M: Fn(&Path) -> bool,
{
    let f = std::fs::File::open(compilations)?;
    let commands: Vec<Command> = serde_json::from_reader(f)?;

    let mut sources = Vec::with_capacity(commands.len());
    let mut includes = HashSet::new();

    for Command { file, command } in commands {
        println!("file {file:?}");
        if matcher(&file) {
            sources.push(file);
            for include in command_parse_includes(&command) {
                includes.insert(include);
            }
        }
    }

    Ok((sources, includes))
}

fn command_parse_includes(command: &str) -> impl Iterator<Item = PathBuf> + '_ {
    CMD_INCLUD_RE
        .captures_iter(command)
        .map(|m| PathBuf::from(&m[2]))
}
