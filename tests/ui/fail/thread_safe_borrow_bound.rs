use std::rc::Rc;

use qubit_reflect::SendReflectedRef;

fn main() {
    let value = Rc::new(1_u8);
    let _ = SendReflectedRef::new(&value);
}
