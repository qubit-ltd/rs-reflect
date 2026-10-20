// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitDescriptor;
use crate::runtime::identity::IntoLookupCandidate;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::LookupCandidate;
use crate::runtime::identity::StableItemId;

macro_rules! descriptor_collection {
    ($name:ident, $item:ty, $description:literal) => {
        #[doc = $description]
        pub struct $name<'ctx> {
            values: Box<[$item]>,
        }

        impl<'ctx> $name<'ctx> {
            pub(crate) fn new(values: Box<[$item]>) -> Self {
                Self { values }
            }

            /// Returns the number of registered descriptors.
            pub fn len(&self) -> usize {
                self.values.len()
            }

            /// Returns whether the collection contains no descriptors.
            pub fn is_empty(&self) -> bool {
                self.values.is_empty()
            }

            /// Returns the descriptor at `index`, if it exists.
            pub fn get(&self, index: usize) -> Option<$item> {
                self.values.get(index).copied()
            }

            /// Iterates over the descriptors in registration order.
            pub fn iter(&self) -> impl ExactSizeIterator<Item = $item> + '_ {
                self.values.iter().copied()
            }
        }
    };
}

descriptor_collection!(Types, TypeDescriptor<'ctx>, "Registered concrete type descriptors.");
descriptor_collection!(
    TypeDefinitions,
    TypeDefinitionDescriptor<'ctx>,
    "Registered source type definition descriptors."
);
descriptor_collection!(
    TraitDefinitions,
    TraitDefinitionDescriptor<'ctx>,
    "Registered source trait definition descriptors."
);
descriptor_collection!(
    Traits,
    TraitDescriptor<'ctx>,
    "Registered concrete trait application descriptors."
);

impl IntoLookupCandidate for TypeDefinitionDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.path().into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::TypeDefinition,
                stable_id: StableItemId(display_name.clone()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

impl IntoLookupCandidate for TraitDefinitionDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.path().into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::TraitDefinition,
                stable_id: StableItemId(format!("{}:{:?}", display_name, self.id()).into()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}
