use std::env;
use std::fs;
use std::path::Path;

use anyhow::Context as _;
use lightningcss::{
    bundler::{Bundler, FileProvider},
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions},
};

fn main() -> anyhow::Result<()> {
    println!("cargo::rerun-if-changed=static");

    let out_dir = env::var_os("OUT_DIR").context("env var OUT_DIR not found")?;
    let cargo_manifest_dir =
        env::var_os("CARGO_MANIFEST_DIR").context("env var CARGO_MANIFEST_DIR not found")?;

    let src_static_dir = Path::new(&cargo_manifest_dir).join("static");
    let out_static_dir = Path::new(&out_dir).join("static");

    // delete old generated
    if out_static_dir.exists() {
        fs::remove_dir_all(&out_static_dir).context("failed to remove all generated static/")?;
    }

    let is_css_dir = |p: &Path| p.is_dir() && p.file_name().and_then(|x| x.to_str()) == Some("css");
    fs::create_dir_all(&out_static_dir)?;
    copy_dir(&src_static_dir, &out_static_dir, is_css_dir).context("failed to copy static/")?;

    build_css(
        src_static_dir.join("css/main.css"),
        out_static_dir.join("styles.css"),
    )
    .context("failed to generate static/styles.css")?;

    Ok(())
}

fn copy_dir(
    source_dir: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
    should_skip: fn(&Path) -> bool,
) -> std::io::Result<()> {
    let source_dir = source_dir.as_ref();
    let dest_dir = dest_dir.as_ref();

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if should_skip(&entry.path()) {
            continue;
        }

        if ty.is_dir() {
            copy_dir(entry.path(), dest_dir.join(entry.file_name()), should_skip)?;
        } else {
            fs::create_dir_all(dest_dir)?;
            fs::copy(entry.path(), dest_dir.join(entry.file_name()))?;
        }
    }

    Ok(())
}

pub fn build_css(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let fs = FileProvider::new();
    let mut bundler = Bundler::new(&fs, None, ParserOptions::default());

    let mut stylesheet = bundler
        .bundle(input_path.as_ref())
        .map_err(|e| anyhow::anyhow!("failed to build stylesheet: {:?}", e))?;

    stylesheet.minify(MinifyOptions::default())?;

    let res = stylesheet.to_css(PrinterOptions {
        minify: true,
        ..Default::default()
    })?;

    fs::write(output_path, res.code)?;

    Ok(())
}
