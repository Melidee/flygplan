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
            }
            
            let mut router = $crate::Flygplan::new();
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
        GET "/" => |c| { c.string("hello") },
        
        #[Logger]
        #[BasicAuth(authFn)]
        "/admin" => {
            GET "/dashboard" => |c| { c.file("dashboard.html") }
        }
    };
    server.listen_and_serve("localhost:3000").unwrap();
}
*/