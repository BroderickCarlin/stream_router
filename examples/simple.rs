use std::pin::Pin;
use std::time::Duration;
use tokio;
use tokio::time::{self, Interval};

use stream_router;

use futures::{
    ready,
    stream::{Stream, StreamExt},
    task::{Context, Poll},
    {channel::mpsc, future, sink, sink::SinkExt},
};

// NumStream is just a simple struct that will yield a Stream of numbers at
// a specified intervel and increment at a specifed rate
pub struct NumStream {
    num: u64,
    timer: Interval,
    inc: u64,
}

impl NumStream {
    pub fn new(period: Duration, inc: u64) -> Self {
        NumStream {
            num: 0,
            timer: time::interval(period),
            inc,
        }
    }
}

impl Stream for NumStream {
    type Item = u64;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(Pin::new(&mut self.timer).poll_next(cx)) {
            Some(_) => {
                self.num += self.inc;
                return Poll::Ready(Some(self.num));
            }
            None => return Poll::Ready(None),
        }
    }
}

// This is the enum we will use to tag our sinks
#[derive(Hash, PartialEq, Eq)]
enum NumType {
    A,
    B,
    C,
}

#[tokio::main]
async fn main() {
    let nums = NumStream::new(Duration::from_millis(30), 1);
    let nums2 = NumStream::new(Duration::from_millis(10), 4);
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
    router.add_source(Box::new(nums), Box::new(num_transform1));
    router.add_source(Box::new(nums2), Box::new(num_transform2));
    router.add_sink(Box::new(chan_tx), NumType::A);
    router.add_sink(Box::new(black_hole), NumType::B);

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
