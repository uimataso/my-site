use std::{env, path::Path, time::Duration};

use anyhow::Context as _;
use my_site_generator::build;

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args: Vec<String> = env::args().collect();

    let name = &args[0];
    let src_dir = &args.get(1).with_context(|| help(name))?;
    let dst_dir = &args.get(2).with_context(|| help(name))?;

    if Path::new(dst_dir).exists() {
        log::warn!("dest dir `{}` already exists, delete it...", dst_dir);
        std::thread::sleep(Duration::from_secs(1));
        let _res = std::fs::remove_dir_all(dst_dir);
    }

    build(src_dir, dst_dir)?;

    Ok(())
}

fn help(name: &str) -> String {
    format!("Usage: {} <src-dir> <dst-dir>", name)
}
