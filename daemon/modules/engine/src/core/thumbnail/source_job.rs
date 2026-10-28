//! The table of source jobs: at most one job waiting for its slot and one job decoding, per source, in the whole daemon.
//!
//! The table only decides who waits for which job. It knows nothing about images: the caller says what an item wants (`wanted`),
//! what the items of a job make (`start`), and runs the job. See Thumbnail Design §7.
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::watch;

use crate::{
    core::{schedule::RaiseHandle, work::WorkClass},
    prelude::*,
};

/// What every item of a job receives: the output the job made, or the error that ended it.
pub(super) type Outcome<O> = std::result::Result<Arc<O>, Arc<Error>>;

/// The end of a job. It is published once, and every waiter, including one that comes after, reads it.
pub(super) struct Done<O> {
    sender: watch::Sender<Option<Outcome<O>>>,
}

impl<O> Done<O> {
    fn new() -> Arc<Self> {
        Arc::new(Self { sender: watch::Sender::new(None) })
    }

    /// Only the first outcome is kept.
    fn publish(&self, outcome: Outcome<O>) {
        self.sender.send_if_modified(|current| {
            if current.is_some() {
                return false;
            }
            *current = Some(outcome);
            true
        });
    }

    pub(super) async fn wait(&self) -> Outcome<O> {
        let mut receiver = self.sender.subscribe();
        // The sender lives in `self`, so the channel is never closed while this waits.
        match receiver.wait_for(Option::is_some).await {
            Ok(current) => current.clone().expect("the wait ended on a published outcome"),
            Err(error) => Err(Arc::new(Error::from_error(error, ErrorKind::TaskError))),
        }
    }
}

struct Running<K, O> {
    id: u64,
    /// The keys of every image the job makes, as fixed when it started.
    keys: HashSet<K>,
    done: Arc<Done<O>>,
}

struct Waiting<P, O> {
    id: u64,
    class: WorkClass,
    items: Vec<P>,
    /// The items that wait for this job and have not been dropped. A job with none is discarded.
    handles: usize,
    /// Where the job waits for its slot, once the driver has queued it. Raising or discarding the job goes through it.
    ticket: Option<RaiseHandle>,
    done: Arc<Done<O>>,
}

struct Entry<K, P, O> {
    running: Option<Running<K, O>>,
    waiting: Option<Waiting<P, O>>,
}

impl<K, P, O> Default for Entry<K, P, O> {
    fn default() -> Self {
        Self { running: None, waiting: None }
    }
}

struct Table<S, K, P, O> {
    entries: Mutex<HashMap<S, Entry<K, P, O>>>,
    next_id: AtomicU64,
}

pub(super) struct SourceJobs<S, K, P, O> {
    table: Arc<Table<S, K, P, O>>,
}

impl<S, K, P, O> Clone for SourceJobs<S, K, P, O> {
    fn clone(&self) -> Self {
        Self { table: self.table.clone() }
    }
}

/// The part of a placement that only the caller that created the job needs: what it takes to run it.
pub(super) struct Created<O> {
    pub(super) id: u64,
    /// The job decoding the source when this one was created. This job starts waiting for its slot when that one ends.
    pub(super) previous: Option<Arc<Done<O>>>,
}

pub(super) struct Placed<S, K, P, O>
where
    S: Clone + Eq + Hash,
    K: Eq + Hash,
{
    pub(super) handle: JobHandle<S, K, P, O>,
    /// `Some` when this call created the job, which the caller then has to run.
    pub(super) created: Option<Created<O>>,
}

/// An item's place in a job. Dropping it before the job starts takes the item out of the job.
pub(super) struct JobHandle<S, K, P, O>
where
    S: Clone + Eq + Hash,
    K: Eq + Hash,
{
    jobs: SourceJobs<S, K, P, O>,
    source: S,
    id: u64,
    /// Whether the item is counted in a waiting job. An item that waits for a running job is not.
    counted: bool,
    done: Arc<Done<O>>,
}

impl<S, K, P, O> JobHandle<S, K, P, O>
where
    S: Clone + Eq + Hash,
    K: Eq + Hash,
{
    /// Waits for the job to end. `Error` cannot be cloned, so each waiter gets a copy with the same kind and message.
    pub(super) async fn wait(&self) -> Result<Arc<O>> {
        self.done.wait().await.map_err(|error| {
            let message = error.message().map(str::to_string).unwrap_or_else(|| error.kind().to_string());
            Error::new(error.kind().clone()).with_message(message)
        })
    }
}

impl<S, K, P, O> Drop for JobHandle<S, K, P, O>
where
    S: Clone + Eq + Hash,
    K: Eq + Hash,
{
    fn drop(&mut self) {
        if self.counted {
            self.jobs.release(&self.source, self.id);
        }
    }
}

impl<S, K, P, O> SourceJobs<S, K, P, O>
where
    S: Clone + Eq + Hash,
    K: Eq + Hash,
{
    pub(super) fn new() -> Self {
        Self {
            table: Arc::new(Table {
                entries: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(0),
            }),
        }
    }

    /// Places an item that wants the image `wanted` of `source`. The first of the following applies.
    ///
    /// 1. The job decoding the source makes `wanted`: the item waits for it.
    /// 2. A job is waiting for its slot: the item joins it, and an Interactive item raises its class.
    /// 3. A job is decoding the source: the item starts the next job, which waits for that one to end.
    /// 4. The source has no job: the item starts one.
    pub(super) fn place(&self, source: &S, wanted: &K, item: P, class: WorkClass) -> Placed<S, K, P, O> {
        let handle = |id: u64, counted: bool, done: Arc<Done<O>>| JobHandle {
            jobs: self.clone(),
            source: source.clone(),
            id,
            counted,
            done,
        };

        let mut entries = self.table.entries.lock();
        let entry = entries.entry(source.clone()).or_default();

        if let Some(running) = &entry.running
            && running.keys.contains(wanted)
        {
            return Placed {
                handle: handle(running.id, false, running.done.clone()),
                created: None,
            };
        }

        if let Some(waiting) = &mut entry.waiting {
            waiting.items.push(item);
            waiting.handles += 1;
            if class == WorkClass::Interactive && waiting.class != WorkClass::Interactive {
                waiting.class = WorkClass::Interactive;
                // The scheduler lock is taken inside the table lock, never the other way around.
                if let Some(ticket) = &waiting.ticket {
                    ticket.raise(WorkClass::Interactive);
                }
            }
            return Placed {
                handle: handle(waiting.id, true, waiting.done.clone()),
                created: None,
            };
        }

        let id = self.table.next_id.fetch_add(1, Ordering::Relaxed);
        let done = Done::new();
        let previous = entry.running.as_ref().map(|running| running.done.clone());
        entry.waiting = Some(Waiting {
            id,
            class,
            items: vec![item],
            handles: 1,
            ticket: None,
            done: done.clone(),
        });
        Placed {
            handle: handle(id, true, done),
            created: Some(Created { id, previous }),
        }
    }

    /// The class a waiting job asks its slot with. `None` once the job was discarded or has started.
    pub(super) fn class_of(&self, source: &S, id: u64) -> Option<WorkClass> {
        let entries = self.table.entries.lock();
        let waiting = entries.get(source)?.waiting.as_ref()?;
        (waiting.id == id).then_some(waiting.class)
    }

    /// Hands the waiting job the ticket it was queued with, so that a later Interactive item raises it and the last dropped item discards it.
    /// Returns the class the job has now, which the caller raises its ticket to: an item may have raised the job before the ticket existed.
    /// `None` if the job was discarded or has started; the caller then drops its ticket.
    pub(super) fn attach(&self, source: &S, id: u64, ticket: RaiseHandle) -> Option<WorkClass> {
        let mut entries = self.table.entries.lock();
        let waiting = entries.get_mut(source)?.waiting.as_mut()?;
        if waiting.id != id {
            return None;
        }
        waiting.ticket = Some(ticket);
        Some(waiting.class)
    }

    /// Moves the waiting job to running. `plan` gets the items it holds, which are fixed from here on, and returns the keys of
    /// every image the job makes and the work to run. `None` if the job was discarded.
    pub(super) fn start<W>(&self, source: &S, id: u64, plan: impl FnOnce(&[P]) -> (HashSet<K>, W)) -> Option<W> {
        let mut entries = self.table.entries.lock();
        let entry = entries.get_mut(source)?;
        if entry.waiting.as_ref().is_none_or(|waiting| waiting.id != id) {
            return None;
        }
        let waiting = entry.waiting.take()?;
        let (keys, work) = plan(&waiting.items);
        entry.running = Some(Running { id, keys, done: waiting.done });
        Some(work)
    }

    /// Ends a job, whether it ran, was discarded, or died, and tells everything that waits for it.
    pub(super) fn finish(&self, source: &S, id: u64, outcome: Outcome<O>) {
        let done = {
            let mut entries = self.table.entries.lock();
            let Some(entry) = entries.get_mut(source) else {
                return;
            };
            let done = if entry.running.as_ref().is_some_and(|running| running.id == id) {
                entry.running.take().map(|running| running.done)
            } else if entry.waiting.as_ref().is_some_and(|waiting| waiting.id == id) {
                entry.waiting.take().map(|waiting| waiting.done)
            } else {
                None
            };
            if entry.running.is_none() && entry.waiting.is_none() {
                entries.remove(source);
            }
            done
        };
        // After the lock, so that a waiter that wakes up can place its item again.
        if let Some(done) = done {
            done.publish(outcome);
        }
    }

    /// An item of the waiting job `id` was dropped. The job is discarded with its last item.
    fn release(&self, source: &S, id: u64) {
        let mut entries = self.table.entries.lock();
        let Some(entry) = entries.get_mut(source) else {
            return;
        };
        if let Some(waiting) = &mut entry.waiting
            && waiting.id == id
        {
            waiting.handles -= 1;
            if waiting.handles == 0
                && let Some(waiting) = entry.waiting.take()
                && let Some(ticket) = &waiting.ticket
            {
                ticket.discard();
            }
        }
        if entry.running.is_none() && entry.waiting.is_none() {
            entries.remove(source);
        }
    }

    /// The number of sources that have a job. The tests use it to check that finished jobs leave nothing behind.
    #[cfg(test)]
    pub(super) fn sources(&self) -> usize {
        self.table.entries.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::core::schedule::{PoolLimits, PriorityPool, SchedulePermit, Scheduler};

    type Jobs = SourceJobs<u32, u32, u32, String>;

    fn ok(value: &str) -> Outcome<String> {
        Ok(Arc::new(value.to_string()))
    }

    fn keys(values: &[u32]) -> HashSet<u32> {
        values.iter().copied().collect()
    }

    #[tokio::test]
    async fn an_item_joins_the_job_waiting_for_its_slot_test() {
        let jobs = Jobs::new();
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let created = first.created.as_ref().expect("the first item starts the job");
        let second = jobs.place(&1, &20, 20, WorkClass::Foreground);
        assert!(second.created.is_none(), "the second item joins the waiting job");

        let items = jobs.start(&1, created.id, |items| (keys(&[10, 20]), items.to_vec()));
        assert_eq!(items, Some(vec![10, 20]));
        jobs.finish(&1, created.id, ok("done"));
        assert_eq!(*first.handle.wait().await.unwrap(), "done");
        assert_eq!(*second.handle.wait().await.unwrap(), "done");
        assert_eq!(jobs.sources(), 0, "a finished job leaves nothing behind");
    }

    #[tokio::test]
    async fn an_item_for_an_image_the_running_job_makes_waits_for_it_test() {
        let jobs = Jobs::new();
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let id = first.created.as_ref().unwrap().id;
        jobs.start(&1, id, |_| (keys(&[10, 11]), ())).unwrap();

        let second = jobs.place(&1, &11, 11, WorkClass::Foreground);
        assert!(second.created.is_none(), "11 is made by the running job");
        jobs.finish(&1, id, ok("done"));
        assert_eq!(*second.handle.wait().await.unwrap(), "done");
    }

    #[tokio::test]
    async fn an_item_for_another_image_starts_the_next_job_behind_the_running_one_test() {
        let jobs = Jobs::new();
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let id = first.created.as_ref().unwrap().id;
        jobs.start(&1, id, |_| (keys(&[10]), ())).unwrap();

        let next = jobs.place(&1, &99, 99, WorkClass::Interactive);
        let created = next.created.as_ref().expect("99 is not made by the running job, so it starts the next job");
        let previous = created.previous.clone().expect("the next job waits for the running one");
        assert!(
            tokio::time::timeout(Duration::from_millis(30), previous.wait()).await.is_err(),
            "the running job has not ended"
        );

        // An item for an image of the running job still waits for the running job, not for the next one.
        let companion = jobs.place(&1, &10, 10, WorkClass::Foreground);
        assert!(companion.created.is_none());

        jobs.finish(&1, id, ok("first"));
        assert_eq!(*previous.wait().await.unwrap(), "first");
        assert_eq!(*companion.handle.wait().await.unwrap(), "first");
        assert_eq!(jobs.class_of(&1, created.id), Some(WorkClass::Interactive));
        let items = jobs.start(&1, created.id, |items| (keys(&[99]), items.to_vec()));
        assert_eq!(items, Some(vec![99]), "only the item that made the next job is in it");
    }

    /// A pool whose only shared slot is taken, so that a ticket queued in it waits.
    async fn full_pool() -> (PriorityPool, SchedulePermit) {
        let pool = Arc::new(Scheduler::default()).pool(PoolLimits {
            shared: 1,
            interactive: 0,
            background: 1,
        });
        let held = pool.acquire(WorkClass::Foreground).await;
        (pool, held)
    }

    #[tokio::test]
    async fn an_interactive_item_raises_the_class_of_the_waiting_job_test() {
        let jobs = Jobs::new();
        let (pool, _held) = full_pool().await;
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let created = first.created.as_ref().unwrap();
        assert_eq!(jobs.class_of(&1, created.id), Some(WorkClass::Foreground));
        let ticket = pool.enqueue(WorkClass::Foreground);
        assert_eq!(jobs.attach(&1, created.id, ticket.handle()), Some(WorkClass::Foreground));

        let _second = jobs.place(&1, &20, 20, WorkClass::Interactive);
        assert_eq!(jobs.class_of(&1, created.id), Some(WorkClass::Interactive));
        assert_eq!(
            (pool.waiting(WorkClass::Interactive), pool.waiting(WorkClass::Foreground)),
            (1, 0),
            "the queued ticket is raised"
        );

        // A Foreground item does not lower the class.
        let _third = jobs.place(&1, &30, 30, WorkClass::Foreground);
        assert_eq!(jobs.class_of(&1, created.id), Some(WorkClass::Interactive));
        assert_eq!(pool.waiting(WorkClass::Interactive), 1);
    }

    #[tokio::test]
    async fn a_job_raised_before_its_ticket_is_attached_hands_the_new_class_to_the_ticket_test() {
        let jobs = Jobs::new();
        let (pool, _held) = full_pool().await;
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let created = first.created.as_ref().unwrap();
        let ticket = pool.enqueue(WorkClass::Foreground);
        let _second = jobs.place(&1, &20, 20, WorkClass::Interactive);
        assert_eq!(pool.waiting(WorkClass::Foreground), 1, "the ticket does not exist for the job yet, so nothing is raised");

        let class = jobs.attach(&1, created.id, ticket.handle()).expect("the job is still waiting");
        assert_eq!(class, WorkClass::Interactive);
        ticket.handle().raise(class);
        assert_eq!(pool.waiting(WorkClass::Interactive), 1);
    }

    #[tokio::test]
    async fn a_waiting_job_with_no_item_left_is_discarded_and_its_ticket_ends_test() {
        let jobs = Jobs::new();
        let (pool, _held) = full_pool().await;
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let id = first.created.as_ref().unwrap().id;
        let second = jobs.place(&1, &20, 20, WorkClass::Foreground);
        let ticket = pool.enqueue(WorkClass::Foreground);
        assert!(jobs.attach(&1, id, ticket.handle()).is_some());
        let waiting = tokio::spawn(ticket.acquire());

        drop(first);
        assert_eq!(jobs.class_of(&1, id), Some(WorkClass::Foreground), "one item is still waiting");
        assert_eq!(pool.waiting(WorkClass::Foreground), 1);
        drop(second);
        assert_eq!(jobs.class_of(&1, id), None);
        let error = tokio::time::timeout(Duration::from_millis(100), waiting)
            .await
            .expect("the job's ticket is discarded so that the driver ends")
            .unwrap()
            .err()
            .expect("a discarded ticket gets no slot");
        assert_eq!(error.kind(), &ErrorKind::Cancelled);
        assert_eq!(pool.waiting(WorkClass::Foreground), 0);
        assert_eq!(jobs.start(&1, id, |_| (keys(&[]), ())), None, "a discarded job does not start");
        assert_eq!(jobs.sources(), 0);
    }

    #[tokio::test]
    async fn a_job_discarded_before_its_ticket_exists_refuses_the_ticket_test() {
        let jobs = Jobs::new();
        let (pool, _held) = full_pool().await;
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let id = first.created.as_ref().unwrap().id;
        drop(first);
        assert_eq!(jobs.class_of(&1, id), None);
        let ticket = pool.enqueue(WorkClass::Foreground);
        assert_eq!(jobs.attach(&1, id, ticket.handle()), None, "the driver sees that the job is gone");
        drop(ticket);
        assert_eq!(pool.waiting(WorkClass::Foreground), 0);
        assert_eq!(jobs.sources(), 0);
    }

    #[tokio::test]
    async fn a_started_job_does_not_end_when_its_items_are_dropped_test() {
        let jobs = Jobs::new();
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let id = first.created.as_ref().unwrap().id;
        jobs.start(&1, id, |_| (keys(&[10, 11]), ())).unwrap();
        drop(first);

        let second = jobs.place(&1, &11, 11, WorkClass::Foreground);
        assert!(second.created.is_none(), "the job still runs and makes 11");
        jobs.finish(&1, id, ok("done"));
        assert_eq!(*second.handle.wait().await.unwrap(), "done");
    }

    #[tokio::test]
    async fn a_job_that_dies_before_it_starts_fails_its_items_test() {
        let jobs = Jobs::new();
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let id = first.created.as_ref().unwrap().id;
        jobs.finish(&1, id, Err(Arc::new(Error::new(ErrorKind::TaskError).with_message("died"))));
        let error = first.handle.wait().await.unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::TaskError);
        assert_eq!(jobs.sources(), 0);
    }

    #[tokio::test]
    async fn sources_do_not_share_jobs_test() {
        let jobs = Jobs::new();
        let first = jobs.place(&1, &10, 10, WorkClass::Foreground);
        let other = jobs.place(&2, &10, 10, WorkClass::Foreground);
        assert!(first.created.is_some() && other.created.is_some());
        assert_eq!(jobs.sources(), 2);
    }

    #[tokio::test]
    async fn the_first_outcome_is_kept_test() {
        let done = Done::<String>::new();
        done.publish(ok("first"));
        done.publish(ok("second"));
        assert_eq!(*done.wait().await.unwrap(), "first");
    }
}
