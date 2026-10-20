// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::error::AdapterValidationError;
use crate::runtime::error::ContextError;
use crate::runtime::error::ContextIssue;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;
use crate::runtime::error::MetadataValidationError;
use crate::runtime::error::RecoverableFailureParts;
use crate::runtime::error::UnsupportedOperationError;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::LookupCandidate;
use crate::runtime::identity::StableItemId;
use crate::runtime::lookup::Lookup;

fn candidate(name: &str) -> LookupCandidate {
    LookupCandidate {
        identity: ItemIdentity {
            kind: ItemKind::Type,
            stable_id: StableItemId(name.into()),
            display_name: name.into(),
        },
        display_name: name.into(),
        origin: None,
    }
}

#[test]
fn into_unique_reports_missing_with_the_original_query() {
    let error = Lookup::<LookupCandidate>::Missing
        .into_unique(LookupSubject::Type, LookupQuery::Name("Widget".into()))
        .unwrap_err();

    assert!(matches!(
        error,
        LookupError::Missing {
            subject: LookupSubject::Type,
            query: LookupQuery::Name(name),
        } if name.as_ref() == "Widget"
    ));
}

#[test]
fn into_unique_retains_all_ambiguous_candidates_as_owned_summaries() {
    let error = Lookup::Ambiguous(vec![candidate("first"), candidate("second")].into_boxed_slice())
        .into_unique(LookupSubject::Type, LookupQuery::Name("Widget".into()))
        .unwrap_err();

    match error {
        LookupError::Ambiguous { candidates, .. } => {
            assert_eq!(candidates.len(), 2);
            assert_eq!(candidates[0].display_name.as_ref(), "first");
            assert_eq!(candidates[1].display_name.as_ref(), "second");
        }
        other => panic!("expected ambiguous lookup error, got {other:?}"),
    }
}

#[test]
fn context_error_clone_shares_structured_issues() {
    let error = ContextError::new("invalid registration");
    let clone = error.clone();

    assert_eq!(error.issues().len(), 1);
    assert!(std::ptr::eq(error.primary_issue(), clone.primary_issue()));
    assert_eq!(error.to_string(), "invalid registration");
}

#[test]
fn lookup_state_accessors_and_consuming_extractors_cover_each_variant() {
    let missing = Lookup::<&str>::Missing;
    assert!(missing.is_missing());
    assert!(!missing.is_unique());
    assert!(!missing.is_ambiguous());
    assert!(missing.is_empty());
    assert_eq!(missing.len(), 0);
    assert!(missing.candidates().is_empty());
    assert_eq!(missing.only(), None);

    let unique = Lookup::Unique("one");
    assert!(!unique.is_missing());
    assert!(unique.is_unique());
    assert!(!unique.is_ambiguous());
    assert!(!unique.is_empty());
    assert_eq!(unique.len(), 1);
    assert_eq!(unique.candidates(), &["one"]);
    assert_eq!(unique.only(), Some("one"));

    let ambiguous = Lookup::Ambiguous(vec!["first", "second"].into_boxed_slice());
    assert!(!ambiguous.is_missing());
    assert!(!ambiguous.is_unique());
    assert!(ambiguous.is_ambiguous());
    assert!(!ambiguous.is_empty());
    assert_eq!(ambiguous.len(), 2);
    assert_eq!(ambiguous.candidates(), &["first", "second"]);
    assert_eq!(ambiguous.only(), None);
}

#[test]
fn into_unique_returns_the_unique_value_without_changing_it() {
    let result = Lookup::Unique(candidate("Widget"))
        .into_unique(LookupSubject::Type, LookupQuery::Name("Widget".into()))
        .expect("a unique lookup should return its candidate");

    assert_eq!(result.display_name.as_ref(), "Widget");
    assert_eq!(result.identity.display_name.as_ref(), "Widget");
}

#[test]
fn lookup_errors_display_and_clone_all_variants() {
    let missing = LookupError::Missing {
        subject: LookupSubject::Method,
        query: LookupQuery::ShortName("run".into()),
    };
    assert_eq!(missing.to_string(), "missing Method for ShortName(\"run\")");
    assert_eq!(missing.clone(), missing);
    assert!(std::error::Error::source(&missing).is_none());

    let ambiguous = LookupError::Ambiguous {
        subject: LookupSubject::Type,
        query: LookupQuery::Name("Widget".into()),
        candidates: vec![candidate("first"), candidate("second")].into_boxed_slice(),
    };
    assert_eq!(
        ambiguous.to_string(),
        "ambiguous Type for Name(\"Widget\") (2 candidates)"
    );
    assert_eq!(ambiguous.clone(), ambiguous);
    assert!(std::error::Error::source(&ambiguous).is_none());

    let context = LookupError::Context(ContextError::new("invalid registration"));
    assert_eq!(context.to_string(), "reflection context failed: invalid registration");
    assert_eq!(context.clone(), context);
    assert_eq!(
        std::error::Error::source(&context)
            .expect("context lookup errors expose their source")
            .to_string(),
        "invalid registration"
    );
}

#[test]
fn context_error_from_issues_handles_empty_and_preserves_issue_order() {
    assert!(ContextError::from_issues(Vec::<ContextIssue>::new()).is_none());

    let first = ContextIssue::Message("first issue".into());
    let second = ContextIssue::Message("second issue".into());
    let error = ContextError::from_issues(vec![first.clone(), second.clone()])
        .expect("nonempty validation issues should produce an error");

    assert_eq!(error.issues(), &[first, second]);
    assert_eq!(error.primary_issue(), &ContextIssue::Message("first issue".into()));
    assert_eq!(error.message(), "first issue");
    assert_eq!(error.to_string(), "first issue");
}

#[test]
fn context_error_uses_generic_message_for_structured_primary_issue() {
    let dependency = ItemIdentity {
        kind: ItemKind::Type,
        stable_id: StableItemId("crate::Widget".into()),
        display_name: "Widget".into(),
    };
    let item = ItemIdentity {
        kind: ItemKind::Method,
        stable_id: StableItemId("crate::Widget::render".into()),
        display_name: "render".into(),
    };
    let issue = ContextIssue::MissingDependency { item, dependency };
    let error = ContextError::from_issues(vec![issue.clone()]).expect("a structured issue should produce an error");

    assert_eq!(error.primary_issue(), &issue);
    assert_eq!(error.message(), "reflection context validation failed");
    assert_eq!(error.to_string(), issue.to_string());
}

#[test]
fn validation_and_unsupported_errors_preserve_messages_and_clone() {
    let adapter = AdapterValidationError::new("adapter has an invalid signature");
    assert_eq!(adapter.to_string(), "adapter has an invalid signature");
    assert_eq!(adapter.clone(), adapter);

    let metadata = MetadataValidationError::new("invalid field metadata");
    assert_eq!(metadata.to_string(), "invalid field metadata");
    assert_eq!(metadata.clone(), metadata);

    let unsupported = UnsupportedOperationError::new("operation unavailable");
    assert_eq!(unsupported.to_string(), "operation unavailable");
    assert_eq!(unsupported.clone(), unsupported);
    assert!(std::error::Error::source(&unsupported).is_none());
}

#[test]
fn recoverable_failure_parts_expose_and_consume_both_values() {
    let parts = RecoverableFailureParts::new("rejected", String::from("retry input"));
    assert_eq!(parts.error, "rejected");
    assert_eq!(parts.recovery, "retry input");
    assert_eq!(parts.into_parts(), ("rejected", String::from("retry input")));
}
