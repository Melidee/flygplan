use std::fs::File;
use std::io::Read;
use std::rc::Rc;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::http::{Params, Request, Response, Status};

pub type Handler = Rc<dyn Fn(Context) -> Result<Context>>;

pub struct Context<'a> {
    pub request: Request<'a>,
    pub response: Response<'a>,
    url_params: Params<'a>,
}

impl<'a> Context<'a> {
    pub fn new(
        request: Request<'a>,
        url_params: Params<'a>,
    ) -> Self {
        Self {
            request,
            response: Response::default(),
            url_params,
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
        Ok(self)
    }

    pub fn file(mut self, path: &str) -> Result<Self> {
        let mut file = File::open(path).map_err(Error::ConnectionError)?;
        let mut body = vec![];
        file.read_to_end(&mut body).expect("failed to open file");
        self.response.body = String::from_utf8(body).expect("response file is not UTF-8 encoded");
        Ok(self)
    }

    pub fn json<S: Serialize>(mut self, value: S) -> Result<Self> {
        self.response.body = serde_json::to_string(&value).map_err(|_| Error::SerializationError)?;
        Ok(self)
    }

    pub fn redirect(mut self, route: &'a str) -> Result<Self> {
        self.response.status = Status::SeeOther303;
        self.response.headers.set("Location", route);
        Ok(self)
    }

    /*
     * Respond with a generic HTTP response status handler
     */
    pub fn status(self, status: Status) -> Result<Self> {
        Err(Error::Status(status))
    }
}
