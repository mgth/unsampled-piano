//! Parallel map for the instrument build (off the audio thread): the note
//! table and the radiation integrals are independent per key and per mode.

use std::sync::atomic::{AtomicUsize, Ordering};

/// `(0..n).map(f).collect()` over the available cores, in order. Items are
/// handed out one by one (their costs differ widely: bass keys, low modes).
pub fn map<T: Send, F: Fn(usize) -> T + Sync>(n: usize, f: F) -> Vec<T> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |p| p.get())
        .min(n);
    if threads <= 1 {
        return (0..n).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let mut parts: Vec<Vec<(usize, T)>> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= n {
                            break out;
                        }
                        out.push((i, f(i)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("build worker panicked"))
            .collect()
    });
    let mut slots: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for (i, v) in parts.iter_mut().flat_map(std::mem::take) {
        slots[i] = Some(v);
    }
    slots
        .into_iter()
        .map(|v| v.expect("every item mapped"))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_order() {
        let v = super::map(1000, |i| i * i);
        assert!(v.iter().enumerate().all(|(i, &x)| x == i * i));
        assert!(super::map(0, |i| i).is_empty());
    }
}
