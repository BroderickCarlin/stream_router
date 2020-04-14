use futures::{future, stream, stream::StreamExt};
use stream_router;
use tokio;

#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    router.add_source(stream::iter(0..100), |_| future::ready(0));

    loop {
        println!("{:?}", router.next().await.unwrap());
    }
}
