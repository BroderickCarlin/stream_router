use futures::{future, sink, stream::StreamExt};
use num_stream::num_stream;
use std::time::Duration;
use stream_router;
use tokio;

// This is a simple example showing how a filter can be easily created to remove
// all odd values from a Stream
#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = num_stream(0, 1, Duration::from_millis(1));
    let black_hole = sink::drain();

    let is_even = |x| {
        let retval = x % 2 == 0;
        future::ready(retval)
    };

    router.add_source(nums, is_even);
    router.add_sink(black_hole, false);

    // Expected Output:
    // Val: 0
    // Val: 2
    // Val: 4
    // Val: 6
    // ...
    loop {
        let val = router.next().await;
        println!("Val: {:?}", val.unwrap());
    }
}
