use super::{Pool, TaskBoxed};
use ::std::mem::ManuallyDrop;
use crossbeam_utils::sync::WaitGroup;

pub struct Scope<'s, 'p: 's, const NUM: usize> {
    pool: &'s Pool<'p, NUM>,
    wait_group: ManuallyDrop<WaitGroup>,
}

impl<'s, 'p: 's, const NUM: usize> Scope<'s, 'p, NUM> {
    #[inline(always)]
    fn change_lifetime(task: TaskBoxed<'s>) -> TaskBoxed<'p> {
        unsafe { ::std::mem::transmute(task) }
    }

    /// better to use `move` keyword for the task closure
    #[inline]
    pub fn send<F: FnOnce() + Send + 's>(
        &self,
        task: F,
    ) -> Result<(), crossbeam_channel::SendError<TaskBoxed>> {
        let wait_group = ManuallyDrop::into_inner(self.wait_group.clone());
        let new_task = move || {
            task();
            drop(wait_group);
        };
        self.pool
            .sender
            .send(Self::change_lifetime(Box::new(new_task)))
    }
}

impl<const NUM: usize> Drop for Scope<'_, '_, NUM> {
    /// must be always manually called at the end of the scope or there will be UB
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.wait_group) }.wait();
    }
}

impl<'s, 'p, const NUM: usize> From<&'s Pool<'p, NUM>> for Scope<'s, 'p, NUM> {
    #[inline]
    fn from(pool: &'s Pool<'p, NUM>) -> Self {
        Self {
            pool,
            wait_group: ManuallyDrop::new(WaitGroup::new()),
        }
    }
}
