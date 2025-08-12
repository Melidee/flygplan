use flygplan::Flygplan;

fn main() {
    let mut flyg = Flygplan::new();
    flyg.get("/", |c| c.string("Hello, world!"));
    flyg.get("/amelia", |c| c.string("Hello, Amelia!"));
    flyg.listen_and_serve("localhost:8080").unwrap();
}
