#![feature(arbitrary_self_types, futures_api, pin)]

use std::sync::mpsc::{sync_channel, SyncSender};
use std::future::{Future, FutureObj};
use std::mem::PinMut;
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex};
use std::task::{
    Context,
    Executor,
    local_waker_from_nonlocal,
    Poll,
    SpawnObjError,
    Wake,
};


struct Exec;
impl Executor for Exec {
    fn spawn_obj(&mut self, _obj: FutureObj<'static, ()>) -> Result<(), SpawnObjError> {
        Ok(())
    }
}

pub struct Timer {
    end: Instant,
}
    
impl Timer {
    pub fn new(secs: u64) -> Timer {
        Timer {
            end: Instant::now() + Duration::from_secs(secs),
        }
    }
}

impl Future for Timer {
    type Output = ();
    fn poll(self: PinMut<Self>, cx: &mut Context) -> Poll<()> {
        match self.end < Instant::now() {
            true    => Poll::Ready(()),
            false   => {
                cx.waker().wake();
                Poll::Pending
            }
        }
    }
}
struct Task {
    sender: SyncSender<Arc<Task>>,
    future: Mutex<Timer>
}
impl Wake for Task {
    fn wake(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        let _ = arc_self.sender.send(cloned);
    }
}

fn main() {
    let mut exec = Exec;
    let (tx, rx) = sync_channel(1000);
    let task = Arc::new(Task { future: Mutex::new(Timer::new(8)), sender: tx.clone() });
    let waker = local_waker_from_nonlocal(task.clone());
    let cx = &mut Context::new(&waker, &mut exec);
    let _ = tx.send(task);

    while let Ok(task) = rx.recv() {
        let mut future = task.future.lock().unwrap();
        match PinMut::new(&mut *future).poll(cx) {
            Poll::Pending => {},
            Poll::Ready(()) => {
                println!("8 seconds have passed");
                break;
            },
        }
    }

}