use hypertext::{Raw, prelude::*};

use crate::pages;

#[derive(Clone)]
pub struct Article<'a> {
    pub raw_html: &'a str,
}

impl Renderable for Article<'_> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <article>
                (Raw::dangerously_create(&self.raw_html))
            </article>
        }
        .render_to(buffer);
    }
}
