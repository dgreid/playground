#![feature(async_await)]

use std::collections::HashMap;
use std::future::Future;
use std::io::{stdin, Read, StdinLock};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::task::{RawWaker, RawWakerVTable, Waker};

use sys_util::{PollContext, PollToken};

struct ExampleStream<'a> {
    stdin_lock: StdinLock<'a>,
    wakers: Arc<Mutex<WakerContexts>>,
    started: bool, // hack because first poll can't check stdin for readable.
}

impl<'a> ExampleStream<'a> {
    pub fn new(stdin_lock: StdinLock<'a>, wakers: Arc<Mutex<WakerContexts>>) -> Self {
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
        self.wakers
            .lock()
            .unwrap()
            .add_waker(&self.stdin_lock, Token::StdIn, cx.waker().clone());
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

#[derive(PollToken, Hash, Clone, Copy, PartialEq, Eq)]
enum Token {
    StdIn,
}

struct SavedFd(RawFd);

impl AsRawFd for SavedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

struct WakerContexts {
    poll_ctx: PollContext<Token>,
    token_map: HashMap<Token, (SavedFd, Waker)>,
}

impl WakerContexts {
    pub fn add_waker(&mut self, fd: &dyn AsRawFd, token: Token, waker: Waker) {
        self.poll_ctx.add(fd, token).unwrap();
        self.token_map
            .insert(token, (SavedFd(fd.as_raw_fd()), waker));
    }

    pub fn wait_wake_readable(&mut self) {
        let events = self.poll_ctx.wait().unwrap();
        for e in events.iter_readable() {
            if let Some((fd, waker)) = self.token_map.remove(&e.token()) {
                self.poll_ctx.delete(&fd).unwrap();
                waker.wake_by_ref();
            }
        }
    }
}

fn main() {
    let stdin = stdin();
    let stdin_lock = stdin.lock();

    let wakers_arc = Arc::new(Mutex::new(WakerContexts {
        poll_ctx: PollContext::new().unwrap(),
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

        let mut wakers = wakers_arc.lock().unwrap();
        wakers.wait_wake_readable();
    }
}
