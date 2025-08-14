use std::io::Write;
use std::net::TcpStream;

use crate::error::{Error, Result};
use crate::http::{Request, Response};

pub type Handler = fn(Context);

pub struct Context<'a> {
    pub request: Request<'a>,
    pub response: Response<'a>,
    stream: TcpStream,
}

impl<'a> Context<'a> {
    pub fn new(request: Request<'a>, stream: TcpStream) -> Self {
        Self {
            request,
            response: Response::default(),
            stream,
        }
    }

    pub fn string(mut self, body: &str) {
        self.response.body = body.to_string();
        self.write().unwrap();
    }

    pub fn write(mut self) -> Result<()> {
        let response = self.response.to_string();
        self.stream
            .write(response.as_bytes())
            .map_err(|e| Error::ConnectionError(e))?;
        Ok(())
    }
}
