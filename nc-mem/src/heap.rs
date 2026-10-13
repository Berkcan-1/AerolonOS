use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, Ordering};

struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    fn lock(&self) -> SpinlockGuard<'_, T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinlockGuard { lock: self }
    }
}

struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<'a, T> core::ops::Deref for SpinlockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

struct HeapState {
    head: *mut FreeBlock,
}

unsafe impl Send for HeapState {}

impl HeapState {
    const fn empty() -> Self {
        Self { head: null_mut() }
    }

    unsafe fn init(&mut self, start: usize, size: usize) {
        let aligned_start = (start + 15) & !15;
        let usable_size = (start + size).saturating_sub(aligned_start);
        if usable_size >= 32 {
            let block = aligned_start as *mut FreeBlock;
            (*block).size = usable_size;
            (*block).next = null_mut();
            self.head = block;
        }
    }

    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let needed_payload = (layout.size() + 15) & !15;
        let needed_total = needed_payload + 16;

        let mut prev: *mut FreeBlock = null_mut();
        let mut curr = self.head;

        while !curr.is_null() {
            let block_size = (*curr).size;
            if block_size >= needed_total {
                if block_size >= needed_total + 32 {
                    let next_block = (curr as usize + needed_total) as *mut FreeBlock;
                    (*next_block).size = block_size - needed_total;
                    (*next_block).next = (*curr).next;

                    if prev.is_null() {
                        self.head = next_block;
                    } else {
                        (*prev).next = next_block;
                    }
                } else {
                    if prev.is_null() {
                        self.head = (*curr).next;
                    } else {
                        (*prev).next = (*curr).next;
                    }
                }

                *(curr as *mut usize) = needed_total;
                return (curr as usize + 16) as *mut u8;
            }

            prev = curr;
            curr = (*curr).next;
        }

        null_mut()
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let block = (ptr as usize - 16) as *mut FreeBlock;
        let block_size = *(block as *mut usize);
        (*block).size = block_size;
        (*block).next = null_mut();

        if self.head.is_null() {
            self.head = block;
            return;
        }

        if (block as usize) < (self.head as usize) {
            (*block).next = self.head;
            self.head = block;
        } else {
            let mut curr = self.head;
            while !(*curr).next.is_null() && ((*curr).next as usize) < (block as usize) {
                curr = (*curr).next;
            }
            (*block).next = (*curr).next;
            (*curr).next = block;
        }

        let mut curr = self.head;
        while !curr.is_null() && !(*curr).next.is_null() {
            let next = (*curr).next;
            if curr as usize + (*curr).size == next as usize {
                (*curr).size += (*next).size;
                (*curr).next = (*next).next;
            } else {
                curr = (*curr).next;
            }
        }
    }
}

pub struct LockedHeap {
    state: Spinlock<HeapState>,
}

unsafe impl Send for LockedHeap {}
unsafe impl Sync for LockedHeap {}

impl LockedHeap {
    pub const fn empty() -> Self {
        Self {
            state: Spinlock::new(HeapState::empty()),
        }
    }

    pub unsafe fn init(&self, start: usize, size: usize) {
        self.state.lock().init(start, size);
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.state.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        self.state.lock().dealloc(ptr)
    }
}
