use alloc::raw_vec::RawVec;
use std::ptr;

pub struct RingVec<T> {
    buffer: RawVec<T>,
    size: usize,
    head: usize,
    tail: usize,
    len: usize
}

impl<T> RingVec<T> {
    pub fn new(size: usize) -> RingVec<T> {
        RingVec {
            buffer: RawVec::with_capacity(size + 1),
            size: size,
            head: 0,
            tail: 0,
            len: 0
        }
    }

    pub fn push(&mut self, item: T) {
        if self.size == 0 {
            return;
        }

        if self.len == 0 {
            unsafe {ptr::write(self.buffer.ptr().offset(self.head as isize), item)};
            self.len += 1;
            return;
        }

        self.head += 1;

        if self.head == self.size {
            self.head = 0;
        }

        if self.head == self.tail {
            // drop item
            unsafe {ptr::drop_in_place(self.buffer.ptr().offset(self.tail as isize))};

            self.tail += 1;

            if self.tail == self.size {
                self.tail = 0;
            }
        } else {
            self.len += 1;
        }

        unsafe {ptr::write(self.buffer.ptr().offset(self.head as isize), item)};
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let item = unsafe {ptr::read(self.buffer.ptr().offset(self.head as isize))};

        // zero-out memory just in case
        unsafe {ptr::write_bytes(self.buffer.ptr().offset(self.head as isize), 0, 1)};

        if self.len > 1 {
            if self.head == 0 {
                self.head = self.size;
            }
            self.head -= 1;
        }

        self.len -= 1;

        Some(item)
    }
}
