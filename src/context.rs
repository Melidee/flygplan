use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::rc::Rc;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::http::{Params, Request, Response, Status};

pub type Handler = Rc<dyn Fn(Context) -> Result<Context>>;

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

    pub fn path_param(&self, key: &str) -> Option<String> {
        self.url_params.get(key)
    }

    pub fn query_param(&self, key: &str) -> Option<String> {
        self.request.resource.query_params.get(key)
    }

    pub fn string(mut self, body: &str) -> Result<Self> {
        self.response.body = body.to_string();
        self.write()
    }

    pub fn file(mut self, path: &str) -> Result<Self> {
        let mut file = File::open(path).map_err(Error::ConnectionError)?;
        let mut body = vec![];
        file.read_to_end(&mut body).expect("failed to open file");
        self.response.body = String::from_utf8(body).expect("response file is not UTF-8 encoded");
        self.write()
    }

    pub fn json<S: Serialize>(self, value: S) -> Result<Self> {
        serde_json::to_writer(&self.stream, &value).map_err(|_| Error::SerializationError)?;
        Ok(self)
    }

    pub fn redirect(mut self, route: &'a str) -> Result<Self> {
        self.response.status = Status::SeeOther303;
        self.response.headers.set("Location", route);
        self.write()
    }

    /*
     * Respond with a generic HTTP response status handler
     */
    pub fn status(self, status: Status) -> Result<Self> {
        if let Some((_, handler)) = self
            .status_handlers
            .iter()
            .find(|(req_status, _)| *req_status == status)
        {
            (handler)(self)
        } else {
            self.string(&status.to_string())
        }
    }

    pub fn write(mut self) -> Result<Self> {
        let response = self.response.to_string();
        self.stream
            .write(response.as_bytes())
            .map_err(Error::ConnectionError)?;
        Ok(self)
    }
}
