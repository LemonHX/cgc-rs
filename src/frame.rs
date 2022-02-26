use std::{
    alloc::{alloc, Layout},
    intrinsics::size_of,
};

use dashmap::DashSet as Set;

use crate::gc_box::GCCellLayout;

use super::{
    gc_box::{GCCell, GCHeader},
    state::State,
    trace::Trace,
};

pub struct GCFrame {
    state: &'static State,
    registed_gc_objects: Set<*mut GCHeader>,
    escaped_gc_objects: *mut GCHeader,
}

impl GCFrame {
    pub(crate) fn allocate_gc_cell<T: Trace>(&self, value: T) -> GCCell<T> {
        unsafe {
            let layout = Layout::new::<GCCellLayout<T>>();
            self.state
                .minor_heap_size
                .fetch_add(layout.size(), std::sync::atomic::Ordering::Acquire);
            let header_ptr = alloc(layout) as *mut GCHeader;
            let header = &mut *header_ptr;
            header.init::<T>();
            let data = header_ptr.add(1) as *mut T;
            *data = value;
            if !self.registed_gc_objects.insert(header_ptr) {
                panic!("[FALTAL ERROR] failed to allocate gc cell");
            }
            GCCell {
                header,
                data,
                phantom: std::marker::PhantomData,
            }
        }
    }
}
