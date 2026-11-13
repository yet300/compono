use compono::{Spawner, TaskGuard};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// Runs tasks on the browser's event loop.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebSpawner;

#[derive(Default)]
struct Cancel {
    cancelled: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

struct Cancellable {
    task: Pin<Box<dyn Future<Output = ()> + Send>>,
    cancel: Rc<Cancel>,
}

impl Future for Cancellable {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.cancel.cancelled.get() {
            return Poll::Ready(());
        }
        if let Ok(mut waker) = self.cancel.waker.try_borrow_mut() {
            waker.clone_from(&Some(cx.waker().clone()));
        }
        self.task.as_mut().poll(cx)
    }
}

struct CancelOnDrop(Rc<Cancel>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancelled.set(true);
        let waker = self
            .0
            .waker
            .try_borrow_mut()
            .ok()
            .and_then(|mut w| w.take());
        if let Some(waker) = waker {
            // The executor polls once more, sees the flag and drops the task.
            waker.wake();
        }
    }
}

impl Spawner for WebSpawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) -> TaskGuard {
        let cancel = Rc::new(Cancel::default());
        wasm_bindgen_futures::spawn_local(Cancellable {
            task,
            cancel: Rc::clone(&cancel),
        });
        TaskGuard::new(CancelOnDrop(cancel))
    }
}

struct Timer {
    handle: i32,
    fired: bool,
    waker: Option<Waker>,
    _callback: Closure<dyn FnMut()>,
}

#[derive(Default)]
struct Timers {
    next: u64,
    active: BTreeMap<u64, Timer>,
}

thread_local! {
    static TIMERS: RefCell<Timers> = RefCell::new(Timers::default());
}

fn fire(id: u64) {
    let waker = TIMERS.with(|timers| {
        let mut timers = timers.try_borrow_mut().ok()?;
        let timer = timers.active.get_mut(&id)?;
        timer.fired = true;
        timer.waker.take()
    });
    if let Some(waker) = waker {
        waker.wake();
    }
}

fn clear(handle: i32) {
    if let Some(window) = web_sys::window() {
        window.clear_timeout_with_handle(handle);
    }
}

/// A timer on the browser's `setTimeout`, from [`sleep`].
///
/// It holds only numbers, so it is `Send` and can run inside a
/// [`Cx::perform`](compono::Cx::perform) task. It must be polled on the page's
/// own thread, where the timer lives.
#[derive(Debug)]
pub struct Sleep {
    delay: i32,
    id: Option<u64>,
    done: bool,
}

/// Completes after `delay`.
#[must_use]
pub fn sleep(delay: Duration) -> Sleep {
    Sleep {
        delay: i32::try_from(delay.as_millis()).unwrap_or(i32::MAX),
        id: None,
        done: false,
    }
}

impl Sleep {
    fn start(&mut self, waker: &Waker) -> Poll<()> {
        let Some(window) = web_sys::window() else {
            return Poll::Ready(());
        };
        TIMERS.with(|timers| {
            let Ok(mut timers) = timers.try_borrow_mut() else {
                return Poll::Ready(());
            };
            let id = timers.next;
            timers.next += 1;
            let callback = Closure::<dyn FnMut()>::new(move || fire(id));
            let Ok(handle) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                self.delay,
            ) else {
                return Poll::Ready(());
            };
            timers.active.insert(
                id,
                Timer {
                    handle,
                    fired: false,
                    waker: Some(waker.clone()),
                    _callback: callback,
                },
            );
            self.id = Some(id);
            Poll::Pending
        })
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done {
            return Poll::Ready(());
        }
        let Some(id) = self.id else {
            return self.start(cx.waker());
        };
        let fired = TIMERS.with(|timers| {
            let Ok(mut timers) = timers.try_borrow_mut() else {
                return false;
            };
            match timers.active.get_mut(&id) {
                Some(timer) if !timer.fired => {
                    timer.waker = Some(cx.waker().clone());
                    false
                }
                // Removing a fired timer here, outside its callback, frees the closure safely.
                _ => {
                    timers.active.remove(&id);
                    true
                }
            }
        });
        if fired {
            self.done = true;
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        let Some(id) = self.id.filter(|_| !self.done) else {
            return;
        };
        let removed = TIMERS.with(|timers| timers.try_borrow_mut().ok()?.active.remove(&id));
        if let Some(timer) = removed {
            clear(timer.handle);
        }
    }
}
