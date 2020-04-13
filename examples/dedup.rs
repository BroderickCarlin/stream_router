use std::time::Duration;
use std::sync::Arc;
use tokio;
use num_stream::num_stream;
use futures::{
    lock::Mutex,
    future::FutureExt,
    sink,
    sink::SinkExt,
    stream::StreamExt,
};
use stream_router;

struct State {
    val: u64
}

#[tokio::main]
async fn main() {
    let nums1 = num_stream(0, 1, Duration::from_millis(50));
    let nums2 = num_stream(0, 2, Duration::from_millis(100));
    let black_hole = sink::drain().sink_map_err(|_| ());
    let state = Arc::new(Mutex::new(State{val: 0}));

    
    let is_dup = move |x| {
        let state = state.clone();
        async move {
            state.lock().map(|mut state|{
                let is_dup = state.val == x;
                state.val = x;
                is_dup
            }).await
        }.boxed()
    };

    let mut router = stream_router::StreamRouter::new();
    router.add_source(nums1, is_dup.clone());
    router.add_source(nums2, is_dup);
    router.add_sink(black_hole, true);
    
    loop {
        let val = router.next().await;
        println!("Val: {:?}", val.unwrap());
    }
    
}
