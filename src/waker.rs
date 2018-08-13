#![feature(arbitrary_self_types, futures_api, pin)]

use std::sync::mpsc::{sync_channel, SyncSender};
use std::future::{Future, FutureObj};
use std::mem::PinMut;
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

struct MyFuture(bool);
impl Future for MyFuture {
    type Output = ();
    fn poll(mut self: PinMut<Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        cx.waker().wake();
        Poll::Pending
    }
}

struct Task {
    sender: SyncSender<Arc<Task>>,
    future: Mutex<MyFuture>
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
    let task = Arc::new(Task { future: Mutex::new(MyFuture(false)), sender: tx.clone() });
    let waker = local_waker_from_nonlocal(task.clone());
    let cx = &mut Context::new(&waker, &mut exec);
    let _ = tx.send(task);

    while let Ok(task) = rx.recv() {
        let mut future = task.future.lock().unwrap();
        match PinMut::new(&mut *future).poll(cx) {
            Poll::Pending => println!("Pending"),
            Poll::Ready(()) => {
                println!("Ready");
                break;
            },
        }
    }

}