#![feature(futures_api)]
#![feature(pin)]
#![feature(async_await)]
#![feature(await_macro)]
#![feature(arbitrary_self_types)]

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Condvar};
use std::task::{Wake, Poll, LocalWaker};

struct MainWaker {
    locker: Mutex<bool>,
    cvar: Condvar,
}

impl MainWaker {
    fn new() -> Arc<Self> {
        Arc::new( Self {
            locker: Mutex::new(true),
            cvar: Condvar::new(),
        })
    }

    /// This can only be safely called by one thread
    fn local(self: &Arc<Self>) -> LocalWaker {
        ::std::task::local_waker_from_nonlocal(self.clone())
    }

    /// Must be called after local
    ///
    /// The wait on the cvar is prone to spurious wakeups, but this is ok so long as `wait` is
    /// called after the Poll::Pending is returned from a call to `poll` on a future.
    fn wait(&self) {
        let flag_lock = self.locker.lock().unwrap();

        if *flag_lock {
            let _unused = self.cvar.wait(flag_lock).unwrap();
        }
    }

    fn release(&self) {
        *self.locker.lock().unwrap() = false;
        self.cvar.notify_one()
    }
}

impl Wake for MainWaker {
    fn wake(arc_self: &Arc<Self>) {
        arc_self.release()
    }
}

/// A structure for polling a future
struct Poller<T,O> where T: Future<Output=O> {
    future: T,
    waker: Arc<MainWaker>,
}

impl<T,O> Poller<T,O> where T: Future<Output=O> {

    fn new( future: T ) -> Self {
        let waker = MainWaker::new();

        Poller {
            future: future,
            waker: waker.clone(),
        }
    }

    fn poll_once(mut self) -> FuturePair<T,O> {
        match unsafe { Pin::new_unchecked(&mut self.future) }.poll(&self.waker.local()) {
            Poll::Ready(val) => FuturePair::Val(val),
            Poll::Pending    => FuturePair::Fut(self),
        }
    }

    fn poll_to_completion(mut self) -> O {
        loop {
            match unsafe { Pin::new_unchecked(&mut self.future) }.poll(&self.waker.local()) {
                Poll::Ready(val) => break val,
                Poll::Pending    => self.waker.wait(),
            }
        }
    }
}

/// An enum for switching between a Future object and its Output
enum FuturePair<T,O> where T: Future<Output=O> {
    Fut(Poller<T,O>),
    Val(O),
}

impl<T,O> FuturePair<T,O> where T: Future<Output=O> {

    /// If self is a FuturePair::Fut(Poller<T,O>) then the future is polled to completion and self
    /// is set FuturePair::Val(O). Nothing is done if self is a FuturePair::Val(O).
    fn poll_into_val(&mut self) -> &mut Self {
        use std::mem;

        let lame_duck_fut = FuturePair::Fut( unsafe { mem::uninitialized() } );

        if mem::discriminant(self) == mem::discriminant(&lame_duck_fut)
        {
            match mem::replace(self, unsafe{ mem::uninitialized() } )
            {
                FuturePair::Fut(poller) => mem::replace(self, FuturePair::Val(poller.poll_to_completion()) ),
                FuturePair::Val(_) => panic!("File a bug if you get this panic"),
            };
        }

        self
    }

    /// Get a reference to the contained value
    #[inline]
    fn get_val_ref(&mut self) -> &mut O {
        match *self {
            FuturePair::Val(ref mut v) => v,
            FuturePair::Fut(_)         => self.poll_into_val().get_val_ref(),
        }
    }

    /// Convert self into O
    fn into( self ) -> O {
        match self {
            FuturePair::Val(v) => v,
            FuturePair::Fut(f) => f.poll_to_completion(),
        }
    }
}

impl<T,O> FuturePair<T,O>  where T: Future<Output=O>, O: Clone {
    /// Return a clone from a cell
    ///
    /// If self is a FuturePair::Fut(Poller<T,O>) then the future is polled to completion and self
    /// is set FuturePair::Val(O).
    fn clone_in_cell(cell: &Cell<Self>) -> Self {
        use std::mem;

        let val = match cell.replace(unsafe{ mem::uninitialized() }) {
            FuturePair::Val(val) => val,
            FuturePair::Fut(fut) => fut.poll_to_completion(),
        };

        cell.set(FuturePair::Val(val.clone()));

        FuturePair::Val(val)
    }
}

/// A wrapper for retuned future objects
///
/// The purpose of `Later` is to create a wrapper that polls its contained future object to
/// completion only at the point where the output of the future is required. The first call to any
/// implemented function of `Later` that returns the output or a reference to the output of the
/// future will cause `Later` to poll the future (for any subsequent calls `Later` will not poll).
pub struct Later<T,O> where T: Future<Output=O>{
    fut_pair: Cell<FuturePair<T,O>>,
}

impl<T,O> Later<T,O> where T: Future<Output=O> {

    /// Create a new `Later` object from the future object
    ///
    /// This will poll the future once.
    pub fn new( future: T ) -> Self {
        Later {
            fut_pair: Cell::new( Poller::new(future).poll_once() ),
        }
    }

    /// Consume self and return the output value of the future.
    ///
    /// This will poll the future unitl completion.
    pub fn into_inner(self) -> O {
        self.fut_pair.into_inner().into()
    }
}

impl<T,O> Later<T,O> where T: Future<Output=O>, O: Clone {

    /// Get the returned value of the future
    pub fn get(&self) -> O {
        FuturePair::clone_in_cell(&self.fut_pair).into()
    }
}

impl<T,O> ::std::ops::Deref for Later<T,O> where T: Future<Output=O> {
    type Target = O;

    fn deref(&self) -> &O {
        // unsafe{ *self.fut_pair.as_ptr() }.get_val_ref()
        unimplemented!()
    }
}

impl<T,O> ::std::ops::DerefMut for Later<T,O> where T: Future<Output=O> {
    fn deref_mut(&mut self) -> &mut O {
        // unsafe{ *self.fut_pair.as_ptr() }.get_val_ref()
        unimplemented!()
    }
}

#[macro_export]
macro_rules! later {
    ( $future:expr ) => {
        ::alligator::Later::new($future)
    }
}

#[macro_export]
macro_rules! l {
    ( $future:expr ) => { later!($future)}
}
