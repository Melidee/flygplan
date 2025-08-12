pub mod error;
pub mod http;
pub use crate::error::{Error, Result};

use crate::http::{Request, Response};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

pub struct Flygplan {
    routes: Vec<(String, Handler)>,
}

impl Flygplan {
    pub fn new() -> Self {
        Self { routes: vec![] }
    }

    pub fn get(&mut self, route: &str, handler: Handler) {
        self.routes.push((route.to_string(), handler));
    }

    pub fn listen_and_serve<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr).map_err(|e| Error::ConnectionError(e))?;
        self.serve(listener)
    }

    fn serve(self, listener: TcpListener) -> Result<()> {
        for c in listener.incoming() {
            let mut stream = c.map_err(|e| Error::ConnectionError(e))?;
            let mut buf = [0u8; 2048];
            stream
                .read(&mut buf)
                .map_err(|e| Error::ConnectionError(e))?;
            let raw_request = String::from_utf8(buf.to_vec()).unwrap();
            let request = Request::parse(&raw_request).unwrap();
            println!("`{}`", request.resource);
            for (route, handler) in self.routes.clone() {
                if &request.resource == &route {
                    let ctx = Context {
                        stream,
                        request,
                        response: Response::default(),
                    };
                    handler(ctx);
                    break;
                }
            }
        }
        Ok(())
    }
}

type Handler = fn(Context);

pub struct Context {
    pub request: Request,
    pub response: Response,
    stream: TcpStream,
}

impl Context {
    pub fn new(request: Request, stream: TcpStream) -> Self {
        Self {
            request,
            response: Response::default(),
            stream,
        }
    }

    pub fn string(mut self, body: &str) {
        self.response.headers.set_content_type("text".to_string());
        self.response.body = body.to_string();
        self.finalize().unwrap();
    }

    pub fn finalize(mut self) -> Result<()> {
        let response = self.response.to_string();
        self.stream
            .write(response.as_bytes())
            .map_err(|e| Error::ConnectionError(e))?;
        Ok(())
    }
}
