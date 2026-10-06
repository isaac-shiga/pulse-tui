# pulse-tui

Try the [Pulse](https://pulseshiga.io) API from your terminal. Get a live quote, buy or sell USDT and USDC with naira, and follow the order until it settles.

## Install

```sh
mise use -g github:isaac-shiga/pulse-tui
```

Or build from source:

```sh
cargo install --git https://github.com/isaac-shiga/pulse-tui --locked
```

Binaries for Linux, macOS, and Windows are on the [releases page](https://github.com/isaac-shiga/pulse-tui/releases).

## Run

```sh
PULSE_TEST_KEY=<your test key> pulse-tui
```

You can also add keys under Settings. The app starts in test mode, where reserved addresses and accounts play out each ending without real money. Press `e` on the home screen to switch to live. Live orders need a second `enter` to place.

Settings live in `~/.config/pulse-tui/config.json`, or `%APPDATA%\pulse-tui` on Windows.

## Keys

| Key | Action |
| --- | --- |
| `↑` `↓` | Move between fields |
| `←` `→` | Change a choice or move the cursor |
| `enter` | Continue |
| `esc` | Back |
| `c` | Copy the highlighted value on an order |
| `s` | Save the payer or recipient after an order |
| `ctrl+c` | Quit |

## License

[MIT](LICENSE)
