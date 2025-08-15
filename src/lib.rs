pub mod context;
pub mod error;
pub mod http;
pub use crate::context::{Context, Handler};
pub use crate::error::{Error, Result};

use crate::http::{Method, Request, Status};
use std::sync::Arc;
use std::vec;
use std::{
    io::Read,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

pub struct Flygplan<'a> {
    routes: Vec<Route<'a>>,
    status_handlers: Vec<(Status, Handler)>,
}

impl<'a> Flygplan<'a> {
    pub fn new() -> Self {
        Self {
            routes: vec![],
            status_handlers: vec![],
        }
    }

    pub fn get<F: Fn(Context) + 'static>(&mut self, pattern: &'a str, handler: F) {
        self.routes
            .push(Route::new(Method::Get, pattern, Arc::new(handler)));
    }

    pub fn post<F: Fn(Context) + 'static>(&mut self, pattern: &'a str, handler: F) {
        self.routes
            .push(Route::new(Method::Post, pattern, Arc::new(handler)));
    }

    pub fn status_handler<F: Fn(Context) + 'static>(&mut self, status: Status, handler: F) {
        self.status_handlers.push((status, Arc::new(handler)));
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
            let request = Request::parse(&buf).unwrap();
            Self::handle_request(self.routes.clone(), &self.status_handlers, stream, request);
        }
        Ok(())
    }

    fn handle_request(
        routes: Vec<Route>,
        status_handlers: &Vec<(Status, Handler)>,
        stream: TcpStream,
        request: Request,
    ) {
        let ctx = Context::new(request, status_handlers, stream);
        for route in routes {
            if route.matches(&ctx.request) {
                (route.handler)(ctx);
                return;
            }
        }
        ctx.status(Status::NotFound404).unwrap();
    }
}

#[derive(Clone)]
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
        return self.method == request.method && self.pattern == request.resource.path;
    }
}
