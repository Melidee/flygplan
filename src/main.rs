use flygplan::Flygplan;

fn main() {
    let mut flyg = Flygplan::new();
    flyg.get("/", |c| c.string("Hello, world!"));
    flyg.listen_and_serve("localhost:8080").unwrap();
}
