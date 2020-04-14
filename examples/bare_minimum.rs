use futures::{future, stream, stream::StreamExt};
use stream_router;
use tokio;

// The absolute bare minimum needed to get get a StreamRouter running.
// NOTE: This example doesn't do anything except forward all values
#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    router.add_source(stream::iter(0..100), |_| future::ready(()));

    // Expected Output:
    // 0
    // 1
    // 2
    // 3
    // ...
    loop {
        println!("{:?}", router.next().await.unwrap());
    }
}
