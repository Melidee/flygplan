#[macro_export]
macro_rules! router_inner {
    ([] $r:ident) => {};
    // match global middleware
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

fn test() {
    use crate::middleware::Logger;
    let x = router! {
        #![Logger::new()]

        #[Logger::new()]
        GET "/" => |c| {
            c.string("hello")
        },

        status 404 => |c| {
            c.string("not found")
        },
    };
}

/*
fn test() {
    let server = router! {
        #![Logger]
        #![RemoveTrailingSlash]
    
        GET "/" => |c| { c.string("hello") },
        
        #[Inherit]
        #[BasicAuth(authFn)]
        "/admin" => {
            GET "/dashboard" => |c| { c.file("dashboard.html") }
        }
    };
    server.listen_and_serve("localhost:3000").unwrap();
}
*/