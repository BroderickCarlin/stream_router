# Stream Router
![Latest Version](https://img.shields.io/crates/v/stream_router)
![License](https://img.shields.io/crates/l/stream_router)
![Downloads](https://img.shields.io/crates/d/stream_router)

This crate provides a `StreamRouter` struct that is capable of dynamically routing values between [`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) and [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html).

[API Documentation](https://docs.rs/stream_router/0.1.0/stream_router/)

[crates.io](https://crates.io/crates/stream_router)

---

It's common when working with [`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) and [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) to build up boilerplate code comprised of chained [Stream combinators](https://docs.rs/futures/0.3.4/futures/stream/trait.StreamExt.html) and bespoke business logic for safely 
routing between [`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) and [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html). This crate attempts to provide a generic implementation of
a universal combinator and dynamic future-aware router while having minimal dependencies and also being executor-agnostic.

`StreamRouter` is the primary Struct of this crate that is capable of dynamically routing values between
[`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) and 
[`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html). A `StreamRouter` is at it's core a
[`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) that can take ownership of any number of 
other [`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) and any number of
[`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) and dynamically route values yielded from the
[`Stream`s](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) to any one of the
provided [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) through user-defined routing rules. 

Each [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) provided to the `StreamRouter` 
is tagged with a user-defined [`Hash`able](https://doc.rust-lang.org/std/hash/trait.Hash.html) value.
This tag is utilized by the router to identify and differentiate [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html)
and is what the user will utilize to reference a specific [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html)
when defining the routing logic.

Each [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) is provided with a matching closure
that consumes the values yielded by the accompanying [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
and returns a [`Future`](https://docs.rs/futures/0.3.4/futures/prelude/trait.Future.html) that will resolve to one of the tags
identifying a specific [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) that the yielded value will be
forwarded to. If no [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) is found for the returned routing tag
the value will be yielded from the `StreamRouter` itself. 

The `StreamRouter` makes the guarantee that order will be preserved for values yielded from [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
"A" and sent to [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html) "B" such that "A" will not attempt to sink any values into "B" until all
previous values from "A" sent to "B" have been processed. There are no cross-Stream or cross-Sink timing or ordering guarentees. 

##### Example

The following example is [`simple.rs`](https://github.com/BroderickCarlin/stream_router/blob/master/examples/simple.rs)
from the [examples](https://github.com/BroderickCarlin/stream_router/tree/master/examples) folder. This simple example
illustrates the `StreamRouter` forwarding all <b>even</b> values to the `even_chan_tx` while all <b>odd</b> numbers are yielded by
the `StreamRouter` itself. A user could decide to provide a second [`Sink`](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html)
to explicitly consume <b>odd</b> values if desired, in which case the `StreamRouter` would never yield any values itself.


```rust
use futures::{channel::mpsc, future, stream, stream::StreamExt};
use stream_router;
use tokio;

#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = stream::iter(0..1_000);
    let (even_chan_tx, mut even_chan_rx) = mpsc::channel(10);

    router.add_source(nums, |x| future::lazy(move |_| (x, x % 2 == 0)));
    router.add_sink(even_chan_tx, true);

    loop {
        tokio::select! {
            v = router.next() => {
                println!("odd number:  {:?}", v.unwrap());
            }
            v = even_chan_rx.next() => {
                println!("even number: {:?}", v.unwrap());
            }
        }
    }
}
```

# Routing Logic

The `StreamRouter`'s routing logic is provided by the user in the form of closures that can map values yielded by
a specific [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) into tags that identify
specific [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html). These closures follow the form of
`Fn(A) -> Future<Output=T>` where `A` is a value yielded by the [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) 
and where `T` is a tag that the user has assigned to one of their [`Sink`s](https://docs.rs/futures/0.3.4/futures/sink/trait.Sink.html). It should be noted that the closure takes ownership of the values yielded by the stream and is responsible for also returning the values as part of the tuple that contains the  [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html) tag. This is done to avoid the need to `clone()` each value but also allows the user to potentially "map" the values if beneficial to their specific use-case.
While simple routing (such as shown above) has no real need to utilize the flexibility provided by returning a
[`Future`](https://docs.rs/futures/0.3.4/futures/prelude/trait.Future.html), the option to return a 
[`Future`](https://docs.rs/futures/0.3.4/futures/prelude/trait.Future.html) allows for more complex state-ful routing. 
An example of utilizing state-ful routing to dedup an incoming [`Stream`](https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html)
can be found in the [`dedup.rs`](https://github.com/BroderickCarlin/stream_router/blob/master/examples/dedup.rs) example. 

#### License

<sup>
Licensed under <a href="LICENSE">Apache License, Version
2.0</a>
</sup>
