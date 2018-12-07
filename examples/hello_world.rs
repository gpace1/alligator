#![feature(async_await)]
#![feature(futures_api)]
#[macro_use] extern crate alligator;

async fn hello_world() -> &'static str {
  "Hello World"
}

fn main() {
  println!("{}", later!{ hello_world() });
}
