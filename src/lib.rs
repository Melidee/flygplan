pub mod context;
pub mod error;
pub mod http;
pub mod middleware;
pub use crate::context::{Context, Handler};
pub use crate::error::{Error, Result};
use crate::middleware::Middleware;

use crate::http::{Method, Params, Request, Status};
use std::sync::Arc;
use std::vec;
use std::{
    io::Read,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

pub struct Flygplan<'a> {
    routes: Vec<Route<'a>>,
    status_handlers: Vec<(Status, Handler)>,
    middlewares: Vec<Middleware>,
}

impl<'a> Flygplan<'a> {
    pub fn new() -> Self {
        Self {
            routes: vec![],
            status_handlers: vec![],
            middlewares: vec![],
        }
    }

    pub fn get<F: Fn(Context) -> Result<Context> + 'static>(
        &mut self,
        pattern: &'a str,
        handler: F,
    ) -> &mut Route<'a> {
        let route = Route::new(Method::Get, pattern, Arc::new(handler));
        self.routes
            .push(route);
        return self.routes.last_mut().unwrap();
    }

    pub fn post<F: Fn(Context) -> Result<Context> + 'static>(
        &mut self,
        pattern: &'a str,
        handler: F,
    ) -> &mut Route<'a> {
        let route = Route::new(Method::Post, pattern, Arc::new(handler));
        self.routes
            .push(route);
        return self.routes.last_mut().unwrap();
    }

    pub fn status_handler<F: Fn(Context) -> Result<Context> + 'static>(
        &mut self,
        status: Status,
        handler: F,
    ) {
        self.status_handlers.push((status, Arc::new(handler)));
    }

    pub fn use_middleware<F: Fn(Handler) -> Handler + 'static>(&mut self, middleware: F) {
        self.middlewares.push(Arc::new(middleware));
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
            Self::handle_request(&self, stream, request);
        }
        Ok(())
    }

    fn handle_request(&self, stream: TcpStream, request: Request) {
        for route in self.routes.iter() {
            if let Some(url_params) = route.matches(&request) {
                let ctx =
                    Context::new(request.clone(), url_params, &self.status_handlers, stream);
                let handler = self
                    .middlewares
                    .iter()
                    .fold(route.handler.clone(), |route, middleware| middleware(route));
                let _err = handler(ctx).unwrap();
                return;
            }
        }
        Context::new(request, Params::default(), &self.status_handlers, stream)
            .status(Status::NotFound404)
            .unwrap();
    }
}

#[derive(Clone)]
pub struct Route<'a> {
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

    fn matches(&self, request: &'a Request) -> Option<Params<'a>> {
        if request.method != self.method {
            return None;
        }
        let mut params: Params<'_> = Params::new();
        let pattern_segments = self.pattern.split("/").collect::<Vec<_>>();
        let request_segments = request.resource.path.split("/").collect::<Vec<_>>();
        if pattern_segments.len() != request_segments.len() {
            return None;
        }
        for (pattern_seg, request_seg) in pattern_segments.iter().zip(request_segments.iter()) {
            let segment_is_dynamic = pattern_seg.chars().next() == Some('{')
                && pattern_seg.chars().next_back() == Some('}');
            if segment_is_dynamic {
                params.push((
                    &pattern_seg[1..pattern_seg.len() - 1],
                    request_seg.to_owned(),
                ));
                continue;
            }
            if pattern_seg != request_seg {
                return None;
            }
        }
        Some(params)
    }
}