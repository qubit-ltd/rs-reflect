// qubit-style: allow explicit-imports
use qubit_reflect as reflect;
use reflect::Reflect;

#[derive(Reflect)]
union Unsupported {
    number: u32,
}

fn main() {}
