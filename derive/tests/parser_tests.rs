//! Parser smoke tests for the three reflection macro entry points.

#![allow(dead_code)]

use qubit_reflect_derive::{Reflect, reflect, reflect_impl};

#[derive(Reflect)]
#[reflect(rename = "Record", capabilities(Clone, Default))]
struct Record<T> {
    #[reflect(rename = "identifier", read_only, no_construct, default)]
    id: u64,
    #[reflect(opaque)]
    value: T,
}

#[derive(Reflect)]
enum Event {
    #[reflect(rename = "ready", no_construct)]
    Ready,
    #[reflect(skip)]
    Hidden(#[reflect(default = default_code)] u32),
}

#[derive(Reflect)]
#[reflect(opaque)]
struct Opaque;

#[reflect(external_trait(Send, id = "core.marker.Send"))]
trait Service: Send {
    #[reflect(
        rename = "execute",
        catch_unwind,
        thread_safe,
        specialize(T = String)
    )]
    fn run<T>(&self, value: T);

    #[reflect(no_invoke)]
    fn disabled(&self);
}

#[reflect_impl(specialize(T = String))]
impl<T> Record<T> {
    #[reflect(rename = "current", skip)]
    fn value(&self) -> &T {
        &self.value
    }
}

trait ExternalFormat {
    fn format(&self) -> String;
}

#[reflect_impl(external_trait_id = "example.ExternalFormat")]
impl<T> ExternalFormat for Record<T> {
    fn format(&self) -> String {
        String::new()
    }
}

/// Supplies the field default used by the parser fixture.
fn default_code() -> u32 {
    0
}

#[test]
fn test_macro_parsers_accept_supported_declarations() {
    let _ = Record {
        id: 1,
        value: "value",
    };
    let _ = Event::Ready;
    let _ = Opaque;
}
