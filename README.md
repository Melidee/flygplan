# Flygplan: A bad web server!
This is an attempt to make a little web server library to learn about rust and the web!
It is inspired by the [echo web framework](https://echo.labstack.com/) for the go programming language.

## Example

```rs
use flygplan::{Flygplan};

let mut flyg = Flygplan::new();
// the Context type is used to get information about the request and write out a response
flyg.get("/", |c: Context| c.string("Hello, world!"));
flyg.get("/hello", |c| {
    let name = c.query_params().get("name").unwrap();
    c.string(&format!("Hello, {}!", name))
});
// status handlers automatically handle certain HTTP statuses to centralize error handling and common responses
// some are automatic and others can be called by the Context status method
flyg.status_handler(Status::NotFound404, |c| {
    c.string("oopsie whoopsie, page not found")
});
flyg.listen_and_serve("localhost:8080").unwrap();
```