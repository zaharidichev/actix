use std::mem;
use futures::{Async, Poll};

use fut::CtxFuture;


#[derive(Debug)]
pub enum Chain<A, B, C> where A: CtxFuture {
    First(A, C),
    Second(B),
    Done,
}

impl<A, B, C> Chain<A, B, C>
    where A: CtxFuture,
          B: CtxFuture<Context=A::Context, Service=A::Service>,
{
    pub fn new(a: A, c: C) -> Chain<A, B, C> {
        Chain::First(a, c)
    }

    pub fn poll<F>(&mut self, srv: &mut A::Service, ctx: &mut A::Context, f: F)
                   -> Poll<B::Item, B::Error>
        where F: FnOnce(Result<A::Item, A::Error>, C, &mut A::Service, &mut A::Context)
                        -> Result<Result<B::Item, B>, B::Error>,
    {
        let a_result = match *self {
            Chain::First(ref mut a, _) => {
                match a.poll(srv, ctx) {
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Ok(Async::Ready(t)) => Ok(t),
                    Err(e) => Err(e),
                }
            }
            Chain::Second(ref mut b) => return b.poll(srv, ctx),
            Chain::Done => panic!("cannot poll a chained future twice"),
        };
        let data = match mem::replace(self, Chain::Done) {
            Chain::First(_, c) => c,
            _ => panic!(),
        };
        match try!(f(a_result, data, srv, ctx)) {
            Ok(e) => Ok(Async::Ready(e)),
            Err(mut b) => {
                let ret = b.poll(srv, ctx);
                *self = Chain::Second(b);
                ret
            }
        }
    }
}
