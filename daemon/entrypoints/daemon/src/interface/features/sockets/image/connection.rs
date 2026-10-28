//! One connection of the image socket: its wanted set, the items it passes to generation, and the images it sends (Image Socket Design §4 to §7).
use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use bytes::Bytes;
use futures::{SinkExt as _, StreamExt as _};
use tokio::{
    sync::mpsc,
    task::{AbortHandle, JoinSet},
    time::MissedTickBehavior,
};
use tracing::{Instrument as _, field::Empty};

use omnius_pxna_engine::{
    model::{ImageAnchorPositionType, ImageFormatType, ImageResizeType},
    perf_span,
    service::{Thumbnail, ThumbnailRequest, ThumbnailVariant, WorkClass},
};

use super::message::{ClientMessage, FrameCodec, FrameHeader, ItemKind, RequestClass, ServerMessage, Shape, WantedItem};
use crate::{
    error::ApiError,
    interface::session::SessionAuth,
    prelude::*,
    service::sockets::image::{ImageSocketLimits, SocketSlot},
    state::DaemonState,
};

/// Close codes the client acts on (Image Socket Design §3).
struct CloseCode;

impl CloseCode {
    /// The daemon is shutting down.
    const GOING_AWAY: u16 = 1001;
    /// The daemon could not go on, such as a client that stopped answering pings.
    const UNEXPECTED: u16 = 1011;
    /// The client broke the contract or a limit. The client does not reconnect.
    const BROKE_CONTRACT: u16 = 4400;
    /// The session ended. The client returns to login.
    const SESSION_ENDED: u16 = 4401;
}

/// Why a connection ends by the daemon's decision: the close frame it sends.
struct Closing {
    code: u16,
    reason: &'static str,
}

impl Closing {
    fn new(code: u16, reason: &'static str) -> Self {
        Self { code, reason }
    }

    fn contract(reason: &'static str) -> Self {
        Self::new(CloseCode::BROKE_CONTRACT, reason)
    }
}

/// What a task passed to the connection when it ended.
enum Outcome {
    /// The held image is still the current one.
    Unchanged,
    /// The held image is not the current one, or the item holds none: the item goes to generation.
    Changed,
    Image(Thumbnail),
    Failed(Error),
}

struct Finished {
    id: u64,
    /// Which time the item was wanted. A result for an item that was dropped and wanted again under the same id is not for the new item.
    generation: u64,
    outcome: Outcome,
}

enum Stage {
    /// Not passed to a task.
    Waiting,
    /// Compared with the `ETag` of the image the client holds.
    Checking(AbortHandle),
    /// Passed to generation. `interactive` tells which of the two budgets of generations it counts in.
    Generating { abort: AbortHandle, interactive: bool },
    /// Its image waits in the send queue.
    Sending,
}

struct Item {
    wanted: WantedItem,
    request: ThumbnailRequest,
    /// Set when the item was wanted. Items are served in the order of class, order, and this.
    generation: u64,
    /// Whether the `ETag` of the item has been compared. An item with no `ETag` has nothing to compare.
    checked: bool,
    stage: Stage,
}

impl Item {
    fn rank(&self) -> (RequestClass, f64, u64) {
        (self.wanted.class, self.wanted.order, self.generation)
    }
}

/// A finished image that is being sent in chunks.
struct Outgoing {
    id: u64,
    class: RequestClass,
    order: f64,
    generation: u64,
    etag: Option<String>,
    content_type: &'static str,
    frame_count: Option<u32>,
    bytes: Bytes,
    offset: usize,
}

pub(super) struct Connection {
    state: Arc<DaemonState>,
    /// The session's token, checked again at each ping.
    credential: String,
    _slot: SocketSlot,
}

/// The state of one connection while it runs.
struct Run {
    state: Arc<DaemonState>,
    limits: ImageSocketLimits,
    items: HashMap<u64, Item>,
    next_generation: u64,
    tasks: JoinSet<Finished>,
    /// The item each task works for, by the task's id.
    task_items: HashMap<tokio::task::Id, (u64, u64)>,
    checking: usize,
    /// The items in generation, apart from the interactive ones, which have a budget of their own.
    generating: usize,
    generating_interactive: usize,
    /// Text messages to send before the next chunk.
    control: std::collections::VecDeque<Message>,
    outgoing: Vec<Outgoing>,
    /// The bytes of the images in `outgoing` that are not sent yet.
    unsent: usize,
    /// How many pings in a row got no answer.
    missed_pongs: u32,
}

impl Connection {
    pub(super) fn new(state: Arc<DaemonState>, credential: String, slot: SocketSlot) -> Self {
        Self { state, credential, _slot: slot }
    }

    pub(super) async fn run(self, socket: WebSocket) {
        let limits = self.state.image_sockets.limits().clone();
        let closing = self.state.image_sockets.closing().clone();
        let (mut sink, mut stream) = socket.split();

        // Only the writer waits for the socket. The connection hands it one message at a time, so that it chooses the next chunk
        // when the socket takes it, not when the chunk is queued.
        let (sender, mut receiver) = mpsc::channel::<Message>(1);
        let writer = tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                let last = matches!(message, Message::Close(_));
                if sink.send(message).await.is_err() || last {
                    break;
                }
            }
        });

        let mut run = Run::new(self.state.clone(), limits.clone());
        let mut ping = tokio::time::interval_at(tokio::time::Instant::now() + limits.ping_interval, limits.ping_interval);
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let ending: Option<Closing> = loop {
            run.dispatch();
            let can_send = run.can_send();
            tokio::select! {
                biased;
                () = closing.cancelled() => break Some(Closing::new(CloseCode::GOING_AWAY, "the daemon is shutting down")),
                received = stream.next() => match received {
                    None | Some(Err(_)) => break None,
                    Some(Ok(message)) => match run.receive(message) {
                        Ok(true) => {}
                        Ok(false) => break None,
                        Err(closing) => break Some(closing),
                    },
                },
                Some(joined) = run.tasks.join_next_with_id(), if !run.tasks.is_empty() => run.joined(joined),
                permit = sender.reserve(), if can_send => match permit {
                    Ok(permit) => {
                        if let Some(message) = run.next_message() {
                            permit.send(message);
                        }
                    }
                    Err(_) => break None,
                },
                _ = ping.tick() => {
                    if let Some(closing) = run.ping(&self.credential).await {
                        break Some(closing);
                    }
                }
            }
        };

        if let Some(closing) = ending {
            let frame = CloseFrame {
                code: closing.code,
                reason: closing.reason.into(),
            };
            // A writer that does not take the close frame is stuck on a socket nobody reads.
            let sent = tokio::time::timeout(Duration::from_secs(2), sender.send(Message::Close(Some(frame)))).await;
            if matches!(sent, Ok(Ok(()))) {
                // Wait for the client's close, which also reads what it sent meanwhile. A socket closed with unread data resets the
                // connection, and the reset can discard the close frame before the client reads it.
                let _ = tokio::time::timeout(Duration::from_secs(2), async {
                    while let Some(Ok(message)) = stream.next().await {
                        if matches!(message, Message::Close(_)) {
                            break;
                        }
                    }
                })
                .await;
            }
        }
        drop(sender);
        let _ = tokio::time::timeout(Duration::from_secs(2), writer).await;
        // The tasks of the items are dropped with `run`: an item that is not generating yet starts nothing, and a job that started runs to the end.
    }
}

impl Run {
    fn new(state: Arc<DaemonState>, limits: ImageSocketLimits) -> Self {
        Self {
            state,
            limits,
            items: HashMap::new(),
            next_generation: 0,
            tasks: JoinSet::new(),
            task_items: HashMap::new(),
            checking: 0,
            generating: 0,
            generating_interactive: 0,
            control: std::collections::VecDeque::new(),
            outgoing: Vec::new(),
            unsent: 0,
            missed_pongs: 0,
        }
    }

    /// Handles a message from the client. `Ok(false)` ends the connection without a close frame of ours.
    fn receive(&mut self, message: Message) -> std::result::Result<bool, Closing> {
        self.missed_pongs = 0;
        match message {
            Message::Text(text) => {
                if text.len() > self.limits.max_text_bytes {
                    return Err(Closing::contract("a text message is too large"));
                }
                let message: ClientMessage = serde_json::from_str(text.as_str()).map_err(|_| Closing::contract("a message is not in the contract"))?;
                match message {
                    ClientMessage::Want { items } => self.want(items)?,
                    ClientMessage::Drop { ids } => {
                        for id in ids {
                            self.drop_item(id);
                        }
                    }
                }
                Ok(true)
            }
            Message::Binary(_) => Err(Closing::contract("the client sends text messages only")),
            Message::Ping(_) | Message::Pong(_) => Ok(true),
            Message::Close(_) => Ok(false),
        }
    }

    fn want(&mut self, items: Vec<WantedItem>) -> std::result::Result<(), Closing> {
        let mut new_items = 0;
        for wanted in &items {
            wanted.validate().map_err(|_| Closing::contract("an item is not in the contract"))?;
            match self.items.get(&wanted.id) {
                Some(existing) if !existing.wanted.asks_for_the_same_image(wanted) => return Err(Closing::contract("an id was reused for another image")),
                Some(_) => {}
                None => new_items += 1,
            }
        }
        if self.items.len() + new_items > self.limits.max_items {
            return Err(Closing::contract("too many wanted items"));
        }

        for wanted in items {
            if let Some(item) = self.items.get_mut(&wanted.id) {
                // The same image again: only the class, the order, and the `ETag` of the held image may change.
                if item.wanted.etag != wanted.etag && matches!(item.stage, Stage::Waiting) {
                    item.checked = wanted.etag.is_none();
                }
                item.wanted.class = wanted.class;
                item.wanted.order = wanted.order;
                item.wanted.etag = wanted.etag;
                let (id, class, order) = (item.wanted.id, item.wanted.class, item.wanted.order);
                if let Some(outgoing) = self.outgoing.iter_mut().find(|outgoing| outgoing.id == id) {
                    outgoing.class = class;
                    outgoing.order = order;
                }
                continue;
            }
            let request = Self::request(&wanted, self.state.conf.thumbnail.format);
            let generation = self.next_generation;
            self.next_generation += 1;
            let checked = wanted.etag.is_none();
            self.items.insert(
                wanted.id,
                Item {
                    wanted,
                    request,
                    generation,
                    checked,
                    stage: Stage::Waiting,
                },
            );
        }
        Ok(())
    }

    fn drop_item(&mut self, id: u64) {
        let Some(item) = self.items.remove(&id) else {
            return;
        };
        match item.stage {
            Stage::Checking(abort) => {
                abort.abort();
                self.checking -= 1;
            }
            Stage::Generating { abort, interactive } => {
                abort.abort();
                self.end_generation(interactive);
            }
            Stage::Sending => {
                if let Some(index) = self.outgoing.iter().position(|outgoing| outgoing.id == id) {
                    let outgoing = self.outgoing.remove(index);
                    self.unsent -= outgoing.bytes.len() - outgoing.offset;
                }
            }
            Stage::Waiting => {}
        }
    }

    /// The thumbnail request of an item. The format is the daemon's configuration, and a display image always fits.
    fn request(wanted: &WantedItem, format: ImageFormatType) -> ThumbnailRequest {
        let (variant, default_side) = match wanted.kind {
            ItemKind::Thumbnail => (ThumbnailVariant::Thumbnail, Some(256)),
            ItemKind::Sprite => (ThumbnailVariant::Sprite, Some(256)),
            ItemKind::Display => (ThumbnailVariant::Display, None),
        };
        ThumbnailRequest {
            variant,
            width: wanted.width.or(default_side),
            height: wanted.height.or(default_side),
            format,
            resize: match wanted.shape {
                Some(Shape::Fit) => ImageResizeType::Fit,
                _ => ImageResizeType::Crop,
            },
            anchor: ImageAnchorPositionType::Center,
            class: match wanted.class {
                RequestClass::Interactive => WorkClass::Interactive,
                RequestClass::Visible | RequestClass::Prefetch => WorkClass::Foreground,
            },
        }
    }

    /// Starts what can start: the comparisons of held `ETag`s, then the generations, each in the order of class, order, and arrival.
    fn dispatch(&mut self) {
        while self.checking < self.limits.max_checking {
            let Some(id) = self.best(|item| matches!(item.stage, Stage::Waiting) && !item.checked) else {
                break;
            };
            self.spawn_check(id);
        }
        // The image the user waits for does not wait behind the items that are only expected: it has a budget of its own.
        while self.generating_interactive < self.limits.max_generating {
            let Some(id) = self.best(|item| matches!(item.stage, Stage::Waiting) && item.checked && item.wanted.class == RequestClass::Interactive) else {
                break;
            };
            self.spawn_generation(id);
        }
        while self.generating < self.limits.max_generating && self.unsent < self.limits.max_unsent_bytes {
            let Some(id) = self.best(|item| matches!(item.stage, Stage::Waiting) && item.checked && item.wanted.class != RequestClass::Interactive) else {
                break;
            };
            self.spawn_generation(id);
        }
    }

    fn end_generation(&mut self, interactive: bool) {
        if interactive {
            self.generating_interactive -= 1;
        } else {
            self.generating -= 1;
        }
    }

    fn best(&self, wanted: impl Fn(&Item) -> bool) -> Option<u64> {
        self.items
            .values()
            .filter(|item| wanted(item))
            .min_by(|a, b| {
                let ((class_a, order_a, arrival_a), (class_b, order_b, arrival_b)) = (a.rank(), b.rank());
                class_a.cmp(&class_b).then(order_a.total_cmp(&order_b)).then(arrival_a.cmp(&arrival_b))
            })
            .map(|item| item.wanted.id)
    }

    fn spawn_check(&mut self, id: u64) {
        let Some(item) = self.items.get_mut(&id) else {
            return;
        };
        let (wanted, request, generation, state) = (item.wanted.clone(), item.request.clone(), item.generation, self.state.clone());
        let handle = self.tasks.spawn(async move {
            let outcome = async {
                let source = state.library.thumbnail_source(&wanted.library_id, &wanted.path).await?;
                let key = state.thumbnails.current_key(source, &request).await?;
                Ok::<_, Error>(match (key, &wanted.etag) {
                    (Some(key), Some(etag)) if &key == etag => Outcome::Unchanged,
                    _ => Outcome::Changed,
                })
            }
            .await
            .unwrap_or_else(Outcome::Failed);
            Finished { id, generation, outcome }
        });
        self.task_items.insert(handle.id(), (id, generation));
        item.stage = Stage::Checking(handle);
        self.checking += 1;
    }

    fn spawn_generation(&mut self, id: u64) {
        let Some(item) = self.items.get_mut(&id) else {
            return;
        };
        let (wanted, request, generation, state) = (item.wanted.clone(), item.request.clone(), item.generation, self.state.clone());
        let handle = self.tasks.spawn(async move {
            let span = perf_span!("thumbnail_request", library_id = &wanted.library_id, path = &wanted.path, variant = ?request.variant, ok = Empty, bytes = Empty);
            let result = async {
                let source = state
                    .library
                    .thumbnail_source(&wanted.library_id, &wanted.path)
                    .instrument(perf_span!("thumbnail_resolve"))
                    .await?;
                Ok::<_, Error>(state.thumbnails.generate_resolved(source, &request).await?)
            }
            .instrument(span.clone())
            .await;
            span.record("ok", result.is_ok())
                .record("bytes", result.as_ref().map_or(0, |thumbnail| thumbnail.bytes.len()));
            drop(span);
            Finished {
                id,
                generation,
                outcome: match result {
                    Ok(thumbnail) => Outcome::Image(thumbnail),
                    Err(error) => Outcome::Failed(error),
                },
            }
        });
        self.task_items.insert(handle.id(), (id, generation));
        let interactive = item.wanted.class == RequestClass::Interactive;
        item.stage = Stage::Generating { abort: handle, interactive };
        if interactive {
            self.generating_interactive += 1;
        } else {
            self.generating += 1;
        }
    }

    /// Takes in the end of a task. A task that was aborted has nothing to report: its item was dropped.
    fn joined(&mut self, joined: std::result::Result<(tokio::task::Id, Finished), tokio::task::JoinError>) {
        match joined {
            Ok((task, finished)) => {
                self.task_items.remove(&task);
                self.finished(finished);
            }
            Err(error) => {
                let Some((id, generation)) = self.task_items.remove(&error.id()) else {
                    return;
                };
                if error.is_panic() {
                    error!(error = ?error, "a task of the image socket panicked");
                    let failure = Error::new(ErrorKind::UnexpectedError).with_message("the image could not be made");
                    self.finished(Finished {
                        id,
                        generation,
                        outcome: Outcome::Failed(failure),
                    });
                }
            }
        }
    }

    fn finished(&mut self, finished: Finished) {
        let Finished { id, generation, outcome } = finished;
        let Some(item) = self.items.get_mut(&id) else {
            return;
        };
        if item.generation != generation {
            return;
        }
        match item.stage {
            Stage::Checking(_) => self.checking -= 1,
            Stage::Generating { interactive, .. } => {
                if interactive {
                    self.generating_interactive -= 1;
                } else {
                    self.generating -= 1;
                }
            }
            Stage::Waiting | Stage::Sending => return,
        }

        match outcome {
            Outcome::Unchanged => {
                self.items.remove(&id);
                self.control_message(&ServerMessage::Unchanged { id });
            }
            Outcome::Changed => {
                item.checked = true;
                item.stage = Stage::Waiting;
            }
            Outcome::Failed(error) => {
                self.items.remove(&id);
                let body = ApiError::from_error(&error).body;
                self.control_message(&ServerMessage::Failed {
                    id,
                    kind: body.kind,
                    message: body.message,
                });
            }
            Outcome::Image(thumbnail) => {
                item.stage = Stage::Sending;
                let content_type = match thumbnail.format {
                    ImageFormatType::Webp => "image/webp",
                    ImageFormatType::Png => "image/png",
                    _ => "image/jpeg",
                };
                self.unsent += thumbnail.bytes.len();
                self.outgoing.push(Outgoing {
                    id,
                    class: item.wanted.class,
                    order: item.wanted.order,
                    generation,
                    etag: thumbnail.key,
                    content_type,
                    frame_count: thumbnail.frame_count,
                    bytes: thumbnail.bytes,
                    offset: 0,
                });
            }
        }
    }

    fn control_message(&mut self, message: &ServerMessage) {
        match serde_json::to_string(message) {
            Ok(text) => self.control.push_back(Message::Text(text.into())),
            Err(error) => error!(error = ?error, "failed to serialize a message of the image socket"),
        }
    }

    fn can_send(&self) -> bool {
        !self.control.is_empty() || !self.outgoing.is_empty()
    }

    /// The next message for the socket: a text message first, then the next chunk of the image that is served first.
    fn next_message(&mut self) -> Option<Message> {
        if let Some(message) = self.control.pop_front() {
            return Some(message);
        }
        let index = self
            .outgoing
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.class.cmp(&b.class).then(a.order.total_cmp(&b.order)).then(a.generation.cmp(&b.generation)))
            .map(|(index, _)| index)?;
        let outgoing = &mut self.outgoing[index];
        let end = (outgoing.offset + self.limits.chunk_bytes).min(outgoing.bytes.len());
        let header = FrameHeader {
            id: outgoing.id,
            etag: outgoing.etag.clone(),
            content_type: outgoing.content_type.to_string(),
            frame_count: outgoing.frame_count,
            offset: outgoing.offset as u64,
            total: outgoing.bytes.len() as u64,
        };
        let frame = FrameCodec::encode(&header, &outgoing.bytes[outgoing.offset..end]);
        self.unsent -= end - outgoing.offset;
        outgoing.offset = end;
        let (id, finished) = (outgoing.id, end == outgoing.bytes.len());
        if finished {
            // The image is sent, so the item leaves the wanted set.
            self.outgoing.remove(index);
            self.items.remove(&id);
        }
        match frame {
            Ok(frame) => Some(Message::Binary(frame.into())),
            Err(error) => {
                error!(error = ?error, "failed to write a frame of the image socket");
                None
            }
        }
    }

    /// At each ping: the session must still hold, and the client must have answered the pings before.
    async fn ping(&mut self, credential: &str) -> Option<Closing> {
        if SessionAuth::authenticate_token(&self.state, credential).await.is_none() {
            return Some(Closing::new(CloseCode::SESSION_ENDED, "the session ended"));
        }
        if self.missed_pongs >= 2 {
            return Some(Closing::new(CloseCode::UNEXPECTED, "the client does not answer pings"));
        }
        self.missed_pongs += 1;
        self.control.push_back(Message::Ping(Bytes::new()));
        None
    }
}

#[cfg(test)]
mod tests {
    use omnius_pxna_engine::service::ArchiveOptions;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::fixture;

    fn item(id: u64, class: RequestClass, order: f64) -> WantedItem {
        WantedItem {
            id,
            library_id: "photos".to_string(),
            path: format!("/photos/{id}.png"),
            kind: ItemKind::Thumbnail,
            width: Some(256),
            height: Some(256),
            shape: None,
            class,
            order,
            etag: None,
        }
    }

    fn image(len: usize) -> Thumbnail {
        Thumbnail {
            bytes: Bytes::from(vec![7; len]),
            format: ImageFormatType::Webp,
            frame_count: Some(8),
            key: Some("a".repeat(64)),
        }
    }

    async fn run_with(limits: ImageSocketLimits) -> TestResult<Run> {
        let fixture = fixture(ArchiveOptions::default()).await?;
        Ok(Run::new(fixture.server.state.clone(), limits))
    }

    /// Puts the item where a generation that has not ended would leave it, without starting one.
    fn pretend_generating(run: &mut Run, id: u64) {
        let abort = run.tasks.spawn(std::future::pending());
        let item = run.items.get_mut(&id).expect("the item is wanted");
        let interactive = item.wanted.class == RequestClass::Interactive;
        item.stage = Stage::Generating { abort, interactive };
        if interactive {
            run.generating_interactive += 1;
        } else {
            run.generating += 1;
        }
    }

    fn finish(run: &mut Run, id: u64, thumbnail: Thumbnail) {
        pretend_generating(run, id);
        let generation = run.items[&id].generation;
        run.finished(Finished {
            id,
            generation,
            outcome: Outcome::Image(thumbnail),
        });
    }

    /// The `(id, offset, length)` of the next binary frame.
    fn next_frame(run: &mut Run) -> Option<(u64, u64, usize)> {
        let Some(Message::Binary(frame)) = run.next_message() else {
            return None;
        };
        let (header, chunk) = FrameCodec::decode(&frame).expect("a frame the connection wrote is readable");
        Some((header.id, header.offset, chunk.len()))
    }

    fn stages(run: &Run, ids: &[u64]) -> Vec<&'static str> {
        ids.iter()
            .map(|id| match run.items[id].stage {
                Stage::Waiting => "waiting",
                Stage::Checking(_) => "checking",
                Stage::Generating { .. } => "generating",
                Stage::Sending => "sending",
            })
            .collect()
    }

    #[tokio::test]
    async fn an_image_leaves_in_chunks_and_a_more_urgent_image_goes_between_them_test() -> TestResult {
        let mut run = run_with(ImageSocketLimits {
            chunk_bytes: 4,
            ..ImageSocketLimits::default()
        })
        .await?;
        run.want(vec![item(1, RequestClass::Prefetch, 0.0), item(2, RequestClass::Visible, 0.0)])
            .map_err(|closing| closing.reason)?;

        finish(&mut run, 1, image(10));
        assert_eq!(next_frame(&mut run), Some((1, 0, 4)));
        // A visible image finishes while the prefetched one is half sent: its chunks go first.
        finish(&mut run, 2, image(6));
        assert_eq!(next_frame(&mut run), Some((2, 0, 4)));
        assert_eq!(next_frame(&mut run), Some((2, 4, 2)));
        assert!(!run.items.contains_key(&2), "a sent image leaves the wanted set");
        assert_eq!(next_frame(&mut run), Some((1, 4, 4)));
        assert_eq!(next_frame(&mut run), Some((1, 8, 2)));
        assert!(run.items.is_empty());
        assert_eq!((run.unsent, run.can_send()), (0, false));
        Ok(())
    }

    #[tokio::test]
    async fn the_order_within_a_class_and_the_arrival_decide_between_equal_images_test() -> TestResult {
        let mut run = run_with(ImageSocketLimits::default()).await?;
        run.want(vec![
            item(1, RequestClass::Visible, 5.0),
            item(2, RequestClass::Visible, 1.0),
            item(3, RequestClass::Visible, 1.0),
        ])
        .map_err(|closing| closing.reason)?;
        for id in [1, 2, 3] {
            finish(&mut run, id, image(2));
        }
        let order: Vec<u64> = std::iter::from_fn(|| next_frame(&mut run)).map(|(id, ..)| id).collect();
        assert_eq!(order, [2, 3, 1], "smaller order first, and the earlier of equal orders");
        Ok(())
    }

    #[tokio::test]
    async fn the_header_carries_the_etag_content_type_and_frame_count_test() -> TestResult {
        let mut run = run_with(ImageSocketLimits::default()).await?;
        run.want(vec![item(1, RequestClass::Visible, 0.0)]).map_err(|closing| closing.reason)?;
        finish(&mut run, 1, image(3));
        let Some(Message::Binary(frame)) = run.next_message() else {
            panic!("a frame is ready");
        };
        let (header, chunk) = FrameCodec::decode(&frame)?;
        assert_eq!(
            header,
            FrameHeader {
                id: 1,
                etag: Some("a".repeat(64)),
                content_type: "image/webp".to_string(),
                frame_count: Some(8),
                offset: 0,
                total: 3,
            }
        );
        assert_eq!(chunk, [7, 7, 7]);
        Ok(())
    }

    #[tokio::test]
    async fn items_go_to_generation_by_class_and_order_within_their_budgets_test() -> TestResult {
        let mut run = run_with(ImageSocketLimits {
            max_generating: 2,
            ..ImageSocketLimits::default()
        })
        .await?;
        run.want(vec![
            item(1, RequestClass::Prefetch, 0.0),
            item(2, RequestClass::Visible, 9.0),
            item(3, RequestClass::Visible, 3.0),
            item(4, RequestClass::Visible, 3.0),
            item(5, RequestClass::Interactive, 0.0),
            item(6, RequestClass::Interactive, 1.0),
            item(7, RequestClass::Interactive, 2.0),
        ])
        .map_err(|closing| closing.reason)?;

        run.dispatch();
        // Two of the items that are only expected go first by class and order, and the interactive ones have a budget of their own.
        assert_eq!(stages(&run, &[3, 4, 2, 1]), ["generating", "generating", "waiting", "waiting"]);
        assert_eq!(stages(&run, &[5, 6, 7]), ["generating", "generating", "waiting"]);
        assert_eq!((run.generating, run.generating_interactive), (2, 2));
        Ok(())
    }

    #[tokio::test]
    async fn images_that_wait_to_be_sent_hold_back_new_generations_test() -> TestResult {
        let mut run = run_with(ImageSocketLimits {
            max_unsent_bytes: 10,
            chunk_bytes: 8,
            ..ImageSocketLimits::default()
        })
        .await?;
        run.want(vec![item(1, RequestClass::Visible, 0.0), item(2, RequestClass::Visible, 1.0)])
            .map_err(|closing| closing.reason)?;

        finish(&mut run, 1, image(20));
        run.dispatch();
        assert_eq!(stages(&run, &[2]), ["waiting"], "20 unsent bytes are more than the 10 allowed");

        // Sent down to the allowance, the next item may go.
        assert_eq!(next_frame(&mut run), Some((1, 0, 8)));
        run.dispatch();
        assert_eq!(stages(&run, &[2]), ["waiting"], "12 bytes are still unsent");
        assert_eq!(next_frame(&mut run), Some((1, 8, 8)));
        run.dispatch();
        assert_eq!(stages(&run, &[2]), ["generating"]);
        Ok(())
    }

    #[tokio::test]
    async fn dropping_an_item_takes_it_out_wherever_it_is_test() -> TestResult {
        let mut run = run_with(ImageSocketLimits::default()).await?;
        run.want(vec![
            item(1, RequestClass::Visible, 0.0),
            item(2, RequestClass::Visible, 1.0),
            item(3, RequestClass::Visible, 2.0),
        ])
        .map_err(|closing| closing.reason)?;
        pretend_generating(&mut run, 1);
        finish(&mut run, 2, image(12));
        assert_eq!((run.generating, run.unsent), (1, 12));

        run.drop_item(1);
        assert_eq!(run.generating, 0, "an item in generation gives its place back");
        run.drop_item(2);
        assert_eq!((run.unsent, run.can_send()), (0, false), "an image that was not sent leaves the send queue");
        run.drop_item(3);
        run.drop_item(99);
        assert!(run.items.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn a_want_is_checked_against_the_contract_and_the_limits_test() -> TestResult {
        let mut run = run_with(ImageSocketLimits {
            max_items: 2,
            ..ImageSocketLimits::default()
        })
        .await?;
        run.want(vec![item(1, RequestClass::Visible, 0.0)]).map_err(|closing| closing.reason)?;

        // The same id may move in class and order, and may not ask for another image.
        run.want(vec![WantedItem {
            class: RequestClass::Interactive,
            order: 4.0,
            ..item(1, RequestClass::Visible, 0.0)
        }])
        .map_err(|closing| closing.reason)?;
        assert_eq!((run.items[&1].wanted.class, run.items[&1].wanted.order), (RequestClass::Interactive, 4.0));
        let reused = run.want(vec![WantedItem {
            path: "/photos/other.png".to_string(),
            ..item(1, RequestClass::Visible, 0.0)
        }]);
        assert_eq!(reused.unwrap_err().code, CloseCode::BROKE_CONTRACT);

        // A third item is over the limit, and the items of a refused message are not kept.
        let too_many = run.want(vec![item(2, RequestClass::Visible, 0.0), item(3, RequestClass::Visible, 0.0)]);
        assert_eq!(too_many.unwrap_err().code, CloseCode::BROKE_CONTRACT);
        assert_eq!(run.items.len(), 1);

        let out_of_range = run.want(vec![WantedItem {
            width: Some(0),
            ..item(2, RequestClass::Visible, 0.0)
        }]);
        assert_eq!(out_of_range.unwrap_err().code, CloseCode::BROKE_CONTRACT);
        Ok(())
    }
}
