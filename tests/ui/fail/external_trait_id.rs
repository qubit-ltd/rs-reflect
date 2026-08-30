use qubit_reflect::reflect_impl;

struct Subject;
trait External {}

#[reflect_impl(external_trait_id = "invalid..identifier")]
impl External for Subject {}

fn main() {}
