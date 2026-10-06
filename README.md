# pulse-tui

A terminal app for moving between stablecoins and local currencies with live Pulse quotes.

## Install

With [mise](https://mise.jdx.dev):

```sh
mise use -g github:isaac-shiga/pulse-tui
```

The repository is private, so mise needs a `GITHUB_TOKEN` that can read it.

Each [release](https://github.com/isaac-shiga/pulse-tui/releases) also has archives for Linux and macOS on x86_64 and arm64, with a `SHA256SUMS` file. To build from source:

```sh
cargo install --git https://github.com/isaac-shiga/pulse-tui --locked
```

## Use

Run `pulse-tui`.

Add your keys under Settings (`4`), or set `PULSE_TEST_KEY` and `PULSE_LIVE_KEY`. Press `e` on the home screen to switch between test and live.

## Flows

- **Buy stablecoins:** Amount, your details, wallet, review, track.
- **Sell stablecoins:** Amount, local currency account, recipient, review, track.

In test mode, the wallet and local currency account steps offer sandbox outcomes. The app fills in the reserved address or account number. Live orders need a second `enter` on Review.

After an order, press `s` to save the payer or recipient. Saved profiles, keys and settings live in `~/.config/pulse-tui/config.json`, readable by your user only.

## Keys

| Key | Action |
| --- | --- |
| `↑` `↓` / `tab` | Move between fields |
| `←` `→` | Change a choice, move the cursor in a text field, or switch between typing the send or receive amount |
| `enter` | Continue |
| `esc` | Back |
| `c` | On an order, copy the highlighted value. Pick it with `↑` `↓` |
| `delete` | Clear the focused field |
| `ctrl+c` | Quit |
