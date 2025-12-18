use std::{fs, path::Path};

use include_dir::{Dir, include_dir};

static STATIC_DIR: Dir = include_dir!("$OUT_DIR/static");

pub fn copy_static_dir_to(out_dir: impl AsRef<Path>) -> std::io::Result<()> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)?;
    copy_dir(&STATIC_DIR, out_dir)
}

fn copy_dir(dir: &Dir, out_dir: impl AsRef<Path>) -> std::io::Result<()> {
    let out_dir = out_dir.as_ref();

    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(subdir) => {
                copy_dir(subdir, out_dir)?;
            }
            include_dir::DirEntry::File(file) => {
                // only create dir when needed
                let path = out_dir.join(file.path());
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::write(path, file.contents())?;
            }
        }
    }

    Ok(())
}
