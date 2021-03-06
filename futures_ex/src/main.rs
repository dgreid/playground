#![feature(async_closure)]
// TODO - remove nightly drain_filter code
#![feature(drain_filter)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::io::{stdin, Read, StdinLock};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::task::{RawWaker, RawWakerVTable, Waker};

use sys_util::PollContext;

struct ExampleStream<'a> {
    stdin_lock: StdinLock<'a>,
    // waker has to be a member as there isn't a good way to communicate it to the future. tokio
    // uses thread local storage, which is just as bad. Each future that can block has to know the
    // details about the executor it is running in.
    waker: Arc<RefCell<dyn FdExecutorInterface>>,
    started: bool, // hack because first poll can't check stdin for readable.
}

impl<'a> ExampleStream<'a> {
    pub fn new(stdin_lock: StdinLock<'a>, waker: Arc<RefCell<dyn FdExecutorInterface>>) -> Self {
        ExampleStream {
            stdin_lock,
            waker,
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
        self.waker
            .borrow_mut()
            .add_waker(&self.stdin_lock, cx.waker().clone());
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

// Saved FD exists becaus RawFd doesn't impl AsRawFd.
struct SavedFd(RawFd);

impl AsRawFd for SavedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub trait FdExecutorInterface {
    /// Tells the waking system to wake `waker` when `fd` becomes readable.
    fn add_waker(&mut self, fd: &dyn AsRawFd, waker: Waker);
    /// Adds a new top level future to the Executor.
    fn add_future(&mut self, future: Pin<Box<dyn Future<Output = ()>>>);
}

/// Handles tracking the state of any futures blocked on FDs and allows adding a wake up request
/// from the poll funciton of a future.
pub struct InterfaceState {
    poll_ctx: PollContext<u64>,
    token_map: HashMap<u64, (SavedFd, Waker)>,
    next_token: u64,
    new_futures: Vec<(Pin<Box<dyn Future<Output = ()>>>, AtomicBool)>,
}

/// Used by futures who want to block until an FD becomes readable.
/// Keeps a list of FDs and associated wakers that will be woekn with `wake_by_ref` when the FD
/// becomes readable.
impl InterfaceState {
    /// Create an empty InterfaceState.
    pub fn new() -> InterfaceState {
        InterfaceState {
            poll_ctx: PollContext::new().unwrap(),
            token_map: HashMap::new(),
            next_token: 0,
            new_futures: Vec::new(),
        }
    }

    /// Waits until one of the FDs is readable and wakes the associated waker.
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

impl FdExecutorInterface for InterfaceState {
    /// Tells the waking system to wake `waker` when `fd` becomes readable.
    fn add_waker(&mut self, fd: &dyn AsRawFd, waker: Waker) {
        while self.token_map.contains_key(&self.next_token) {
            self.next_token += 1;
        }
        self.poll_ctx.add(fd, self.next_token).unwrap();
        self.token_map
            .insert(self.next_token, (SavedFd(fd.as_raw_fd()), waker));
    }

    fn add_future(&mut self, future: Pin<Box<dyn Future<Output = ()>>>) {
        self.new_futures.push((future, AtomicBool::new(true)));
    }
}

pub struct FdExecutor {
    futures: Vec<(Pin<Box<dyn Future<Output = ()>>>, AtomicBool)>,
    state: Arc<RefCell<InterfaceState>>,
    // TODO - add ability to append futures, maybe keep a refcell to a vec of new features and
    // share it with the running futures.
}

impl FdExecutor {
    pub fn new(
        futures: Vec<Pin<Box<dyn Future<Output = ()>>>>,
        state: Arc<RefCell<InterfaceState>>,
    ) -> FdExecutor {
        FdExecutor {
            futures: futures
                .into_iter()
                .map(|fut| (fut, AtomicBool::new(true)))
                .collect(),
            state,
        }
    }

    pub fn run(mut self) {
        loop {
            self.futures.drain_filter(|(fut, ready)| {
                if !ready.load(Ordering::Relaxed) {
                    return false;
                }

                ready.store(false, Ordering::Relaxed);
                let raw_waker = unsafe { create_waker(ready as *mut _ as *const _) };

                let waker = unsafe { Waker::from_raw(raw_waker) };
                let mut ctx = Context::from_waker(&waker);
                let f = fut.as_mut();
                match f.poll(&mut ctx) {
                    Poll::Pending => false,
                    Poll::Ready(_) => true,
                }
            });

            // Add any new futures to the list.
            let mut state = self.state.borrow_mut();
            self.futures.append(&mut state.new_futures);

            if self.futures.is_empty() {
                return;
            }

            state.wait_wake_readable();
        }
    }
}

fn main() {
    let wakers = Arc::new(RefCell::new(InterfaceState::new()));

    let clone_wakers = wakers.clone();
    let closure = async || {
        let stdin = stdin();
        let stdin_lock = stdin.lock();

        let ex = ExampleStream::new(stdin_lock, clone_wakers);
        println!("Hello from async closure.");
        let buf = ex.await;
        println!("Hello from async closure again {}.", buf.len());
    };
    println!("Hello from main");
    let future = closure();
    println!("Hello from main again");

    //need pin
    let fut = Box::pin(future);

    let mut futures: Vec<Pin<Box<dyn Future<Output = ()>>>> = Vec::new();
    futures.push(fut);

    let ex = FdExecutor::new(futures, wakers);
    ex.run();
}
