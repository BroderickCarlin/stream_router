use futures::future::ready;
use futures::sink::drain;
use futures::stream::{self, Stream};
use futures::task::Poll;
use futures_test::stream::StreamTestExt;
use futures_test::task::noop_context;
use std::pin::Pin;
use stream_router::StreamRouter;

#[test]
fn single_poll_no_routing() {
    let nums = stream::iter(vec![1, 2]);
    let mut cx = noop_context();
    let mut router = StreamRouter::new();

    router.add_source(nums, |x| ready((x, ())));

    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(1))
    );
    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(2))
    );
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Ready(None));
}

#[test]
fn handling_pending_no_routing() {
    let nums = stream::iter(vec![1, 2]).interleave_pending();
    let mut cx = noop_context();
    let mut router = StreamRouter::new();

    router.add_source(nums, |x| ready((x, ())));

    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(1))
    );
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(2))
    );
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Ready(None));
}

#[test]
fn single_poll_routing() {
    let nums = stream::iter(vec![1, 2, 3, 4]);
    let mut cx = noop_context();
    let mut router = stream_router::StreamRouter::new();
    let black_hole = drain();

    let is_even = |x| {
        let retval = x % 2 == 0;
        ready((x, retval))
    };

    router.add_source(nums, is_even);
    router.add_sink(black_hole, false);

    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(2))
    );
    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(4))
    );
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Ready(None));
}

#[test]
fn handling_pending_routing() {
    let nums = stream::iter(vec![1, 2, 3, 4]).interleave_pending();
    let mut cx = noop_context();
    let mut router = stream_router::StreamRouter::new();
    let black_hole = drain();

    let is_even = |x| {
        let retval = x % 2 == 0;
        ready((x, retval))
    };

    router.add_source(nums, is_even);
    router.add_sink(black_hole, false);

    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    // Value `1` yielded to `black_hole` on the next call TO `poll_next`
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(2))
    );
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    // Value `3` yielded to `black_hole` on the next call to `poll_next`
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    assert_eq!(
        Pin::new(&mut router).poll_next(&mut cx),
        Poll::Ready(Some(4))
    );
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Pending);
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Ready(None));
}

#[test]
fn single_poll_forward() {
    let nums = stream::iter(vec![1, 2, 3, 4]);
    let mut cx = noop_context();
    let mut router = stream_router::StreamRouter::new();
    let black_hole = drain();

    let is_even = |x| ready((x, true));

    router.add_source(nums, is_even);
    router.add_sink(black_hole, true);

    // This single call to `poll_next` will forward all value into `black_hole`
    // and exhaust the input `Stream`.
    assert_eq!(Pin::new(&mut router).poll_next(&mut cx), Poll::Ready(None));
}
