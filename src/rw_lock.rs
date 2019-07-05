use crate::SetTag;
use futures::{try_ready, Async, Future, IntoFuture, Poll};
use futures_locks::{self as locks, RwLockReadFut, RwLockReadGuard};
use std::mem::replace;
use std::ops::{Deref, DerefMut};
use version_tag::VersionTag;

/// A Futures-aware RwLock.
///
/// This class supports also a tagging mechanism on the data.
///
/// On Write, the data is tagged with a new version. We can cheaply detect changes and
/// rebuild resources if he lock has been accessed on write since the last read.
pub struct RwLock<T: ?Sized>(locks::RwLock<T>);

impl<T> RwLock<T> {
    /// Create a new `RwLock` in the unlocked state.
    pub fn new(value: T) -> Self {
        Self(locks::RwLock::new(value))
    }

    /// Acquire the `RwLock` in read-only.
    ///
    /// When the returned `Future` is ready, then this task will have read-only
    /// access to the protected data.
    pub fn read(&self) -> RwLockReadFut<T> {
        self.0.read()
    }

    /// Acquire the `RwLock` in exclusive read-write mode.
    ///
    /// When the returned `Future` is ready, then this task will have read-write
    /// access to the protected data.
    pub fn write(&self) -> RwLockWriteFut<T>
    where
        T: SetTag,
    {
        RwLockWriteFut(self.0.write())
    }
}

impl<T> RwLock<Option<T>> {
    pub fn read_or_init<F, FUT>(&self, init: F) -> RwLockReadInitFut<F, FUT>
    where
        F: Fn() -> FUT,
        FUT: IntoFuture<Item = T>,
    {
        RwLockReadInitFut {
            init,
            lock: self.0.clone(),
            state: RwLockReadInitState::Read(self.0.read()),
        }
    }

    pub fn write_or_init<F, FUT>(&self, init: F) -> RwLockWriteInitFut<F, FUT>
    where
        F: Fn() -> FUT,
        FUT: IntoFuture<Item = T>,
    {
        RwLockWriteInitFut {
            init,
            state: RwLockWriteInitState::Write(self.0.write()),
        }
    }
}

impl<T: ?Sized> Clone for RwLock<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> Default for RwLock<T>
where
    T: Default,
{
    fn default() -> Self {
        Self(locks::RwLock::new(Default::default()))
    }
}

pub struct RwLockReadInitFut<F, FUT: IntoFuture> {
    init: F,
    lock: locks::RwLock<Option<FUT::Item>>,
    state: RwLockReadInitState<FUT>,
}

impl<F, FUT> Future for RwLockReadInitFut<F, FUT>
where
    F: Fn() -> FUT,
    FUT: IntoFuture,
{
    type Item = RwLockReadInitGuard<FUT::Item>;
    type Error = FUT::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let state = match &mut self.state {
                RwLockReadInitState::Init(guard, f) => {
                    let v = try_ready!(f.poll());
                    **guard = Some(v);
                    RwLockReadInitState::Read(self.lock.read())
                }
                RwLockReadInitState::Read(f) => match f.poll() {
                    Ok(Async::Ready(guard)) => {
                        if guard.is_some() {
                            return Ok(Async::Ready(RwLockReadInitGuard(guard)));
                        }

                        RwLockReadInitState::Write(self.lock.write())
                    }
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(_) => unreachable!("Lock error"),
                },
                RwLockReadInitState::Write(f) => match f.poll() {
                    Ok(Async::Ready(guard)) => {
                        if guard.is_some() {
                            RwLockReadInitState::Read(self.lock.read())
                        } else {
                            RwLockReadInitState::Init(guard, (self.init)().into_future())
                        }
                    }
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(_) => unreachable!("Lock error"),
                },
            };

            self.state = state;
        }
    }
}

/// A `Future` representing a pending `RwLock` shared acquisition.
///
/// If the data is not available,
/// eg: value is `None`, the value is initialized.
pub struct RwLockReadInitGuard<T>(RwLockReadGuard<Option<T>>);

impl<T> Deref for RwLockReadInitGuard<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("RwLockReadInitGuard")
    }
}

enum RwLockReadInitState<FUT: IntoFuture> {
    Init(locks::RwLockWriteGuard<Option<FUT::Item>>, FUT::Future),
    Read(RwLockReadFut<Option<FUT::Item>>),
    Write(locks::RwLockWriteFut<Option<FUT::Item>>),
}

pub struct RwLockWriteFut<T: ?Sized + SetTag>(locks::RwLockWriteFut<T>);

impl<T: ?Sized + SetTag> Future for RwLockWriteFut<T> {
    type Item = RwLockWriteGuard<T>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::Ready(RwLockWriteGuard::new(try_ready!(self
            .0
            .poll()))))
    }
}

/// An RAII guard, much like `std::sync::RwLockWriteGuard`.  The wrapped data
/// can be accessed via its `Deref`  and `DerefMut` implementations.
pub struct RwLockWriteGuard<T: ?Sized + SetTag> {
    guard: locks::RwLockWriteGuard<T>,
    new_tag: VersionTag,
}

impl<T: ?Sized + SetTag> RwLockWriteGuard<T> {
    fn new(guard: locks::RwLockWriteGuard<T>) -> Self {
        Self {
            guard,
            new_tag: VersionTag::new(),
        }
    }

    pub fn new_tag(&self) -> VersionTag {
        self.new_tag
    }
}

impl<T: ?Sized + SetTag> Deref for RwLockWriteGuard<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T: ?Sized + SetTag> DerefMut for RwLockWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<T: ?Sized + SetTag> Drop for RwLockWriteGuard<T> {
    fn drop(&mut self) {
        self.guard.set_tag(self.new_tag);
    }
}

pub struct RwLockWriteInitFut<F, FUT: IntoFuture> {
    init: F,
    state: RwLockWriteInitState<FUT>,
}

impl<F, FUT> Future for RwLockWriteInitFut<F, FUT>
where
    F: Fn() -> FUT,
    FUT: IntoFuture,
    FUT::Item: SetTag,
{
    type Item = RwLockWriteGuard<Option<FUT::Item>>;
    type Error = FUT::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match replace(&mut self.state, RwLockWriteInitState::Done) {
                RwLockWriteInitState::Done => panic!("Cannot poll twice"),
                RwLockWriteInitState::Init(mut guard, mut f) => match f.poll() {
                    Ok(Async::NotReady) => {
                        self.state = RwLockWriteInitState::Init(guard, f);
                        return Ok(Async::NotReady);
                    }
                    Ok(Async::Ready(v)) => {
                        *guard = Some(v);
                        return Ok(Async::Ready(RwLockWriteGuard::new(guard)));
                    }
                    Err(e) => return Err(e),
                },
                RwLockWriteInitState::Write(mut f) => match f.poll() {
                    Ok(Async::Ready(guard)) => {
                        if guard.is_some() {
                            return Ok(Async::Ready(RwLockWriteGuard::new(guard)));
                        } else {
                            self.state =
                                RwLockWriteInitState::Init(guard, (self.init)().into_future());
                            continue;
                        }
                    }
                    Ok(Async::NotReady) => {
                        self.state = RwLockWriteInitState::Write(f);
                        return Ok(Async::NotReady);
                    }
                    Err(_) => unreachable!("Lock error"),
                },
            }
        }
    }
}

enum RwLockWriteInitState<FUT: IntoFuture> {
    Done,
    Init(locks::RwLockWriteGuard<Option<FUT::Item>>, FUT::Future),
    Write(locks::RwLockWriteFut<Option<FUT::Item>>),
}
