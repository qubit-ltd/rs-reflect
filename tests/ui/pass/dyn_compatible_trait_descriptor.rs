use qubit_reflect::TypeDescriptor;
use qubit_reflect::reflect;

#[reflect]
trait Service {
    fn value(&self) -> usize;

    fn borrowed<'a>(&'a self, value: &'a str) -> &'a str;

    fn create() -> Self
    where
        Self: Sized;
}

#[reflect]
trait GenericService<T, const N: usize> {
    fn repeat(&self, value: T) -> [T; N];
}

#[reflect]
trait AssociatedService {
    type Output;

    fn output(&self) -> Self::Output;
}

#[reflect]
trait ReceiverService {
    fn consume(self);

    fn boxed(self: Box<Self>);

    fn pinned(self: std::pin::Pin<Box<Self>>);
}

#[reflect]
trait ParentService {
    fn parent(&self) -> usize;
}

#[reflect(supertrait(ParentService), dyn_compatible)]
trait ChildService: ParentService {
    fn child(&self) -> usize;
}

#[reflect(external_trait(std::fmt::Debug, id = "core.fmt.Debug"))]
trait DebugService: std::fmt::Debug {
    fn debug_value(&self) -> usize;
}

mod dependency {
    use qubit_reflect::reflect;

    #[reflect]
    pub trait DependencyParent {
        fn dependency_parent(&self) -> usize;
    }
}

use dependency as renamed_dependency;

#[reflect(
    supertrait(renamed_dependency::DependencyParent),
    dyn_compatible
)]
trait RenamedDependencyChild: renamed_dependency::DependencyParent {
    fn renamed_dependency_child(&self) -> usize;
}

#[reflect]
trait SizedAssociatedService {
    type Hidden
    where
        Self: Sized;

    type Family<T>
    where
        Self: Sized;

    fn visible(&self) -> usize;
}

trait ExternalParent<T> {
    fn external<U>(&self, value: U) -> T;
}

#[reflect(external_trait(ExternalParent<u8>, id = "example.ExternalParent"))]
trait ExternalChild: ExternalParent<u8> {
    fn external_child(&self) -> usize;
}

#[reflect]
trait Debug {
    fn generic<T>(&self, value: T) -> T;
}

#[reflect(supertrait(Debug))]
trait LocalDebugChild: Debug {
    fn local_debug_child(&self) -> usize;
}

#[reflect]
trait InheritedAssociatedParent {
    type Item;

    fn inherited_item(&self) -> Self::Item;
}

#[reflect(
    supertrait(InheritedAssociatedParent),
    dyn_compatible(InheritedAssociatedParent::Item)
)]
trait InheritedAssociatedChild: InheritedAssociatedParent {
    fn inherited_child(&self) -> usize;
}

fn main() {
    let descriptor = TypeDescriptor::of::<dyn Service>();
    let linked = descriptor
        .as_trait_object()
        .expect("dyn-compatible reflected trait must expose a typed view")
        .trait_descriptor();
    assert_eq!(linked.rust_name(), "Service");

    let generic = TypeDescriptor::of::<dyn GenericService<u8, 4>>();
    assert_eq!(
        generic
            .as_trait_object()
            .expect("generic dyn trait must expose a typed view")
            .trait_descriptor()
            .arguments()
            .len(),
        2
    );

    let associated = TypeDescriptor::of::<dyn AssociatedService<Output = u8>>();
    assert_eq!(
        associated
            .as_trait_object()
            .expect("associated binding must expose a typed view")
            .trait_descriptor()
            .associated_type_arguments()
            .len(),
        1
    );

    let _ = TypeDescriptor::of::<dyn ReceiverService>();
    let child = TypeDescriptor::of::<dyn ChildService>();
    assert_eq!(
        child
            .as_trait_object()
            .expect("reflected supertrait must retain navigation")
            .trait_descriptor()
            .direct_supertraits()[0]
            .rust_name(),
        "ParentService"
    );
    let _ = TypeDescriptor::of::<dyn DebugService>();
    let renamed_dependency = TypeDescriptor::of::<dyn RenamedDependencyChild>();
    assert_eq!(
        renamed_dependency
            .as_trait_object()
            .expect("renamed dependency supertrait must retain navigation")
            .trait_descriptor()
            .direct_supertraits()[0]
            .rust_name(),
        "DependencyParent"
    );
    let sized_associated = TypeDescriptor::of::<dyn SizedAssociatedService>();
    assert_eq!(
        sized_associated
            .as_trait_object()
            .expect("Self: Sized associated items do not require dyn bindings")
            .trait_descriptor()
            .rust_name(),
        "SizedAssociatedService"
    );
    let inherited = TypeDescriptor::of::<dyn InheritedAssociatedChild<Item = u8>>();
    let inherited = inherited
        .as_trait_object()
        .expect("explicit inherited associated proof generates an applied dyn root")
        .trait_descriptor();
    assert_eq!(inherited.associated_type_arguments().len(), 1);
    assert_eq!(
        inherited.direct_supertraits()[0]
            .associated_type_arguments()
            .len(),
        1
    );
}
