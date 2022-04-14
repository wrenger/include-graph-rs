use std::collections::HashSet;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{self, Deserializer};

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

    let mut sources = Vec::new();
    let mut includes = HashSet::new();

    for elem in iter_json_array(f) {
        let Command { file, command } = elem.unwrap();
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

pub fn iter_json_array<T: DeserializeOwned, R: Read>(
    mut reader: R,
) -> impl Iterator<Item = Result<T, io::Error>> {
    let mut first = true;
    std::iter::from_fn(move || yield_next_obj(&mut reader, &mut first).transpose())
}

fn yield_next_obj<T: DeserializeOwned, R: Read>(
    mut reader: R,
    first: &mut bool,
) -> io::Result<Option<T>> {
    if *first {
        *first = false;
        if read_skipping_ws(&mut reader)? == b'[' {
            // read the next char to see if the array is empty
            let peek = read_skipping_ws(&mut reader)?;
            if peek == b']' {
                Ok(None)
            } else {
                deserialize_single(io::Cursor::new([peek]).chain(reader)).map(Some)
            }
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "`[` not found"))
        }
    } else {
        match read_skipping_ws(&mut reader)? {
            b',' => deserialize_single(reader).map(Some),
            b']' => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "`,` or `]` not found",
            )),
        }
    }
}

fn deserialize_single<T: DeserializeOwned, R: Read>(reader: R) -> io::Result<T> {
    match Deserializer::from_reader(reader).into_iter::<T>().next() {
        Some(result) => result.map_err(Into::into),
        None => Err(io::Error::new(io::ErrorKind::InvalidData, "premature EOF")),
    }
}

fn read_skipping_ws(mut reader: impl Read) -> io::Result<u8> {
    loop {
        let mut byte = 0u8;
        reader.read_exact(std::slice::from_mut(&mut byte))?;
        if !byte.is_ascii_whitespace() {
            return Ok(byte);
        }
    }
}
