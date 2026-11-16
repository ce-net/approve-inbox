//! `approve-inbox serve`: the approve.inbox cell behind this machine's door (ce-cap, default deny). A background loop
//! reads grant.bundle (List, then Get of every bundle with an item waiting) and needs.list every EVERY_S seconds with
//! the cell's own key; the ops answer from that projection at once. `approve-inbox probe` prints one round.
use approve_inbox::sources::{grant_bundle as gb, needs_list as nl};
use approve_inbox::*;
use approve_inbox_api::*;
use ce_wire::Server;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Default)]
struct State {
    rows: Vec<Pending>,
    bundles: BTreeMap<String, CardView>,
    sources: Vec<Source>,
    as_of_ms: u64,
}

async fn bundles() -> Result<(Vec<gb::Bundle>, String), String> {
    let tok = ui_chain::source_token::<gb::GrantBundle>(&["grant.bundle:read"], 60, None)?;
    let mut c = ce_wire::Client::<gb::GrantBundle>::by_name(tok).await.map_err(|e| e.to_string())?;
    c.deadline = Duration::from_secs(20);
    let list = c.call::<gb::List>(&()).await.map_err(|e| e.to_string())?.map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::new();
    for s in list.iter().filter(|s| s.waiting > 0) {
        if let Ok(Ok(b)) = c.call::<gb::Get>(&s.id).await {
            out.push(b);
        }
    }
    Ok((out, String::new()))
}

async fn needs() -> Result<nl::Needs, String> {
    let tok = ui_chain::source_token::<nl::NeedsList>(&["needs.list:read"], 60, None)?;
    let mut c = ce_wire::Client::<nl::NeedsList>::by_name(tok).await.map_err(|e| e.to_string())?;
    c.deadline = Duration::from_secs(8);
    c.call::<nl::List>(&nl::NeedsReq { without_owner: vec![] }).await.map_err(|e| e.to_string())
}

fn src(cell: &str, r: &Result<impl Sized, String>, ms: u32) -> Source {
    let state = match r {
        Ok(_) => SourceState::Live,
        Err(why) => SourceState::Down { why: why.chars().take(300).collect() },
    };
    Source { cell: cell.into(), node: String::new(), state, ms }
}

async fn round(st: &Arc<RwLock<State>>) {
    let t0 = Instant::now();
    let b = tokio::time::timeout(Duration::from_secs(60), bundles()).await.unwrap_or_else(|_| Err("no answer in 60 s".into()));
    let bms = t0.elapsed().as_millis() as u32;
    let t1 = Instant::now();
    let n = tokio::time::timeout(Duration::from_secs(12), needs()).await.unwrap_or_else(|_| Err("no answer in 12 s".into()));
    let nms = t1.elapsed().as_millis() as u32;
    let mut s = st.write().await;
    let sources = vec![src("grant.bundle", &b, bms), src("needs.list", &n, nms)];
    // a source that did not answer keeps its last rows (shown Stale through Sources), never an empty inbox
    let mut rows: Vec<Pending> = Vec::new();
    match &b {
        Ok((bs, _)) => {
            s.bundles = bs.iter().map(|x| (x.id.clone(), card_of(x))).collect();
            rows.extend(bs.iter().flat_map(bundle_rows));
        }
        Err(_) => rows.extend(s.rows.iter().filter(|r| r.kind == "grant").cloned()),
    }
    match &n {
        Ok(n) => rows.extend(need_rows(n)),
        Err(_) => rows.extend(s.rows.iter().filter(|r| r.kind == "decision").cloned()),
    }
    rows.sort_by(|a, b| b.at.cmp(&a.at).then(a.id.cmp(&b.id)));
    s.rows = rows;
    s.sources = sources;
    s.as_of_ms = ui_chain::now_ms();
}

fn sources_now(s: &State) -> Vec<Source> {
    let age = ui_chain::now_ms().saturating_sub(s.as_of_ms) / 1000;
    s.sources
        .iter()
        .cloned()
        .map(|mut x| {
            if matches!(x.state, SourceState::Live) && age > EVERY_S * 3 {
                x.state = SourceState::Stale { age_s: age };
            }
            x.node = ui_chain::hostname();
            x
        })
        .collect()
}

async fn serve() -> Result<(), String> {
    let me = ui_chain::init(CELL)?.root();
    let started = Instant::now();
    let st: Arc<RwLock<State>> = Arc::default();
    let bg = st.clone();
    tokio::spawn(async move {
        loop {
            round(&bg).await;
            tokio::time::sleep(Duration::from_secs(EVERY_S)).await;
        }
    });
    let (s1, s2, s3, s4) = (st.clone(), st.clone(), st.clone(), st.clone());
    let gate = ce_gate::Gate::<ApproveInbox>::new(me, ce_gate::mesh_roots()).into_fn();
    Server::<ApproveInbox>::new(gate)
        .on::<HealthOp, _, _>(move |_: (), _| async move { Ok(Health { ok: true, version: env!("CARGO_PKG_VERSION").into(), uptime_s: started.elapsed().as_secs() }) })
        .on::<List, _, _>(move |q: InboxQuery, who| {
            let st = s1.clone();
            async move { Ok(for_caller(&who, &q, &st.read().await.rows)) }
        })
        .on::<Badge, _, _>(move |_: (), who| {
            let st = s2.clone();
            async move {
                let g = st.read().await;
                let n = for_caller(&who, &InboxQuery { about: None, kind: None }, &g.rows).len() as u32;
                Ok(Waiting { waiting: n, as_of_ms: g.as_of_ms })
            }
        })
        .on::<Bundle, _, _>(move |id: String, who| {
            let st = s3.clone();
            async move {
                let g = st.read().await;
                let cv = ui_chain::Caveats::of(&who, CELL);
                // a bundle the caller may not see is answered exactly like one that does not exist
                Ok(g.bundles.get(&id).and_then(|c| card_for_caveats(&cv, c)).ok_or(InboxError::NotFound(id)))
            }
        })
        .on::<Sources, _, _>(move |_: (), _| {
            let st = s4.clone();
            async move { Ok(sources_now(&*st.read().await)) }
        })
        .audit(ce_gate::audit::file())
        .serve_door()
        .await
        .map_err(|e| e.to_string())
}

fn main() {
    let rt = || tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string());
    let r = match std::env::args().nth(1).as_deref() {
        Some("serve") => rt().and_then(|rt| rt.block_on(serve())),
        Some("probe") => rt().and_then(|rt| {
            rt.block_on(async {
                ui_chain::init(CELL)?;
                let st: Arc<RwLock<State>> = Arc::default();
                round(&st).await;
                let g = st.read().await;
                for r in &g.rows {
                    println!("{:<10} {:<28} {:<22} {}", r.kind, r.id, r.requester, r.line.chars().take(90).collect::<String>());
                }
                for s in &g.sources {
                    println!("source {} {:?} {} ms", s.cell, s.state, s.ms);
                }
                Ok(())
            })
        }),
        _ => Err("usage: approve-inbox serve | probe".into()),
    };
    if let Err(e) = r {
        eprintln!("approve-inbox: {e}");
        std::process::exit(1);
    }
}
