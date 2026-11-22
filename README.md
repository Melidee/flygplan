# Flygplan: A bad web server!
This is an attempt to make a little web server library to learn about rust and the web!
It is inspired by the [echo web framework](https://echo.labstack.com/) for the go programming language.
This project includes parsers for http and urls and a router, but there are future plans for a templating system and more.
The name flygplan comes from the swedish word which means airplane, I picked it because I started writing this on an airplane to Sweden. 


## Example

```rs
use std::collections::HashMap;

use flygplan::{Context, Flygplan, Result, http::Status, middleware};

fn main() {
use std::collections::HashMap;

use flygplan::{Context, Flygplan, Result, http::Status, middleware, router};

fn main() {
    let mut flyg = router! {
        // Context is used to get data from the request and write out a response
        GET "/" => |c: Context| c.file("pages/index.html"),
        POST "/" => |c| {
            println!("{}", c.request);
            c.redirect("/whatever")
        },
        GET "/hej" => |c| match c.query_param("name") {
            Some(name) => c.string(&format!("Hej hej, {}", name)),
            None => c.status(Status::BadRequest400),
        },
        // Dynamic routes start with : and can match an identifier
        GET "/hello/:name" => |c| {
            let name = c.path_param("name").unwrap();
            c.string(&format!("Hello, {}!", name))
        },
        GET "/**/wildcard" => |c| c.string("wildcard!"),
        GET "/some-data" => |c| c.json(serializable_data()),
        GET "/home" => |c| c.redirect("/"),
    };
    
    flyg.status_handler(Status::NotFound404, |c| {
        c.string("oopsie whoopsie, page not found")
    });
    flyg.use_middleware(middleware::Logger {});
    flyg.use_middleware(middleware::RemoveTrailingSlash {});
    
    println!("Listening on http://localhost:3333");
    flyg.listen_and_serve("localhost:3333").unwrap();
}

fn serializable_data() -> HashMap<String, i32> {
    let mut map = HashMap::new();
    map.insert("amelia".to_string(), 19);
    map.insert("matteo".to_string(), 25);
    return map;
}
```