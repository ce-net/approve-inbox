//! Source contracts, copied verbatim from each source cell's `-api` crate (same token text, so the same schema hash: a
//! source that changes its contract refuses these at connect). Copied, not depended on, so this cell builds against one
//! ce-wire. repo:grant-bundle crates/grant-bundle-api at fa9f63e, repo:needs-list crates/needs-list-api at 532be81.

/// repo:grant-bundle/crates/grant-bundle-api
pub mod grant_bundle {
ce_wire::api! {
    api GrantBundle = "grant.bundle", version 1;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        /// Where a shown value came from (ce-ref): kind msg, goal, commit, file, url, api or cell.
        pub struct Ref { pub label: String, pub kind: String, pub to: String }
        /// What an item asks for. The owner cell decides how it is applied; this cell only proves Leif said yes.
        pub enum Kind {
            /// a filed cap.request (`source` = its req id) granted as asked, never wider
            CapRequest,
            /// a ce.iam grant on one node (cell.install and the like)
            IamGrant,
            /// a secret Leif pastes on the card, sealed straight into the keys cell (`seal` says where)
            Seal,
            /// a key or root change that root.approve redeems (device key, second approval, time lock)
            RootKey,
            /// key shares the keys autopilot re-seals
            KeyShares,
            /// posting rights for a poster (Trana org and the like)
            Post,
            /// anything else an owner cell redeems with Verify
            Other,
        }
        /// The capability format the owner cell's gate checks today: "v3" (ce-net-proto CAL), "v2", "v1", or "none"
        /// (a record only, e.g. a keys put).
        pub struct Seal {
            /// the keys cell name the value is put under
            pub key: String,
            /// keys `--service` and `--purpose`
            pub service: String,
            pub purpose: String,
            /// what the value may do, said to Leif before he pastes (e.g. Workers, Pages, DNS edit on ce-net.com zones only)
            pub allowed: String,
            /// keys `--lanes`: the lanes that may `keys run` it (values only in the child's environment); empty = owner only
            pub lanes: Vec<String>,
            /// where Leif makes the value, said on the card
            pub make_at: String,
        }
        /// One item. `line` is the plain-English sentence Leif reads; everything here is in the text his passkey signs.
        pub struct Item {
            pub kind: Kind,
            pub line: String,
            /// the cell that applies it, e.g. "state.store", "ce.iam", "keys", "root.approve", "trana.org"
            pub owner: String,
            /// the node it applies on ("" = the owner cell's own)
            pub node: String,
            /// exact abilities / resources, narrowest first; never "*"
            pub scope: Vec<String>,
            /// who receives it (identity hex, lane or cell); "" when the owner cell is the receiver
            pub to: String,
            pub cap: String,
            /// unix seconds the grant ends; at most 90 days ahead
            pub until: u64,
            /// ce-ref of why it is needed (cap.request id, doc, goal)
            pub source: String,
            pub seal: Option<Seal>,
        }
        pub struct ProposeReq {
            pub title: String,
            pub items: Vec<Item>,
            /// true: drafts go to the test approval store (feed.approve's test session), never Leif's
            pub dry_run: bool,
        }
        /// Where one item is now.
        pub enum State {
            /// waiting for Leif on the card
            Waiting,
            /// Leif's passkey signed it (approval.py check passed here): the key fingerprint and when
            Approved { key: String, at: u64 },
            /// approved and the secret is in the keys cell at this version
            Sealed { version: String, at: u64 },
            /// approved but past `until`, or the draft was rejected
            Ended(String),
        }
        pub struct Row { pub n: u32, pub item: Item, pub draft: String, pub text: String, pub state: State }
        pub struct Bundle { pub id: String, pub title: String, pub at: u64, pub dry_run: bool, pub card: String, pub rows: Vec<Row>, pub refs: Vec<Ref> }
        pub struct Summary { pub id: String, pub title: String, pub at: u64, pub dry_run: bool, pub waiting: u32, pub approved: u32, pub card: String }
        /// feed.approve after a passkey signature: the drafts of this bundle it saw approved. Each is re-checked here.
        pub struct ApprovedReq { pub bundle: String, pub drafts: Vec<String> }
        /// feed.approve after `keys put` took a Seal item's value: the version keys answered (never the value).
        pub struct SealedReq { pub bundle: String, pub n: u32, pub version: String }
        /// An owner cell, before applying item `n` of `bundle`: is it approved, in time, and exactly what was signed?
        pub struct VerifyReq { pub bundle: String, pub n: u32 }
        pub struct Proof { pub bundle: String, pub n: u32, pub item: Item, pub draft: String, pub text: String, pub key: String, pub at: u64 }
        pub enum BundleError { Invalid(String), NotFound(String), NotApproved(String), Store(String) }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "grant.bundle:health";
        2 => Propose(ProposeReq) -> Result<Bundle, BundleError>, needs "grant.bundle:propose";
        3 => Get(String) -> Result<Bundle, BundleError>, needs "grant.bundle:read";
        4 => List(()) -> Result<Vec<Summary>, BundleError>, needs "grant.bundle:read";
        5 => Approved(ApprovedReq) -> Result<Bundle, BundleError>, needs "grant.bundle:approved";
        6 => Sealed(SealedReq) -> Result<Bundle, BundleError>, needs "grant.bundle:approved";
        7 => Verify(VerifyReq) -> Result<Proof, BundleError>, needs "grant.bundle:read";
    }
}
}

/// repo:needs-list/crates/needs-list-api
pub mod needs_list {
ce_wire::api! {
    api NeedsList = "needs.list", version 1;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        /// A lane on a home.
        pub struct Lane { pub home: String, pub lane: String }
        /// The lanes the caller found without a project owner (Leif's own hand-typed terminals are left out here).
        pub struct NeedsReq { pub without_owner: Vec<Lane> }
        /// One row. kind `approval` (a lane with no project owner; `home` set) or `decision` (a sheet). `t` = the
        /// sheet's updated time (epoch s, this machine's zone), 0 for approvals.
        pub struct Need { pub id: String, pub kind: String, pub lane: String, pub home: Option<String>, pub title: String, pub text: String, pub t: i64, pub open: i64, pub more: i64, pub verbs: Vec<String> }
        /// Approvals first, then sheets newest first. `sheets_error`: the sheet file could not be read whole (the
        /// rows read before the error are kept, as in the Python).
        pub struct Needs { pub rows: Vec<Need>, pub sheets_error: Option<String> }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "needs.list:health";
        2 => List(NeedsReq) -> Needs, needs "needs.list:read";
    }
}
}
