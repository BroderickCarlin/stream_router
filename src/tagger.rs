use futures::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct StreamTagger<F, V> {
    map_func: Box<dyn Fn(V) -> F>,
    map_val: Option<V>,
    map_fut: Option<F>,
}

impl<F, V> StreamTagger<F, V> {
    pub fn new(map_func: Box<dyn Fn(V) -> F>) -> StreamTagger<F, V> {
        StreamTagger {
            map_func,
            map_val: None,
            map_fut: None,
        }
    }

    pub fn start_map<T>(&mut self, val: V)
    where
        F: Future<Output = T>,
        V: Clone,
        T: Hash,
    {
        self.map_val = Some(val.clone());
        self.map_fut = Some((self.map_func)(val));
    }
}

impl<F, V, T> Future for StreamTagger<F, V>
where
    F: Future<Output = T> + Unpin,
    V: Unpin,
    T: Hash,
{
    type Output = (V, T);

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        let map_fut = this.map_fut.take();

        match map_fut {
            None => Poll::Pending,
            Some(mut fut) => {
                match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.map_fut = Some(fut);
                        Poll::Pending
                    }
                    Poll::Ready(tag) => {
                        // map_val here must ALWAYS be Some()
                        let val = this.map_val.take().unwrap();
                        Poll::Ready((val, tag))
                    }
                }
            }
        }
    }
}
