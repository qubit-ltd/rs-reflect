use qubit_reflect::Reflect;

fn label() -> String {
    String::from("default")
}

#[derive(Reflect)]
struct Record {
    #[reflect(default = label)]
    label: String,
}

fn main() {
    let _ = Record::type_descriptor();
}
