use futures::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct StreamTagger<F, A> {
    map_func: Box<dyn Fn(A) -> F>,
    map_fut: Option<F>,
}

impl<F, A> StreamTagger<F, A> {
    pub fn new(map_func: Box<dyn Fn(A) -> F>) -> StreamTagger<F, A> {
        StreamTagger {
            map_func,
            map_fut: None,
        }
    }

    pub fn start_map<T>(&mut self, val: A)
    where
        F: Future<Output = (A, T)>,
        T: Hash,
    {
        self.map_fut = Some((self.map_func)(val));
    }
}

impl<F, A, T> Future for StreamTagger<F, A>
where
    F: Future<Output = (A, T)> + Unpin,
    A: Unpin,
    T: Hash,
{
    type Output = (A, T);

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        let map_fut = this.map_fut.take();

        match map_fut {
            None => Poll::Pending,
            Some(mut fut) => match Pin::new(&mut fut).poll(cx) {
                Poll::Pending => {
                    this.map_fut = Some(fut);
                    Poll::Pending
                }
                Poll::Ready(val) => Poll::Ready(val),
            },
        }
    }
}
