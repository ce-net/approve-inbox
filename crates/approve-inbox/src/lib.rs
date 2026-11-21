//! The one function of `approve.inbox`: fold grant.bundle bundles and needs.list rows into [`Pending`] rows, and show a
//! caller only the rows its chain's `approve.inbox:where:<key>=<v>` caveats allow.
#![forbid(unsafe_code)]

pub mod sources;

use approve_inbox_api::*;
use sources::{grant_bundle as gb, needs_list as nl};
use ui_chain::Caveats;

pub const CELL: &str = "approve.inbox";
/// The one grant this cell's own key holds (`ce-iam grant --to <cell key> --action <each>`, in
/// `<ce data dir>/cell-keys/approve.inbox/`): reading its two sources, nothing else. A taken inbox approves nothing.
pub const GRANT: &[&str] = &["grant.bundle:read", "needs.list:read", "delegate:1"];
pub const EVERY_S: u64 = 10;

fn state_word(s: &gb::State) -> String {
    match s {
        gb::State::Waiting => "waiting".into(),
        gb::State::Approved { .. } => "approved".into(),
        gb::State::Sealed { .. } => "sealed".into(),
        gb::State::Ended(why) => format!("ended: {why}"),
    }
}

fn kind_word(k: &gb::Kind) -> &'static str {
    match k {
        gb::Kind::CapRequest => "cap-request",
        gb::Kind::IamGrant => "iam-grant",
        gb::Kind::Seal => "seal",
        gb::Kind::RootKey => "root-key",
        gb::Kind::KeyShares => "key-shares",
        gb::Kind::Post => "post",
        gb::Kind::Other => "other",
    }
}

/// The ce-ref a grant item is about: its node when it names one, else its owner cell.
fn about_of(i: &gb::Item) -> String {
    if !i.node.is_empty() {
        format!("node:{}", i.node)
    } else {
        format!("cell:{}", i.owner)
    }
}

/// Every still-waiting item of a bundle as a row.
pub fn bundle_rows(b: &gb::Bundle) -> Vec<Pending> {
    b.rows
        .iter()
        .filter(|r| matches!(r.state, gb::State::Waiting))
        .map(|r| Pending {
            id: format!("{}#{}", b.id, r.n),
            kind: "grant".into(),
            title: b.title.clone(),
            line: r.item.line.clone(),
            requester: if r.item.to.is_empty() { r.item.owner.clone() } else { r.item.to.clone() },
            owner: r.item.owner.clone(),
            node: r.item.node.clone(),
            scope: r.item.scope.clone(),
            until: r.item.until,
            at: b.at,
            bundle: b.id.clone(),
            n: r.n,
            dry_run: b.dry_run,
            sign_at: b.card.clone(),
            about: about_of(&r.item),
        })
        .collect()
}

pub fn need_rows(n: &nl::Needs) -> Vec<Pending> {
    n.rows
        .iter()
        .map(|r| Pending {
            id: format!("need:{}", r.id),
            kind: "decision".into(),
            title: r.title.clone(),
            line: r.text.chars().take(400).collect(),
            requester: r.lane.clone(),
            owner: "needs.list".into(),
            node: r.home.clone().unwrap_or_default(),
            scope: vec![],
            until: 0,
            at: r.t.max(0) as u64,
            bundle: String::new(),
            n: 0,
            dry_run: false,
            sign_at: String::new(),
            about: format!("lane:{}", r.lane),
        })
        .collect()
}

pub fn card_of(b: &gb::Bundle) -> CardView {
    CardView {
        id: b.id.clone(),
        title: b.title.clone(),
        at: b.at,
        dry_run: b.dry_run,
        sign_at: b.card.clone(),
        rows: b
            .rows
            .iter()
            .map(|r| CardRow {
                n: r.n,
                kind: kind_word(&r.item.kind).into(),
                line: r.item.line.clone(),
                text: r.text.clone(),
                state: state_word(&r.state),
                owner: r.item.owner.clone(),
                node: r.item.node.clone(),
                scope: r.item.scope.clone(),
                requester: if r.item.to.is_empty() { r.item.owner.clone() } else { r.item.to.clone() },
                until: r.item.until,
                seal_key: r.item.seal.as_ref().map(|s| s.key.clone()),
            })
            .collect(),
    }
}

fn field<'a>(key: &str, requester: &'a str, kind: &'a str, owner: &'a str, node: &'a str) -> Option<&'a str> {
    match key {
        "requester" => Some(requester),
        "kind" => Some(kind),
        "owner" => Some(owner),
        "node" => Some(node),
        _ => None,
    }
}

/// May a chain with these caveats see a row with these fields? Every caveat key must name a field whose value is one
/// of the allowed values; an unknown key or an empty (disjoint) set hides the row.
pub fn allowed(cv: &Caveats, requester: &str, kind: &str, owner: &str, node: &str) -> bool {
    cv.0.iter().all(|(k, vals)| field(k, requester, kind, owner, node).is_some_and(|v| vals.iter().any(|x| x == v)))
}

/// Names of test persons (scoped, ended after a test); a dry run's drafts sit in the test approval store for them.
pub const TEST_PREFIXES: &[&str] = &["t-", "test-"];

/// A test session: its chain is cut by `requester` to test persons only. Only such a chain is shown dry-run bundles;
/// the owner (and any other chain, scoped or not) never is (repo:ce/docs/CRITICAL-DECISIONS-2026-11-21.md section 7:
/// two thirds of the owner's rows were dry runs).
pub fn test_session(cv: &Caveats) -> bool {
    cv.0.get("requester").is_some_and(|v| !v.is_empty() && v.iter().all(|p| TEST_PREFIXES.iter().any(|t| p.starts_with(t))))
}

pub fn for_caveats(cv: &Caveats, q: &InboxQuery, rows: &[Pending]) -> Vec<Pending> {
    let test = test_session(cv);
    rows.iter()
        .filter(|r| !r.dry_run || test)
        .filter(|r| allowed(cv, &r.requester, &r.kind, &r.owner, &r.node))
        .filter(|r| q.about.as_ref().is_none_or(|a| &r.about == a))
        .filter(|r| q.kind.as_ref().is_none_or(|k| &r.kind == k))
        .cloned()
        .collect()
}

pub fn for_caller(who: &ce_wire::Caller, q: &InboxQuery, rows: &[Pending]) -> Vec<Pending> {
    for_caveats(&Caveats::of(who, CELL), q, rows)
}

/// The card for a caller: only the items it may see; a bundle with none of them does not exist for it.
pub fn card_for_caveats(cv: &Caveats, c: &CardView) -> Option<CardView> {
    if c.dry_run && !test_session(cv) {
        return None;
    }
    let rows: Vec<CardRow> = c.rows.iter().filter(|r| allowed(cv, &r.requester, "grant", &r.owner, &r.node)).cloned().collect();
    (!rows.is_empty()).then(|| CardView { rows, ..c.clone() })
}
