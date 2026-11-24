use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::rust_2015,
    sync::{Arc, Mutex},
};

use crate::{
    Context, Error, Result,
    http::{Method, Params, Request, Status},
    middleware::{self, Middleware},
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
    fn new() -> Self {
        Self {
            routes: vec![],
            status_handlers: vec![],
            middlewares: vec![],
        }
    }

    fn route<F: Fn(Context) -> Context + 'static>(
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

    fn use_middleware<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middlewares.push(Arc::new(Mutex::new(middleware)));
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
            self.handle_request(&mut stream, request);
        }
        Ok(())
    }

    fn handle_request(&self, stream: &mut TcpStream, mut request: Request) {
        let route_pair = self
            .routes
            .iter()
            .find_map(|route| Some((route, route.parse(&request)?)));
        if let Some((route, url_params)) = route_pair {
            match &route.target {
                RouteTarget::Handler(handler) => {
                    let context = Context::new(request, url_params);
                    let result = handler(context);
                    self.write_handled(stream, result.unwrap()).unwrap();
                }
                RouteTarget::Group(group) => {
                    group.handle_request(stream, request);
                }
            }
        } else {
            let handler = self.get_status_handler(Status::NotFound404);
            let context = Context::new(request, Params::new());
            let result = handler(context);
            self.write_handled(stream, result.unwrap()).unwrap();
            return;
        }
    }

    fn write_handled(&self, stream: &mut TcpStream, context: Context) -> Result<()> {
        stream
            .write_all(context.response.to_string().as_bytes())
            .map_err(Error::ConnectionError)
    }

    fn get_status_handler(&self, status: Status) -> Handler {
        self.status_handlers
            .iter()
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
    fn parse(&self, request: &Request) -> Option<PatternMatches> {
        todo!()
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
}

pub struct PatternMatches<'a> {
    matches: Vec<(PatternSegment, &'a str)>,
}

impl<'a> PatternMatches<'a> {
    fn params(&'a self) -> Params<'a> {
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

pub enum PatternSegment {
    Static(String),
    Capture(String),
    Wildcard,
    DoubleWildcard,
    Never,
}
