pub mod context;
pub mod error;
pub mod http;
pub mod middleware;
pub use crate::context::{Context, Handler};
pub use crate::error::{Error, Result};
use crate::middleware::Middleware;

use crate::http::{Method, Params, Request, Status};
use std::cell::RefCell;
use std::rc::Rc;
use std::vec;
use std::{
    io::Read,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

#[derive(Default)]
pub struct Flygplan<'a> {
    routes: Vec<Route<'a>>,
    status_handlers: Vec<(Status, Handler)>,
    middlewares: Vec<RefCell<Box<dyn Middleware>>>,
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
        let route = Route::new(Method::Get, pattern, Rc::new(handler));
        self.routes.push(route);
        self.routes.last_mut().unwrap()
    }

    pub fn post<F: Fn(Context) -> Result<Context> + 'static>(
        &mut self,
        pattern: &'a str,
        handler: F,
    ) -> &mut Route<'a> {
        let route = Route::new(Method::Post, pattern, Rc::new(handler));
        self.routes.push(route);
        self.routes.last_mut().unwrap()
    }

    pub fn status_handler<F: Fn(Context) -> Result<Context> + 'static>(
        &mut self,
        status: Status,
        handler: F,
    ) {
        self.status_handlers.push((status, Rc::new(handler)));
    }

    pub fn use_middleware<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middlewares.push(RefCell::new(Box::new(middleware)));
    }

    pub fn listen_and_serve<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr).map_err(Error::ConnectionError)?;
        self.serve(listener)
    }

    fn serve(self, listener: TcpListener) -> Result<()> {
        for c in listener.incoming() {
            let mut stream = c.map_err(Error::ConnectionError)?;
            let mut buf = [0u8; 2048];
            stream.read(&mut buf).map_err(Error::ConnectionError)?;
            let request = Request::parse(&buf).unwrap();
            Self::handle_request(&self, stream, request);
        }
        Ok(())
    }

    fn handle_request(&self, stream: TcpStream, request: Request) {
        for route in self.routes.iter() {
            if let Some(url_params) = route.matches(&request) {
                let ctx = Context::new(request.clone(), url_params, &self.status_handlers, stream);
                let handler = self
                    .middlewares
                    .iter()
                    .fold(route.handler.clone(), |route, middleware| {
                        middleware.borrow_mut().apply(route)
                    });
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
    pattern: Vec<PatternSegment<'a>>,
    handler: Handler,
}

impl<'a> Route<'a> {
    fn new(method: Method, pattern: &'a str, handler: Handler) -> Self {
        Self {
            method,
            pattern: PatternSegment::parse(pattern),
            handler,
        }
    }

    fn matches(&self, request: &'a Request) -> Option<Params<'a>> {
        if request.method != self.method {
            return None;
        }
        let mut splits = request.resource.path.split("/").peekable();
        let mut params = Params::new();
        let mut pattern = self.pattern.iter().peekable();
        while let Some(seg) = pattern.next() {
            match seg {
                PatternSegment::Static(s) => {
                    if Some(*s) != splits.next() {
                        return None;
                    }
                }
                PatternSegment::Capture(s) => params.push((s, splits.next()?)),
                PatternSegment::Wildcard => {
                    splits.next();
                }
                PatternSegment::DoubleWildcard => {
                    while let Some(part) = splits.peek() {
                        if Some(&&PatternSegment::Static(part)) == pattern.peek() {
                            break;
                        }
                        splits.next();
                    }
                }
            }
        }
        Some(params)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PatternSegment<'a> {
    Static(&'a str),
    Capture(&'a str),
    Wildcard,
    DoubleWildcard,
}

impl<'a> PatternSegment<'a> {
    fn parse(pattern: &'a str) -> Vec<PatternSegment<'a>> {
        pattern
            .split("/")
            .map(|seg| match seg {
                "*" => PatternSegment::Wildcard,
                "**" => PatternSegment::DoubleWildcard,
                seg if seg.starts_with(":") => PatternSegment::Capture(&seg[1..]),
                seg => PatternSegment::Static(seg),
            })
            .collect()
    }
}
