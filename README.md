# approve-inbox

`approve.inbox@1`, the inbox of the APPROVALS cell system (repo:ce-net-net; repo:ce/docs/SITES-ON-CELLS.md section 5).
Approvals is a ce-net system of its own, not a part of Foreman. One row per thing that waits for an approver: every
`Waiting` item of a `grant.bundle`, every open `needs.list` decision. The rows fill the Approvals tab of
apps.ce-net.com (repo:apps-shell), its badge, and the Pending slot of the Core tab.

Ops: `HealthOp`, `List{about, kind}`, `Badge`, `Bundle(id)` (the card: every item with the exact text the passkey
signs), `Sources`. Rights `approve.inbox:read`; rows are cut by `approve.inbox:where:requester=..` (and `kind`,
`owner`, `node`) caveats signed into the caller's chain; an unknown key hides every row. A session never approves:
signing stays one passkey assertion at the bundle's `sign_at` (feed.approve's card) until `feed.approve Act` is typed.

Its own key (`<ce data dir>/cell-keys/approve.inbox/`, `GRANT` = `grant.bundle:read`, `needs.list:read`,
`delegate:1`) reads the sources; the source contracts are copied verbatim in `src/sources.rs`.
`cargo test`: 4. `approve-inbox probe` prints one round. Apache-2.0.
