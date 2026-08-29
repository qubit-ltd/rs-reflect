use qubit_reflect::Reflect;

#[derive(Reflect)]
#[reflect(rename = "first", rename = "second")]
struct DuplicateRename;

fn main() {}
