use server::buckets;


/// Simple trait for the various backends to implement.
pub trait Backend {
    /// This should do whatever is necessary to flush the current set of data
    /// to the backend.
    ///
    /// Called on server `flush` events, which occur on a timer (every 10
    /// seconds by default).
    fn flush_buckets(&mut self, &buckets::Buckets) -> ();
}