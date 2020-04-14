use futures::{future::FutureExt, lock::Mutex, sink, stream::StreamExt};
use num_stream::num_stream;
use std::sync::Arc;
use std::time::Duration;
use stream_router;
use tokio;

struct State {
    count: u64,
    output: bool,
}

// This is a basic state-ful example that will take an input stream and alternate between yielding
// 10 sequential values and dropping 10 sequential values.
#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = num_stream(0, 1, Duration::from_millis(50));
    let black_hole = sink::drain();
    let state = Arc::new(Mutex::new(State {
        count: 0,
        output: true,
    }));

    let is_enabled = move |_| {
        let state = state.clone();
        async move {
            state
                .lock()
                .map(|mut state| {
                    if state.count % 10 == 0 {
                        state.output ^= true;
                    }
                    state.count += 1;
                    state.output
                })
                .await
        }
            .boxed()
    };

    router.add_source(nums, is_enabled);
    router.add_sink(black_hole, false);

    // Expected Output:
    // Val: 10
    // Val: 11
    // ...
    // Val: 19
    // Val: 30
    // Val: 31
    // Val: 32
    // ...
    loop {
        let val = router.next().await;
        println!("Val: {:?}", val.unwrap());
    }
}
