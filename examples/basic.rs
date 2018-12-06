#![feature(async_await)]
#![feature(await_macro)]
// The next 3 are required for the sleeper module
#![feature(arbitrary_self_types)]
#![feature(futures_api)]
#![feature(pin)]

#[macro_use] extern crate alligator;

mod sleeper;

use std::time::Duration;

/// There is nothing asynchronous about this 'async' function. Anything that polls the return of
/// this function will immediately return a Poll::Ready('message')
async fn not_actually_async() -> String {
    String::from("This message is not actually asychronous")
}

/// The sleeper object just takes a message and a duration for when to return the message.
async fn totally_async<T: Into<String>>(sleep_time: Duration, message: T) -> String {

    await!(sleeper::Sleeper::new( sleep_time, message.into()))
}


fn main() {
    // When a later object is created, it calls poll for the future once. This kicks starts the
    // future (by passing a LocalWaker) into performing any asynchronous operations.
    let non_async_msg  = l!(not_actually_async());

    let async_msg = l!(totally_async( Duration::from_millis(1500), "async_msg return message"));

    // l! and later! are the same, they're just a quick macro for Later::new.
    let long_wait_async_msg = later!(totally_async( Duration::from_millis(4500), "long_wait_async_msg return message"));

    // Although `not_actually_async` is tagged with async, there isn't anything asynchronous
    // happening in the function. When the return is polled, it will only return ready with the
    // future Output value.
    println!("{}", *non_async_msg);

    // Here the task needs to wait for the message for the given length. If there was some
    // operation that took longer then the specified duration for `long_wait_async_msg` then this
    // would have printed immediately. However since the all there really is is a print message
    // between the initialization and here, the full time is effectively waitied fot.
    println!("{}", long_wait_async_msg);

    // This prints immediatly after long_wait_async_msg's message is printed because the timer had
    // already expired. Internally, the future had already finished, so when the future is polled
    // it returns `Ready` with the contained message.
    println!("{}", async_msg.get());

    println!("A normal message");
}
