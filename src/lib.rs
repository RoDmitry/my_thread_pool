use ::std::{
    mem::ManuallyDrop,
    thread::{self, JoinHandle},
};
use crossbeam_channel::Sender;

mod scope;
pub use scope::Scope;

type TaskBoxed<'t> = Box<dyn FnOnce() + Send + 't>;

pub struct Pool<'p, const NUM: usize> {
    handles: ManuallyDrop<[JoinHandle<()>; NUM]>,
    sender: ManuallyDrop<Sender<TaskBoxed<'p>>>,
}

impl<'p, const NUM: usize> Pool<'p, NUM> {
    pub fn new(cap: usize) -> Self {
        let (send, recv) = crossbeam_channel::bounded::<TaskBoxed<'p>>(cap);
        Self {
            handles: ManuallyDrop::new(::core::array::from_fn(|_| {
                let recv_clone = recv.clone();
                let run = move || {
                    while let Ok(task) = recv_clone.recv() {
                        task();
                    }
                };
                unsafe {
                    thread::Builder::new()
                        .spawn_unchecked(run)
                        .expect("failed to spawn thread")
                }
            })),
            sender: ManuallyDrop::new(send),
        }
    }

    /// better to use `move` keyword for the task closure
    #[inline]
    pub fn send<F: FnOnce() + Send + 'p>(
        &self,
        task: F,
    ) -> Result<(), crossbeam_channel::SendError<TaskBoxed>> {
        self.sender.send(Box::new(task))
    }

    /// `Scope::drop` must be always manually called at the end of the scope or there will be UB
    #[inline]
    pub fn scope<'s>(&'s self) -> Scope<'s, 'p, NUM> {
        Scope::from(self)
    }

    /* pub fn scope<'s, F>(&'s self, f: F)
    where
        F: for<'a> FnOnce(&'a Scope<'s, 't, NUM>),
    {
        let scope = Scope {
            pool: self,
            wait_group: ManuallyDrop::new(WaitGroup::new()),
        };
        f(&scope);
        drop(scope);
    } */
}

impl<const NUM: usize> Drop for Pool<'_, NUM> {
    /// must be always manually called at the end of the scope or there will be UB
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.sender) };
        unsafe { ManuallyDrop::take(&mut self.handles) }
            .into_iter()
            .for_each(|v| v.join().unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::Pool;
    use ::std::{thread, time::Duration};

    #[test]
    fn test_new() {
        let pool = Pool::<4>::new(2);
        drop(pool);
    }

    #[test]
    fn test_send() {
        let pool = Pool::<2>::new(2);

        let (send, recv) = crossbeam_channel::bounded::<u32>(2);
        let send_cloned = send.clone();
        pool.send(move || {
            thread::sleep(Duration::from_secs(1));
            send_cloned.send(1).unwrap();
        })
        .unwrap();

        pool.send(move || {
            thread::sleep(Duration::from_secs(2));
            send.send(2).unwrap();
        })
        .unwrap();

        assert_eq!(recv.recv().unwrap(), 1);
        assert_eq!(recv.recv().unwrap(), 2);
        drop(pool);
    }

    #[test]
    fn test_scope() {
        let pool = Pool::<2>::new(2);

        let (send, recv) = crossbeam_channel::bounded::<u32>(2);
        let send_cloned = send.clone();
        pool.send(move || {
            thread::sleep(Duration::from_secs(1));
            send_cloned.send(1).unwrap();
        })
        .unwrap();

        // pool.scope(|scope| {
        {
            let scope = pool.scope();
            let text = "TESTTEXT".to_owned();
            let text_ref = &text;
            scope
                .send(move || {
                    thread::sleep(Duration::from_secs(2));
                    assert_eq!(text_ref, "TESTTEXT");
                    send.send(2).unwrap();
                })
                .unwrap();
            drop(scope);
        }
        // });

        assert_eq!(recv.recv().unwrap(), 1);
        assert_eq!(recv.recv().unwrap(), 2);
        drop(pool);
    }
}
