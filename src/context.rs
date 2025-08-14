use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::http::{Request, Response, Status};

pub type Handler = Arc<dyn Fn(Context)>;

pub struct Context<'a> {
    pub request: Request<'a>,
    pub response: Response<'a>,
    status_handlers: &'a Vec<(Status, Handler)>,
    stream: TcpStream,
}

impl<'a> Context<'a> {
    pub fn new(
        request: Request<'a>,
        status_handlers: &'a Vec<(Status, Handler)>,
        stream: TcpStream,
    ) -> Self {
        Self {
            request,
            response: Response::default(),
            status_handlers,
            stream,
        }
    }

    pub fn string(mut self, body: &str) {
        self.response.body = body.to_string();
        self.write().unwrap();
    }

    /*
     * Respond with a generic HTTP response status handler
     */
    pub fn status(self, status: Status) -> Result<()> {
        if let Some((_, handler)) = self
            .status_handlers
            .iter()
            .filter(|pair| pair.0 == status)
            .next()
        {
            (handler)(self)
        } else {
            self.string(&status.to_string())
        }
        Ok(())
    }

    pub fn write(mut self) -> Result<()> {
        let response = self.response.to_string();
        self.stream
            .write(response.as_bytes())
            .map_err(|e| Error::ConnectionError(e))?;
        Ok(())
    }
}
