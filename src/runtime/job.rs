use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use base64::Engine;

use crate::api::Client;
use crate::app::{Job, Msg};

pub fn spawn(job: Job, client: Client, sender: Sender<Msg>) {
    if let Job::Copy(text) = job {
        copy(&text);
        return;
    }

    thread::spawn(move || {
        let message = match job {
            Job::Quote { seq, req } => Msg::Quote {
                seq,
                res: client.create_quote(&req),
            },
            Job::Refresh { seq, id } => Msg::Quote {
                seq,
                res: client.refresh_quote(&id),
            },
            Job::Banks => Msg::Banks(client.banks()),
            Job::Resolve {
                seq,
                bank_code,
                account,
            } => Msg::Resolved {
                seq,
                res: client.resolve(&bank_code, &account),
            },
            Job::Create(request) => Msg::Created(client.create_order(&request)),
            Job::Order { target, id } => Msg::Order {
                target,
                res: client.order(&id),
                id,
            },
            Job::Orders { kind, cursor } => Msg::Orders {
                append: cursor.is_some(),
                res: client.orders(kind, cursor.as_deref()),
            },
            Job::Copy(_) => unreachable!(),
        };
        let _ = sender.send(message);
    });
}

/// Uses `pbcopy` on macOS, and the OSC 52 escape elsewhere.
fn copy(text: &str) {
    let piped = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(text.as_bytes())?;
            child.wait()
        })
        .is_ok_and(|status| status.success());

    if !piped {
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        let mut output = io::stdout();
        let _ = write!(output, "\x1b]52;c;{encoded}\x07");
        let _ = output.flush();
    }
}
