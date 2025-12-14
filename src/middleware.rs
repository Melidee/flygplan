use std::{rc::Rc};

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
                "{} {} HTTP/1.1\n{}",
                c.request.method, c.request.resource, c.response.status
            );
            Ok(c)
        })
    }
}
