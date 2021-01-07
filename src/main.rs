#![feature(osstring_ascii)]

use std::{
    error::Error as StdError,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

use multimap::MultiMap;
use promptly::prompt_default;

type Result<T> = std::result::Result<T, Box<dyn StdError>>;

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

fn process(options: &Options) -> Result<()> {
    let file_map = get_file_map(options)?;

    for stem in file_map.keys() {
        let entries = file_map.get_vec(stem).unwrap();

        let keeping = get_kept_file(&options.keep[..], &entries[..]);

        if let Some(keeping) = keeping {
            let discarding = entries
                .iter()
                .filter(|path| *path != keeping)
                .collect::<Vec<_>>();

            if !discarding.is_empty() {
                println!("Keeping {:?}.", keeping);
            }

            let mut created = false;
            for discard in discarding.iter() {
                if let Some(move_to) = &options.move_to {
                    if prompt_default(format!("  Move {:?}?", *discard), false)? {
                        if !created {
                            fs::create_dir_all(move_to)?;
                            created = true;
                        }
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

    Ok(())
}

fn get_file_map(options: &Options) -> Result<MultiMap<OsString, PathBuf>> {
    let mut file_map = MultiMap::new();
    for entry in fs::read_dir(&options.in_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && options.recursive {
            process(&options.with_subdir(entry.file_name()))?;
        } else {
            let file = entry.path();
            if let (Some(stem), Some(_)) = (file.file_stem(), file.extension()) {
                let stem = stem.to_owned();
                file_map.insert(stem, file);
            }
        }
    }
    Ok(file_map)
}

fn get_kept_file<'a, S: AsRef<OsStr>>(
    keep_extensions: &'_ [S],
    entries: &'a [PathBuf],
) -> Option<&'a PathBuf> {
    // Check each kept extension one-by-one.
    for keep_ext in keep_extensions.iter() {
        // If there is an entry with this extension, it is the one to keep.
        let keeping = entries.iter().find(|path| {
            path.extension()
                .unwrap()
                .eq_ignore_ascii_case(keep_ext.as_ref())
        });
        if keeping.is_some() {
            return keeping;
        }
    }
    None
}

#[paw::main]
fn main(options: Options) -> Result<()> {
    process(&options)
}
