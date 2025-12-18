use hypertext::{Raw, prelude::*};

use crate::{config, generator, pages};

pub struct BlogPage<'a> {
    pub publish_time: chrono::NaiveDate,
    pub last_update_time: chrono::NaiveDate,
    pub last_commit: Option<&'a generator::BlogCommit>,
    pub markdown: &'a crate::markdown::Markdown,
}

impl Renderable for BlogPage<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        let article = pages::Article {
            raw_html: &self.markdown.html,
        };

        rsx! {
            <div class="blog">
                <div class="blog-info">
                    <p> "publish: " (self.publish_time.to_string()) </p>
                    <p> "update: " (self.last_update_time.to_string()) </p>
                    @if let Some(commit) = self.last_commit {
                        <p>
                            <span>"commit: "</span>
                            <a href=(format!("{}/{}", commit.base_url, commit.hash))>
                                (commit.hash[..7]) " - " (commit.summary)
                            </a>
                        </p>
                    }
                    <p>
                        <span>"tags:"</span>
                        @for tag in &self.markdown.meta.tags {
                            <span>" "</span>
                            <a href=(config::tag_to_link(tag))>
                                "#"(tag)
                            </a>
                        }
                    </p>
                </div>

                (article)
            </div>
        }
        .render_to(buffer);
    }
}
