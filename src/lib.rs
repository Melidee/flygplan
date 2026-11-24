pub mod context;
pub mod error;
pub mod http;
pub mod middleware;
pub mod router;

use crate::RouteTarget::Group;
pub use crate::context::{Context, Handler};
pub use crate::error::{Error, Result};
use crate::middleware::Middleware;

use crate::http::{Method, Params, Request, Status};
use std::cell::{RefCell};
use std::rc::Rc;
use std::vec;
use std::{
    io::Read,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

pub use router::{Router};
use router::Route;

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
