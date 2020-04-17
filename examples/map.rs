use futures::{future, stream, stream::StreamExt};
use stream_router;
use tokio;

// A simple example of using a StreamRouter as a `map` 
#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = stream::iter(0..1_000);

    router.add_source(nums, |x| future::lazy(move |_| (x * 2, ())));

    // Expected Output:
    // even number: 0
    // odd number:  2
    // even number: 4
    // odd number:  6
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
        }
    }
}
