#![feature(async_await, await_macro, futures_api, pin)]

use std::future::{Future, FutureObj};
use std::mem::PinMut;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{sync_channel, SyncSender, SendError, Receiver};
use std::task::{
    self,
    Executor,
    local_waker_from_nonlocal,
    Poll,
    SpawnErrorKind,
    SpawnObjError,
    Wake,
};

struct Exec {
    task_sender: SyncSender<Arc<Task>>,
    task_receiver: Receiver<Arc<Task>>,
}

impl<'a> Executor for &'a Exec {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>)
        -> Result<(), SpawnObjError>
    {
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });

        self.task_sender.send(task).map_err(|SendError(task)| {
            SpawnObjError {
                kind: SpawnErrorKind::shutdown(),
                future: task.future.lock().unwrap().take().unwrap(),
            }
        })
    }
}

struct Task {
    future: Mutex<Option<FutureObj<'static, ()>>>,
    task_sender: SyncSender<Arc<Task>>,
}

impl Wake for Task {
    fn wake(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        let _ = arc_self.task_sender.send(cloned);
    }
}

impl Exec {
    fn new() -> Self {
        let (task_sender, task_receiver) = sync_channel(1000);
        Exec { task_sender, task_receiver }
    }

    fn run(&self) {
        let mut executor = &*self;
        while let Ok(task) = self.task_receiver.recv() {
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                let waker = local_waker_from_nonlocal(task.clone());
                let cx = &mut task::Context::new(&waker, &mut executor);
                if let Poll::Pending = PinMut::new(&mut future).poll(cx) {
                    *future_slot = Some(future);
                }
            }
        }
    }
}

fn main() {
    let executor = Exec::new();
    (&executor).spawn_obj(FutureObj::new(Box::new(async {
        println!("I'm a spawned future!");
    }))).unwrap();
    executor.run();
}