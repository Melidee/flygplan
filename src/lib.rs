pub mod context;
pub mod error;
pub mod http;
pub use crate::context::{Context, Handler};
pub use crate::error::{Error, Result};

use crate::http::{Method, Request};
use std::{
    io::Read,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

pub struct Flygplan<'a> {
    routes: Vec<Route<'a>>,
    not_found: Handler,
}

impl<'a> Flygplan<'a> {
    pub fn new() -> Self {
        Self {
            routes: vec![],
            not_found: |c| c.string("404 not found"),
        }
    }

    pub fn get(&mut self, pattern: &'a str, handler: Handler) {
        self.routes.push(Route::new(Method::Get, pattern, handler));
    }

    pub fn post(&mut self, pattern: &'a str, handler: Handler) {
        self.routes.push(Route::new(Method::Post, pattern, handler));
    }

    pub fn not_found(&mut self, handler: Handler) {
        self.not_found = handler;
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
            Self::handle_request(self.routes.clone(), self.not_found, stream, request);
        }
        Ok(())
    }

    fn handle_request(routes: Vec<Route>, not_found: Handler, stream: TcpStream, request: Request) {
        let ctx = Context::new(request, stream);
        for route in routes {
            if route.matches(&ctx.request) {
                (route.handler)(ctx);
                return;
            }
        }
        not_found(ctx)
    }
}

#[derive(Debug, Clone)]
struct Route<'a> {
    method: Method,
    pattern: &'a str,
    handler: Handler,
}

impl<'a> Route<'a> {
    fn new(method: Method, pattern: &'a str, handler: Handler) -> Self {
        Self {
            method,
            pattern,
            handler,
        }
    }

    fn matches(&self, request: &Request) -> bool {
        return self.method == request.method && self.pattern == request.resource;
    }
}
