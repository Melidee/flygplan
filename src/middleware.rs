use crate::Context;

pub fn logger(c: Context) -> Context {
    println!("{} {}", c.request.method, c.request.resource);
    c
}