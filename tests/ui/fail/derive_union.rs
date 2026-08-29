use qubit_reflect::Reflect;

#[derive(Reflect)]
union Unsupported {
    number: u32,
}

fn main() {}
