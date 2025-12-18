#![allow(dead_code)]

use std::path::PathBuf;

mod config;
mod generator;
mod git_repo;
mod markdown;
mod pages;
mod static_dir;

pub fn build(in_dir: impl Into<PathBuf>, out_dir: impl Into<PathBuf>) -> anyhow::Result<()> {
    let generator = generator::Generator::new(in_dir, out_dir)?;
    generator.build()?;
    Ok(())
}
