use flygplan::{Flygplan, http::Status};

fn main() {
    let mut flyg = Flygplan::new();
    flyg.get("/", |c| c.string("Hello, world!"));
    flyg.get("/amelia", |c| c.string("Hello, Amelia!"));
    flyg.status_handler(Status::NotFound, |c| {
        c.string("oopsie whoopsie, page not found")
    });
    flyg.listen_and_serve("localhost:8080").unwrap();
}
