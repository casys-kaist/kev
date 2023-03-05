//! Interrupt management.
use crate::sync::SpinLock;
use alloc::sync::Arc;

const INIT: SpinLock<Option<Arc<dyn Fn() + Send + Sync>>> = SpinLock::new(None);
static HANDLERS: [SpinLock<Option<Arc<dyn Fn() + Send + Sync>>>; 224] = [INIT; 224];

#[doc(hidden)]
#[no_mangle]
pub fn do_handle_interrupt(idx: usize) {
    let handler = HANDLERS.get(idx).unwrap().lock().clone();
    if let Some(handler) = handler {
        handler()
    } else {
        panic!("Unknown interrupt #{}", idx + 32);
    }
}

/// Register interrupt handler
pub fn register(vec: usize, handler: impl Fn() + Send + Sync + 'static) {
    *HANDLERS.get(vec - 32).expect("Invalid index").lock() = Some(Arc::new(handler));
}
