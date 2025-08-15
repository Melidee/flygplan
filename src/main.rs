use flygplan::{Flygplan, http::Status};

fn main() {
    let mut flyg = Flygplan::new();
    flyg.get("/", |c| c.string("Hello, world!"));
    flyg.get("/hello", |c| {
        let name = c.query_params().get("name").unwrap();
        c.string(&format!("Hello, {}!", name))
    });
    flyg.status_handler(Status::NotFound404, |c| {
        c.string("oopsie whoopsie, page not found")
    });
    flyg.listen_and_serve("localhost:8080").unwrap();
}
