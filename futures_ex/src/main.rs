#![feature(async_closure)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::io::{stdin, Read, StdinLock};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc};
use std::task::{Context, Poll};
use std::task::{RawWaker, RawWakerVTable, Waker};

use mio::unix::EventedFd;
use mio::Poll as MioPoll;

struct ExampleStream<'a> {
    stdin_lock: StdinLock<'a>,
    wakers: Arc<RefCell<WakerContexts>>,
    started: bool, // hack because first poll can't check stdin for readable.
}

impl<'a> ExampleStream<'a> {
    pub fn new(stdin_lock: StdinLock<'a>, wakers: Arc<RefCell<WakerContexts>>) -> Self {
        ExampleStream {
            stdin_lock,
            wakers,
            started: false,
        }
    }
}

impl<'a> Future for ExampleStream<'a> {
    type Output = Vec<u8>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        println!("poll");
        if self.started {
            let mut b = [0u8; 2];
            self.stdin_lock.read(&mut b).unwrap();
            if b[0] >= b'0' && b[0] <= b'9' {
                return Poll::Ready((0..(b[0] - b'0')).collect());
            }
        }
        self.started = true;
        self.wakers.borrow_mut().add_waker(
            &self.stdin_lock,
            PollToken::StdIn,
            cx.waker().clone(),
        );
        Poll::Pending
    }
}

unsafe fn waker_drop(_: *const ()) {}
unsafe fn waker_wake(_: *const ()) {}
unsafe fn waker_wake_by_ref(data_ptr: *const ()) {
    println!("wake by ref");
    let bool_atomic_ptr = data_ptr as *const AtomicBool;
    let bool_atomic_ref = bool_atomic_ptr.as_ref().unwrap();
    bool_atomic_ref.store(true, Ordering::Relaxed);
}
unsafe fn waker_clone(data_ptr: *const ()) -> RawWaker {
    create_waker(data_ptr)
}

static WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

unsafe fn create_waker(data_ptr: *const ()) -> RawWaker {
    RawWaker::new(data_ptr, &WAKER_VTABLE)
}

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
enum PollToken {
    StdIn,
}

struct SavedFd(RawFd);

impl AsRawFd for SavedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

struct WakerContexts {
    mio_poll: MioPoll,
    token_map: HashMap<PollToken, (SavedFd, Waker)>,
}

impl WakerContexts {
    pub fn add_waker(&mut self, fd: &dyn AsRawFd, token: PollToken, waker: Waker) {
        self.mio_poll
            .register(
                &EventedFd(&fd.as_raw_fd()),
                mio::Token(token as usize),
                mio::Ready::readable(),
                mio::PollOpt::edge(),
            )
            .unwrap();
        self.token_map
            .insert(token, (SavedFd(fd.as_raw_fd()), waker));
    }

    pub fn wait_wake_readable(&mut self) {
        let mut events = mio::Events::with_capacity(10);
        self.mio_poll.poll(&mut events, None).unwrap();
        for e in &events {
            if !e.readiness().is_readable() {
                continue;
            }
            let token = match e.token().0 {
                0 => PollToken::StdIn,
                _ => panic!("invalid token"),
            };
            if let Some((fd, waker)) = self.token_map.remove(&token) {
                self.mio_poll.deregister(&EventedFd(&fd.0)).unwrap();
                waker.wake_by_ref();
            }
        }
    }
}

fn main() {
    let stdin = stdin();
    let stdin_lock = stdin.lock();

    let wakers_arc = Arc::new(RefCell::new(WakerContexts {
        mio_poll: MioPoll::new().unwrap(),
        token_map: HashMap::new(),
    }));

    let ex = ExampleStream::new(stdin_lock, wakers_arc.clone());
    let closure = async || {
        println!("Hello from async closure.");
        let buf = ex.await;
        println!("Hello from async closure again {}.", buf.len());
    };
    println!("Hello from main");
    let future = closure();
    println!("Hello from main again");

    //need pin
    let fut = Box::pin(future);
    let mut futures = Vec::new();

    //TODO - use atomic instead of refcell
    futures.push((fut, AtomicBool::new(true)));

    // Executer.
    loop {
        for (fut, ready) in futures
            .iter_mut()
            .filter(|(_fut, ready)| ready.load(Ordering::Relaxed))
        {
            ready.store(false, Ordering::Relaxed);
            let raw_waker = unsafe { create_waker(ready as *mut _ as *const _) };

            let waker = unsafe { Waker::from_raw(raw_waker) };
            let mut ctx = Context::from_waker(&waker);
            let f = fut.as_mut();
            match f.poll(&mut ctx) {
                Poll::Pending => (),
                Poll::Ready(_) => return,
            }
        }

        let mut wakers = wakers_arc.borrow_mut();
        wakers.wait_wake_readable();
    }
}
