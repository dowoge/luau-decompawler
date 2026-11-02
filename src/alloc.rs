//! Workers allocate small strings by the million, and the system heap is shared
//! by all of them: it cost a third of the running time on 24 threads. Small
//! blocks here come from lists kept per thread, so no thread waits for another.
//! A block freed on another thread just joins that thread's list, what a thread
//! holds when it ends stays unused, and nothing goes back to the system. That
//! costs about a quarter more memory, for a process that is over in seconds.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ptr;

/// Blocks are multiples of this, which is also the strictest alignment they
/// satisfy: a chunk starts aligned to it and is cut into multiples of it.
const STEP: usize = 16;
/// Larger limits cost memory quickly and gain little.
const LARGEST: usize = 1024;
const CLASSES: usize = LARGEST / STEP;
const CHUNK: usize = 1 << 20;

struct Lists {
    free: [Cell<*mut u8>; CLASSES],
    next: Cell<*mut u8>,
    left: Cell<usize>,
}

// `const` and nothing to drop: native thread-local storage, with no allocation
// or registration behind it, which an allocator could not use.
thread_local! {
    static LISTS: Lists = const {
        Lists {
            free: [const { Cell::new(ptr::null_mut()) }; CLASSES],
            next: Cell::new(ptr::null_mut()),
            left: Cell::new(0),
        }
    };
}

pub struct Alloc;

#[inline]
fn small(size: usize, align: usize) -> bool {
    size <= LARGEST && align <= STEP
}

#[inline]
fn class(size: usize) -> usize {
    size.max(1).wrapping_sub(1) / STEP
}

// SAFETY: a small block is at least STEP bytes, so it can hold the list link
// while free, and comes from a chunk no other block overlaps. Each block is on
// one list or in use, never both, and lists are never shared between threads.
// `dealloc` and `realloc` get the layout the block was allocated with, so the
// class they compute is the list it came from.
unsafe impl GlobalAlloc for Alloc {
    #[inline]
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if !small(l.size(), l.align()) {
            return System.alloc(l);
        }
        let c = class(l.size());
        LISTS.with(|t| {
            let head = t.free[c].get();
            if !head.is_null() {
                t.free[c].set(*head.cast::<*mut u8>());
                return head;
            }
            let size = (c + 1) * STEP;
            if t.left.get() < size {
                let chunk = System.alloc(Layout::from_size_align_unchecked(CHUNK, STEP));
                if chunk.is_null() {
                    return chunk;
                }
                t.next.set(chunk);
                t.left.set(CHUNK);
            }
            let p = t.next.get();
            t.next.set(p.add(size));
            t.left.set(t.left.get() - size);
            p
        })
    }

    #[inline]
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if !small(l.size(), l.align()) {
            return System.dealloc(p, l);
        }
        let c = class(l.size());
        LISTS.with(|t| {
            *p.cast::<*mut u8>() = t.free[c].get();
            t.free[c].set(p);
        })
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if !small(l.size(), l.align()) {
            return System.alloc_zeroed(l);
        }
        let p = self.alloc(l);
        if !p.is_null() {
            ptr::write_bytes(p, 0, l.size());
        }
        p
    }

    #[inline]
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let was = small(l.size(), l.align());
        let is = small(new, l.align());
        if !was && !is {
            return System.realloc(p, l, new);
        }
        if was && is && class(l.size()) == class(new) {
            return p;
        }
        let q = self.alloc(Layout::from_size_align_unchecked(new, l.align()));
        if !q.is_null() {
            ptr::copy_nonoverlapping(p, q, l.size().min(new));
            self.dealloc(p, l);
        }
        q
    }
}
