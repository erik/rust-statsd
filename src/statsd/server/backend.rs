use server::buckets;


pub trait Backend {
    fn flush_buckets(&mut self, &buckets::Buckets) -> ();
}