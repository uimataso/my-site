#![allow(clippy::unused_self)]

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use comrak::{
    arena_tree::Node,
    nodes::{Ast, NodeLink, NodeValue},
};
use normalize_path::NormalizePath as _;
use serde::Deserialize;

use crate::{PathExt as _, blog_entry::BlogEntry};

#[derive(Clone)]
pub struct Markdown {
    pub base_dir: PathBuf,
    pub file_path: PathBuf,
    pub content: Content,
    pub title: Content,
    pub description: Option<Content>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Content {
    pub markdown: String,
    pub html: String,
}

#[derive(Debug, Default, Deserialize)]
struct Frontmatter {
    title: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

pub struct MarkdownBuilder<'a> {
    base_dir: PathBuf,
    file_path: PathBuf,
    original_content: String,
    options: comrak::Options<'a>,
}

impl Markdown {
    pub fn builder(
        base_dir: impl Into<PathBuf>,
        file_path: impl Into<PathBuf>,
    ) -> Result<MarkdownBuilder<'static>, std::io::Error> {
        MarkdownBuilder::new(base_dir, file_path)
    }
}

impl MarkdownBuilder<'_> {
    pub fn new(
        base_dir: impl Into<PathBuf>,
        file_path: impl Into<PathBuf>,
    ) -> Result<Self, std::io::Error> {
        let base_dir = base_dir.into();
        let file_path = file_path.into();

        let original_content = std::fs::read_to_string(base_dir.join(&file_path))?;

        Ok(Self {
            base_dir,
            file_path,
            original_content,
            options: Self::options(),
        })
    }

    pub fn build(self) -> anyhow::Result<Markdown> {
        let arena = comrak::Arena::new();

        let root = comrak::parse_document(&arena, &self.original_content, &self.options);

        for node in root.descendants() {
            match &mut node.data.borrow_mut().value {
                NodeValue::Link(link) => self.patch_link(link)?,
                NodeValue::Image(link) => self.patch_image_link(link)?,
                // NodeValue::WikiLink(wiki_link) => todo!(),
                _ => {}
            }
        }

        let frontmatter = self.get_frontmatter(root)?;
        let title = self
            .get_title(&frontmatter, root)
            .context("title not found")?;
        let description = self.get_description(&frontmatter, root);

        let mut bw = std::io::BufWriter::new(Vec::new());
        comrak::format_html(root, &self.options, &mut bw)?;
        let html = String::from_utf8(bw.into_inner()?)?;

        Ok(Markdown {
            base_dir: self.base_dir,
            file_path: self.file_path,
            content: Content {
                markdown: self.original_content,
                html,
            },
            title,
            description,
            tags: frontmatter.tags,
        })
    }

    fn options() -> comrak::Options<'static> {
        let extension = comrak::ExtensionOptions::builder()
            .strikethrough(true)
            .table(true)
            .autolink(true)
            .tasklist(true)
            .superscript(true)
            .header_ids("heading-".to_string())
            .footnotes(true)
            .description_lists(true)
            .front_matter_delimiter("---".to_string())
            .alerts(true)
            .math_dollars(true)
            .math_code(true)
            .shortcodes(true)
            .subscript(true)
            .cjk_friendly_emphasis(true)
            .build();
        let parse = comrak::ParseOptions::builder().build();
        let render = comrak::RenderOptions::builder()
            .experimental_minimize_commonmark(true)
            .build();

        comrak::Options {
            extension,
            parse,
            render,
        }
    }

    fn get_frontmatter<'a>(&self, root: &'a Node<'a, RefCell<Ast>>) -> anyhow::Result<Frontmatter> {
        let get_frontmatter_value = |node: &Node<'_, RefCell<Ast>>| -> Option<String> {
            if let NodeValue::FrontMatter(str) = &node.data.borrow().value {
                let str = str.trim().trim_matches('-').trim();
                Some(str.to_string())
            } else {
                None
            }
        };

        let ret = root
            .descendants()
            .find_map(get_frontmatter_value)
            .map(|s| serde_yaml::from_str::<Frontmatter>(&s))
            .transpose()
            .context("failed to parse yaml frontmatter")?
            .unwrap_or_default();

        Ok(ret)
    }

    fn get_title<'a>(
        &self,
        frontmatter: &Frontmatter,
        root: &'a Node<'a, RefCell<Ast>>,
    ) -> Option<Content> {
        let markdown = match &frontmatter.title {
            Some(title) => Some(title.to_string()),
            // find first h1 heading
            None => match root.descendants().find(|node| {
                matches!(
                    node.data.borrow().value,
                    NodeValue::Heading(heading) if heading.level == 1
                )
            }) {
                Some(node) => {
                    let mut output = Vec::new();
                    let _ = comrak::format_commonmark(node, &self.options, &mut output);
                    let output = String::from_utf8(output).unwrap();
                    // trim `# ` at the start and `\n` at the end
                    let output = output[2..].trim_end().to_string();
                    Some(output)
                }
                None => None,
            },
        };

        markdown.map(|t| Content {
            html: comrak::markdown_to_html(&t, &self.options),
            markdown: t,
        })
    }

    fn get_description<'a>(
        &self,
        frontmatter: &Frontmatter,
        root: &'a Node<'a, RefCell<Ast>>,
    ) -> Option<Content> {
        let markdown = match &frontmatter.description {
            Some(title) => Some(title.to_string()),
            // find first paragraph
            None => match root
                .descendants()
                .find(|node| matches!(node.data.borrow().value, NodeValue::Paragraph))
            {
                Some(node) => {
                    let mut output = Vec::new();
                    let _ = comrak::format_commonmark(node, &self.options, &mut output);
                    let output = String::from_utf8(output).unwrap();
                    Some(output)
                }
                None => None,
            },
        };

        markdown.map(|t| Content {
            html: comrak::markdown_to_html(&t, &self.options),
            markdown: t,
        })
    }

    fn patch_link(&self, link: &mut NodeLink) -> anyhow::Result<()> {
        if is_url(&link.url) {
            return Ok(());
        }

        let path = Path::new(&link.url);
        let mut new_path = get_abs_path(&self.file_path, path)
            .context("the path is points outside current directory")?;

        if new_path.extension_is(Some("md")) && new_path.starts_with("/blog/") {
            let (_date, name) = BlogEntry::parse_path(&new_path)?;
            new_path.set_file_name(Path::new(&name));
        }

        if new_path.extension_is(Some("md")) {
            new_path.set_extension("");
        }

        link.url = new_path
            .to_str()
            .context("the path isn't valid")?
            .to_string();

        Ok(())
    }

    fn patch_image_link(&self, link: &mut NodeLink) -> anyhow::Result<()> {
        if is_url(&link.url) {
            return Ok(());
        }

        let path = Path::new(&link.url);
        let mut new_path = get_abs_path(&self.file_path, path)
            .context("the path is points outside current directory")?;

        // remove `.md` extension
        if new_path.extension_is(Some("md")) {
            new_path.set_extension("");
        }

        link.url = new_path.as_os_str().to_string_lossy().to_string();

        Ok(())
    }
}

fn is_url(str: &str) -> bool {
    str.parse::<url::Url>().is_ok()
}

fn get_abs_path(
    markdown_path: impl AsRef<Path>,
    link_file_path: impl AsRef<Path>,
) -> Option<PathBuf> {
    let dir_path = markdown_path
        .as_ref()
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let path = dir_path.join(link_file_path).try_normalize()?;
    Some(Path::new("/").join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(result: Option<PathBuf>, expected: Option<&str>) {
        println!("result: {:?}", result);
        assert_eq!(
            result.and_then(|x| x.to_str().map(|x| x.to_string())),
            expected.map(|x| x.to_string())
        );
    }

    #[test]
    fn path_join() {
        test_path(
            get_abs_path("foo/bar.md", "images/image.png"),
            Some("/foo/images/image.png"),
        );

        test_path(
            get_abs_path("foo/bar.md", "./images/image.png"),
            Some("/foo/images/image.png"),
        );

        test_path(
            get_abs_path("foo/bar.md", "../images/image.png"),
            Some("/images/image.png"),
        );

        test_path(get_abs_path("foo/bar.md", "../../images/image.png"), None);

        test_path(get_abs_path("foo/bar.md", "/images/image.png"), None);
    }

    #[test]
    fn basic_test_not_found() {
        assert!(Markdown::builder("", "tests/not_found.md").is_err());
    }

    #[test]
    fn basic_test_md_1() {
        let md = Markdown::builder("", "tests/test-1.md")
            .unwrap()
            .build()
            .unwrap();

        println!("{}", md.content.html);
        println!("title: {:?}", md.title);
        println!("description: {:?}", md.description);
        println!("tags: {:?}", md.tags);

        // panic!()
    }

    #[test]
    fn basic_test_md_2() {
        let md = Markdown::builder("", "tests/test-2.md")
            .unwrap()
            .build()
            .unwrap();

        println!("{}", md.content.html);
        println!("title: {:?}", md.title);
        println!("description: {:?}", md.description);
        println!("tags: {:?}", md.tags);

        // panic!()
    }
}
