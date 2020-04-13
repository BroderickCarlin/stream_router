use std::time::Duration;
use tokio;
use num_stream::num_stream;
use futures::{
    future,
    sink,
    sink::SinkExt,
    channel::mpsc,
    stream::StreamExt,
};
use stream_router;

#[derive(Hash, PartialEq, Eq)]
enum NumType {
    A,
    B,
    C,
}

#[tokio::main]
async fn main() {
    let nums = num_stream(0, 1, Duration::from_millis(30));
    let nums2 = num_stream(0, 4, Duration::from_millis(10));
    let (chan_tx, mut chan_rx) = mpsc::channel(10);

    let num_transform1 = |x| {
        let retval = match x % 7 {
            0 => NumType::A,
            1 => NumType::B,
            _ => NumType::C,
        };

        future::ready(retval)
    };

    let num_transform2 = |x| {
        let retval = match x % 5 {
            0 => NumType::A,
            1 => NumType::B,
            _ => NumType::C,
        };

        future::ready(retval)
    };

    let black_hole = sink::drain().sink_map_err(|_| ());
    let chan_tx = chan_tx.sink_map_err(|_| ());

    let mut router = stream_router::StreamRouter::new();
    router.add_source(nums, num_transform1);
    router.add_source(nums2, num_transform2);
    router.add_sink(chan_tx, NumType::A);
    router.add_sink(black_hole, NumType::B);

    loop {
        tokio::select! {
            f = router.next() => {
                println!("default: {:?}", f.unwrap());
            }
            v = chan_rx.next() => {
                println!("Val:     {:?}", v.unwrap());
            }
        }
    }
}
