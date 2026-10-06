use std::time::{Duration, Instant};

use crate::api::Env;
use crate::api::{ApiError, CreateOrderRequest, Order, Quote, QuoteRequest, Status};
use crate::app::sandbox::{self, OUTCOMES};
use crate::app::{App, Job, Mark, Msg, Step, Tracker};
use crate::config::Config;
use crate::ui;
use ratatui::{
    Terminal,
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
};

fn app() -> App {
    let config = Config {
        test_key: "sk_test_abcd1234".into(),
        ..Config::default()
    };
    App::new(config, false)
}

fn press(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new_with_kind(
        code,
        KeyModifiers::NONE,
        KeyEventKind::Press,
    ));
}

fn type_text(app: &mut App, text: &str) {
    text.chars().for_each(|c| press(app, KeyCode::Char(c)));
}

fn screen(app: &App) -> String {
    let mut term = Terminal::new(TestBackend::new(100, 34)).unwrap();
    term.draw(|f| ui::render(f, app)).unwrap();
    let buf = term.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                + "\n"
        })
        .collect()
}

#[test]
fn home_arrow_keys_move_through_the_menu() {
    let mut app = app();

    press(&mut app, KeyCode::Down);
    assert_eq!(app.home_sel, 1);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.home_sel, 2);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.home_sel, 1);
}

#[test]
fn ui_renders_at_common_terminal_sizes() {
    let app = app();

    for (width, height) in [(60, 20), (100, 34), (140, 45)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| ui::render(frame, &app)).unwrap();
    }
}

fn quote(seq: u64) -> Msg {
    let expires = (chrono::Utc::now() + chrono::Duration::seconds(120)).to_rfc3339();
    Msg::Quote {
        seq,
        res: Ok(Quote {
            id: "q1".into(),
            rate: "1538.46".into(),
            source_amount: "150000".into(),
            destination_amount: "97.499512".into(),
            expires_at: expires,
        }),
    }
}

fn take_quote_job(app: &mut App) -> (u64, QuoteRequest) {
    match app
        .take_jobs()
        .into_iter()
        .find(|job| matches!(job, Job::Quote { .. }))
        .expect("quote request")
    {
        Job::Quote { seq, req } => (seq, req),
        _ => unreachable!(),
    }
}

/// Opens the on-ramp and gets a quote for ₦150,000.
fn onramp_with_quote() -> App {
    let mut app = app();
    press(&mut app, KeyCode::Char('1'));
    type_text(&mut app, "150000");
    app.tick(app.now + Duration::from_secs(1));
    let (seq, _) = take_quote_job(&mut app);
    app.on_msg(quote(seq));
    app
}

#[test]
fn typing_an_amount_requests_and_shows_a_quote() {
    let mut app = app();
    press(&mut app, KeyCode::Char('1'));
    type_text(&mut app, "150000");
    app.tick(app.now + Duration::from_secs(1));
    let (seq, req) = take_quote_job(&mut app);
    assert_eq!(req.source_currency, "NGN");
    assert_eq!(req.destination_currency, "USDT");
    assert_eq!(req.source_amount.as_deref(), Some("150000"));
    assert_eq!(req.destination_amount, None);

    app.on_msg(quote(seq));
    let s = screen(&app);
    assert!(s.contains("₦150,000"), "{s}");
    assert!(s.contains("97.499512 USDT"), "{s}");
    assert!(s.contains("₦1,538.46 per USDT"), "{s}");
}

#[test]
fn payer_step_rejects_a_short_nin() {
    let mut app = onramp_with_quote();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Right); // sandbox identity
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down); // NIN
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Enter);
    let s = screen(&app);
    assert_eq!(app.flow.step(), Step::Party);
    assert!(s.contains("Needs 11 digits"), "{s}");
}

#[test]
fn a_changed_price_on_create_fetches_the_new_price() {
    let mut app = onramp_with_quote();
    press(&mut app, KeyCode::Enter); // payer
    press(&mut app, KeyCode::Right); // sandbox identity
    press(&mut app, KeyCode::Enter); // wallet
    press(&mut app, KeyCode::Enter); // review
    let seq = match app.take_jobs().pop() {
        Some(Job::Refresh { seq, id }) if id == "q1" => seq,
        other => panic!("expected a quote refresh, got {other:?}"),
    };
    app.on_msg(quote(seq));
    press(&mut app, KeyCode::Enter); // place order
    let request = match app.take_jobs().pop() {
        Some(Job::Create(CreateOrderRequest::Onramp(request))) => request,
        other => panic!("expected an order request, got {other:?}"),
    };
    assert_eq!(request.destination.address, OUTCOMES[0].evm);
    assert_eq!(request.payer.nin, "12345678901");

    app.on_msg(Msg::Created(Err(ApiError::new(
        "quote_changed",
        "Quote changed",
    ))));
    assert!(matches!(
        app.take_jobs().last(),
        Some(Job::Refresh { id, .. }) if id == "q1"
    ));
    assert!(screen(&app).contains("The price changed while the order was being created."));
}

#[test]
fn a_failed_order_shows_the_states_it_reached() {
    let order = |status| Order {
        id: "o1".into(),
        reference: "r1".into(),
        kind: "onramp".into(),
        status,
        rate: None,
        source: Default::default(),
        destination: Default::default(),
        funding_account: None,
        party: None,
        expires_at: None,
        created_at: None,
    };
    let now = Instant::now();
    let mut t = Tracker::new(order(Status::AwaitingPayment), None);
    t.update(Ok(order(Status::Failed)), now);
    assert_eq!(
        t.timeline(),
        vec![
            (Status::AwaitingPayment, Mark::Done),
            (Status::Failed, Mark::Bad)
        ]
    );

    let mut t = Tracker::new(order(Status::Processing), None);
    t.update(Ok(order(Status::Failed)), now);
    assert_eq!(
        t.timeline(),
        vec![
            (Status::AwaitingPayment, Mark::Done),
            (Status::Processing, Mark::Done),
            (Status::Failed, Mark::Bad)
        ]
    );
    assert_eq!(t.next_poll, None);
}

#[test]
fn live_orders_need_a_second_enter() {
    let mut app = onramp_with_quote();
    app.config.env = Env::Live;
    app.config.live_key = "sk_live_x".into();
    let person = sandbox::identity();
    app.flow.name.set(person.name);
    app.flow.email.set(person.email);
    app.flow.nin.set(person.nin);
    app.flow.bvn.set(person.bvn);
    app.flow
        .address
        .set("0x1111111111111111111111111111111111111111");
    app.flow.step_idx = 3;
    app.take_jobs();
    press(&mut app, KeyCode::Enter);
    assert!(app.take_jobs().is_empty());
    assert!(screen(&app).contains("Press enter again"));
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.take_jobs().last(), Some(Job::Create(_))));
}

#[test]
fn an_amount_below_the_minimum_drops_the_quote() {
    let mut app = onramp_with_quote(); // ₦150,000
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Backspace); // ₦1,500
    app.tick(app.now + Duration::from_secs(1));
    press(&mut app, KeyCode::Enter);

    assert!(
        !app.take_jobs()
            .iter()
            .any(|j| matches!(j, Job::Quote { .. }))
    );
    assert!(app.flow.quote.is_none());
    assert_eq!(app.flow.step(), Step::Amount);
    assert!(screen(&app).contains("Minimum ₦15,000"));
}

#[test]
fn a_sell_waits_for_the_minimum_before_asking_for_a_price() {
    let mut app = app();
    app.banks_loading = true; // no bank list request
    press(&mut app, KeyCode::Char('2'));
    type_text(&mut app, "5");
    app.tick(app.now + Duration::from_secs(1));
    assert!(
        !app.take_jobs()
            .iter()
            .any(|j| matches!(j, Job::Quote { .. }))
    );

    type_text(&mut app, "0");
    app.tick(app.now + Duration::from_secs(2));
    let (_, req) = take_quote_job(&mut app);
    assert_eq!(req.source_amount.as_deref(), Some("50"));
}

#[test]
fn cycling_networks_sends_one_quote_request() {
    let mut app = onramp_with_quote();
    app.take_jobs();
    press(&mut app, KeyCode::Up); // network
    press(&mut app, KeyCode::Right);
    app.tick(app.now);
    press(&mut app, KeyCode::Right);
    app.tick(app.now);
    press(&mut app, KeyCode::Right);
    app.tick(app.now + Duration::from_secs(1));

    let quotes: Vec<_> = app
        .take_jobs()
        .into_iter()
        .filter_map(|job| match job {
            Job::Quote { req, .. } => Some(req),
            _ => None,
        })
        .collect();
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].network, "OPTIMISM");
}

fn order(status: Status) -> Order {
    Order {
        id: "o1".into(),
        reference: "r1".into(),
        kind: "onramp".into(),
        status,
        rate: None,
        source: Default::default(),
        destination: Default::default(),
        funding_account: None,
        party: None,
        expires_at: None,
        created_at: None,
    }
}

#[test]
fn tracking_copies_the_value_the_user_picks() {
    let mut app = onramp_with_quote();
    let mut o = order(Status::AwaitingPayment);
    o.funding_account = Some(crate::api::FundingAccount {
        account_number: Some("8012345678".into()),
        ..Default::default()
    });
    app.flow.tracker = Some(Tracker::new(o, None));
    app.flow.step_idx = 4;
    app.take_jobs();

    press(&mut app, KeyCode::Char('c')); // starts on the account number
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('c')); // order ID

    assert_eq!(
        app.take_jobs(),
        vec![Job::Copy("8012345678".into()), Job::Copy("o1".into())]
    );
}

#[test]
fn a_settled_order_does_not_wait_for_a_payment_account() {
    let mut app = onramp_with_quote();
    app.flow.tracker = Some(Tracker::new(order(Status::Completed), None));
    app.flow.step_idx = 4;

    let s = screen(&app);
    assert!(!s.contains("Opening an account"), "{s}");
    assert!(!s.contains("Send exactly"), "{s}");
}
