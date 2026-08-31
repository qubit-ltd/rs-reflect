use qubit_reflect::reflect;

trait External {
    fn generic<T>(&self, value: T);
}

#[reflect(
    external_trait(External, id = "example.External"),
    dyn_compatible
)]
trait InvalidProof: External {}

fn main() {}
