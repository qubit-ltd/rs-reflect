use qubit_reflect::ReflectedRef;

fn wrap(value: &[u8]) {
    let _ = ReflectedRef::new(value);
}

fn main() {}
