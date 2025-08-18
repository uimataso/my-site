use std::{
    collections::HashMap,
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use chrono::TimeZone as _;

use crate::{
    Config, PathExt as _, blog_entry::BlogEntry, css, git_repo::GitRepo, markdown::Markdown, pages,
    to_datetime,
};

pub struct Generator {
    config: Config,
    git_repo: GitRepo,
}

impl Generator {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let git_repo = GitRepo::new(&config.source_dir, &config.remote_url)?;

        Ok(Self { config, git_repo })
    }

    fn source_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.source_dir.join(path)
    }
    fn output_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.output_dir.join(path)
    }

    pub fn build(&self) -> anyhow::Result<()> {
        println!(
            "build site from {} to {}",
            self.config.source_dir.display(),
            self.config.output_dir.display()
        );

        // static
        copy_dir_all_check(&self.config.static_dir, self.output_path("static"), |p| {
            !p.extension_is(Some("css"))
        })?;
        fs::copy("/static/favicon.ico", self.output_path("favicon.ico"))?;

        css::bundle_and_minify(
            "/static/css/main.css",
            self.output_path("static/styles.css"),
        )?;

        self.copy_dir("images", "images")?;

        let page_names = self.get_pages_name()?;

        let mut list = vec![("/blog".to_string(), "blog".to_string())];
        for name in &page_names {
            list.push((format!("/{name}"), name.clone()));
        }

        let nav = pages::Nav {
            list,
            active: None,
            home_name: self.config.content.site_name.clone(),
        };
        let footer = pages::Footer {
            author: self.config.content.author.clone(),
            links: self.config.content.footer_links.clone(),
        };

        self.build_blog(nav.clone(), footer.clone())?;

        self.build_home(nav.clone(), footer.clone())?;
        for name in &page_names {
            self.build_page(name, nav.clone(), footer.clone(), None)?;
        }
        self.build_page("not_found", nav, footer, Some("not_found.html"))?;

        Ok(())
    }

    fn get_pages_name(&self) -> anyhow::Result<Vec<String>> {
        let excluded_list = ["README", "index", "home", "not_found"];

        let mut page_names = Vec::new();

        for entry in fs::read_dir(&self.config.source_dir)? {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();

            let path_without_ext = path.with_extension("");
            let Some(name) = path_without_ext.file_name().and_then(|x| x.to_str()) else {
                continue;
            };

            if !path.is_dir() && path.extension_is(Some("md")) && !excluded_list.contains(&name) {
                page_names.push(name.to_string());
            }
        }

        Ok(page_names)
    }

    fn build_blog(&self, mut nav: pages::Nav, footer: pages::Footer) -> anyhow::Result<()> {
        nav.active = Some("/blog".to_string());

        self.copy_dir("blog/images", "blog/images")?;

        let now = chrono::Utc::now().into();

        // get blogs list
        let mut blog_entries = Vec::new();

        for entry in fs::read_dir(self.source_path("blog"))? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() && path.extension_is(Some("md")) {
                let rel_path = path.strip_prefix(&self.config.source_dir)?.to_path_buf();
                let blog_entry = BlogEntry::new(&self.config.source_dir, rel_path)?;

                blog_entries.push(blog_entry.clone());
            }
        }

        blog_entries.sort_by_key(|x| x.date);
        blog_entries.reverse();

        let mut last_update_time = chrono::FixedOffset::east_opt(0)
            .expect("hard coded offset")
            .timestamp_opt(0, 0)
            .unwrap();

        let mut blog_tags: HashMap<String, Vec<BlogEntry>> = HashMap::new();

        // build blog page
        for blog_entry in &blog_entries {
            let commits = self.git_repo.commits_for_file(&blog_entry.path)?;

            last_update_time =
                last_update_time.max(commits.first().map_or(now, |c| to_datetime(c.time())));

            let markdown = &blog_entry.markdown;

            let page = pages::Base {
                header: pages::Header {
                    author: self.config.content.author.clone(),
                    title: self.title_with_author(&markdown.title.markdown),
                    description: markdown.description.as_ref().map(|x| x.markdown.clone()),
                },
                nav: nav.clone(),
                main_content: pages::BlogPage {
                    commits: &commits,
                    markdown,
                    commit_base_url: &self.config.content.commit_base_url,
                },
                footer: footer.clone(),
            };

            let output_path = Path::new("blog").join(&blog_entry.slug).join("index.html");
            page.render_into(self.output_path(output_path))?;

            for tag in blog_entry.markdown.tags.clone() {
                blog_tags
                    .entry(tag.clone())
                    .or_default()
                    .push(blog_entry.clone());
            }
        }

        // build blog tags page
        for (tag, blogs) in &blog_tags {
            let page = pages::Base {
                header: pages::Header {
                    author: self.config.content.author.clone(),
                    title: self.title_with_author("blog"),
                    description: None,
                },
                nav: nav.clone(),
                main_content: pages::BlogTagPage { tag, blogs },
                footer: footer.clone(),
            };

            page.render_into(self.output_path(format!("blog/tags/{}/index.html", tag)))?;
        }

        // build blog home
        let page = pages::Base {
            header: pages::Header {
                author: self.config.content.author.clone(),
                title: self.title_with_author("blog"),
                description: None,
            },
            nav,
            main_content: pages::BlogHome {
                blogs: &blog_entries,
            },
            footer,
        };

        page.render_into(self.output_path("blog/index.html"))?;

        // build rss
        let items: Vec<_> = blog_entries.iter().map(|x| self.to_rss_item(x)).collect();

        let mut atom_link = rss::extension::atom::Link::default();
        atom_link.set_href(format!("{}/blog/rss.xml", self.config.content.site_url));
        atom_link.set_rel("self");
        atom_link.set_mime_type(Some("application/rss+xml".to_string()));
        let atom_ext = rss::extension::atom::AtomExtension {
            links: vec![atom_link],
        };

        let rss = rss::ChannelBuilder::default()
            .title(&self.config.content.site_name)
            .link(&self.config.content.site_url)
            .description(&self.config.content.site_name)
            .pub_date(last_update_time.to_rfc2822())
            .last_build_date(last_update_time.to_rfc2822())
            .items(items)
            .atom_ext(atom_ext)
            .build();

        let rss_result = rss.to_string();
        fs::write(self.output_path("blog/rss.xml"), rss_result.into_bytes())?;

        Ok(())
    }

    fn build_home(&self, nav: pages::Nav, footer: pages::Footer) -> anyhow::Result<()> {
        let markdown = Markdown::builder(&self.config.source_dir, "home.md")?.build()?;
        let description = format!("home page of {}", self.config.content.site_name);

        let page = pages::Base {
            header: pages::Header {
                author: self.config.content.author.clone(),
                title: self.config.content.site_name.clone(),
                description: Some(description),
            },
            nav,
            main_content: hypertext::Raw::dangerously_create(markdown.content.html),
            footer,
        };

        page.render_into(self.output_path("index.html"))?;

        Ok(())
    }

    fn build_page(
        &self,
        page_name: &str,
        nav: pages::Nav,
        footer: pages::Footer,
        alt_output_path: Option<&str>,
    ) -> anyhow::Result<()> {
        let file_path = format!("{page_name}.md");

        let markdown = Markdown::builder(&self.config.source_dir, &file_path)?.build()?;

        let mut nav = nav;
        nav.active = Some(format!("/{page_name}"));

        let page = pages::Base {
            header: pages::Header {
                author: self.config.content.author.clone(),
                title: self.title_with_author(&markdown.title.markdown),
                description: markdown.description.as_ref().map(|x| x.markdown.clone()),
            },
            nav,
            main_content: pages::Article {
                raw_html: &markdown.content.html,
            },
            footer,
        };

        let default_path = format!("{page_name}/index.html");
        let output_path = alt_output_path.unwrap_or(&default_path);

        page.render_into(self.output_path(output_path))?;

        Ok(())
    }

    fn to_rss_item(&self, blog_entry: &BlogEntry) -> rss::Item {
        let link = format!("{}/blog/{}", self.config.content.site_url, blog_entry.slug);

        let description = blog_entry
            .markdown
            .description
            .as_ref()
            .map(|x| x.html.clone());

        let author = format!(
            "{} ({})",
            self.config.content.author_email, self.config.content.author
        );

        let categories: Vec<_> = blog_entry
            .markdown
            .tags
            .iter()
            .map(|x| rss::Category {
                name: x.clone(),
                domain: None,
            })
            .collect();

        let pub_date = blog_entry
            .date
            .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_utc()
            .to_rfc2822();

        rss::ItemBuilder::default()
            .title(blog_entry.markdown.title.markdown.clone())
            .link(Some(link.clone()))
            .description(description)
            .author(Some(author))
            .categories(categories)
            .guid(Some(rss::Guid {
                value: link,
                permalink: true,
            }))
            .pub_date(Some(pub_date))
            .content(blog_entry.markdown.content.html.clone())
            .build()
    }

    fn copy_dir(&self, source: impl AsRef<Path>, output: impl AsRef<Path>) -> anyhow::Result<()> {
        let source = self.source_path(&source);
        let output = self.output_path(&output);

        if !source.exists() {
            println!("the source not exists, source: {}", source.display());
            return Ok(());
        }
        if !source.is_dir() {
            println!("the source is not dir, source: {}", source.display());
            return Ok(());
        }

        copy_dir_all(&source, &output).with_context(|| {
            format!(
                "failed to copy dir, source: {}, output: {}",
                source.display(),
                output.display()
            )
        })
    }

    fn title_with_author(&self, text: &str) -> String {
        format!("{} - {}", text, self.config.content.author)
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    copy_dir_all_check(src, dst, |_| true)
}

fn copy_dir_all_check(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    should_copy: fn(&Path) -> bool,
) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all_check(
                entry.path(),
                dst.as_ref().join(entry.file_name()),
                should_copy,
            )?;
        } else if should_copy(&entry.path()) {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

trait RenderIntoExt {
    fn render_into(&self, output_path: impl AsRef<Path>) -> io::Result<usize>;
}

impl<T: hypertext::Renderable> RenderIntoExt for T {
    fn render_into(&self, output_path: impl AsRef<Path>) -> io::Result<usize> {
        let rendered = self.render().into_inner();

        let content = if output_path.as_ref().extension_is(Some("html")) {
            minify_html::minify(rendered.as_bytes(), &minify_html::Cfg::new())
        } else {
            rendered.into_bytes()
        };

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
