use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{LocalWaker, Poll, Waker};
use std::thread;
use std::time::Duration;

pub struct Sleeper {
    msg: String,
    time: Duration,
    waker: Arc<Mutex<(Option<Waker>,bool)>>,
}

impl Sleeper {
    pub fn new( time: Duration, message: String ) -> Self {
        Sleeper {
            msg: message,
            time: time,
            waker: Arc::new(Mutex::new((None,false))),
        }
    }
}

impl Future for Sleeper {
    type Output = String;

    fn poll(self: Pin<&mut Self>, ls: &LocalWaker) -> Poll<Self::Output> {
        let mut guard = self.waker.lock().unwrap();

        if let None = (*guard).0 {
            let waker_clone = self.waker.clone();
            let duration_clone = self.time.clone();

            *guard = (Some(ls.as_waker().clone()), false);

            thread::spawn( move || {
                thread::sleep(duration_clone);

                let ref mut waker_pair = *waker_clone.lock().unwrap();

                match waker_pair {
                    (Some(ref waker), ref mut flag) => {
                        *flag = true;
                        waker.wake();
                    }
                    (None, _) => { panic!() }
                }
            });

            Poll::Pending
        }
        else if ! (*guard).1 {
            Poll::Pending
        }
        else {
            Poll::Ready(self.msg.clone())
        }
    }
}
