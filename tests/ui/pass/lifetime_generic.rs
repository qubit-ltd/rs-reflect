use qubit_reflect::Reflect;

#[derive(Reflect)]
struct Borrowed<'a> {
    value: &'a str,
}

fn main() {
    let _ = Borrowed { value: "static" };
    let _ = Borrowed::<'static>::type_descriptor();
}
