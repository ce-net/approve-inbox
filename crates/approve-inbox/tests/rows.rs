//! Rows from a bundle, and what a narrower chain sees.
use approve_inbox::sources::grant_bundle as gb;
use approve_inbox::*;
use approve_inbox_api::*;
use ui_chain::Caveats;

fn item(to: &str, node: &str) -> gb::Item {
    gb::Item { kind: gb::Kind::IamGrant, line: format!("let {to} list cells"), owner: "ce.iam".into(), node: node.into(), scope: vec!["cell.install:list".into()], to: to.into(), cap: "v2".into(), until: 2_000_000_000, source: "doc:x".into(), seal: None }
}

fn bundle() -> gb::Bundle {
    let row = |n, to, st| gb::Row { n, item: item(to, "leif-desk"), draft: format!("d{n}"), text: format!("signed text {n}"), state: st };
    gb::Bundle { id: "gb-1".into(), title: "t".into(), at: 10, dry_run: true, card: "https://apps.ce-net.com/grant?b=gb-1".into(), refs: vec![],
        rows: vec![row(1, "t-approver", gb::State::Waiting), row(2, "foreman/lead", gb::State::Waiting), row(3, "t-approver", gb::State::Approved { key: "k".into(), at: 1 })] }
}

fn cv(pairs: &[(&str, &[&str])]) -> Caveats {
    Caveats(pairs.iter().map(|(k, v)| (k.to_string(), v.iter().map(|x| x.to_string()).collect())).collect())
}

#[test]
fn only_waiting_items_are_pending() {
    let rows = bundle_rows(&bundle());
    assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["gb-1#1", "gb-1#2"]);
    assert_eq!(rows[0].about, "node:leif-desk");
    assert!(rows[0].dry_run);
}

#[test]
fn a_requester_caveat_shows_only_its_own_rows() {
    let rows = bundle_rows(&bundle());
    let all = InboxQuery { about: None, kind: None };
    let live = live_rows();
    assert_eq!(for_caveats(&Caveats::default(), &all, &live).len(), 2, "unscoped: every live row");
    let mine = for_caveats(&cv(&[("requester", &["t-approver"])]), &all, &rows);
    assert_eq!(mine.iter().map(|r| r.requester.as_str()).collect::<Vec<_>>(), ["t-approver"]);
    assert!(for_caveats(&cv(&[("requester", &[])]), &all, &rows).is_empty(), "a disjoint caveat hides every row");
    assert!(for_caveats(&cv(&[("colour", &["x"])]), &all, &rows).is_empty(), "an unknown key fails closed");
}

#[test]
fn a_card_shows_only_the_items_the_chain_may_see() {
    let c = card_of(&bundle());
    let mine = card_for_caveats(&cv(&[("requester", &["t-approver"])]), &c).unwrap();
    assert_eq!(mine.rows.iter().map(|r| r.n).collect::<Vec<_>>(), [1, 3]);
    assert!(card_for_caveats(&cv(&[("requester", &["nobody"])]), &c).is_none(), "nothing visible: no such bundle");
}

#[test]
fn the_pending_slot_filters_by_what_it_is_about() {
    let rows = bundle_rows(&bundle());
    let q = InboxQuery { about: Some("node:leif-desk".into()), kind: None };
    assert_eq!(for_caveats(&cv(&[("requester", &["t-approver"])]), &q, &rows).len(), 1);
    assert_eq!(for_caveats(&Caveats::default(), &q, &live_rows()).len(), 2);
    let q = InboxQuery { about: Some("node:ops-1".into()), kind: None };
    assert!(for_caveats(&Caveats::default(), &q, &live_rows()).is_empty());
}

fn live_rows() -> Vec<Pending> {
    bundle_rows(&gb::Bundle { dry_run: false, ..bundle() })
}

#[test]
fn dry_runs_reach_test_sessions_only() {
    let rows = bundle_rows(&bundle());
    let all = InboxQuery { about: None, kind: None };
    // the owner (an unscoped chain, or one cut to a real person) never sees a dry run, row or card
    assert!(for_caveats(&Caveats::default(), &all, &rows).is_empty());
    assert!(for_caveats(&cv(&[("requester", &["leif"])]), &all, &rows).is_empty());
    assert!(for_caveats(&cv(&[("requester", &["t-approver", "leif"])]), &all, &rows).is_empty(), "a mixed chain is not a test session");
    assert!(card_for_caveats(&Caveats::default(), &card_of(&bundle())).is_none());
    // a test session sees its own dry-run rows and card
    assert_eq!(for_caveats(&cv(&[("requester", &["t-approver"])]), &all, &rows).len(), 1);
    assert!(card_for_caveats(&cv(&[("requester", &["t-approver"])]), &card_of(&bundle())).is_some());
    // live rows are unchanged
    assert_eq!(for_caveats(&Caveats::default(), &all, &live_rows()).len(), 2);
    assert!(card_for_caveats(&Caveats::default(), &card_of(&gb::Bundle { dry_run: false, ..bundle() })).is_some());
}
