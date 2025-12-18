use hypertext::prelude::*;

#[derive(Clone)]
pub struct Base<'a, T: Renderable> {
    pub head: Head<'a>,
    pub body: T,
}

#[derive(Clone)]
pub struct Head<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub author: &'a str,
}

impl<T: Renderable> Renderable for Base<'_, T> {
    fn render_to(&self, buffer: &mut hypertext::Buffer<hypertext::context::Node>) {
        rsx! {
            <!DOCTYPE html>
            <html>
                (self.head)
                (self.body)
            </html>
        }
        .render_to(buffer);
    }
}

impl Renderable for Head<'_> {
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

                <link rel="icon" href="/favicon.svg" type="image/svg+xml" >
                <link rel="stylesheet" href="/static/styles.css">
            </head>
        }
        .render_to(buffer);
    }
}
