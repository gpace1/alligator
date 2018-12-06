#![feature(async_await)]
#![feature(await_macro)]
// The next 3 are required for the sleeper module
#![feature(arbitrary_self_types)]
#![feature(futures_api)]
#![feature(pin)]

#[macro_use] extern crate alligator;

mod sleeper;

/// There is nothing asynchronous about this 'async' function. Anything that polls the return of
/// this function will immediately return a Poll::Ready('message')
async fn not_actually_async() -> String {
    String::from("This message is not actually asychronous")
}

/// The sleeper object just takes a message and a duration for when to return the message.
async fn totally_async() -> String {
    use std::time::Duration;
    use sleeper::Sleeper;

    await!(Sleeper::new( Duration::from_millis(1500), String::from("This is an asychronous message")))
}

fn main() {
    let async_msg     = l!(totally_async());
    let non_async_msg = l!(not_actually_async());

    //println!("{}", *non_async_msg);
    println!("{}", async_msg.get());
    println!("A normal message");
}
