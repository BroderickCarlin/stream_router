use futures::{future::FutureExt, lock::Mutex, sink, stream, stream::StreamExt};
use std::sync::Arc;
use stream_router;
use tokio;

struct State {
    val: u64,
}

// This is a basic state-ful example that will take an input stream and remove sequential duplicates
#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = stream::iter(vec![1, 2, 3, 3, 3, 4, 5, 6, 6, 7, 7, 7, 7, 8, 9, 10]);
    let black_hole = sink::drain();
    let state = Arc::new(Mutex::new(State { val: 0 }));

    let is_dup = move |x| {
        let state = state.clone();
        async move {
            state
                .lock()
                .map(|mut state| {
                    let prev_val = state.val;
                    state.val = x;
                    prev_val == x
                })
                .await
        }
            .boxed()
    };

    router.add_source(nums, is_dup);
    router.add_sink(black_hole, true);

    // Expected Output:
    // Val: 1
    // Val: 2
    // Val: 3
    // Val: 4
    // Val: 5
    // Val: 6
    // Val: 7
    // Val: 8
    // Val: 9
    // Val: 10
    loop {
        let val = router.next().await;
        println!("Val: {:?}", val.unwrap());
    }
}
