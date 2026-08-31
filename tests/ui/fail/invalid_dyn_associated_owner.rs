use qubit_reflect::reflect;

#[reflect]
trait Parent {
    type Item;
}

#[reflect(supertrait(Parent), dyn_compatible(Fake::Item))]
trait InvalidOwner: Parent {}

fn main() {}
