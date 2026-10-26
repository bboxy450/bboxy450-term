//! MA-5 replies validate visibility before any message or signing effect.
use std::sync::Arc;

use bbt_mux::{
    Ulid,
    messaging::{
        Messaging,
        api::{Request, Response},
        keys::LocalSeatKeys,
        wire::SendRequest,
    },
    registry::{Registry, SeatRecord},
    store::{object_dir, sqlite::SqliteStore},
};
use bbt_proto::messaging::MessageRefusal;
use ed25519_dalek::SigningKey;

struct Fixture {
    root: tempfile::TempDir,
    account: SqliteStore,
    messaging: Messaging,
    caller: SeatRecord,
    peer: SeatRecord,
    hidden: SeatRecord,
    hidden_peer: SeatRecord,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let account = SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap();
        let registry = Registry::new(account.clone());
        let swarm = Ulid::new();
        let caller = SeatRecord::new(Ulid::new(), swarm, "caller", "zsh");
        let peer = SeatRecord::new(Ulid::new(), swarm, "peer", "zsh");
        let hidden = SeatRecord::new(Ulid::new(), swarm, "hidden", "zsh");
        let hidden_peer = SeatRecord::new(Ulid::new(), swarm, "hidden peer", "zsh");
        for seat in [&caller, &peer, &hidden, &hidden_peer] {
            registry.insert(seat).unwrap();
        }
        let messaging = Messaging::open(
            root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[70; 32]))),
        )
        .unwrap();
        Self {
            root,
            account,
            messaging,
            caller,
            peer,
            hidden,
            hidden_peer,
        }
    }
    fn send(&mut self, from: Ulid, to: Ulid, text: &str, now: u64) -> Ulid {
        self.messaging
            .send(
                from,
                1,
                SendRequest::text(format!("seats/{to}/inbox"), text),
                now,
            )
            .unwrap()
            .view()
            .id
    }
    fn reply(&mut self, parent: Ulid, thread: Option<Ulid>) -> Response {
        let mut message = SendRequest::text("", "a reply");
        message.reply_to = Some(parent);
        message.thread_id = thread;
        self.messaging
            .request(
                self.caller.id,
                1,
                Request::Send { message },
                false,
                None,
                3_000,
            )
            .unwrap()
    }
    fn refusal(&mut self, parent: Ulid, thread: Option<Ulid>) -> MessageRefusal {
        let Response::Refused { error } = self.reply(parent, thread) else {
            panic!("unknown or invisible thread must refuse")
        };
        assert_eq!(error.kind, "unknown_thread");
        assert!(!error.message.is_empty());
        assert!(!error.hint.is_empty());
        error
    }
    fn effects(&self) -> (i64, i64) {
        let ledger = SqliteStore::open(
            &object_dir(self.root.path(), &format!("swarm-{}", self.caller.swarm)).unwrap(),
        )
        .unwrap();
        let messages = ledger
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM msg_messages", [], |row| row.get(0))
            .unwrap();
        let counters = self
            .account
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM msg_counters WHERE seat=?1",
                [self.caller.id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        (messages, counters)
    }
}

// keeps: messaging-attention §5 unknown_thread, visible hints and zero effects.
#[test]
fn unknown_reply_refuses_with_a_visible_hint_before_storing_or_signing() {
    let mut f = Fixture::new();
    let visible = f.send(f.peer.id, f.caller.id, "visible root", 1_000);
    let before = f.effects();
    let error = f.refusal(Ulid::new(), None);
    assert!(error.hint.contains(&visible.to_string()));
    assert_eq!(f.effects(), before);
}

// keeps: a private thread id is not confirmed or leaked by the error hint.
#[test]
fn invisible_reply_uses_the_same_refusal_without_leaking_its_id() {
    let mut f = Fixture::new();
    let visible = f.send(f.peer.id, f.caller.id, "visible root", 1_000);
    let hidden = f.send(f.hidden.id, f.hidden_peer.id, "private root", 1_001);
    let before = f.effects();
    let missing = f.refusal(Ulid::new(), None);
    let invisible = f.refusal(hidden, None);
    assert_eq!(invisible, missing);
    assert!(invisible.hint.contains(&visible.to_string()));
    assert!(!invisible.hint.contains(&hidden.to_string()));
    assert_eq!(f.effects(), before);
}

// keeps: a mismatched thread refuses while a valid reply preserves the real root.
#[test]
fn mismatched_thread_refuses_but_a_visible_parent_still_routes_and_inherits() {
    let mut f = Fixture::new();
    let visible = f.send(f.peer.id, f.caller.id, "visible root", 1_000);
    let before = f.effects();
    f.refusal(visible, Some(Ulid::new()));
    assert_eq!(f.effects(), before);
    let Response::Stored { message } = f.reply(visible, None) else {
        panic!("valid reply")
    };
    assert_eq!(message.view().reply_to, Some(visible));
    assert_eq!(message.view().thread_id, visible);
    assert_eq!(message.view().context, format!("seats/{}/inbox", f.peer.id));
}

// keeps: open retained threads remain answerable after application acknowledgement.
#[test]
fn acknowledged_thread_tips_stay_visible_and_only_the_latest_tip_is_suggested() {
    let mut f = Fixture::new();
    let root = f.send(f.peer.id, f.caller.id, "visible root", 1_000);
    f.messaging
        .acknowledge(f.caller.id, root, None, 1_001)
        .unwrap();
    let Response::Stored { message } = f.reply(root, None) else {
        panic!("valid reply")
    };
    let latest = message.view().id;
    let error = f.refusal(Ulid::new(), None);
    assert!(error.hint.contains(&latest.to_string()));
    assert!(
        !error.hint.contains(&root.to_string()),
        "one latest tip per actual chain"
    );
}

// keeps: an old sender-only cache migrates without changing the retained reply chain.
#[test]
fn legacy_thread_cache_reopens_with_the_same_visible_tip_and_inherited_root() {
    let mut f = Fixture::new();
    let root = f.send(f.peer.id, f.caller.id, "visible root", 1_000);
    let Response::Stored { message } = f.reply(root, None) else {
        panic!("valid reply")
    };
    let tip = message.view().id;
    drop(f.messaging);
    let ledger = SqliteStore::open(
        &object_dir(f.root.path(), &format!("swarm-{}", f.caller.swarm)).unwrap(),
    )
    .unwrap();
    ledger.connection().unwrap().execute_batch(
        "ALTER TABLE msg_thread_roots RENAME TO saved_thread_roots;
         CREATE TABLE msg_thread_roots(message INTEGER PRIMARY KEY REFERENCES msg_messages(seq),sender TEXT NOT NULL);
         INSERT INTO msg_thread_roots SELECT message,sender FROM saved_thread_roots;
         DROP TABLE saved_thread_roots;",
    ).unwrap();
    f.messaging = Messaging::open(
        f.root.path(),
        Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[70; 32]))),
    )
    .unwrap();
    let error = f.refusal(Ulid::new(), None);
    assert!(error.hint.contains(&tip.to_string()));
    assert!(!error.hint.contains(&root.to_string()));
    let Response::Stored { message } = f.reply(tip, None) else {
        panic!("migrated reply remains valid")
    };
    assert_eq!(message.view().thread_id, root);
    assert_eq!(message.view().reply_to, Some(tip));
}

// keeps: hints expose at most eight recent visible chains, newest first.
#[test]
fn hints_bound_the_recent_visible_thread_tips_to_eight() {
    let mut f = Fixture::new();
    let roots: Vec<_> = (0..10)
        .map(|n| f.send(f.peer.id, f.caller.id, "another root", 1_000 + n))
        .collect();
    let error = f.refusal(Ulid::new(), None);
    for root in &roots[..2] {
        assert!(!error.hint.contains(&root.to_string()));
    }
    let mut prior = 0;
    for root in roots[2..].iter().rev() {
        let at = error.hint.find(&root.to_string()).expect("visible tip");
        assert!(at > prior);
        prior = at;
    }
}
