use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context as _;
use comrak::{Arena, Node, nodes::NodeValue, plugins::syntect::SyntectAdapter};
use normalize_path::NormalizePath as _;
use serde::Deserialize;

pub fn read_md(
    base_dir: impl Into<PathBuf>,
    file_path: impl Into<PathBuf>,
) -> anyhow::Result<Markdown> {
    let source = MarkdownSource::new(base_dir, file_path)?;
    let ast = source.parse();
    let meta = ast.to_meta()?;
    let html = ast.to_html()?;
    Ok(Markdown { meta, html })
}

/// Parse blog file name: `yyyy-mm-dd-blog-slug`
///
/// note: without `.md`
pub fn parse_blog_file_name(name: &str) -> anyhow::Result<(chrono::NaiveDate, &str)> {
    let split: Vec<_> = name.splitn(4, '-').collect();

    if let &[year, month, day, slug] = split.as_slice() {
        let year = year.parse().context("failed to parse year")?;
        let month = month.parse().context("failed to parse month")?;
        let day = day.parse().context("failed to parse day")?;

        let date =
            chrono::NaiveDate::from_ymd_opt(year, month, day).context("the date isn't valid")?;

        Ok((date, slug))
    } else {
        Err(anyhow::anyhow!(
            "expected the filename have the format of \"yyyy-mm-dd-blog-name\""
        ))
    }
}

#[derive(Debug, Clone)]
pub struct Markdown {
    pub meta: MarkdownMeta,
    pub html: String,
}

#[derive(Debug, Clone)]
pub struct MarkdownMeta {
    pub title: String,
    pub description_md: Option<String>,
    pub description_html: Option<String>,
    pub tags: Vec<String>,
}

struct MarkdownSource<'a> {
    base_dir: PathBuf,
    file_path: PathBuf,

    content: String,
    arena: Arena<'a>,
}

struct MarkdownAst<'a> {
    root: Node<'a>,
    options: comrak::Options<'static>,
}

#[derive(Debug, Default, Deserialize)]
struct Frontmatter {
    title: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

impl<'a> MarkdownSource<'a> {
    fn new(base_dir: impl Into<PathBuf>, file_path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let base_dir = base_dir.into();
        let file_path = file_path.into();

        let content = std::fs::read_to_string(base_dir.join(&file_path))?;
        let arena = Arena::new();

        Ok(Self {
            base_dir,
            file_path,
            content,
            arena,
        })
    }

    fn parse(&'a self) -> MarkdownAst<'a> {
        let options = self.options();
        let root = comrak::parse_document(&self.arena, &self.content, &options);
        MarkdownAst { root, options }
    }

    fn options(&self) -> comrak::Options<'static> {
        let dir_path = self
            .file_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();

        let link_url_rewriter = move |url: &str| {
            // if `url` is real url (not a path)
            if url.contains("://") || url.starts_with("mailto:") {
                return url.to_string();
            }

            // get path relative to base dir
            let mut p = Path::new("/").join(&dir_path).join(url).normalize();

            // remove `.md` extension
            if p.extension().and_then(|x| x.to_str()) == Some("md") {
                p.set_extension("");
            }

            p.to_str().unwrap_or_default().to_string()
        };

        let mut options = default_option();

        options.extension.link_url_rewriter = Some(Arc::new(link_url_rewriter));

        options
    }
}

impl MarkdownAst<'_> {
    fn to_meta(&self) -> anyhow::Result<MarkdownMeta> {
        let frontmatter = self
            .get_frontmatter()
            .context("failed to get frontmatter")?;

        let title = frontmatter
            .title
            .or_else(|| self.find_title())
            .context("cannot get title")?;

        let description_md = frontmatter.description.or_else(|| self.find_description());
        let description_html = description_md
            .as_deref()
            .map(|md| comrak::markdown_to_html(md, &self.options));

        Ok(MarkdownMeta {
            title,
            description_md,
            description_html,
            tags: frontmatter.tags,
        })
    }

    fn to_html(&self) -> anyhow::Result<String> {
        let mut ret = String::new();

        // code highlight
        let adapter = SyntectAdapter::new(None);
        let mut plugins = comrak::options::Plugins::default();
        plugins.render.codefence_syntax_highlighter = Some(&adapter);

        comrak::format_html_with_plugins(self.root, &self.options, &mut ret, &plugins)?;

        Ok(ret)
    }

    fn find_first_node<T>(&self, find: impl FnMut(Node<'_>) -> Option<T>) -> Option<T> {
        self.root.descendants().find_map(find)
    }

    fn node_to_markdown(&self, node: Node<'_>) -> String {
        let mut output = String::new();
        let _ = comrak::format_commonmark(node, &self.options, &mut output);
        output
    }

    fn get_frontmatter(&self) -> anyhow::Result<Frontmatter> {
        let get_frontmatter_value = |node: Node<'_>| match &node.data().value {
            NodeValue::FrontMatter(str) => {
                let str = str.trim().trim_matches('-').trim();
                Some(str.to_string())
            }
            _ => None,
        };

        let Some(text) = self.find_first_node(get_frontmatter_value) else {
            return Ok(Frontmatter::default());
        };

        let frontmatter =
            serde_yaml::from_str(&text).context("failed to parse yaml frontmatter")?;

        Ok(frontmatter)
    }

    fn find_title(&self) -> Option<String> {
        let get_title = |node: Node<'_>| match &node.data().value {
            NodeValue::Heading(heading) if heading.level == 1 => Some(self.node_to_markdown(node)),
            _ => None,
        };

        self.find_first_node(get_title)
            // trim `# ` at the start and `\n` at the end
            .map(|t| t[2..].trim_end().to_string())
    }

    fn find_description(&self) -> Option<String> {
        let get_paragraph = |node: Node<'_>| match node.data().value {
            NodeValue::Paragraph => Some(self.node_to_markdown(node)),
            _ => None,
        };

        // find first paragraph
        self.find_first_node(get_paragraph)
    }
}

pub fn default_option() -> comrak::Options<'static> {
    let extension = comrak::options::Extension {
        strikethrough: true,
        table: true,
        autolink: true,
        tasklist: true,
        superscript: true,
        header_ids: Some("heading-".to_string()),
        footnotes: true,
        description_lists: true,
        front_matter_delimiter: Some("---".to_string()),
        alerts: true,
        math_dollars: true,
        math_code: true,
        shortcodes: true,
        subscript: true,
        cjk_friendly_emphasis: true,
        ..Default::default()
    };
    let parse = comrak::options::Parse {
        ..Default::default()
    };
    let render = comrak::options::Render {
        experimental_minimize_commonmark: true,
        ..Default::default()
    };

    comrak::Options {
        extension,
        parse,
        render,
    }
}
