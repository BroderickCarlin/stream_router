use futures::{future::FutureExt, lock::Mutex, sink, stream, stream::StreamExt};
use std::sync::Arc;
use stream_router;
use tokio;

struct State {
    val: u64,
}

#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = stream::iter(vec![0, 1, 2, 3, 3, 3, 4, 5, 6, 6, 7, 7, 7, 7, 8, 9, 10]);
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

    loop {
        let val = router.next().await;
        println!("Val: {:?}", val.unwrap());
    }
}
