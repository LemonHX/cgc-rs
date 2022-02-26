use super::gc_box::GCBox;
pub trait Trace where Self: 'static {
    fn trace(&self) -> Vec<GCBox<Self>> where Self: Sized;
}