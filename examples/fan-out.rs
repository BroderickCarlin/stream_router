use futures::{channel::mpsc, future::FutureExt, lock::Mutex, stream::StreamExt};
use num_stream::num_stream;
use std::sync::Arc;
use std::time::Duration;
use stream_router;
use tokio;

struct State {
    max_outputs: u64,
    last_output: u64,
}

#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = num_stream(0, 1, Duration::from_millis(50));
    let (chan0_tx, mut chan0_rx) = mpsc::channel(10);
    let (chan1_tx, mut chan1_rx) = mpsc::channel(10);
    let (chan2_tx, mut chan2_rx) = mpsc::channel(10);
    let (chan3_tx, mut chan3_rx) = mpsc::channel(10);
    let (chan4_tx, mut chan4_rx) = mpsc::channel(10);

    router.add_sink(chan0_tx, 0);
    router.add_sink(chan1_tx, 1);
    router.add_sink(chan2_tx, 2);
    router.add_sink(chan3_tx, 3);
    router.add_sink(chan4_tx, 4);

    let state = Arc::new(Mutex::new(State {
        max_outputs: 5,
        last_output: 0,
    }));

    let fan = move |_| {
        let state = state.clone();
        async move {
            state
                .lock()
                .map(|mut state| {
                    state.last_output = (state.last_output + 1) % state.max_outputs;
                    state.last_output
                })
                .await
        }
            .boxed()
    };

    router.add_source(nums, fan);

    loop {
        tokio::select! {
            _ = router.next() => {
                panic!("We should never yield anything from the router itself in this example!");
            },
            a = chan0_rx.next() => {
                println!("0: {:?}", a.unwrap());
            },
            b = chan1_rx.next() => {
                println!("1: {:?}", b.unwrap());
            },
            c = chan2_rx.next() => {
                println!("2: {:?}", c.unwrap());
            },
            d = chan3_rx.next() => {
                println!("3: {:?}", d.unwrap());
            },
            e = chan4_rx.next() => {
                println!("4: {:?}", e.unwrap());
            },
        }
    }
}
