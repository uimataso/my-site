use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow};

use crate::markdown::Markdown;

#[derive(Clone)]
pub struct BlogEntry {
    pub path: PathBuf,
    pub date: chrono::NaiveDate,
    pub slug: String,
    pub markdown: Markdown,
}

impl BlogEntry {
    pub fn new(base_dir: impl AsRef<Path>, path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();

        let (date, slug) = Self::parse_path(&path)?;

        let markdown = Markdown::builder(base_dir.as_ref(), &path)
            .with_context(|| format!("failed to open the file: {}", path.display()))?
            .build()?;

        Ok(Self {
            path,
            date,
            slug,
            markdown,
        })
    }

    /// Parse blog entry path: `xxx/yyyy-mm-dd-blog-slug.md`
    pub fn parse_path(path: impl Into<PathBuf>) -> anyhow::Result<(chrono::NaiveDate, String)> {
        let path = path.into();

        let file_name = {
            let mut path: PathBuf = path;
            path.set_extension("");
            path.file_name()
                .context("file name not found")?
                .to_str()
                .context("file name isn't valid")?
                .to_owned()
        };

        let split: Vec<_> = file_name.splitn(4, '-').collect();
        if let &[year, month, day, name] = split.as_slice() {
            let date = (
                year.parse().context("failed to parse year")?,
                month.parse().context("failed to parse month")?,
                day.parse().context("failed to parse day")?,
            );
            let date = chrono::NaiveDate::from_ymd_opt(date.0, date.1, date.2)
                .context("the date isn't valid")?;

            Ok((date, name.to_string()))
        } else {
            Err(anyhow!(
                "expected the filename have the format of \"yyyy-mm-dd-blog-name\""
            ))
        }
    }
}
