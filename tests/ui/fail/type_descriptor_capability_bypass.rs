// qubit-style: allow test-file-name
use qubit_reflect::TypeDescriptor;

fn main() {
    let descriptor = TypeDescriptor::of::<u32>();
    let _ = descriptor.capabilities();
}
