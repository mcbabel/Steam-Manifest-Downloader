use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::App;
use crate::backend::{Backend, Wake};

pub type Job = Box<dyn FnOnce(&mut App) + Send>;

struct Inner {
    tx: Mutex<Sender<Job>>,
    runtime: tokio::runtime::Handle,
    backend: Arc<dyn Backend>,
    wake: Mutex<Option<Wake>>,
    pending: AtomicUsize,
}

#[derive(Clone)]
pub struct Tasks {
    inner: Arc<Inner>,
}

struct PendingGuard(Arc<Inner>);

impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.0.pending.fetch_sub(1, Ordering::SeqCst);
        wake(&self.0);
    }
}

fn wake(inner: &Inner) {
    let wake = inner.wake.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(wake) = wake {
        wake();
    }
}

impl Tasks {
    pub fn new(runtime: tokio::runtime::Handle, backend: Arc<dyn Backend>) -> (Self, Receiver<Job>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let inner = Inner { tx: Mutex::new(tx), runtime, backend, wake: Mutex::new(None), pending: AtomicUsize::new(0) };
        (Self { inner: Arc::new(inner) }, rx)
    }

    pub fn set_wake(&self, wake: Wake) {
        *self.inner.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(wake);
    }

    pub fn pending(&self) -> usize {
        self.inner.pending.load(Ordering::SeqCst)
    }

    pub fn backend(&self) -> Arc<dyn Backend> {
        self.inner.backend.clone()
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.inner.pending.fetch_add(1, Ordering::SeqCst);
        let guard = PendingGuard(self.inner.clone());
        self.inner.runtime.spawn(async move {
            future.await;
            drop(guard);
        });
    }

    pub fn spawn_quiet<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.inner.runtime.spawn(future);
    }

    pub fn post(&self, job: impl FnOnce(&mut App) + Send + 'static) {
        let _ = self.inner.tx.lock().unwrap_or_else(|e| e.into_inner()).send(Box::new(job));
        wake(&self.inner);
    }

    pub async fn ask<R: Send + 'static>(&self, f: impl FnOnce(&mut App) -> R + Send + 'static) -> Option<R> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.post(move |app| {
            let _ = tx.send(f(app));
        });
        rx.await.ok()
    }

    pub async fn invoke_raw(&self, command: &str, args: Value) -> Result<Value, String> {
        self.inner.backend.invoke(command, args).await
    }

    pub async fn invoke<T: DeserializeOwned>(&self, command: &str, args: Value) -> Result<T, String> {
        let value = self.invoke_raw(command, args).await?;
        serde_json::from_value(value).map_err(|e| format!("{}: unexpected reply: {}", command, e))
    }

    pub fn call<T>(&self, command: &'static str, args: Value, then: impl FnOnce(&mut App, Result<T, String>) + Send + 'static)
    where
        T: DeserializeOwned + Send + 'static,
    {
        let tasks = self.clone();
        self.spawn(async move {
            let result = tasks.invoke::<T>(command, args).await;
            tasks.post(move |app| then(app, result));
        });
    }

    pub fn fire(&self, command: &'static str, args: Value) {
        let tasks = self.clone();
        self.spawn(async move {
            let _ = tasks.invoke_raw(command, args).await;
        });
    }

    pub async fn sleep(&self, millis: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    }
}
