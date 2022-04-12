use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_recursion::async_recursion;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::fs::{self, File};
use tokio::io::{AsyncBufReadExt, BufReader};

static INCLUDE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"[ \t]*#[ \t]*include[ \t]+"([\w./]+\.(h|hpp))""#).unwrap());

pub async fn generate<M>(
    sources: &[PathBuf],
    include_dirs: HashSet<PathBuf>,
    matcher: M,
) -> io::Result<HashMap<PathBuf, HashSet<PathBuf>>>
where
    M: Fn(&Path) -> bool + Send + Sync + 'static,
{
    let mut files = HashSet::new();

    let include_dirs = Arc::new(include_dirs);
    let matcher = Arc::new(matcher);

    for source in sources {
        let path = source.canonicalize()?;
        if matcher(&path) {
            files.insert(path);
        }
    }

    for include_dir in include_dirs.iter() {
        walk_recursive(include_dir, &mut |f| {
            if f.extension().map_or(false, |f| f == "h")
                || f.extension().map_or(false, |f| f == "hpp")
            {
                println!("file {f:?}");
                files.insert(f);
            }
        })
        .await?;
    }

    let handles = files
        .into_iter()
        .map(|f| {
            let include_dirs = include_dirs.clone();
            let matcher = matcher.clone();
            tokio::spawn(async move { parse_file(f, &include_dirs, matcher.as_ref()).await })
        })
        .collect::<Vec<_>>();

    let mut graph = HashMap::<PathBuf, HashSet<PathBuf>>::new();
    for h in handles {
        match h.await? {
            Ok((k, v)) => {
                graph.insert(k, v);
            }
            Err(e) => eprintln!("Error file parse {e:?}"),
        }
    }

    Ok(graph)
}

async fn parse_file<M>(
    file: PathBuf,
    include_dirs: &HashSet<PathBuf>,
    matcher: &M,
) -> io::Result<(PathBuf, HashSet<PathBuf>)>
where
    M: Fn(&Path) -> bool,
{
    let mut includes = HashSet::new();

    let f = File::open(&file).await?;
    let reader = BufReader::new(f);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if let Some(m) = INCLUDE_RE.captures(&line) {
            if let Some(path) = find_abspath(&file, &Path::new(&m[1]), include_dirs) {
                if matcher(&path) {
                    includes.insert(path);
                }
            }
        }
    }

    Ok((file, includes))
}

/// Find the included file and return its absolute path.
fn find_abspath(file: &Path, include: &Path, include_dirs: &HashSet<PathBuf>) -> Option<PathBuf> {
    let filedir = file.parent()?;

    // Allow relative includes
    let abspath = relative_abspath(filedir, include);
    println!("{file:?} {include:?} {abspath:?}");
    let include = abspath.as_ref().map_or(include, |p| p);

    for directory in include_dirs {
        let path = directory.join(include);
        if path.exists() {
            return path.canonicalize().ok();
        }
    }

    None
}

/// Checks if the include is relative and returns an absolute one
fn relative_abspath(filedir: &Path, include: &Path) -> Option<PathBuf> {
    for pattern in ["src", "include"] {
        if let Some(Ok(subpath)) = filedir
            .ancestors()
            .skip(1)
            .find_map(|a| a.ends_with(pattern).then(|| filedir.strip_prefix(a)))
        {
            return Some(subpath.join(include));
        }
    }
    None
}

#[async_recursion(?Send)]
async fn walk_recursive<F: FnMut(PathBuf)>(dir: &Path, f: &mut F) -> io::Result<()> {
    let mut reader = fs::read_dir(dir).await?;
    while let Some(entry) = reader.next_entry().await? {
        let ty = entry.file_type().await?;
        if ty.is_file() {
            f(entry.path())
        } else if ty.is_dir() {
            walk_recursive(&entry.path(), f).await?;
        }
    }
    Ok(())
}
