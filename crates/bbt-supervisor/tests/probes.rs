#![allow(clippy::disallowed_methods)] // Disposable PTY fixture, never the UI thread.
use bbt_supervisor::protocol::InputContext;
use bbt_supervisor::{
    Action, Attachment, Client, Config, Event, LaunchSpec, Reply, Supervisor, Ulid,
};
use std::time::{Duration, Instant};

fn output(client: &Client, seat: Ulid, needle: &[u8]) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .expect("output watchdog elapsed");
        if let Event::Output {
            seat: from,
            bytes: chunk,
            ..
        } = client.recv_timeout(remaining).unwrap()
            && from == seat
        {
            bytes.extend(chunk);
            if bytes.windows(needle.len()).any(|window| window == needle) {
                return bytes;
            }
        }
    }
}

fn fixture(config: Config) -> (Supervisor, Client, Attachment) {
    let supervisor = Supervisor::start(config).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = LaunchSpec::new("/bin/sh", std::env::current_dir().unwrap());
    spec.args = vec![
        "-c".into(),
        "stty raw -echo; printf READY; exec /bin/cat".into(),
    ];
    let info = client.spawn(spec).unwrap();
    let attachment = client.attach(info.seat, info.generation).unwrap();
    output(&client, info.seat, b"READY");
    (supervisor, client, attachment)
}

fn context(client: &Client, attachment: &Attachment) -> InputContext {
    match client
        .request(Action::InputContext {
            attachment: attachment.clone(),
        })
        .unwrap()
    {
        Reply::InputContext { context } => context,
        other => panic!("unexpected context reply: {other:?}"),
    }
}

#[test]
fn unconfigured_inputs_always_fence_the_context() {
    let (_supervisor, client, attachment) = fixture(Config::default());
    client.input(&attachment, Ulid::new(), b"draft").unwrap();
    output(&client, attachment.seat, b"draft");
    let mut before = context(&client, &attachment);
    assert!(before.draft);
    // No payload has special meaning unless the caller explicitly configures it.
    let stem = [0x1b, b']', b'7', b'5', b'0', b'1', b';', b'?'];
    for ending in [&[7][..], &[0x1b, b'\\'][..]] {
        let bytes = [stem.as_slice(), ending].concat();
        client.input(&attachment, Ulid::new(), &bytes).unwrap();
        assert_eq!(output(&client, attachment.seat, &bytes), bytes);
        let after = context(&client, &attachment);
        assert_eq!(
            after.epoch,
            before.epoch + 1,
            "unconfigured input did not fence"
        );
        assert!(after.draft);
        before = after;
    }
}
