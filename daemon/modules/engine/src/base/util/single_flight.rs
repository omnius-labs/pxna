use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use futures::{
    FutureExt as _,
    future::{BoxFuture, Shared},
};

use crate::prelude::*;

type SharedResult<V> = std::result::Result<Arc<V>, Arc<Error>>;

struct Flight<V> {
    id: u64,
    shared: Shared<BoxFuture<'static, SharedResult<V>>>,
}

impl<V> Clone for Flight<V> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            shared: self.shared.clone(),
        }
    }
}

/// 同じ key への同時要求を 1 回の処理に畳む。
///
/// 処理は独立した task で走らせるため、最初の要求者が cancel されても待機者は結果を受け取れる。
/// 結果は保持せず、処理の完了後に来た要求は新しい処理を始める。
/// 完了した処理は、待機者がすべて cancel されていても、task 自身が一覧から外す。
pub struct SingleFlight<K, V> {
    flights: Arc<Mutex<HashMap<K, Flight<V>>>>,
    next_id: AtomicU64,
}

/// task が終わったときに、自分の flight を一覧から外す。panic で終わったときも外す。
struct FlightCleanup<K: Eq + Hash, V> {
    flights: Arc<Mutex<HashMap<K, Flight<V>>>>,
    key: K,
    id: u64,
}

impl<K: Eq + Hash, V> Drop for FlightCleanup<K, V> {
    fn drop(&mut self) {
        let mut flights = self.flights.lock();
        if flights.get(&self.key).is_some_and(|flight| flight.id == self.id) {
            flights.remove(&self.key);
        }
    }
}

impl<K, V> SingleFlight<K, V>
where
    K: Clone + Eq + Hash + Send + 'static,
    V: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            flights: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(0),
        }
    }

    /// 実行中の処理があればその完了を待ち、なければ `work` を始める。
    pub async fn run<F>(&self, key: K, work: F) -> Result<Arc<V>>
    where
        F: Future<Output = Result<V>> + Send + 'static,
    {
        self.run_gated(key, async { Ok(()) }, move |()| work).await
    }

    /// 実行中の処理があればその完了を待ち、なければ `gate` を待ってから `work` を始める。
    ///
    /// `gate` を待つ間にこの呼び出しが drop されたら、何も始まらない。
    /// `gate` を待つ間に同じ key の処理が始まっていたら、`gate` の結果を捨ててその処理を待つ。
    /// 同時実行数の上限を待つ要求が、切断された後に処理を始めてしまうのを防ぐために使う。
    pub async fn run_gated<G, P, F, Fut>(&self, key: K, gate: G, work: F) -> Result<Arc<V>>
    where
        G: Future<Output = Result<P>>,
        P: Send + 'static,
        F: FnOnce(P) -> Fut,
        Fut: Future<Output = Result<V>> + Send + 'static,
    {
        let running = self.flights.lock().get(&key).map(|flight| flight.shared.clone());
        let shared = match running {
            Some(shared) => shared,
            None => {
                let permit = gate.await?;
                let mut flights = self.flights.lock();
                match flights.get(&key) {
                    Some(flight) => flight.shared.clone(),
                    None => {
                        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                        let cleanup = FlightCleanup {
                            flights: self.flights.clone(),
                            key: key.clone(),
                            id,
                        };
                        let work = work(permit);
                        // task は `flights` の lock を取って終わるため、lock を持ったまま起動して登録を済ませる。
                        let handle = tokio::spawn(async move {
                            let _cleanup = cleanup;
                            work.await
                        });
                        let shared = async move {
                            match handle.await {
                                Ok(Ok(value)) => Ok(Arc::new(value)),
                                Ok(Err(error)) => Err(Arc::new(error)),
                                Err(error) => Err(Arc::new(Error::from_error(error, ErrorKind::TaskError))),
                            }
                        }
                        .boxed()
                        .shared();
                        flights.insert(key, Flight { id, shared: shared.clone() });
                        shared
                    }
                }
            }
        };

        shared.await.map_err(|error| Self::clone_error(&error))
    }

    /// 実行中の flight の数。完了した flight が残らないことを、テストが確かめるために使う。
    #[cfg(test)]
    pub(crate) fn in_flight(&self) -> usize {
        self.flights.lock().len()
    }

    /// `Error` は clone できないため、待機者ごとに kind と message を写した値を作る。
    fn clone_error(error: &Error) -> Error {
        let message = error.message().map(str::to_string).unwrap_or_else(|| error.kind().to_string());
        Error::new(error.kind().clone()).with_message(message)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use super::*;

    #[tokio::test]
    async fn concurrent_requests_share_one_run_test() {
        let flights = Arc::new(SingleFlight::<u32, u32>::new());
        let runs = Arc::new(AtomicUsize::new(0));

        let mut tasks = Vec::new();
        for _ in 0..4 {
            let flights = flights.clone();
            let runs = runs.clone();
            tasks.push(tokio::spawn(async move {
                flights
                    .run(1, async move {
                        runs.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        Ok(7)
                    })
                    .await
            }));
        }
        for task in tasks {
            assert_eq!(*task.await.unwrap().unwrap(), 7);
        }
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        // 完了後の要求は新しく走る。
        let runs_after = runs.clone();
        flights
            .run(1, async move {
                runs_after.fetch_add(1, Ordering::SeqCst);
                Ok(8)
            })
            .await
            .unwrap();
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn waiters_receive_error_kind_test() {
        let flights = SingleFlight::<u32, u32>::new();
        let error = flights
            .run(1, async { Err(Error::new(ErrorKind::InvalidFormat).with_message("broken")) })
            .await
            .unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::InvalidFormat);
        assert_eq!(error.message(), Some("broken"));
    }

    #[tokio::test]
    async fn dropped_request_waiting_for_the_gate_starts_nothing_test() {
        let flights = Arc::new(SingleFlight::<u32, u32>::new());
        let runs = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(tokio::sync::Semaphore::new(0));

        let task = {
            let (flights, runs, gate) = (flights.clone(), runs.clone(), gate.clone());
            tokio::spawn(async move {
                let acquire = async move { gate.acquire_owned().await.map_err(|error| Error::from_error(error, ErrorKind::TaskError)) };
                flights
                    .run_gated(1, acquire, move |_permit| async move {
                        runs.fetch_add(1, Ordering::SeqCst);
                        Ok(1)
                    })
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        task.abort();
        let _ = task.await;

        gate.add_permits(1);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(runs.load(Ordering::SeqCst), 0, "gate を待つ間に drop された要求は処理を始めない");
    }

    #[tokio::test]
    async fn request_joins_a_flight_started_while_waiting_for_the_gate_test() {
        let flights = Arc::new(SingleFlight::<u32, u32>::new());
        let runs = Arc::new(AtomicUsize::new(0));
        let (release, released) = tokio::sync::oneshot::channel::<()>();

        // 先に始まった処理が、解放されるまで終わらない。
        let first = {
            let (flights, runs) = (flights.clone(), runs.clone());
            tokio::spawn(async move {
                flights
                    .run(1, async move {
                        runs.fetch_add(1, Ordering::SeqCst);
                        let _ = released.await;
                        Ok(7)
                    })
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;

        let second = {
            let (flights, runs) = (flights.clone(), runs.clone());
            tokio::spawn(async move {
                flights
                    .run_gated(1, async { Ok(()) }, move |()| async move {
                        runs.fetch_add(1, Ordering::SeqCst);
                        Ok(8)
                    })
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        release.send(()).unwrap();

        assert_eq!(*first.await.unwrap().unwrap(), 7);
        assert_eq!(*second.await.unwrap().unwrap(), 7, "実行中の処理があれば、その結果を待つ");
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn completed_flight_is_removed_even_if_every_waiter_was_dropped_test() {
        let flights = Arc::new(SingleFlight::<u32, u32>::new());
        let (release, released) = tokio::sync::oneshot::channel::<()>();

        let task = {
            let flights = flights.clone();
            tokio::spawn(async move {
                flights
                    .run(1, async move {
                        let _ = released.await;
                        Ok(7)
                    })
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(flights.in_flight(), 1);

        // 唯一の待機者が cancel された後に処理が終わっても、結果を保持した flight が残らない。
        task.abort();
        let _ = task.await;
        release.send(()).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(flights.in_flight(), 0);
    }

    #[tokio::test]
    async fn panicking_flight_is_removed_test() {
        let flights = SingleFlight::<u32, u32>::new();
        let error = flights
            .run(1, async {
                let zero = 0;
                if zero == 0 {
                    panic!("boom");
                }
                Ok(1)
            })
            .await
            .unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::TaskError);
        assert_eq!(flights.in_flight(), 0);
    }
}
