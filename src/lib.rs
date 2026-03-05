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

// /static/**/*/:capture/

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Headers, Method, Url};

    fn empty_handler() -> Handler {
        return Rc::new(|c| Ok(c));
    }

    fn test_request<'a>(method: Method, path: &'a str) -> Request<'a> {
        return Request {
            method: method,
            resource: Url::parse(path).unwrap(),
            headers: Headers::new(),
            body: &[],
        };
    }

    #[test]
    fn router_matches_single_slash() {
        let route = Route::new(Method::Get, "/", empty_handler());
        assert!(route.matches(&test_request(Method::Get, "/")).is_some());
    }
    
    #[test]
    fn router_matches_simple_path() {
        let route = Route::new(Method::Get, "/hello/world", empty_handler());
        assert!(route.matches(&test_request(Method::Get, "/hello/world")).is_some());
    }
    
    #[test]
    fn router_captures_dynamic_path() {
        let route = Route::new(Method::Get, "/:id", empty_handler());
        let request = test_request(Method::Get, "/1234");
        let matched = route.matches(&request);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().get("id"), Some("1234".to_string()));
    }
    
    #[test]
    fn router_captures_multiple_dynamic_paths() {
        let route = Route::new(Method::Get, "/:id/hello/:name", empty_handler());
        let request = test_request(Method::Get, "/1234/hello/amelia");
        let matched = route.matches(&request);
        assert!(matched.is_some());
        assert_eq!(matched.clone().unwrap().get("id"), Some("1234".to_string()));
        assert_eq!(matched.clone().unwrap().get("name"), Some("amelia".to_string()));
    }
    
    #[test]
    fn everything_matches_wildcard() {
        let route = Route::new(Method::Get, "/*", empty_handler());
        assert!(route.matches(&test_request(Method::Get, "/")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/hi")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/1234")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/trailingslash/")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/params/?key=val&key2=val2")).is_some());
    }
    
    #[test]
    fn everything_matches_double_wildcard() {
        let route = Route::new(Method::Get, "/**", empty_handler());
        assert!(route.matches(&test_request(Method::Get, "/")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/hello")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/hello/world")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/a/b/c/d/e/f/g/")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/hello?key=val&key2=val2")).is_some());
    }
    
    #[test]
    fn wildcard_matches_with_other_patterns() {
        let route = Route::new(Method::Get, "/hello/*/world", empty_handler());
        assert!(route.matches(&test_request(Method::Get, "/hello/name/world")).is_some());
        assert!(route.matches(&test_request(Method::Get, "/hello/12345/world")).is_some());
        assert!(!route.matches(&test_request(Method::Get, "/hello/world")).is_some());
    }
}
