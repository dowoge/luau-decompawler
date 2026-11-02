//! Workers allocate small strings by the million, and the system heap is shared
//! by all of them: it cost a third of the running time on 24 threads.
//!
//! A job (one script's work, see `job`) takes all its memory from chunks of
//! its thread's own, cut in order and let go at once when it ends: freeing is
//! nothing, and what a big job needed goes back to the system instead of
//! staying with its thread for blocks of the sizes it happened to use.
//!
//! Outside jobs, small blocks come from lists kept per thread, so no thread
//! waits for another. A block freed on another thread joins that thread's
//! list, and nothing goes back to the system. A worker that is done parks its
//! lists (`park`), and the next worker to start takes them (`adopt`).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, AtomicUsize, Ordering};

/// Blocks are multiples of this, which is also the strictest alignment they
/// satisfy: a chunk starts aligned to it and is cut into multiples of it.
const STEP: usize = 16;
/// Larger limits cost memory quickly and gain little.
const LARGEST: usize = 1024;
const CLASSES: usize = LARGEST / STEP;
/// Every worker keeps one between jobs, which is all most jobs need; a big
/// one takes more.
const CHUNK: usize = 256 << 10;

struct Lists {
    free: [Cell<*mut u8>; CLASSES],
    next: Cell<*mut u8>,
    left: Cell<usize>,
}

/// A job's small blocks: lists of its own, cut from chunks of its own. The
/// first chunk is kept for the thread's next job, the others go back to the
/// system when the job ends, lists and all.
struct Arena {
    on: Cell<bool>,
    free: [Cell<*mut u8>; CLASSES],
    next: Cell<usize>,
    left: Cell<usize>,
    count: Cell<usize>,
    /// The chunks by address, for telling a job's blocks from others.
    chunks: [Cell<usize>; ARENA_CHUNKS],
    low: Cell<usize>,
    high: Cell<usize>,
}

const ARENA_CHUNKS: usize = 256;

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
    static ARENA: Arena = const {
        Arena {
            on: Cell::new(false),
            free: [const { Cell::new(ptr::null_mut()) }; CLASSES],
            next: Cell::new(0),
            left: Cell::new(0),
            count: Cell::new(0),
            chunks: [const { Cell::new(0) }; ARENA_CHUNKS],
            low: Cell::new(usize::MAX),
            high: Cell::new(0),
        }
    };
}

/// Fill what a job leaves with a pattern, so a block that outlives its job
/// shows at once (`LUAU_POISON`, for tests).
static POISON: AtomicBool = AtomicBool::new(false);

pub fn poison(on: bool) {
    POISON.store(on, Ordering::Relaxed);
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

/// Is p in one of this job's chunks? Chunks are CHUNK long, kept by address.
#[inline]
fn owns(a: &Arena, p: usize) -> bool {
    if p < a.low.get() || p >= a.high.get() {
        return false;
    }
    let n = a.count.get();
    // the last chunk starting at or below p
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if a.chunks[mid].get() <= p {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo > 0 && p < a.chunks[lo - 1].get() + CHUNK
}

// SAFETY: as heap_alloc's lists; a job's blocks are used only by its thread
// and only until the job ends (`job` copies what is kept with the arena off).
#[inline]
unsafe fn arena_alloc(a: &Arena, l: Layout) -> *mut u8 {
    if !small(l.size(), l.align()) {
        return System.alloc(l);
    }
    let c = class(l.size());
    let head = a.free[c].get();
    if !head.is_null() {
        a.free[c].set(*head.cast::<*mut u8>());
        return head;
    }
    let size = (c + 1) * STEP;
    if a.left.get() < size {
        let n = a.count.get();
        if n == ARENA_CHUNKS {
            return heap_alloc(l); // freed as any other block: it is no chunk's
        }
        let chunk = System.alloc(Layout::from_size_align_unchecked(CHUNK, STEP));
        if chunk.is_null() {
            return chunk;
        }
        let s = chunk as usize;
        // in address order
        let mut k = n;
        while k > 0 && a.chunks[k - 1].get() > s {
            a.chunks[k].set(a.chunks[k - 1].get());
            k -= 1;
        }
        a.chunks[k].set(s);
        a.count.set(n + 1);
        a.low.set(a.low.get().min(s));
        a.high.set(a.high.get().max(s + CHUNK));
        a.next.set(s);
        a.left.set(CHUNK);
    }
    let p = a.next.get();
    a.next.set(p + size);
    a.left.set(a.left.get() - size);
    p as *mut u8
}

// SAFETY: a small block is at least STEP bytes, so it can hold the list link
// while free, and comes from a chunk no other block overlaps. Each block is on
// one list or in use, never both, and lists are never shared between threads.
// `dealloc` and `realloc` get the layout the block was allocated with, so the
// class they compute is the list it came from.
#[inline]
unsafe fn heap_alloc(l: Layout) -> *mut u8 {
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
unsafe fn heap_dealloc(p: *mut u8, l: Layout) {
    if !small(l.size(), l.align()) {
        return System.dealloc(p, l);
    }
    let c = class(l.size());
    LISTS.with(|t| {
        *p.cast::<*mut u8>() = t.free[c].get();
        t.free[c].set(p);
    })
}

unsafe impl GlobalAlloc for Alloc {
    #[inline]
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = ARENA.with(|a| if a.on.get() { arena_alloc(a, l) } else { ptr::null_mut() });
        if !p.is_null() {
            return p;
        }
        heap_alloc(l)
    }

    #[inline]
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if !small(l.size(), l.align()) {
            return System.dealloc(p, l);
        }
        let c = class(l.size());
        let mine = ARENA.with(|a| {
            if !owns(a, p as usize) {
                return false;
            }
            *p.cast::<*mut u8>() = a.free[c].get();
            a.free[c].set(p);
            true
        });
        if !mine {
            heap_dealloc(p, l);
        }
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
        // a job's block moves to wherever new blocks come from now; one from
        // before the job stays where such blocks live
        let job = was && ARENA.with(|a| owns(a, p as usize));
        let q = if job { self.alloc(Layout::from_size_align_unchecked(new, l.align())) } else { heap_alloc(Layout::from_size_align_unchecked(new, l.align())) };
        if !q.is_null() {
            ptr::copy_nonoverlapping(p, q, l.size().min(new));
            self.dealloc(p, l);
        }
        q
    }
}

/// Runs `f` with this thread's memory taken from its job chunks, then `keep`
/// with them off, to copy what must outlive the job (what `f` returned is
/// dropped by then); then the job's memory is let go. A job inside a job is
/// part of the outer one.
pub fn job<T, U>(f: impl FnOnce() -> T, keep: impl FnOnce(T) -> U) -> U {
    struct Off;
    impl Drop for Off {
        fn drop(&mut self) {
            ARENA.with(|a| a.on.set(false));
        }
    }
    if ARENA.with(|a| a.on.replace(true)) {
        return keep(f()); // already in one
    }
    let t = {
        let _off = Off; // off again even if f unwinds (its chunks then stay)
        f()
    };
    let u = keep(t);
    end_job(false);
    u
}

/// Let a job's chunks go, but the first (all of them when the thread is done).
fn end_job(all: bool) {
    ARENA.with(|a| {
        let n = a.count.get();
        let poison = POISON.load(Ordering::Relaxed);
        // one chunk stays for the next job: any will do, the lowest is first
        let first = if all || n == 0 { None } else { Some(a.chunks[0].get()) };
        for k in 0..n {
            let s = a.chunks[k].get();
            if poison {
                // SAFETY: the chunk is this thread's, and nothing in it is used
                unsafe { ptr::write_bytes(s as *mut u8, 0xDD, CHUNK) };
            }
            if Some(s) == first {
                continue;
            }
            // SAFETY: a chunk was allocated with this size and alignment
            unsafe { System.dealloc(s as *mut u8, Layout::from_size_align_unchecked(CHUNK, STEP)) };
        }
        for f in &a.free {
            f.set(ptr::null_mut());
        }
        match first {
            Some(s) => {
                a.chunks[0].set(s);
                a.count.set(1);
                a.low.set(s);
                a.high.set(s + CHUNK);
                a.next.set(s);
                a.left.set(CHUNK);
            }
            None => {
                a.count.set(0);
                a.low.set(usize::MAX);
                a.high.set(0);
                a.next.set(0);
                a.left.set(0);
            }
        }
    });
}

/// The lists of a worker that is done, for the next one to take.
struct Parked {
    state: AtomicU8, // EMPTY, BUSY (being filled or emptied) or FULL
    free: [AtomicPtr<u8>; CLASSES],
    next: AtomicPtr<u8>,
    left: AtomicUsize,
}

const EMPTY: u8 = 0;
const BUSY: u8 = 1;
const FULL: u8 = 2;

static PARKED: [Parked; 64] = [const {
    Parked {
        state: AtomicU8::new(EMPTY),
        free: [const { AtomicPtr::new(ptr::null_mut()) }; CLASSES],
        next: AtomicPtr::new(ptr::null_mut()),
        left: AtomicUsize::new(0),
    }
}; 64];

/// Leave this thread's lists for a worker that starts later (when a place is
/// free; else they stay unused). The thread keeps none, and its job chunks go
/// back to the system.
pub fn park() {
    end_job(true);
    let Some(slot) = PARKED.iter().find(|s| s.state.compare_exchange(EMPTY, BUSY, Ordering::Acquire, Ordering::Relaxed).is_ok())
    else {
        return;
    };
    LISTS.with(|t| {
        for (c, f) in t.free.iter().enumerate() {
            slot.free[c].store(f.replace(ptr::null_mut()), Ordering::Relaxed);
        }
        slot.next.store(t.next.replace(ptr::null_mut()), Ordering::Relaxed);
        slot.left.store(t.left.replace(0), Ordering::Relaxed);
    });
    slot.state.store(FULL, Ordering::Release);
}

/// Take the lists a finished worker left, if any: called by a new thread
/// before it allocates, so it has none of its own yet.
pub fn adopt() {
    let Some(slot) = PARKED.iter().find(|s| s.state.compare_exchange(FULL, BUSY, Ordering::Acquire, Ordering::Relaxed).is_ok())
    else {
        return;
    };
    LISTS.with(|t| {
        for (c, f) in t.free.iter().enumerate() {
            // a thread that did allocate keeps what it has: its list goes
            // after the adopted one
            let mine = f.get();
            let theirs = slot.free[c].swap(ptr::null_mut(), Ordering::Relaxed);
            if mine.is_null() {
                f.set(theirs);
            } else if !theirs.is_null() {
                // SAFETY: every free block holds the link to the next one
                unsafe {
                    let mut end = theirs;
                    while !(*end.cast::<*mut u8>()).is_null() {
                        end = *end.cast::<*mut u8>();
                    }
                    *end.cast::<*mut u8>() = mine;
                }
                f.set(theirs);
            }
        }
        let (next, left) = (slot.next.swap(ptr::null_mut(), Ordering::Relaxed), slot.left.swap(0, Ordering::Relaxed));
        if left > t.left.get() {
            t.next.set(next);
            t.left.set(left);
        }
    });
    slot.state.store(EMPTY, Ordering::Release);
}
