use std::rc::Rc;

use crate::{Context, Handler, error::Result};

pub trait Middleware {
    fn apply(&mut self, handler: Handler) -> Handler;
}

pub struct Logger {}

impl Middleware for Logger {
    fn apply(&mut self, handler: Handler) -> Handler {
        Rc::new(move |mut c: Context| -> Result<Context> {
            c = handler(c)?;
            println!(
                "{} {} HTTP/1.1\t{}",
                c.request.method, c.request.resource, c.response.status
            );
            Ok(c)
        })
    }
}

pub struct RemoveTrailingSlash {}

impl Middleware for RemoveTrailingSlash {
    fn apply(&mut self, handler: Handler) -> Handler {
        Rc::new(move |mut c: Context| -> Result<Context> {
            c.request.resource.path = c
                .request
                .resource
                .path
                .trim_end_matches('/')
                .to_owned()
                .into();
            handler(c)
        })
    }
}