use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub author: String,
    pub author_email: String,
    pub site_name: String,
    pub site_url: String,
    pub commit_base_url: String,

    #[serde(default)]
    pub skip: HashSet<PathBuf>,

    pub header: Header,
    pub footer: Footer,
}

pub const HOME_MD: &str = "home.md";
pub const NOT_FOUND_MD: &str = "not_found.md";
pub const BLOG_DIR: &str = "blog";
pub const STATIC_DIR: &str = "static";

pub fn tag_to_link(tag: &str) -> String {
    format!("/blog/tags/{tag}")
}

#[derive(Debug, Clone, Deserialize)]
pub struct Header {
    pub home_name: String,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Footer {
    pub links: Vec<Link>,
    pub cc: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Link {
    pub title: String,
    pub url: String,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = fs::File::open(path)?;
        Ok(serde_yaml::from_reader(file)?)
    }
}

fn default_favicon_path() -> PathBuf {
    "favicon.ico".into()
}
fn default_home_md_path() -> PathBuf {
    "home.md".into()
}
fn default_not_found_md_path() -> PathBuf {
    "not_found.md".into()
}
fn default_blog_dir() -> PathBuf {
    "blog".into()
}
