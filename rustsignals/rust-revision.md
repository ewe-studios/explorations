---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.RustSignals
revised_at: 2026-03-23
---

# Rust Signals Library - Rust Revision Guide

## Overview

This guide explains how to reproduce the RustSignals functionality (futures-signals, dominator, haalka) in modern Rust, focusing on:
- Signal processing primitives
- FRP (Functional Reactive Programming) patterns
- WASM usage for web applications
- Performance characteristics

## Core Concepts to Reproduce

### 1. Signal Trait Implementation

The fundamental building block:

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

/// A value that changes over time
pub trait Signal {
    type Item;

    /// Poll for a new value
    ///
    /// Contract:
    /// - First poll MUST return Poll::Ready(Some(value))
    /// - Can return Poll::Ready(None) to signal end
    /// - Returns Poll::Pending if unchanged (waker registered)
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;
}
```

### 2. Mutable State Container

```rust
use std::sync::{Arc, RwLock, Weak};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::task::{Waker, Context};

struct ChangedWaker {
    changed: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

struct MutableState<A> {
    senders: AtomicUsize,
    lock: RwLock<MutableLockState<A>>,
}

struct MutableLockState<A> {
    value: A,
    signals: Vec<Weak<ChangedWaker>>,
}

pub struct Mutable<A>(Arc<MutableState<A>>);

impl<A> Mutable<A> {
    pub fn new(value: A) -> Self {
        Self(Arc::new(MutableState {
            senders: AtomicUsize::new(1),
            lock: RwLock::new(MutableLockState {
                value,
                signals: vec![],
            }),
        }))
    }

    pub fn set(&self, value: A) {
        let mut state = self.0.lock.write().unwrap();
        state.value = value;
        state.notify(true);
    }

    pub fn lock_mut(&self) -> MutableLockMut<A> {
        // Returns RAII guard that notifies on drop if mutated
    }

    pub fn signal(&self) -> MutableSignal<A>
    where A: Copy {
        // Create signal that polls for changes
    }
}
```

### 3. SignalVec for Collections

For efficient collection updates:

```rust
pub enum VecDiff<A> {
    Replace { values: Vec<A> },
    InsertAt { index: usize, value: A },
    UpdateAt { index: usize, value: A },
    RemoveAt { index: usize },
    Move { old_index: usize, new_index: usize },
    Push { value: A },
    Pop {},
    Clear {},
}

pub trait SignalVec {
    type Item;

    fn poll_vec_change(
        self: Pin<&mut Self>,
        cx: &mut Context
    ) -> Poll<Option<VecDiff<Self::Item>>>;
}
```

## Recommended Crates

### Core Dependencies

```toml
[dependencies]
# Futures ecosystem
futures-core = "0.3"
futures-util = "0.3"
pin-project = "1.1"

# Concurrency
parking_lot = "0.12"  # More efficient than std::sync

# WASM support
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"

# Web APIs
web-sys = { version = "0.3", features = ["..."] }
gloo = "0.10"  # More ergonomic than raw web-sys

# Utilities
discard = "1.0"  # For cancelable operations
once_cell = "1.19"
```

### For Bevy Integration (like haalka)

```toml
[dependencies]
bevy = "0.16"
bevy-async-ecs = "0.8"  # Or implement your own
apply = "0.3"  # Builder pattern helpers
enclose = "1.2"  # Closure capturing helper
```

## Implementation Patterns

### Pattern 1: Observer Pattern with Signals

```rust
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Waker, Context, Poll};
use std::pin::Pin;
use parking_lot::RwLock;

pub struct SignalState<T> {
    value: T,
    observers: RwLock<Vec<Weak<Observer>>>,
}

struct Observer {
    changed: AtomicBool,
    waker: parking_lot::Mutex<Option<Waker>>,
}

pub struct SignalReader<T> {
    state: Arc<SignalState<T>>,
    observer: Arc<Observer>,
}

impl<T: Clone> SignalReader<T> {
    pub fn new(state: Arc<SignalState<T>>) -> Self {
        let observer = Arc::new(Observer {
            changed: AtomicBool::new(true),
            waker: parking_lot::Mutex::new(None),
        });

        state.observers.write().push(Arc::downgrade(&observer));

        Self { state, observer }
    }
}

impl<T: Clone> futures_core::stream::Stream for SignalReader<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.observer.changed.swap(false, Ordering::SeqCst) {
            let value = self.state.value.clone();
            Poll::Ready(Some(value))
        } else {
            *self.observer.waker.lock() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
```

### Pattern 2: Differential Collection Updates

```rust
use std::collections::VecDeque;

pub struct MutableVec<T> {
    vec: Vec<T>,
    pending_changes: VecDeque<VecDiff<T>>,
    subscribers: Vec<Arc<Subscriber>>,
}

impl<T: Clone> MutableVec<T> {
    pub fn push(&mut self, value: T) {
        self.vec.push(value.clone());
        self.notify(VecDiff::Push { value });
    }

    pub fn insert(&mut self, index: usize, value: T) {
        self.vec.insert(index, value.clone());
        self.notify(VecDiff::InsertAt { index, value });
    }

    fn notify(&mut self, diff: VecDiff<T>) {
        self.pending_changes.push_back(diff);

        for subscriber in &self.subscribers {
            subscriber.wake();
        }
    }
}
```

### Pattern 3: Signal Combinators

```rust
use pin_project::pin_project;

#[pin_project]
pub struct Map<S, F> {
    #[pin]
    signal: S,
    f: F,
}

impl<S, F, A, B> Signal for Map<S, F>
where
    S: Signal<Item = A>,
    F: FnMut(A) -> B,
{
    type Item = B;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<B>> {
        let this = self.project();
        match this.signal.poll_change(cx)? {
            Poll::Ready(value) => Poll::Ready(Some((this.f)(value))),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[pin_project]
pub struct Combine<A, B, F> {
    #[pin]
    signal_a: A,
    #[pin]
    signal_b: B,
    f: F,
    cached_a: Option<A::Item>,
    cached_b: Option<B::Item>,
}

impl<A, B, F, Out> Signal for Combine<A, B, F>
where
    A: Signal,
    B: Signal,
    F: FnMut(&A::Item, &B::Item) -> Out,
{
    type Item = Out;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Out>> {
        let mut this = self.project();
        let mut changed = false;

        // Poll first signal
        if let Poll::Ready(Some(a)) = this.signal_a.as_mut().poll_change(cx) {
            *this.cached_a = Some(a);
            changed = true;
        }

        // Poll second signal
        if let Poll::Ready(Some(b)) = this.signal_b.as_mut().poll_change(cx) {
            *this.cached_b = Some(b);
            changed = true;
        }

        if changed {
            if let (Some(a), Some(b)) = (this.cached_a.as_ref(), this.cached_b.as_ref()) {
                Poll::Ready(Some((this.f)(a, b)))
            } else {
                Poll::Pending
            }
        } else {
            Poll::Pending
        }
    }
}
```

## WASM Usage Patterns

### 1. Web Audio Signal Processing

```rust
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, AudioBuffer, AudioBufferSourceNode};

pub struct AudioProcessor {
    context: AudioContext,
    signal: Mutable<f32>,  // Frequency or parameter
}

impl AudioProcessor {
    pub fn new() -> Result<Self, JsValue> {
        let context = AudioContext::new()?;

        Ok(Self {
            context,
            signal: Mutable::new(440.0),  // A4 frequency
        })
    }

    pub fn play(&self) -> Result<(), JsValue> {
        let oscillator = self.context.create_oscillator()?;
        oscillator.frequency().set_value(self.signal.get());

        // React to frequency changes
        let osc = oscillator.clone();
        self.signal.signal().for_each(move |freq| {
            osc.frequency().set_value(freq);
            async {}
        });

        Ok(())
    }
}
```

### 2. DOM Binding with Signals

```rust
use web_sys::{Element, HtmlInputElement};
use wasm_bindgen::closure::Closure;

pub fn bind_input_signal(
    element: &HtmlInputElement,
    signal: &Mutable<String>,
) {
    // Update DOM when signal changes
    let closure = {
        let element = element.clone();
        signal.signal_cloned().for_each(move |value| {
            element.set_value(&value);
            async {}
        })
    };

    // Update signal when DOM changes
    let oninput = Closure::wrap(Box::new(move |_: web_sys::Event| {
        signal.set(element.value());
    }) as Box<dyn FnMut(_)>);

    element.set_oninput(Some(oninput.as_ref().unchecked_ref()));
    oninput.forget();
}
```

### 3. Animation Loop

```rust
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, CanvasRenderingContext2d};

pub struct AnimationLoop {
    signal: Mutable<f64>,  // Progress 0.0 - 1.0
    request_id: Option<i32>,
}

impl AnimationLoop {
    pub fn start(&mut self, duration_ms: f64) {
        let signal = self.signal.clone();
        let start_time = window().unwrap().performance().unwrap().now();

        fn animate(
            signal: Mutable<f64>,
            start_time: f64,
            duration_ms: f64,
        ) -> Result<(), JsValue> {
            let elapsed = window().unwrap()
                .performance().unwrap()
                .now() - start_time;

            let progress = (elapsed / duration_ms).min(1.0);
            signal.set(progress);

            if progress < 1.0 {
                let closure = Closure::once(move || {
                    animate(signal, start_time, duration_ms)
                });

                window().unwrap().request_animation_frame(
                    closure.as_ref().unchecked_ref()
                )?;

                closure.forget();
            }

            Ok(())
        }

        animate(signal, start_time, duration_ms).unwrap();
    }
}
```

## Performance Optimizations

### 1. Lock-Free Atomic State

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

pub struct AtomicOption<T> {
    ptr: AtomicPtr<T>,
}

impl<T> AtomicOption<T> {
    pub fn store(&self, value: Option<T>) {
        let new_ptr = match value {
            Some(v) => Box::into_raw(Box::new(v)),
            None => ptr::null_mut(),
        };

        let old_ptr = self.ptr.swap(new_ptr, Ordering::AcqRel);

        if !old_ptr.is_null() {
            unsafe { drop(Box::from_raw(old_ptr)); }
        }
    }

    pub fn take(&self) -> Option<T> {
        let old_ptr = self.ptr.swap(ptr::null_mut(), Ordering::AcqRel);

        if old_ptr.is_null() {
            None
        } else {
            Some(unsafe { *Box::from_raw(old_ptr) })
        }
    }
}
```

### 2. Efficient Change Batching

```rust
pub struct BatchedSignal<T> {
    pending: Arc<Mutex<Vec<T>>>,
    signal: SignalHandle,
}

impl<T> BatchedSignal<T> {
    pub fn push(&self, value: T) {
        let mut pending = self.pending.lock().unwrap();
        pending.push(value);

        if pending.len() >= BATCH_SIZE {
            self.signal.notify();
        }
    }

    pub fn flush(&self) -> Vec<T> {
        std::mem::take(&mut *self.pending.lock().unwrap())
    }
}
```

### 3. Zero-Allocation Signal Chain

```rust
// Stack-allocated signal chain
let result = mutable
    .signal()           // No allocation
    .map(|x| x * 2)     // No allocation
    .filter(|x| *x > 0) // No allocation
    .dedupe();          // No allocation

// All transformations are zero-cost wrappers
// that compose into a single state machine
```

## SIMD Optimization

For signal processing (audio, etc.):

```rust
use std::arch::x86_64::*;

#[target_feature(enable = "avx2")]
unsafe fn process_samples_avx2(
    input: &[f32],
    output: &mut [f32],
    coefficient: __m256,
) {
    let chunks = input.chunks_exact(8);
    let remainder = chunks.remainder();

    for (in_chunk, out_chunk) in chunks.zip(output.chunks_exact_mut(8)) {
        let x = _mm256_loadu_ps(in_chunk.as_ptr());
        let result = _mm256_mul_ps(x, coefficient);
        _mm256_storeu_ps(out_chunk.as_mut_ptr(), result);
    }

    // Handle remainder
    for (i, &sample) in remainder.iter().enumerate() {
        output[chunks.len() * 8 + i] = sample * /* scalar coefficient */;
    }
}
```

## Production Considerations

### 1. Error Handling

```rust
pub enum SignalError {
    Disconnected,
    InvalidValue(String),
    ProcessingError(Box<dyn std::error::Error>),
}

impl<T> MutableResult<T> {
    pub fn set_result(&self, result: Result<T, SignalError>) {
        match result {
            Ok(value) => self.set(Ok(value)),
            Err(e) => {
                log::error!("Signal error: {:?}", e);
                self.set(Err(e));
            }
        }
    }
}
```

### 2. Memory Leak Prevention

```rust
use std::sync::Weak;

pub struct Subscription {
    _guard: DropGuard,
}

struct DropGuard {
    callback: Box<dyn FnOnce()>,
    consumed: AtomicBool,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        if !self.consumed.swap(true, Ordering::SeqCst) {
            (self.callback)();
        }
    }
}
```

### 3. Thread Safety

```rust
pub struct ThreadSafeSignal<T> {
    inner: Arc<Mutex<SignalInner<T>>>,
}

impl<T: Send + Sync + 'static> ThreadSafeSignal<T> {
    pub fn spawn_worker(&self) -> mpsc::Sender<T> {
        let (tx, rx) = mpsc::channel();
        let inner = Arc::downgrade(&self.inner);

        std::thread::spawn(move || {
            for value in rx {
                if let Some(inner) = inner.upgrade() {
                    inner.lock().unwrap().update(value);
                }
            }
        });

        tx
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use futures_executor::block_on;

    #[test]
    fn test_signal_map() {
        let mutable = Mutable::new(5);
        let mapped = mutable.signal().map(|x| x * 2);

        let result = block_on(async {
            mapped.take(3).collect::<Vec<_>>().await
        });

        assert_eq!(result, vec![10]);
    }

    #[test]
    fn test_signal_vec_changes() {
        let vec = MutableVec::new();
        vec.lock_mut().push(1);
        vec.lock_mut().push(2);

        // Verify VecDiff received
    }
}
```

## Summary: Key Takeaways

1. **Signals are Futures-based** - Use `Poll<Option<T>>` contract
2. **Zero-cost through Pin** - Use `pin_project` for self-referential structs
3. **Weak references prevent leaks** - Store `Weak<Observer>` in state
4. **Differential updates** - Use `VecDiff`/`MapDiff` for collections
5. **WASM compatibility** - Design for `wasm-bindgen` from the start
6. **RAII for mutation** - Lock guards that auto-notify on drop
