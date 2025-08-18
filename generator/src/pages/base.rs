use chrono::Datelike as _;
use hypertext::prelude::*;

#[derive(Clone)]
pub struct Base<T: Renderable> {
    pub header: Header,
    pub nav: Nav,
    pub main_content: T,
    pub footer: Footer,
}

#[derive(Clone)]
pub struct Header {
    pub title: String,
    pub description: Option<String>,
    pub author: String,
}

#[derive(Clone)]
pub struct Footer {
    pub author: String,
    pub links: Vec<(String, String)>,
}

#[derive(Clone)]
pub struct Nav {
    pub home_name: String,
    pub list: Vec<(String, String)>,
    pub active: Option<String>,
}

impl<T: Renderable> Renderable for Base<T> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <!DOCTYPE html>
            <html>
                (self.header)

                <body>
                    (self.nav)
                    <main> (self.main_content) </main>
                    (self.footer)
                </body>
            </html>
        }
        .render_to(buffer);
    }
}

impl Renderable for Header {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">

                <title>(self.title)</title>

                @if let Some(description) = &self.description {
                    <meta name="description" content=(description)>
                }
                <meta name="author" content=(self.author)>

                <link rel="icon" href="/favicon.ico" type="image/x-icon">
                <link rel="stylesheet" href="/static/styles.css">
            </head>
        }
        .render_to(buffer);
    }
}

impl Renderable for Nav {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <nav>
                <div class="home">
                    <a href="/">(self.home_name)</a>
                </div>

                <div class="pages">
                    @for (url, name) in &self.list {
                        <a href=(url) class=@if self.active.as_ref() == Some(url) { "active" }>
                            (name)
                        </a>
                    }
                </div>
            </nav>
        }
        .render_to(buffer);
    }
}

impl Renderable for Footer {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        let year = chrono::Utc::now().year();

        rsx! {
            <footer>
                <div class="footer-links">
                    <ul>
                        @for (link, text) in &self.links {
                            <li>
                                <a href=(link)>(text)</a>
                            </li>
                        }
                    </ul>
                </div>

                <div class="footer-info">
                    <p>"© " (year) " " (self.author)</p>
                </div>
            </footer>
        }
        .render_to(buffer);
    }
}
