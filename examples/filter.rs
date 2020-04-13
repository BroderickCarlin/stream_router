use std::time::Duration;
use tokio;
use num_stream::num_stream;
use futures::{
    future,
    sink,
    sink::SinkExt,
    stream::StreamExt,
};
use stream_router;

#[tokio::main]
async fn main() {
    let nums = num_stream(0, 1, Duration::from_millis(1));
    let black_hole = sink::drain().sink_map_err(|_| ());

    let is_even = |x| {
        let retval = x % 2 == 0;
        future::ready(retval)
    };

    let mut router = stream_router::StreamRouter::new();
    router.add_source(nums, is_even);
    router.add_sink(black_hole, false);

    loop {
        let val = router.next().await;
        println!("Val: {:?}", val.unwrap());
    }
}
