use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::Context as _;

use crate::{
    config::{self, Config},
    git_repo::{self, GitRepo},
    markdown, pages,
};

pub struct Generator {
    src_dir: PathBuf,
    dst_dir: PathBuf,
    config: Config,
    git_repo: GitRepo,
    skip: HashSet<&'static Path>,

    all_blog: Vec<BlogEntry>,
}

#[derive(Debug, Clone)]
struct BlogEntry {
    /// `blog/yyyy-mm-dd-blog-slug.md`
    rel_md_path: PathBuf,
    /// `blog/yyyy-mm-dd-blog-slug`
    rel_path: PathBuf,

    time: chrono::NaiveDate,
    slug: String,

    last_commit: Option<BlogCommit>,

    markdown: markdown::Markdown,
}

#[derive(Debug, Clone)]
pub struct BlogCommit {
    pub time: chrono::DateTime<chrono::FixedOffset>,
    pub hash: String,
    pub summary: Option<String>,
    pub base_url: String,
}

impl Generator {
    pub fn new(src_dir: impl Into<PathBuf>, dst_dir: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let src_dir = src_dir.into();
        let dst_dir = dst_dir.into();

        if dst_dir.try_exists()? {
            return Err(anyhow::anyhow!("output dir is not empty"));
        }

        log::info!("open git repo: {}", src_dir.display());
        let git_repo = GitRepo::new(&src_dir)?;

        let config_file = Path::new("config.yaml");
        log::info!("read config from: {}", config_file.display());
        let config = Config::from_file(src_dir.join(config_file))?;

        let skip = [".git", ".cspell.yaml", "README.md", "config.yaml"];
        let skip: HashSet<_> = skip.into_iter().map(Path::new).collect();

        Ok(Self {
            src_dir,
            dst_dir,
            config,
            git_repo,
            skip,
            all_blog: Vec::new(),
        })
    }

    pub fn build(mut self) -> anyhow::Result<()> {
        log::info!("create dest dir: {}", self.dst_dir.display());
        fs::create_dir_all(&self.dst_dir)?;

        log::info!("copy static dir: {}", config::STATIC_DIR);
        crate::static_dir::copy_static_dir_to(self.dst_dir.join(config::STATIC_DIR))?;

        let src_dir = self.src_dir.clone();
        self.iter_dir(&src_dir)?;

        // handle special page
        std::fs::copy(
            self.dst_dir.join(Self::md_to_html_path(config::HOME_MD)),
            self.dst_dir.join("index.html"),
        )?;
        std::fs::copy(
            self.dst_dir
                .join(Self::md_to_html_path(config::NOT_FOUND_MD)),
            self.dst_dir.join("not_found.html"),
        )?;

        // process blog entries
        self.all_blog.sort_by_key(|x| std::cmp::Reverse(x.time));
        let all_blog_entries: Vec<_> = self.all_blog.iter().map(BlogEntry::as_page).collect();

        log::info!("build blog home");
        self.build_blog_home(&all_blog_entries)?;

        let tag_blog_list = Self::process_tag_blog_list(&all_blog_entries);

        for (tag, blog_entries) in tag_blog_list {
            log::info!("build blog tag home: {tag}");
            self.build_blog_tag_home(&tag, &blog_entries)?;
        }

        log::info!("build rss");
        self.build_rss()?;

        Ok(())
    }

    fn iter_dir(&mut self, rel_cur_dir: &Path) -> anyhow::Result<()> {
        for entry in fs::read_dir(rel_cur_dir)? {
            let entry = entry?;
            let path = entry.path();

            let Ok(rel_path) = path.strip_prefix(&self.src_dir) else {
                log::warn!("cannot get relative path for {}", path.display());
                continue;
            };

            if self.skip.contains(&rel_path) {
                continue;
            }

            if path.is_dir() {
                self.iter_dir(&path)?;
            } else {
                self.handle_file(rel_path)?;
            }
        }

        Ok(())
    }

    fn handle_file(&mut self, rel_path: &Path) -> anyhow::Result<()> {
        let src_path = self.src_dir.join(rel_path);
        let dst_path = self.dst_dir.join(rel_path);

        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if rel_path.extension().and_then(|x| x.to_str()) == Some("md") {
            if let Some(blog_entry) = self.try_get_blog_entry(rel_path)? {
                log::info!("build blog: {}", rel_path.display());
                self.render_blog_page(&blog_entry, &blog_entry.rel_path)?;
                self.all_blog.push(blog_entry);
            } else {
                log::info!("build md: {}", rel_path.display());
                let md = markdown::read_md(&self.src_dir, rel_path)?;
                self.render_markdown(&md, rel_path)?;
            }
        } else {
            log::info!("copy file: {}", rel_path.display());
            std::fs::copy(src_path, self.dst_dir.join(rel_path))?;
        }

        Ok(())
    }

    fn try_get_blog_entry(
        &self,
        rel_md_path: impl AsRef<Path>,
    ) -> anyhow::Result<Option<BlogEntry>> {
        let rel_md_path = rel_md_path.as_ref();

        if rel_md_path.extension().and_then(|x| x.to_str()) != Some("md") {
            return Ok(None);
        }

        if rel_md_path
            .parent()
            .is_none_or(|p| p != Path::new(config::BLOG_DIR))
        {
            return Ok(None);
        }

        let p = rel_md_path.with_extension("");
        let Some((time, slug)) = p
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| markdown::parse_blog_file_name(s).ok())
        else {
            return Ok(None);
        };

        let commits = self.git_repo.commits_for_file(rel_md_path)?;
        let last_commit = commits.first();

        let markdown = markdown::read_md(&self.src_dir, rel_md_path)?;

        Ok(Some(BlogEntry {
            rel_md_path: rel_md_path.to_path_buf(),
            rel_path: rel_md_path.with_extension(""),

            time,
            slug: slug.to_string(),
            last_commit: last_commit.map(|c| BlogCommit {
                time: git_repo::git_time_to_datetime(c.time()),
                hash: c.id().to_string(),
                summary: c.summary().map(|x| x.to_string()),
                base_url: self.config.commit_base_url.clone(),
            }),

            markdown,
        }))
    }

    fn render_markdown(
        &'_ self,
        md: &markdown::Markdown,
        rel_path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let rel_path = rel_path.as_ref();

        let html_path = Self::md_to_html_path(rel_path);

        let title = if rel_path == Path::new(config::HOME_MD) {
            &self.config.site_name
        } else {
            &self.title_with_author(&md.meta.title)
        };

        let page = pages::Base {
            head: pages::Head {
                title,
                description: md.meta.description_md.as_deref(),
                author: &self.config.author,
            },
            body: pages::Body {
                header: self.get_header(html_path.to_str()),
                footer: self.get_footer(),
                main: pages::Article { raw_html: &md.html },
            },
        };

        let output_path = self.dst_dir.join(&html_path);
        page.render_into(output_path)
            .context("failed to render page into file")?;

        Ok(())
    }

    fn render_blog_page(
        &'_ self,
        blog: &BlogEntry,
        rel_path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let html_path = Self::md_to_html_path(rel_path);

        let title = self.title_with_author(&blog.markdown.meta.title);

        let last_update_time = blog.last_commit.as_ref().map(|x| x.time.date_naive());
        let last_update_time = last_update_time.unwrap_or(blog.time);

        let page = pages::Base {
            head: pages::Head {
                title: &title,
                description: blog.markdown.meta.description_md.as_deref(),
                author: &self.config.author,
            },
            body: pages::Body {
                header: self.get_header(html_path.to_str()),
                footer: self.get_footer(),
                main: pages::BlogPage {
                    publish_time: blog.time,
                    last_update_time,
                    last_commit: blog.last_commit.as_ref(),
                    markdown: &blog.markdown,
                },
            },
        };

        let output_path = self.dst_dir.join(&html_path);
        page.render_into(output_path)
            .context("failed to render page into file")?;

        Ok(())
    }

    fn build_blog_home(&self, blog_entries: &[pages::BlogEntry]) -> anyhow::Result<()> {
        let html_path = "blog/index.html";

        let title = self.title_with_author("blog");

        let page = pages::Base {
            head: pages::Head {
                title: &title,
                description: Some("blog"),
                author: &self.config.author,
            },
            body: pages::Body {
                header: self.get_header(Some(html_path)),
                footer: self.get_footer(),
                main: pages::BlogHome { blog_entries },
            },
        };

        let output_path = self.dst_dir.join(html_path);
        page.render_into(output_path)
            .context("failed to render page into file")?;

        Ok(())
    }

    fn build_blog_tag_home(
        &self,
        tag: &str,
        blog_entries: &[pages::BlogEntry],
    ) -> anyhow::Result<()> {
        let html_path = format!("blog/tags/{}/index.html", tag);

        let title = format!("#{tag}");
        let title = self.title_with_author(&title);

        let page = pages::Base {
            head: pages::Head {
                title: &title,
                description: Some(&title),
                author: &self.config.author,
            },
            body: pages::Body {
                header: self.get_header(Some(&html_path)),
                footer: self.get_footer(),
                main: pages::BlogTagHome {
                    tag_name: tag,
                    blog_entries,
                },
            },
        };

        let output_path = self.dst_dir.join(&html_path);
        page.render_into(output_path)
            .context("failed to render page into file")?;

        Ok(())
    }

    fn build_rss(&self) -> anyhow::Result<()> {
        let out_path = "blog/rss.xml";

        let mut atom_link = rss::extension::atom::Link::default();
        atom_link.set_href(format!("{}/{}", self.config.site_url, out_path));
        atom_link.set_rel("self");
        atom_link.set_mime_type(Some("application/rss+xml".to_string()));
        let atom_ext = rss::extension::atom::AtomExtension {
            links: vec![atom_link],
        };

        let last_update_time = self
            .all_blog
            .iter()
            .filter_map(|x| x.last_commit.as_ref())
            .map(|x| x.time.to_utc())
            .max();

        let Some(last_update_time) = last_update_time else {
            return Ok(());
        };

        let items: Vec<_> = self.all_blog.iter().map(|x| self.to_rss_item(x)).collect();

        let rss = rss::ChannelBuilder::default()
            .title(&self.config.site_name)
            .link(&self.config.site_url)
            .description(&self.config.site_name)
            .pub_date(last_update_time.to_rfc2822())
            .last_build_date(last_update_time.to_rfc2822())
            .items(items)
            .atom_ext(atom_ext)
            .build();

        fs::write(self.dst_dir.join(out_path), rss.to_string().into_bytes())?;

        Ok(())
    }

    fn process_tag_blog_list<'b>(
        blog: &[pages::BlogEntry<'b>],
    ) -> HashMap<String, Vec<pages::BlogEntry<'b>>> {
        let mut ret: HashMap<_, Vec<_>> = HashMap::new();

        for &b in blog {
            for t in b.tags {
                match ret.get_mut(t) {
                    Some(l) => {
                        l.push(b);
                    }
                    None => {
                        ret.insert(t.to_string(), vec![b]);
                    }
                }
            }
        }

        ret
    }

    /// `abc.md` -> `abc/index.html`
    /// `/aaa/abc.md` -> `/aaa/abc/index.html`
    fn md_to_html_path(md: impl AsRef<Path>) -> PathBuf {
        md.as_ref().with_extension("").join("index.html")
    }

    fn to_rss_item(&self, blog_entry: &BlogEntry) -> rss::Item {
        let link = format!("{}/{}", self.config.site_url, blog_entry.rel_path.display());
        let author = format!("{} ({})", self.config.author_email, self.config.author);

        let description = blog_entry.markdown.meta.description_html.clone();

        let categories: Vec<_> = blog_entry
            .markdown
            .meta
            .tags
            .iter()
            .map(|x| rss::Category {
                name: x.clone(),
                domain: None,
            })
            .collect();

        let pub_date = blog_entry
            .time
            .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_utc()
            .to_rfc2822();

        rss::ItemBuilder::default()
            .title(blog_entry.markdown.meta.title.clone())
            .link(Some(link.clone()))
            .description(description)
            .author(Some(author))
            .categories(categories)
            .guid(Some(rss::Guid {
                value: link,
                permalink: true,
            }))
            .pub_date(Some(pub_date))
            .content(blog_entry.markdown.html.clone())
            .build()
    }

    fn title_with_author(&self, title: &str) -> String {
        format!("{} - {}", title, self.config.author)
    }

    fn get_header<'a>(&'a self, active_url: Option<&'a str>) -> pages::Header<'a> {
        pages::Header {
            home_name: &self.config.header.home_name,
            links: &self.config.header.links,
            active_url,
        }
    }
    fn get_footer(&self) -> pages::Footer<'_> {
        pages::Footer {
            links: &self.config.footer.links,
            cc_text: &self.config.footer.cc,
        }
    }
}

trait RenderIntoExt {
    fn render_into(&self, output_path: impl AsRef<Path>) -> std::io::Result<usize>;
}

impl<T: hypertext::Renderable> RenderIntoExt for T {
    fn render_into(&self, output_path: impl AsRef<Path>) -> std::io::Result<usize> {
        let rendered = self.render().into_inner();

        let content = minify_html::minify(rendered.as_bytes(), &minify_html::Cfg::new());

        if let Some(parent_dir) = output_path.as_ref().parent() {
            fs::create_dir_all(parent_dir)?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(output_path)?
            .write(&content)
    }
}

impl BlogEntry {
    fn as_page(&'_ self) -> pages::BlogEntry<'_> {
        pages::BlogEntry {
            publish_time: self.time,
            title: &self.markdown.meta.title,
            rel_path: &self.rel_path,
            tags: &self.markdown.meta.tags,
        }
    }
}
