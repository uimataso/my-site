use hypertext::prelude::*;

use crate::to_datetime;

#[derive(Clone)]
pub struct Commits<'a> {
    pub commits: &'a [git2::Commit<'a>],
    pub base_url: &'a str,
}

impl Renderable for Commits<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <div class="commits">
                <span>"commits:"</span>

                <ul>
                    @for commit in self.commits {
                        <li>
                            <a href=(format!("{}/{}", self.base_url, commit.id()))>
                                (commit.id().to_string()[..7])
                            </a>
                            <span>
                                ": "
                                (format_datetime(to_datetime(commit.time())))
                                " - "
                                (commit.summary())
                            </span>
                        </li>
                    }
                </ul>
            </div>
        }
        .render_to(buffer);
    }
}

fn format_datetime(datetime: chrono::DateTime<chrono::FixedOffset>) -> String {
    // datetime.format("%Y-%m-%d %H:%M%:::z").to_string()
    datetime.format("%Y-%m-%d").to_string()
}
