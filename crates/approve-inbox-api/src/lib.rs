//! `approve.inbox@1`: the inbox of the APPROVALS cell system (repo:ce/docs/SITES-ON-CELLS.md section 5). One row per
//! thing that waits for an approver: an item of a `grant.bundle` still `Waiting`, or an open `needs.list` decision.
//! The same rows fill the Approvals tab, the Badge count and the `Pending{about}` slot other tabs place.
//!
//! Rights: `approve.inbox:read` for every op but health. Row level: `approve.inbox:where:<key>=<v>` caveats signed
//! into the caller's chain (keys `requester`, `kind`, `owner`, `node`); an unknown key hides every row (fail closed).
//! Approving is not here: a session never approves. The card's `sign_at` is where one passkey assertion signs.
#![forbid(unsafe_code)]

ce_wire::api! {
    api ApproveInbox = "approve.inbox", version 1;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        pub enum SourceState { Live, Stale { age_s: u64 }, Down { why: String } }
        pub struct Source { pub cell: String, pub node: String, pub state: SourceState, pub ms: u32 }
        /// One thing waiting. kind "grant" (item `n` of grant.bundle `bundle`) or "decision" (a needs.list row).
        /// requester: who receives it (lane, person or cell). about: the ce-ref it is about (a lane, a cell, a node).
        pub struct Pending {
            pub id: String, pub kind: String, pub title: String, pub line: String, pub requester: String,
            pub owner: String, pub node: String, pub scope: Vec<String>, pub until: u64, pub at: u64,
            pub bundle: String, pub n: u32, pub dry_run: bool, pub sign_at: String, pub about: String
        }
        /// about: only rows about this ce-ref (the Pending slot). kind: "grant" or "decision".
        pub struct InboxQuery { pub about: Option<String>, pub kind: Option<String> }
        pub struct Waiting { pub waiting: u32, pub as_of_ms: u64 }
        /// One item of a bundle as the card shows it: the exact text the passkey signs, and its state.
        pub struct CardRow {
            pub n: u32, pub kind: String, pub line: String, pub text: String, pub state: String, pub owner: String,
            pub node: String, pub scope: Vec<String>, pub requester: String, pub until: u64, pub seal_key: Option<String>
        }
        pub struct CardView { pub id: String, pub title: String, pub at: u64, pub dry_run: bool, pub sign_at: String, pub rows: Vec<CardRow> }
        pub enum InboxError { NotFound(String), Source(String) }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "approve.inbox:health";
        2 => List(InboxQuery) -> Vec<Pending>, needs "approve.inbox:read", deadline_ms 15000;
        3 => Badge(()) -> Waiting, needs "approve.inbox:read";
        4 => Bundle(String) -> Result<CardView, InboxError>, needs "approve.inbox:read", deadline_ms 15000;
        5 => Sources(()) -> Vec<Source>, needs "approve.inbox:read";
    }
}
