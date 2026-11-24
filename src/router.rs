use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{
    Context, Error, Result,
    http::{Method, Params, Request, Status},
    middleware::Middleware,
};

#[macro_export]
macro_rules! router_inner {
    ([] $r:ident) => {};
    ([
        #![$middleware:expr]
        $($rest:tt)*
    ] $r:ident) => {
        $r.use_middleware($middleware);
        $crate::router_inner!([$($rest)*] $r)
    };

    ([
        status $code:tt => $func:expr,
        $($rest:tt)*
    ] $r:ident) => {
        macro_rules! status {
            (400) => { $crate::http::Status::BadRequest400 };
            (404) => { $crate::http::Status::NotFound404 };
            (500) => { $crate::http::Status::InternalServerError500 };
            ($code:tt) => { compile_error!("Unknown HTTP status code.") };
        }
        $r.status_handler(status!($code), $func);
        $crate::router_inner!([$($rest)*] $r)
    };

    ([
        $(#[$middleware:expr])*
        $method_end:ident $route:expr => $func:expr,
        $($rest:tt)*
    ] $r:ident) => {
        macro_rules! method {
            (GET) => { $crate::http::Method::Get };
            (POST) => { $crate::http::Method::Post };
            ($ident:ident) => { compile_error!("Unknown HTTP method.") };
        }
        $r.route(method!($method_end), $route, $func);
        $crate::router_inner!([$($rest)*] $r)
    };
}

#[macro_export]
macro_rules! router {
    ($($input:tt)*) => {
        {
            let mut router = $crate::Router::new();
            $crate::router_inner!([$($input)*] router);
            router
        }
    };
}

pub struct Router {
    routes: Vec<Route>,
    status_handlers: Vec<(Status, Handler)>,
    middlewares: Vec<Arc<Mutex<dyn Middleware>>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: vec![],
            status_handlers: vec![],
            middlewares: vec![],
        }
    }

    pub fn route<F: Fn(Context) -> Result<Context> + 'static>(
        &mut self,
        method: Method,
        pattern: &str,
        handler: F,
    ) -> &mut Route {
        let route = Route::new(method, pattern, RouteTarget::Handler(Arc::new(handler)));
        self.routes.push(route);
        self.routes
            .last_mut()
            .expect("the route just pushed is the last element")
    }

    pub fn status_handler<F: Fn(Context) -> Result<Context> + 'static>(
        &mut self,
        status: Status,
        handler: F,
    ) {
        self.status_handlers.push((status, Arc::new(handler)));
    }

    pub fn use_middleware<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middlewares.push(Arc::new(Mutex::new(middleware)));
    }

    pub fn listen_and_serve<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr).map_err(Error::ConnectionError)?;
        self.serve(listener)
    }

    pub fn serve(self, listener: TcpListener) -> Result<()> {
        for c in listener.incoming() {
            let mut stream = c.map_err(Error::ConnectionError)?;
            let mut buf = [0u8; 2048];
            stream.read(&mut buf).map_err(Error::ConnectionError)?;
            let request = Request::parse(&buf).unwrap();
            self.handle_request(&mut stream, request);
        }
        Ok(())
    }

    fn handle_request(&self, stream: &mut TcpStream, request: Request) {
        let (handler, url_params) = self
            .find_handler(&request)
            .unwrap_or((self.get_status_handler(Status::NotFound404), Params::new()));
        let context = Context::new(request.clone(), url_params);
        let result = handler(context);
        self.write_handled(stream, result.unwrap()).expect("todo");
    }

    fn find_handler<'a>(&'a self, request: &'a Request) -> Option<(Handler, Params<'a>)> {
        self.routes
            .iter()
            .find_map(|route| Some((route, route.parse(&request)?)))
            .map(|(route, matches)| match &route.target {
                RouteTarget::Handler(handler) => {
                    let params = matches.params();
                    let dressed = Self::dressed_handler(handler.clone(), self.middlewares.clone());
                    Some((dressed, params))
                }
                RouteTarget::Group(group) => {
                    let (handler, params) = group.find_handler(request)?;
                    let params = params
                        .pairs()
                        .iter()
                        .fold(matches.params(), |params, pair| params.with(*pair));
                    let dressed = Self::dressed_handler(handler, self.middlewares.clone());
                    Some((dressed, params))
                }
            })
            .flatten()
    }

    fn dressed_handler(handler: Handler, middlewares: Vec<Arc<Mutex<dyn Middleware>>>) -> Handler {
        middlewares.iter().fold(handler, |handler, middleware| {
            middleware.lock().unwrap().apply(handler)
        })
    }

    fn write_handled(&self, stream: &mut TcpStream, context: Context) -> Result<()> {
        stream
            .write_all(context.response.to_string().as_bytes())
            .map_err(Error::ConnectionError)
    }

    fn get_status_handler(&self, status: Status) -> Handler {
        self.status_handlers
            .iter()
            .rev()
            .find(|(s, _handler)| *s == status)
            .map(|(_s, handler)| handler.clone())
            .unwrap_or(Arc::new(move |c: Context| {
                c.string(&status.clone().to_string())
            }))
    }
}

pub struct Route {
    method: Method,
    pattern: Pattern,
    target: RouteTarget,
}

impl<'a> Route {
    fn new(method: Method, pattern: &str, target: RouteTarget) -> Self {
        Self {
            method,
            pattern: Pattern::parse(pattern.to_owned()),
            target,
        }
    }
}

impl Route {
    fn parse<'a>(&'a self, request: &'a Request) -> Option<PatternMatches<'a>> {
        if self.method != request.method {
            return None;
        }
        return self.pattern.match_url(&request.resource.path);
    }
}

pub type Handler = Arc<dyn Fn(Context) -> Result<Context>>;

pub enum RouteTarget {
    Handler(Handler),
    Group(Router),
}

impl From<Handler> for RouteTarget {
    fn from(handler: Handler) -> Self {
        Self::Handler(handler)
    }
}

impl From<Router> for RouteTarget {
    fn from(router: Router) -> Self {
        Self::Group(router)
    }
}

pub struct Pattern {
    segments: Vec<PatternSegment>,
}

impl Pattern {
    fn parse(pattern: String) -> Self {
        Self {
            segments: pattern
                .split('/')
                .map(|seg| match seg {
                    "*" => PatternSegment::Wildcard,
                    "**" => PatternSegment::DoubleWildcard,
                    seg if seg.starts_with(":") => PatternSegment::Capture(seg[1..].to_owned()),
                    seg => PatternSegment::Static(seg.to_owned()),
                })
                .collect(),
        }
    }

    fn match_url<'a>(&'a self, url: &'a str) -> Option<PatternMatches<'a>> {
        let mut matches = PatternMatches::new();
        let mut splits = url.split('/').peekable();
        let mut segments = self.segments.iter().peekable();
        let mut current_segment = segments.next();
        while let Some(split) = splits.next() {
            match current_segment? {
                PatternSegment::Static(seg) => {
                    if seg != split {
                        return None;
                    }
                    matches.matches.push((current_segment?, split));
                }
                PatternSegment::Capture(_) | PatternSegment::Wildcard => {
                    matches.matches.push((current_segment?, split));
                }
                PatternSegment::DoubleWildcard => {
                    if let Some(PatternSegment::Static(next_seg)) = segments.peek() {
                        if next_seg == split {
                            current_segment = segments.next();
                            continue;
                        }
                        matches.matches.push((current_segment?, split));
                        continue;
                    } else {
                        matches.matches.push((current_segment?, split));
                    }
                }
            };
            current_segment = segments.next();
        }
        Some(matches)
    }
}

pub enum PatternSegment {
    Static(String),
    Capture(String),
    Wildcard,
    DoubleWildcard,
}

pub struct PatternMatches<'a> {
    matches: Vec<(&'a PatternSegment, &'a str)>,
}

impl<'a> PatternMatches<'a> {
    fn new() -> Self {
        Self { matches: vec![] }
    }

    fn params(self) -> Params<'a> {
        self.matches
            .iter()
            .filter_map(|(seg, matched)| match seg {
                PatternSegment::Capture(key) => Some((&key[1..], *matched)),
                _ => None,
            })
            .fold(Params::new(), |mut params, pair| {
                params.push(pair);
                params
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Headers, Method, Url};

    fn empty_handler() -> RouteTarget {
        return RouteTarget::Handler(Arc::new(|c| Ok(c)));
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
        assert!(route.parse(&test_request(Method::Get, "/")).is_some());
    }

    #[test]
    fn router_matches_simple_path() {
        let route = Route::new(Method::Get, "/hello/world", empty_handler());
        assert!(
            route
                .parse(&test_request(Method::Get, "/hello/world"))
                .is_some()
        );
    }

    #[test]
    fn everything_matches_wildcard() {
        let route = Route::new(Method::Get, "/*", empty_handler());
        assert!(route.parse(&test_request(Method::Get, "/")).is_some());
        assert!(route.parse(&test_request(Method::Get, "/hi")).is_some());
        assert!(route.parse(&test_request(Method::Get, "/1234")).is_some());
        assert!(
            route
                .parse(&test_request(Method::Get, "/trailingslash/"))
                .is_some()
        );
        assert!(
            route
                .parse(&test_request(Method::Get, "/params/?key=val&key2=val2"))
                .is_some()
        );
    }

    #[test]
    fn everything_matches_double_wildcard() {
        let route = Route::new(Method::Get, "/**", empty_handler());
        assert!(route.parse(&test_request(Method::Get, "/")).is_some());
        assert!(route.parse(&test_request(Method::Get, "/hello")).is_some());
        assert!(
            route
                .parse(&test_request(Method::Get, "/hello/world"))
                .is_some()
        );
        assert!(
            route
                .parse(&test_request(Method::Get, "/a/b/c/d/e/f/g/"))
                .is_some()
        );
        assert!(
            route
                .parse(&test_request(Method::Get, "/hello?key=val&key2=val2"))
                .is_some()
        );
    }

    #[test]
    fn wildcard_matches_with_other_patterns() {
        let route = Route::new(Method::Get, "/hello/*/world", empty_handler());
        assert!(
            route
                .parse(&test_request(Method::Get, "/hello/name/world"))
                .is_some()
        );
        assert!(
            route
                .parse(&test_request(Method::Get, "/hello/12345/world"))
                .is_some()
        );
        assert!(
            !route
                .parse(&test_request(Method::Get, "/hello/world"))
                .is_some()
        );
    }
}
