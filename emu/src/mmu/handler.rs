use crate::mmu::{MemHandler, MemRead, MemWrite};
use std::cell::RefCell;
use std::rc::Rc;

/// A wrapper for RefCell<T> that implements MemHandler
/// This allows us to use RefCell<T> as a memory handler
pub struct RefCellMemHandler<T: MemHandler + 'static + ?Sized> {
    inner: Rc<RefCell<T>>,
}

impl<T: MemHandler + 'static + ?Sized> RefCellMemHandler<T> {
    pub fn new(inner: Rc<RefCell<T>>) -> Self {
        Self { inner }
    }
}

impl<T: MemHandler + 'static + ?Sized> MemHandler for RefCellMemHandler<T> {
    fn on_read(&self, addr: u16) -> MemRead {
        self.inner.borrow().on_read(addr)
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        self.inner.borrow_mut().on_write(addr, value)
    }

    fn on_write_shared(&self, addr: u16, value: u8) -> MemWrite {
        self.inner.borrow_mut().on_write(addr, value)
    }
}
