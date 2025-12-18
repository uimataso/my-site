use hypertext::prelude::*;

use crate::config;

#[derive(Clone)]
pub struct Body<'a, T: Renderable> {
    pub header: Header<'a>,
    pub footer: Footer<'a>,
    pub main: T,
}

#[derive(Clone)]
pub struct Header<'a> {
    pub home_name: &'a String,
    pub links: &'a [config::Link],
    pub active_url: Option<&'a str>,
}

#[derive(Clone)]
pub struct Footer<'a> {
    pub links: &'a [config::Link],
    pub cc_text: &'a str,
}

impl<T: Renderable> Renderable for Body<'_, T> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <body>
                (self.header)
                <main>
                    (self.main)
                </main>
                (self.footer)
            </body>
        }
        .render_to(buffer);
    }
}

impl Renderable for Header<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        let is_active = |link_url: &str| {
            let link_url = link_url.trim_matches('/');
            self.active_url.is_some_and(|x| x.starts_with(link_url))
        };

        rsx! {
            <header>
                <div class="header-home">
                    <a href="/">(self.home_name)</a>
                </div>

                <div class="header-links">
                    @for link in self.links {
                        <a href=(link.url) class=@if is_active(&link.url) { "active" }>
                            (link.title)
                        </a>
                    }
                </div>
            </header>
        }
        .render_to(buffer);
    }
}

impl Renderable for Footer<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <footer>
                <div class="footer-links">
                    <ul>
                        @for link in self.links {
                            <li>
                                <a href=(link.url)>(link.title)</a>
                            </li>
                        }
                    </ul>
                </div>

                <div class="footer-cc">
                    <p>(self.cc_text)</p>
                </div>
            </footer>
        }
        .render_to(buffer);
    }
}
