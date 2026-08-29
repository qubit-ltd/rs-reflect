use qubit_reflect::Reflect;

#[derive(Reflect)]
enum Event<T> {
    Value(T),
    Empty,
}

fn main() {
    let _ = Event::Value(7_u8);
    let _ = Event::<String>::Empty;
    let _ = Event::<u8>::type_descriptor();
}
