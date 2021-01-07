#![feature(osstring_ascii)]

use std::{
    collections::hash_map::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use fehler::throws;
use promptly::prompt_default;

#[derive(Debug, structopt::StructOpt, Clone)]
struct Options {
    #[structopt(short, long)]
    recursive: bool,

    #[structopt(short, long, required(true), number_of_values(1))]
    keep: Vec<OsString>,

    #[structopt(short = "m", long = "move")]
    move_to: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    in_dir: PathBuf,
}

impl Options {
    fn with_subdir(&self, subdir: impl AsRef<Path>) -> Self {
        let subdir = subdir.as_ref();
        Options {
            in_dir: self.in_dir.join(subdir),
            move_to: self.move_to.as_ref().map(|m| m.join(subdir)),
            ..self.clone()
        }
    }
}

#[throws(Box<dyn std::error::Error>)]
fn process(options: &Options) {
    let mut file_map = HashMap::new();
    for entry in fs::read_dir(&options.in_dir)?.filter_map(|e| e.ok()) {
        if entry.file_type()?.is_dir() && options.recursive {
            process(&options.with_subdir(entry.file_name()))?;
        } else {
            let file = entry.path();
            if let (Some(stem), Some(extension)) = (file.file_stem(), file.extension()) {
                if !file_map.contains_key(stem) {
                    file_map.insert(stem.to_owned(), vec![]);
                }
                file_map
                    .get_mut(stem)
                    .unwrap()
                    .push((file.clone(), extension.to_owned()));
            }
        }
    }

    for stem in file_map.keys() {
        let entries = file_map.get(stem).unwrap();
        let mut keeping = None;

        // Check each kept extension one-by-one.
        for keep_ext in options.keep.iter() {
            // If there is an entry with this extension, it is the one to keep.
            keeping = entries
                .iter()
                .find(|(_, ext)| ext.eq_ignore_ascii_case(keep_ext))
                .map(|(path, _)| path);
            if keeping.is_some() {
                break;
            }
        }

        if let Some(keeping) = keeping {
            let discarding = entries
                .iter()
                .map(|(path, _)| path)
                .filter(|path| *path != keeping)
                .collect::<Vec<_>>();

            if !discarding.is_empty() {
                println!("Keeping {:?}.", keeping);
            }

            for discard in discarding.iter() {
                if let Some(move_to) = &options.move_to {
                    if prompt_default(format!("  Move {:?}?", *discard), false)? {
                        fs::create_dir_all(move_to)?;
                        let dest = move_to.join(discard.file_name().unwrap());
                        fs::rename(*discard, &dest)?;
                        println!("  * Moved to {:?}!", &dest);
                    }
                } else {
                    if prompt_default(format!("  Delete {:?}?", *discard), false)? {
                        fs::remove_file(*discard)?;
                        println!("  * Deleted!");
                    }
                }
            }
        }
    }
}

#[paw::main]
#[throws(Box<dyn std::error::Error>)]
fn main(options: Options) {
    process(&options)?;
}
