use std::{rc::Rc, sync::Arc};

use crate::{Context, error::Result, router::Handler};

pub trait Middleware {
    fn apply(&mut self, handler: Handler) -> Handler;
}

pub struct Logger {}

impl Logger {
    pub fn new() -> Self {
        Logger {  }
    }
}

impl Middleware for Logger {
    fn apply(&mut self, handler: Handler) -> Handler {
        Arc::new(move |mut c: Context| -> Result<Context> {
            c = handler(c)?;
            println!(
                "{} {} HTTP/1.1\t{}",
                c.request.method, c.request.resource, c.response.status
            );
            Ok(c)
        })
    }
}

pub struct AddTrailingSlash {}

impl Middleware for AddTrailingSlash {
    fn apply(&mut self, handler: Handler) -> Handler {
        Arc::new(move |mut c: Context<'_>| -> Result<Context> {
            if !c.request.resource.path.ends_with('/') {
                c.request.resource.path.to_mut().push('/');
            }
            handler(c)
        })
    }
}

pub struct RemoveTrailingSlash {}

impl RemoveTrailingSlash {
    pub fn new() -> Self {
        RemoveTrailingSlash {  }
    }
}

impl Middleware for RemoveTrailingSlash {
    fn apply(&mut self, handler: Handler) -> Handler {
        Arc::new(move |mut c: Context| -> Result<Context> {
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
