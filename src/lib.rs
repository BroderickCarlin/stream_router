use futures::future::Future;
use futures::sink::{Sink, SinkExt};
use futures::stream::Stream;
use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};

mod rand;
mod tagger;

enum StreamState {
    StreamActive,
    TaggerActive,
    SinkPending,
    SinkActive,
    SinkFlushing,
}

struct StreamManager<F, A, T> {
    tagger: tagger::StreamTagger<F, A>,
    state: StreamState,
    pending_sink_tag: Option<T>,
    pending_item: Option<A>,
    stream: Box<dyn Stream<Item = A> + Unpin>,
}

impl<F, A, T> StreamManager<F, A, T> {
    fn new(
        tagger: tagger::StreamTagger<F, A>,
        stream: Box<dyn Stream<Item = A> + Unpin>,
    ) -> StreamManager<F, A, T> {
        StreamManager {
            tagger,
            state: StreamState::StreamActive,
            pending_sink_tag: None,
            pending_item: None,
            stream,
        }
    }
}

/// The core Struct of this crate that is capable of dynamically routing values between
/// [`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) and [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html).
///
/// A `StreamRouter` is at it's core a [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
/// that can take ownership of any number of other [`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
/// and any number of [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) and dynamically route
/// values yielded from the [`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) to any one of the
/// provided [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) through user-defined routing rules. 
/// 
/// Each [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) provided to the `StreamRouter` 
/// is tagged with a user-defined [`Hash`able](https://doc.rust-lang.org/std/hash/trait.Hash.html) value.
/// This tag is utilized by the router to identify and differentiate [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html)
/// and is what the user will utilize to reference a specific [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html)
/// when defining the routing logic.
/// 
/// Each [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) is provided with a matching closure
/// that consumes the values yielded by the accompanying [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
/// and returns a [`Future`](https://docs.rs/futures/0.3.4/futures/prelude/trait.Future.html) that will resolve to one of the tags
/// identifying a specific [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) that the yielded value will be
/// forwarded to. If no [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) is found for the returned routing tag
/// the value will be yielded from the `StreamRouter` itself. 
/// 
/// The `StreamRouter` makes the guarantee that order will be preserved for values yielded from [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
/// "A" and sent to [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) "B" such that "A" will not attempt to sink any values into "B" until all
/// previous values from "A" sent to "B" have been processed. There are no cross-Stream or cross-Sink timing or ordering guarentees. 
///
/// # Example
/// 
/// The following example is [`simple.rs`](https://github.com/BroderickCarlin/stream_router/blob/master/examples/simple.rs)
/// from the [examples](https://github.com/BroderickCarlin/stream_router/tree/master/examples) folder. This simple example
/// illustrates the `StreamRouter` forwarding all even values to the `even_chan_tx` while all odd numbers are yielded by
/// the `StreamRouter` itself. A user could decide to provide a second [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html)
/// to explicitly consume odd values if desired, in which case the `StreamRouter` would never yield any values itself.
/// 
///
/// ```
/// use futures::{channel::mpsc, future, stream, stream::StreamExt};
/// use stream_router;
/// use tokio;
///
/// #[tokio::main]
/// async fn main() {
///     let mut router = stream_router::StreamRouter::new();
///     let nums = stream::iter(0..1_000);
///     let (even_chan_tx, mut even_chan_rx) = mpsc::channel(10);
///
///     router.add_source(nums, |x| future::lazy(move |_| x % 2 == 0));
///     router.add_sink(even_chan_tx, true);
///
///     loop {
///         tokio::select! {
///             v = router.next() => {
///                 println!("odd number:  {:?}", v.unwrap());
///             }
///             v = even_chan_rx.next() => {
///                 println!("even number: {:?}", v.unwrap());
///             }
///         }
///     }
/// }
/// ```
/// 
/// # Routing Logic
/// 
/// The `StreamRouter`'s routing logic is provided by the user in the form of closures that can map values yielded by
/// a specific [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) into tags that identify
/// specific [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html). These closures follow the form of
/// `Fn(A) -> Future<Output=T>` where `A` is a value yielded by the [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) 
/// and where `T` is a tag that the user has assigned to one of their [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html). 
/// While simple routing (such as shown above) has no real need to utilize the flexibility provided by returning a
/// [`Future`](https://docs.rs/futures/0.3.4/futures/prelude/trait.Future.html), the option to return a 
/// [`Future`](https://docs.rs/futures/0.3.4/futures/prelude/trait.Future.html) allows for more complex state-ful routing. 
/// An example of utilizing state-ful routing to dedup an incoming [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
/// can be found in the [`dedup.rs`](https://github.com/BroderickCarlin/stream_router/blob/master/examples/dedup.rs) example. 
pub struct StreamRouter<F, T, A>
where
    T: Hash + Eq,
{
    streams: Vec<StreamManager<F, A, T>>,
    sinks: HashMap<T, (usize, Box<dyn Sink<A, Error = ()> + Unpin>)>,
}

impl<F, T, A> StreamRouter<F, T, A>
where
    T: Hash + Eq,
{
    /// Creates a new instance of a `StreamRouter`
    pub fn new() -> StreamRouter<F, T, A> {
        StreamRouter {
            streams: vec![],
            sinks: HashMap::new(),
        }
    }

    /// Adds a new [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) to 
    /// the `StreamRouter` and provides the routing function that will be utilized to assign a 
    /// tag to each value yielded by the [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html).
    /// This tag will determine which [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html), if any, the 
    /// value will be forwarded to.
    /// 
    /// The routing function follows the form: `Fn(A) -> Future<Output=T>` where `A` is a value yielded by the
    /// [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) and where `T` is a tag that the user
    /// has assigned to one of their [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html). The returned
    /// [`Future`](https://docs.rs/futures/0.3.4/futures/prelude/trait.Future.html) could be as simple as 
    /// [`future::ready(tag)`](https://docs.rs/futures/0.3.4/futures/future/fn.ready.html) or a more complex `async` block
    /// such as: 
    /// ```
    /// async move {
    ///     let a = b.await;
    ///     let c = a.await;
    ///     c.await
    /// }.boxed()
    /// ```
    pub fn add_source<S, M>(&mut self, stream: S, transform: M)
    where
        S: Stream<Item = A> + Unpin + 'static,
        M: Fn(A) -> F + 'static,
        F: Future<Output = T>,
    {
        let tagger = tagger::StreamTagger::new(Box::new(transform));
        self.streams
            .push(StreamManager::new(tagger, Box::new(stream)));
    }

    /// Adds a new [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) to the 
    /// `StreamRouter` and provides the tag that will be used to identify the [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html)
    /// from within the user-provided routing logic. Tags are intentionally as flexible as possible and 
    /// only have a couple limitations: 
    /// - All tags have to be the same base type
    /// - Tags have to implement [`Hash`](https://doc.rust-lang.org/std/hash/trait.Hash.html)
    /// - Tags have to implement [`Eq`](https://doc.rust-lang.org/std/cmp/trait.Eq.html)
    /// - Tags have to implement [`Unpin`](https://doc.rust-lang.org/std/marker/trait.Unpin.html)
    /// 
    /// Luckily, most of the base types within the Rust std library implement all these. A non-exhaustive list of some built-in types
    /// that can be used: 
    /// - Numerics (`bool`, `u8`, `u16`, `usize`, etc.)
    /// - `Ipv4Addr`/`Ipv6Addr`
    /// - `String`/`&'static str`
    /// 
    /// But there is also no reason a custom type couldn't be used as long as it meets the above requirements! 
    /// For example, the following could be used: 
    /// ```
    /// #[derive(Hash, Eq)]
    /// enum Color {
    ///     Red,
    ///     Green,
    ///     Blue, 
    /// }
    /// ```
    pub fn add_sink<S>(&mut self, sink: S, tag: T)
    where
        S: Sink<A> + Unpin + Sized + 'static,
    {
        self.sinks
            .insert(tag, (0, Box::new(sink.sink_map_err(|_| ()))));
    }
}

impl<F, T, A> StreamRouter<F, T, A>
where
    F: Future<Output = T> + Unpin,
    T: Hash + Eq + Unpin,
    A: Unpin + Clone,
{
    fn poll_next_entry(&mut self, cx: &mut Context<'_>) -> Poll<Option<A>> {
        use Poll::*;

        let start = rand::thread_rng_n(self.streams.len() as u32) as usize;
        let mut idx = start;

        'outterLoop: for _ in 0..self.streams.len() {
            'innerLoop: loop {
                match self.streams[idx].state {
                    StreamState::StreamActive => {
                        match Pin::new(&mut self.streams[idx].stream).poll_next(cx) {
                            Ready(Some(val)) => {
                                self.streams[idx].state = StreamState::TaggerActive;
                                self.streams[idx].tagger.start_map(val);
                                continue 'innerLoop;
                            }
                            Ready(None) => {
                                self.streams.swap_remove(idx);
                                continue 'outterLoop;
                            }
                            Pending => {
                                break 'innerLoop;
                            }
                        }
                    }
                    StreamState::TaggerActive => {
                        match Pin::new(&mut self.streams[idx].tagger).poll(cx) {
                            Ready((val, tag)) => {
                                if let Some((ref_count, _sink)) = self.sinks.get_mut(&tag) {
                                    // We have a sink for this val!
                                    self.streams[idx].pending_sink_tag = Some(tag);
                                    self.streams[idx].pending_item = Some(val);
                                    if *ref_count == 0 {
                                        // Nobody is currently sinking items, so we need to setup the sink
                                        self.streams[idx].state = StreamState::SinkPending;
                                        continue 'innerLoop;
                                    } else {
                                        self.streams[idx].state = StreamState::SinkActive;
                                        *ref_count += 1;
                                        continue 'innerLoop;
                                    }
                                } else {
                                    // We do not have a sink for this, yield it from us!
                                    self.streams[idx].state = StreamState::StreamActive;
                                    return Ready(Some(val));
                                }
                            }
                            Pending => {
                                break 'innerLoop;
                            }
                        }
                    }
                    StreamState::SinkPending => {
                        let tag = self.streams[idx].pending_sink_tag.take().unwrap();
                        if let Some((ref_count, sink)) = self.sinks.get_mut(&tag) {
                            if *ref_count != 0 {
                                // Another stream is actively sending to this sink
                                // so we can just immedietly start sinking
                                self.streams[idx].pending_sink_tag = Some(tag);
                                self.streams[idx].state = StreamState::SinkActive;
                                *ref_count += 1;
                                continue 'innerLoop;
                            }

                            match Pin::new(sink).poll_ready(cx) {
                                Ready(Ok(())) => {
                                    self.streams[idx].pending_sink_tag = Some(tag);
                                    self.streams[idx].state = StreamState::SinkActive;
                                    *ref_count += 1;
                                    continue 'innerLoop;
                                }
                                Ready(Err(_)) => {
                                    // TODO: properly handle sink errors as the sink is most likely dead
                                    self.streams[idx].pending_item = None;
                                    self.streams[idx].state = StreamState::StreamActive;
                                    break 'innerLoop;
                                }
                                Pending => {
                                    self.streams[idx].pending_sink_tag = Some(tag);
                                    break 'innerLoop;
                                }
                            }
                        } else {
                            // The sink we were going to send to is no longer active
                            // so we will drop the value
                            self.streams[idx].state = StreamState::StreamActive;
                            break 'innerLoop;
                        }
                    }
                    StreamState::SinkActive => {
                        let tag = self.streams[idx].pending_sink_tag.take().unwrap();
                        if let Some((ref_count, sink)) = self.sinks.get_mut(&tag) {
                            if Pin::new(sink)
                                .start_send(self.streams[idx].pending_item.take().unwrap())
                                .is_ok()
                            {
                                self.streams[idx].pending_sink_tag = Some(tag);
                                self.streams[idx].state = StreamState::SinkFlushing;
                                continue 'innerLoop;
                            } else {
                                // TODO: properly handle sink errors as the sink is most likely dead
                                self.streams[idx].state = StreamState::StreamActive;
                                *ref_count -= 1;
                                break 'innerLoop;
                            }
                        }
                    }
                    StreamState::SinkFlushing => {
                        let tag = self.streams[idx].pending_sink_tag.take().unwrap();
                        if let Some((ref_count, sink)) = self.sinks.get_mut(&tag) {
                            if *ref_count > 1 {
                                // Someone else is sinking to this sink, so don't flush yet
                                *ref_count -= 1;
                                self.streams[idx].state = StreamState::StreamActive;
                                continue 'innerLoop;
                            } else {
                                // We are the last person trying to sink here! So flush away
                                match Pin::new(sink).poll_flush(cx) {
                                    Ready(Ok(())) => {
                                        self.streams[idx].state = StreamState::StreamActive;
                                        *ref_count -= 1;
                                        continue 'innerLoop;
                                    }
                                    Ready(Err(_)) => {
                                        // TODO: properly handle sink errors as the sink is most likely dead
                                        self.streams[idx].state = StreamState::StreamActive;
                                        *ref_count -= 1;
                                        continue 'innerLoop;
                                    }
                                    Pending => {
                                        self.streams[idx].pending_sink_tag = Some(tag);
                                        break 'innerLoop;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            idx = idx.wrapping_add(1) % self.streams.len();
        }

        // If the map is empty, then the stream is complete.
        if self.streams.is_empty() {
            Ready(None)
        } else {
            Pending
        }
    }
}

#[must_use = "streams do nothing unless you `.await` or poll them"]
impl<F, T, A> Stream for StreamRouter<F, T, A>
where
    F: Future<Output = T> + Unpin,
    T: Hash + Eq + Unpin,
    A: Unpin + Clone,
{
    type Item = A;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.poll_next_entry(cx) {
            Poll::Ready(Some(val)) => Poll::Ready(Some(val)),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut ret = (0, Some(0));

        for stream_manager in &self.streams {
            let hint = stream_manager.stream.size_hint();

            ret.0 += hint.0;

            match (ret.1, hint.1) {
                (Some(a), Some(b)) => ret.1 = Some(a + b),
                (Some(_), None) => ret.1 = None,
                _ => {}
            }
        }

        ret
    }
}
