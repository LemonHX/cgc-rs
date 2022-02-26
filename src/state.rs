use super::frame::GCFrame;
use super::gc_box::GCHeader;
use dashmap::DashSet as Set;
use std::sync::atomic::{AtomicBool, AtomicUsize};

pub trait Monitoring {
    fn start_minor_gc(&self, minor_heap_size: usize);
    fn end_minor_gc(&self, minor_heap_size: usize);
    fn start_major_gc(&self, major_heap_size: usize);
    fn end_major_gc(&self, major_heap_size: usize);
    fn start_stw(&self);
    fn end_stw(&self);
    fn record_memory_usage(&self, major_heap_size: usize, minor_heap_size: usize);
}

struct DummyMonitoring {}
impl Monitoring for DummyMonitoring {
    fn start_minor_gc(&self, _minor_heap_size: usize) {}

    fn end_minor_gc(&self, _minor_heap_size: usize) {}

    fn start_major_gc(&self, _major_heap_size: usize) {}

    fn end_major_gc(&self, _major_heap_size: usize) {}

    fn start_stw(&self) {}

    fn end_stw(&self) {}

    fn record_memory_usage(&self, _major_heap_size: usize, _minor_heap_size: usize) {}
}

enum GCStage {
    /// starting stage
    /// when finishing concurrent sweep, it will be back to ready stage.
    Ready,
    /// when initial scan is finished it will stop stw, and start parallel scan,
    /// write barrier will be set.
    ParallelScan,
    /// start stw, remove write barrier, and start final scan.
    FinalScan,
    /// everything is marked so removing useless objects.
    ConcurrentSweep,
}

enum MinorGCStage {
    Ready,
    Scan,
    Sweep,
}

pub struct GCConfig {
    /// gc thread pool size
    /// default is 1/4 of cpu cores
    thread_pool_size: usize,
    /// force to trigger minor gc when size exceeds this value
    /// default is 10mb
    minor_gc_trigger_size: usize,
    /// when minor heap is greater than this size, OOM will be triggered
    /// OOM usually means that you are allocating too fast
    /// default is 100mb
    minor_heap_size_limit: usize,
    /// for minor_heap generation object, when lived more than this value, it will be moved to major_heap generation.
    /// default is 3
    major_heap_liveness: usize,
    /// when memory exceeds this value * last size, it will trigger major gc
    /// default is 2.0
    major_gc_pacer_rate: f32,
    /// when major gc is greater than this size, OOM will be triggered
    /// OOM usually means that you are leaking memory or you don't have enough memory to run your program
    /// default is 0 for no limit
    major_heap_size_limit: usize,
    /// for enable imm generation
    /// sometimes some memory are static, and we don't want to collect them
    /// they usally live longer than any other object in the program
    /// default is false
    enable_imm_gen: bool,
    /// this value lives longer than 100 times major gc
    /// default is 100
    imm_liveness: usize,
}

impl Default for GCConfig{
    fn default() -> Self {
        Self {
            thread_pool_size: num_cpus::get() / 4,
            minor_gc_trigger_size: 10 * 1024 * 1024,
            minor_heap_size_limit: 100 * 1024 * 1024,
            major_heap_liveness: 3,
            major_gc_pacer_rate: 2.0,
            major_heap_size_limit: 0,
            enable_imm_gen: false,
            imm_liveness: 100,
        }
    }
}

/// according to Rust's lifetime
/// State should always be static lifetime.
/// if you find this is super slow plz use a better allocator
/// if you don't know one you can use [my wrap of mimalloc](https://github.com/LemonHX/mimalloc-rust)
pub struct State {
    /// TODO: discuss if we can change it in runtime
    pub(crate) config: GCConfig,

    pub(crate)  rayon_pool: rayon::ThreadPool,

    /// collect flags
    pub(crate) stw: AtomicBool,
    pub(crate) start_minor_gc_flag: AtomicBool,
    pub(crate) start_major_gc_flag: AtomicBool,

    // ========== insight ==========
    /// the size of minor heap generation
    pub(crate) minor_heap_size: AtomicUsize,
    /// the size of major_heap generation
    pub(crate) major_heap_size: AtomicUsize,
    /// the size of imm generation
    pub(crate) imm_size: AtomicUsize,
    /// the size of total heap
    pub(crate) total_size: AtomicUsize,

    /// monitoring backend
    /// default: DummyMonitoring
    pub(crate) monitoring: Box<dyn Monitoring>,

    // ========== minor_heap generation ==========
    pub(crate) current_frame_count: AtomicUsize,
    // minor_heap generation
    // frame per unit
    // any element shouldn't live more than three round
    // first scan will scan until first major_heapgen reference, then mark as roots
    pub(crate) minor_heap_roots: Set<*mut GCHeader>,
    pub(crate) minor_heap_gen: Set<*mut GCHeader>,
    pub(crate) minor_heap_marked: Set<*mut GCHeader>,
    pub(crate) minor_heap_dead: Set<*mut GCHeader>,


    // ========== major_heap generation ==========
    // after minor gc, this list will be full, and will start concurrent scan process.
    pub(crate) major_heap_roots: Set<*mut GCHeader>,
    pub(crate) major_heap_gen: Set<*mut GCHeader>,
    pub(crate) major_heap_marked: Set<*mut GCHeader>,
    pub(crate) major_heap_rescan_list: Set<*mut GCHeader>,

    // ========== imm generation ==========
    // enable imm gen will greatly increase the peek performance,
    // but it will also increase the memory usage
    // and it will be harder to descover the memory leak
    pub(crate) imm_gen: Set<*mut GCHeader>,
}

impl State {
    pub fn new() -> State {
        // State{}
        todo!()
    }
    pub fn stw(&self) -> () {
        self.monitoring.start_stw();
        self.stw
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Acquire,
            )
            .expect("[FALTAL ERROR] could not stop the world twice");
    }
    pub fn ctw(&self) -> () {
        self.monitoring.end_stw();
        self.stw
            .compare_exchange(
                true,
                false,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Acquire,
            )
            .expect("[FALTAL ERROR] failed to continue the world");
    }
    pub fn minor_heap_gen_gc(&self) {}
}

unsafe impl Send for State {}

unsafe impl Sync for State {}

// ThreadPool
struct ThreadPool {
    threads: Vec<std::thread::JoinHandle<()>>,
}
