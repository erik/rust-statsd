use server::backend::Backend;
use server::buckets;


pub struct Graphite;


impl Backend for Graphite {
    fn flush_buckets(_: &buckets::Buckets) -> () {
    }
}