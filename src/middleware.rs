use std::sync::Arc;

use crate::{Context, Handler, error::Result};

pub type Middleware = Arc<dyn Fn(Handler) -> Handler>;

pub fn logger(handler: Handler) -> Handler {
    Arc::new(move |mut c: Context| -> Result<Context> {
        c = handler(c)?;
        println!("{} {} HTTP/1.1 {}", c.request.method, c.request.resource, c.response.status);
        Ok(c)
    })
}