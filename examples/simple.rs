use futures::{channel::mpsc, future, stream, stream::StreamExt};
use stream_router;
use tokio;

// A simple example of splitting an incoming Stream into individual even and odd Streams
#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = stream::iter(0..1_000);
    let (even_chan_tx, mut even_chan_rx) = mpsc::channel(10);

    router.add_source(nums, |x| future::lazy(move |_| x % 2 == 0));
    router.add_sink(even_chan_tx, true);

    // Expected Output:
    // even number: 0
    // odd number:  1
    // even number: 2
    // odd number:  3
    // ...
    //
    // NOTE: StreamRouter here guarantees that the `even number` logs will always be in
    //       sequential order and it guarantees that the `odd number` logs will always be
    //       in sequential order. StreamRouter does NOT guarantee that when these 2 logs are
    //       interwoven (as done here) that they will jointly be in sequential order.
    loop {
        tokio::select! {
            v = router.next() => {
                println!("odd number:  {:?}", v.unwrap());
            }
            v = even_chan_rx.next() => {
                println!("even number: {:?}", v.unwrap());
            }
        }
    }
}
