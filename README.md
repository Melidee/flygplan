# Flygplan: A bad web server!
This is an attempt to make a little web server library to learn about rust and the web!
It is inspired by the [echo web framework](https://echo.labstack.com/) for the go programming language.
This project includes parsers for http and urls and a router, but there are future plans for a templating system and more.
The name flygplan comes from the swedish word which means airplane, I picked it because I started writing this on an airplane to Sweden. 


## Example

```rs
use flygplan::{Flygplan};

let mut flyg = Flygplan::new();
// the Context type is used to get information about the request and write out a response
flyg.get("/", |c: Context| {
    c.string("Hello, world!").unwrap();
});
flyg.get("/hej", |c| {
    match c.query_params().get("name") {
        Some(name) => c.string(&format!("Hej hej, {}", name)),
        None => c.status(Status::BadRequest400),
    }
    .unwrap();
});
// dynamic routes are surrounded with parentheses
flyg.get("/hello/{name}", |c| {
    let name = c.url_param("name").unwrap();
    c.string(&format!("Hello, {}!", name)).unwrap();
});
// status handlers automatically handle certain HTTP statuses to centralize error handling and common responses
// some are automatic and others can be called by the Context status method
flyg.status_handler(Status::NotFound404, |c| {
    c.string("oopsie whoopsie, page not found").unwrap();
});
flyg.listen_and_serve("localhost:8080").unwrap();
```