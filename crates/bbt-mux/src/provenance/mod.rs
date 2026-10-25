//! Local observations and artifact responsibility. A signer attests an
//! observation; inferred attribution never becomes a claim of authorship.

pub(crate) mod projection;

pub(crate) mod author;
pub(crate) mod artifacts;
pub(crate) mod detector;
pub(crate) mod engine;
pub(crate) mod git;

#[cfg(test)]
mod tests {
    use crate::{
        bus::{Broker, BusHandle, Consumer, Delivery},
        index::Index,
        store::{events::Draft, sqlite::SqliteStore},
    };
    use ed25519_dalek::SigningKey;
    use serde_json::json;

    // keeps: spec provenance
    #[test]
    fn index_and_bus_share_one_order_and_keep_each_actual_signer() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let objects = temp.path().join("objects");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let leaf = SigningKey::from_bytes(&[73; 32]);
        let leaf_public = leaf.verifying_key().to_bytes();
        let broker = SigningKey::from_bytes(&[74; 32]);
        let broker_public = broker.verifying_key().to_bytes();
        let bus = BusHandle::start(
            runtime.handle(),
            Broker::open(&objects, broker).unwrap(),
            128,
        )
        .unwrap();
        let mut index =
            Index::open(&root, SqliteStore::open(&objects.join("index")).unwrap()).unwrap();
        index.route_activity(bus.clone(), runtime.handle().clone());
        let consumer = runtime
            .block_on(bus.subscribe(Consumer::gui("doc/>")))
            .unwrap();
        let writer = std::thread::spawn(move || {
            for n in 0..20 {
                index
                    .write(
                        &format!("{n}.md"),
                        json!({"id":crate::Ulid::new(),"type":"note"}),
                        "fixture",
                        crate::Ulid::new(),
                        &leaf,
                    )
                    .unwrap();
            }
        });
        runtime.block_on(async {
            for n in 0..20 {
                bus.publish(
                    "index",
                    Draft::new(
                        format!("doc/bus-{n}/written"),
                        "fixture/bus",
                        json!({"n":n}),
                    ),
                )
                .await
                .unwrap();
            }
        });
        writer.join().unwrap();
        let rows = runtime.block_on(bus.poll(&consumer, 128)).unwrap();
        assert_eq!(rows.len(), 60);
        let mut leaf_count = 0;
        let mut broker_count = 0;
        for (n, delivery) in rows.into_iter().enumerate() {
            let Delivery::Event {
                stream, envelope, ..
            } = delivery
            else {
                panic!("unexpected gap")
            };
            assert_eq!(stream, "index");
            assert_eq!(envelope.seq, n as u64 + 1);
            envelope.verify().unwrap();
            if envelope.provenance.signer == leaf_public {
                leaf_count += 1;
            } else {
                assert_eq!(envelope.provenance.signer, broker_public);
                broker_count += 1;
            }
        }
        assert_eq!((leaf_count, broker_count), (40, 20));
        runtime.block_on(bus.shutdown()).unwrap();
    }
}
