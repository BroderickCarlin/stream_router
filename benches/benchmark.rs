use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use futures::future::{ready, Future, FutureExt};
use futures::lock::Mutex;
use futures::sink::drain;
use futures::stream::{self, Stream, StreamExt};
use futures_test::task::noop_context;
use std::pin::Pin;
use std::sync::Arc;
use stream_router::StreamRouter;

pub fn bench_router(c: &mut Criterion) {
    c.bench_function("basic_forwarding", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![0]));
                let mut router = StreamRouter::new();
                let cx = noop_context();

                router.add_source(nums, |x| ready((x, ())));
                (router, cx)
            },
            // We can trust because of unit tests that this will pull
            // the next value from the input stream and yield it
            |(mut router, mut cx)| Pin::new(&mut router).poll_next(&mut cx),
            BatchSize::SmallInput,
        )
    });

    c.bench_function("basic_routing", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![1, 2]));
                let mut router = StreamRouter::new();
                let cx = noop_context();
                let black_hole = drain();
                let is_even = |x| ready((x, false));

                router.add_source(nums, is_even);
                router.add_sink(black_hole, false);
                (router, cx)
            },
            // We can trust because of unit tests that this will yield the next
            // "even" number from the stream
            |(mut router, mut cx)| Pin::new(&mut router).poll_next(&mut cx),
            BatchSize::SmallInput,
        )
    });
}

pub fn bench_filtering(c: &mut Criterion) {
    let mut filter_group = c.benchmark_group("Filter");

    filter_group.bench_function("StreamRouter_filtering", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![1, 2]));
                let mut router = StreamRouter::new();
                let cx = noop_context();
                let black_hole = drain();
                let is_even = |x| {
                    let retval = x % 2 == 0;
                    ready((x, retval))
                };

                router.add_source(nums, is_even);
                router.add_sink(black_hole, false);
                (router.collect::<Vec<u64>>(), cx)
            },
            // We can trust because of unit tests that this will yield the next
            // "even" number from the stream
            |(mut router, mut cx)| {
                while Pin::new(&mut router).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });

    filter_group.bench_function("futures_filtering", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![1, 2])).filter(|x| ready(x % 2 == 0));
                let cx = noop_context();

                (nums.collect::<Vec<u64>>(), cx)
            },
            |(mut nums, mut cx)| {
                while Pin::new(&mut nums).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn bench_mapping(c: &mut Criterion) {
    let mut filter_group = c.benchmark_group("Mapping");

    filter_group.bench_function("StreamRouter_mapping", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![1, 2]));
                let mut router = StreamRouter::new();
                let cx = noop_context();
                let map = |x| ready((x + 1, ()));

                router.add_source(nums, map);
                (router.collect::<Vec<u64>>(), cx)
            },
            // We can trust because of unit tests that this will yield the next
            // "even" number from the stream
            |(mut router, mut cx)| {
                while Pin::new(&mut router).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });

    filter_group.bench_function("futures_mapping", move |b| {
        // For a real comparison we will compare against the Stream `then()` function as the Stream
        // `map()` function does not take an async closure
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![1, 2])).then(|x| ready(x + 1));
                let cx = noop_context();

                (nums.collect::<Vec<u64>>(), cx)
            },
            |(mut nums, mut cx)| {
                while Pin::new(&mut nums).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn bench_filter_mapping(c: &mut Criterion) {
    let mut filter_group = c.benchmark_group("Filter Mapping");

    filter_group.bench_function("StreamRouter_filter_mapping", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![1, 2]));
                let mut router = StreamRouter::new();
                let cx = noop_context();
                let black_hole = drain();
                let is_even = |x| {
                    let retval = x % 2 == 0;
                    ready((x + 1, retval))
                };

                router.add_source(nums, is_even);
                router.add_sink(black_hole, false);
                (router.collect::<Vec<u64>>(), cx)
            },
            // We can trust because of unit tests that this will yield the next
            // "even" number from the stream
            |(mut router, mut cx)| {
                while Pin::new(&mut router).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });

    filter_group.bench_function("futures_filter_mapping", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![1, 2])).filter_map(|x| {
                    if x % 2 == 0 {
                        ready(Some(x + 1))
                    } else {
                        ready(None)
                    }
                });
                let cx = noop_context();

                (nums.collect::<Vec<u64>>(), cx)
            },
            |(mut nums, mut cx)| {
                while Pin::new(&mut nums).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn bench_stateful(c: &mut Criterion) {
    let mut filter_group = c.benchmark_group("Stateful");

    filter_group.bench_function("StreamRouter_stateful_dedup", move |b| {
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![
                    1, 2, 3, 3, 3, 4, 5, 6, 6, 7, 7, 7, 7, 8, 9, 10,
                ]));
                let mut router = stream_router::StreamRouter::new();
                let black_hole = drain();
                let state = Arc::new(Mutex::new(0u64));
                let cx = noop_context();

                let is_dup = move |x| {
                    let state = state.clone();
                    async move {
                        state
                            .lock()
                            .map(|mut prev_val| {
                                let retval = *prev_val;
                                *prev_val = x;
                                (x, retval == x)
                            })
                            .await
                    }
                        .boxed()
                };

                router.add_source(nums, is_dup);
                router.add_sink(black_hole, true);
                (router.collect::<Vec<u64>>(), cx)
            },
            // We can trust because of unit tests that this will yield the next
            // "even" number from the stream
            |(mut router, mut cx)| {
                while Pin::new(&mut router).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });

    filter_group.bench_function("futures_stateful_dedup", move |b| {
        // For a real comparison we will compare against the Stream `then()` function as the Stream
        // `map()` function does not take an async closure
        b.iter_batched(
            || {
                let nums = stream::iter(black_box(vec![
                    1, 2, 3, 3, 3, 4, 5, 6, 6, 7, 7, 7, 7, 8, 9, 10,
                ]))
                .scan(0, |state, x| {
                    if *state == x {
                        ready(None)
                    } else {
                        *state = x;
                        ready(Some(x))
                    }
                });
                let cx = noop_context();

                (nums.collect::<Vec<u64>>(), cx)
            },
            |(mut nums, mut cx)| {
                while Pin::new(&mut nums).poll(&mut cx).is_pending() {}
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(basic_benches, bench_router);
criterion_group!(filtering_benches, bench_filtering);
criterion_group!(mapping_benches, bench_mapping);
criterion_group!(filter_mapping_benches, bench_filter_mapping);
criterion_group!(stateful_benches, bench_stateful);
criterion_main!(
    basic_benches,
    filtering_benches,
    mapping_benches,
    filter_mapping_benches,
    stateful_benches
);
