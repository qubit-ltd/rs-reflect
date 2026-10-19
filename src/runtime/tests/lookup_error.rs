use crate::runtime::error::{LookupError, LookupQuery, LookupSubject};
use crate::runtime::identity::{ItemIdentity, ItemKind, LookupCandidate, StableItemId};
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
