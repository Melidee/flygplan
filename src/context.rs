use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::http::{Params, Request, Response, Status};

pub type Handler = Arc<dyn Fn(Context)>;
pub type Middleware = Arc<dyn Fn(Context) -> Context>;

pub struct Context<'a> {
    pub request: Request<'a>,
    pub response: Response<'a>,
    url_params: Params<'a>,
    status_handlers: &'a Vec<(Status, Handler)>,
    stream: TcpStream,
}

impl<'a> Context<'a> {
    pub fn new(
        request: Request<'a>,
        url_params: Params<'a>,
        status_handlers: &'a Vec<(Status, Handler)>,
        stream: TcpStream,
    ) -> Self {
        Self {
            request,
            response: Response::default(),
            url_params,
            status_handlers,
            stream,
        }
    }

    pub fn url_param(&self, key: &str) -> Option<String> {
        self.url_params.get(key)
    }

    pub fn query_params(&self) -> &Params {
        &self.request.resource.query_params
    }

    pub fn string(mut self, body: &str) -> Result<()> {
        self.response.body = body.to_string();
        self.write()
    }

    pub fn file(mut self, path: &str) -> Result<()> {
        let mut file = File::open(path).map_err(|e| Error::ConnectionError(e))?;
        let mut body = vec![];
        file.read_to_end(&mut body).expect("failed to open file");
        self.response.body = String::from_utf8(body).expect("response file is not UTF-8 encoded");
        self.write()
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
            (handler)(self);
            Ok(())
        } else {
            self.string(&status.to_string())
        }
    }

    pub fn write(mut self) -> Result<()> {
        let response = self.response.to_string();
        self.stream
            .write(response.as_bytes())
            .map_err(|e| Error::ConnectionError(e))?;
        Ok(())
    }
}
