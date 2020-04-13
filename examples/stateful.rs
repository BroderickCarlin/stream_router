use std::time::Duration;
use std::sync::Arc;
use tokio;
use num_stream::num_stream;
use futures::{
    future,
    lock::Mutex,
    future::FutureExt,
    sink,
    sink::SinkExt,
    stream::StreamExt,
};
use stream_router;

struct State {
    count: u64,
    output: bool
}

#[tokio::main]
async fn main() {
    let nums = num_stream(0, 1, Duration::from_millis(50));
    let black_hole = sink::drain().sink_map_err(|_| ());
    let state = Arc::new(Mutex::new(State{count: 0, output: true}));

    let is_enabled = move |_x| {
        let state = state.clone();
        async move {
            state.lock().map(|mut state|{
                if state.count % 10 == 0 {
                    state.output ^= true;
                }
                state.count += 1;
                state.output
            }).await
        }.boxed()
    };

    let mut router = stream_router::StreamRouter::new();
    router.add_source(nums, is_enabled);
    router.add_sink(black_hole, false);
    
    loop {
        let val = router.next().await;
        println!("Val: {:?}", val.unwrap());
    }
    
}
