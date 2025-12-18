use std::path::Path;

use hypertext::{Raw, prelude::*};

use crate::{config, pages};

pub struct BlogHome<'a> {
    pub blog_entries: &'a [BlogEntry<'a>],
}

pub struct BlogTagHome<'a> {
    pub tag_name: &'a str,
    pub blog_entries: &'a [BlogEntry<'a>],
}

#[derive(Clone, Copy)]
pub struct BlogEntry<'a> {
    pub publish_time: chrono::NaiveDate,
    pub title: &'a str,
    pub rel_path: &'a Path,
    pub tags: &'a [String],
}

impl Renderable for BlogHome<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <div class="blog-home">
                <div class="blog-list">
                    <ul>
                        @for entry in self.blog_entries {
                            <li>
                                (entry)
                            </li>
                        }
                    </ul>
                </div>
            </div>
        }
        .render_to(buffer);
    }
}

impl Renderable for BlogTagHome<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <div class="blog-tag-home">
                <h3>"#"(self.tag_name)</h3>

                <div class="blog-list">
                    <ul>
                        @for entry in self.blog_entries {
                            <li>
                                (entry)
                            </li>
                        }
                    </ul>
                </div>
            </div>
        }
        .render_to(buffer);
    }
}

impl Renderable for BlogEntry<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        let url = Path::new("/").join(self.rel_path);
        let url = url.to_str().unwrap_or("/");

        rsx! {
            <div class="blog-entry">
                <div class="blog-date">
                    <p>(self.publish_time.to_string())</p>
                </div>

                <div class="blog-link">
                    <div class="blog-title">
                        <h3>
                            <a href=(url)>(self.title)</a>
                        </h3>
                    </div>

                    <div class="blog-tags">
                        <p>
                            @for tag in self.tags {
                                <span>" "</span>
                                <a href=(config::tag_to_link(tag))>
                                    "#"(tag)
                                </a>
                            }
                        </p>
                    </div>
                </div>
            </div>
        }
        .render_to(buffer);
    }
}
