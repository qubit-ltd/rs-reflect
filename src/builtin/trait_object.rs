//! Reflection descriptors for common dyn-compatible trait objects.

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::TraitCompleteness;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDescriptor;
use crate::expression::GenericDefinitionDescriptor;
use crate::identity::ExternalTraitId;

/// Returns the process-lifetime declaration link for the built-in `dyn Debug`
/// descriptor.
fn debug_trait_descriptor() -> &'static TraitDescriptor {
    static GENERICS: std::sync::LazyLock<GenericDefinitionDescriptor> =
        std::sync::LazyLock::new(|| GenericDefinitionDescriptor {
            parameters: Box::new([]),
            predicates: Box::new([]),
            diagnostic: crate::expression::DiagnosticText::default(),
        });
    static DEFINITION: std::sync::LazyLock<TraitDefinitionDescriptor> = std::sync::LazyLock::new(|| {
        TraitDefinitionDescriptor::new(
            TraitId::External(ExternalTraitId::new("core.fmt.Debug").expect("the built-in Debug trait ID is valid")),
            "Debug",
            "std::fmt::Debug",
            "Debug",
            TraitCompleteness::ExternalIncomplete,
            &GENERICS,
        )
    });
    static APPLIED: std::sync::LazyLock<TraitDescriptor> = std::sync::LazyLock::new(|| {
        TraitDescriptor::builder(&DEFINITION)
            .build()
            .expect("the built-in Debug trait descriptor is valid")
    });
    &APPLIED
}

impl Reflect for dyn std::fmt::Debug {
    /// Returns the interned descriptor for `dyn Debug`.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_trait_object::<Self>(std::any::type_name::<Self>(), debug_trait_descriptor)
        })
    }
}
