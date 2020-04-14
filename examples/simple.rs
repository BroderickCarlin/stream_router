use futures::{channel::mpsc, future, stream, stream::StreamExt};
use stream_router;
use tokio;

#[tokio::main]
async fn main() {
    let mut router = stream_router::StreamRouter::new();
    let nums = stream::iter(0..1_000);
    let (even_chan_tx, mut even_chan_rx) = mpsc::channel(10);

    router.add_source(nums, |x| future::lazy(move |_| x % 2 == 0));
    router.add_sink(even_chan_tx, true);

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
