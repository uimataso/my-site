use hypertext::{Raw, prelude::*};

use crate::{
    blog_entry::BlogEntry,
    markdown::Markdown,
    pages::{self, commits::Commits},
};

#[derive(Clone)]
pub struct BlogHome<'a> {
    pub blogs: &'a [BlogEntry],
}

#[derive(Clone)]
pub struct BlogTagPage<'a> {
    pub tag: &'a str,
    pub blogs: &'a [BlogEntry],
}

#[derive(Clone)]
pub struct BlogPage<'a> {
    pub commits: &'a [git2::Commit<'a>],
    pub markdown: &'a Markdown,
    pub commit_base_url: &'a str,
}

#[derive(Clone)]
struct Tags<'a> {
    tags: &'a [String],
}

impl Renderable for BlogHome<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <div class="blogs">
                <ul>
                    @for blog in self.blogs {
                        (blog)
                    }
                </ul>
            </div>
        }
        .render_to(buffer);
    }
}

impl Renderable for BlogTagPage<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <div class="tag-title">
                <h1>"#" (self.tag)</h1>
            </div>

            <div class="blogs">
                <ul>
                    @for blog in self.blogs {
                        (blog)
                    }
                </ul>
            </div>
        }
        .render_to(buffer);
    }
}

impl Renderable for BlogEntry {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        let url = format!("/blog/{}", self.slug);
        let title = &self.markdown.title.html;
        let date = self.date.format("%Y-%m-%d").to_string();

        let tags = Tags {
            tags: &self.markdown.tags,
        };

        rsx! {
            <div class="blog-entry">
                <div class="blog-date">
                    <p>(date)</p>
                </div>

                <div class="blog-link">
                    <div class="blog-title">
                        <a href=(url)>
                            (Raw::dangerously_create(title))
                        </a>
                    </div>

                    (tags)
                </div>
            </div>
        }
        .render_to(buffer);
    }
}

impl Renderable for BlogPage<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        let article = pages::Article {
            raw_html: &self.markdown.content.html,
        };
        let tags = Tags {
            tags: &self.markdown.tags,
        };
        let commits = Commits {
            commits: self.commits,
            base_url: self.commit_base_url,
        };

        rsx! {
            <div class="blog-page">
                <div class="blog-info">
                    (tags)
                    (commits)
                </div>

                (article)
            </div>
        }
        .render_to(buffer);
    }
}

impl Renderable for Tags<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        let to_url = |x: &str| format!("/blog/tags/{}", x);

        rsx! {
            <div class="blog-tags">
                <ul>
                    @for tag in self.tags {
                        <li>
                            <a href=(to_url(tag))>
                                (tag)
                            </a>
                        </li>
                    }
                </ul>
            </div>
        }
        .render_to(buffer);
    }
}
