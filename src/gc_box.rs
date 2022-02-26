use std::any::TypeId;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize};

use super::frame::GCFrame;
use super::trace::Trace;

pub struct GCBox<T: Trace> {
    frame: &'static GCFrame,
    value: GCCell<T>,
}
impl<T: Trace> GCBox<T> {
    fn new(frame: &'static GCFrame, value: T) -> Self {
        Self {
            frame,
            value: frame.allocate_gc_cell(value),
        }
    }
}

impl<T: Trace> AsMut<GCMut<T>> for GCBox<T> {
    fn as_mut(&mut self) -> &mut GCMut<T> {
        todo!()
    }
}

pub struct GCRef<T: Trace> {
    value: GCCell<T>,
}

impl<T: Trace> Deref for GCRef<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*(self.value.data as *mut T) }
    }
}

/// write barrier
pub struct GCMut<T: Trace> {
    prev_ptr: GCCell<T>,
    end_ptr: GCCell<T>,
}

impl<T: Trace> Deref for GCMut<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*(self.end_ptr.data as *mut T) }
    }
}

impl<T: Trace> DerefMut for GCMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.end_ptr.data as *mut T) }
    }
}

impl<T: Trace> Drop for GCMut<T> {
    fn drop(&mut self) {
        if self.prev_ptr != self.end_ptr {
            //TODO: set header
        }
        //TODO: if state is parallel scan, commit to rescan
        unimplemented!()
    }
}

/// a pointer to memory allocated by gc
/// header should be next to data

#[repr(C, align(8))]
pub struct GCCell<T: Trace> {
    pub(crate) header: *mut GCHeader,
    pub(crate) data: *mut dyn Trace,
    pub(crate) phantom: std::marker::PhantomData<T>,
}

#[repr(C, align(8))]
pub(crate) struct GCCellLayout<T: Trace> {
    pub(crate) header: GCHeader,
    pub(crate) data: T,
}

impl<T: Trace> PartialEq for GCCell<T> {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
    }
}
impl<T: Trace> Eq for GCCell<T> {}

#[repr(C, align(8))]
pub struct GCHeader {
    liveness: AtomicUsize,
    marked: AtomicBool,
    pined: AtomicBool,
    generation: AtomicU8,
    type_id: TypeId,
}

impl GCHeader {
    pub(crate) fn init<T: Trace>(&mut self) {
        self.type_id = TypeId::of::<T>();
        self.liveness.store(1, std::sync::atomic::Ordering::SeqCst);
        self.marked
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
