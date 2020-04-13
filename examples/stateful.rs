use std::time::Duration;
use tokio;
use num_stream::num_stream;
use futures::{
    compat::Compat01As03,
    future::FutureExt,
    sink,
    sink::SinkExt,
    stream::StreamExt,
};
use stream_router;
use qutex::Qutex;

struct State {
    count: u64,
    output: bool
}

#[tokio::main]
async fn main() {
    let nums = num_stream(0, 1, Duration::from_millis(50));
    let black_hole = sink::drain().sink_map_err(|_| ());
    let state = Qutex::new(State{count: 0, output: true});

    {
        let is_even = move |_x| {
            Compat01As03::new(state.clone().lock()).map(move |mut wrapped_state| {
                if let Ok(state) = wrapped_state.as_mut() {
                    if state.count % 10 == 0 {
                        state.output ^= true;
                    }
                    state.count += 1;
                    state.output
                } else {
                    true
                }
            })
        };

        let mut router = stream_router::StreamRouter::new();
        router.add_source(nums, is_even);
        router.add_sink(black_hole, false);

        loop {
            let val = router.next().await;
            println!("Val: {:?}", val.unwrap());
        }
    }
}
