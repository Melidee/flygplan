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
    let mut flyg = Flygplan::new();
    // the Context type is used to get information about the request and write out a response
    flyg.get("/", |c: Context| c.file("pages/index.html"));
    flyg.post("/", |c| {
        println!("{}", c.request);
        c.redirect("/whatever")
    });
    flyg.get("/hej", |c| match c.query_param("name") {
        Some(name) => c.string(&format!("Hej hej, {}", name)),
        None => c.status(Status::BadRequest400),
    });
    // dynamic routes start with a colon
    flyg.get("/hello/:name", |c| {
        let name = c.path_param("name").unwrap();
        c.string(&format!("Hello, {}!", name))
    });
    // wildcard patterns can be either * for single sections or ** for multiple
    flyg.get("/**/wildcard", |c| c.string("wildcard!"));
    // types serializable by serde can be returned as JSON
    flyg.get("/some-data", |c| c.json(serializable_data()));
    flyg.get("/home", |c| c.redirect("/"));
    // status handlers automatically handle certain HTTP statuses to centralize error handling and common responses
    // some are automatic and others can be called by the Context status method
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