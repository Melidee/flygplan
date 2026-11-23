use std::sync::{Arc, Mutex};

use crate::{Context, http::Method, middleware::Middleware};

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
    middlewares: Vec<Arc<Mutex<dyn Middleware>>>
}

pub struct Route {
    method: Method,
    pattern: Pattern,
    target: RouteTarget
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

pub type Handler = Arc<dyn Fn(Context) -> Context>;

pub enum RouteTarget {
    Handler(Handler),
    Group(Router)
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
    segments: Vec<PatternSegment>
}

impl Pattern {
    fn parse(pattern: String) -> Self {
        Self {
            segments: pattern.split('/').map(|seg| match seg {
                "*" => PatternSegment::Wildcard,
                "**" => PatternSegment::DoubleWildcard,
                seg if seg.starts_with(":") => PatternSegment::Capture(seg[1..].to_owned()),
                seg => PatternSegment::Static(seg.to_owned()),
            }).collect(),
        }
    }
}

pub enum PatternSegment {
    Static(String),
    Capture(String),
    Wildcard,
    DoubleWildcard,
    Never,
}
