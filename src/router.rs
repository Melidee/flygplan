#[macro_export]
macro_rules! router {
    () => {};
    (
        $($method:ident $route:literal => $func:expr,)*
    ) => {
        {
            macro_rules! method {
                (GET) => { $crate::http::Method::Get };
                (POST) => { $crate::http::Method::Post };
                ($ident:ident) => { compile_error!("Unknown HTTP method.") };
            }
            
            let mut router = $crate::Router::new();
            $(
                router.route(method!($method), $route, $func);
            )*
            router
        }
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