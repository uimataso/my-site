use std::fs;
use std::path::Path;

use lightningcss::{
    bundler::{Bundler, FileProvider},
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions},
};

pub fn bundle_and_minify(
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
