use server::buckets;


pub trait Backend {
    fn flush_buckets(&buckets::Buckets) -> ();
}