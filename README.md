# Financial Discloser

The Financial Discloser is a Rust-based application dedicated to downloading financial disclosure reports in PDF format and generating an index based on available data from specific government repositories. This project is useful for researchers and analysts who need organized access to these documents for further analysis.

_This readme was gen'd with AI. I'm tired and haven't had time to write a proper one. bite me_

## Table of Contents
1. [Features](#features)
2. [Installation](#installation)
3. [Usage](#usage)
4. [Configuration](#configuration)
5. [Logging](#logging)
6. [VPN Rotation](#vpn-rotation)
7. [Error Tracking](#error-tracking)
8. [License](#license)

## Features

- **Download Financial Disclosure Reports:** Automates the downloading of financial disclosure reports in PDF format.
- **Index Generation:** Automatically fetches and parses XML indexes from government repositories, converting them into an easily usable YAML format.
- **Resilient Downloading:** Implements retry logic and VPN rotation for robust downloading.
- **Progress Tracking:** Provides real-time progress updates for the downloading process.
- **Error Tracking:** Integrates with Sentry for comprehensive error tracking.

## Installation

### Prerequisites

- Rust toolchain (https://rustup.rs/)
- [reqwest](https://crates.io/crates/reqwest) crate
- [serde](https://crates.io/crates/serde) crate
- [tokio](https://crates.io/crates/tokio) crate
- VPN rotation scripts (`connect_vpn.sh`, `rotate_vpn.sh`)

### Steps

1. Clone the repository:
    ```bash
    git clone https://github.com/yourusername/financial_discloser.git
    cd financial_discloser
    ```

2. Install required crates by building the project:
    ```bash
    cargo build
    ```

## Usage

### Running the Application

Execute the following command in the root directory of the project to start the entire process:
```bash
cargo run
```

This will initiate the index fetching, parsing, and PDF downloading processes.

### Relevant Scripts

- `connect_vpn.sh`: Connects to the VPN before initiating the downloads.
- `rotate_vpn.sh`: Rotates the VPN server, used for retry logic when downloads fail.

Place these scripts in a `scripts` directory at the root level.

## Configuration

### Environment Variables

You can configure the application using the following environment variables:

- `LOG_LEVEL`: Set the logging level (`error`, `warn`, `info`, `debug`, `trace`). Default is `info`.
- `SENTRY_DSN`: Set your Sentry DSN for error tracking.

### Example

Set environment variables before running the application:
```bash
export LOG_LEVEL=debug
export SENTRY_DSN=https://examplePublicKey@o0.ingest.sentry.io/0
```

## Logging

The application uses `tracing` for logging. You can configure the logging level via the `LOG_LEVEL` environment variable. Logs are printed to the console.

## VPN Rotation

The application relies on the VPN scripts to maintain anonymity and bypass potential blocks:

- `connect_vpn.sh`: This script connects the machine to a VPN server before starting the downloads.
- `rotate_vpn.sh`: This script rotates the VPN server when a download fails.

Both scripts should return an error code if they fail, which will trigger the error handling logic in the Rust application.

## Error Tracking

Sentry integration is available for tracking errors. To enable Sentry, set the `SENTRY_DSN` environment variable with your Sentry DSN.

You can also send a test event to Sentry by uncommenting and calling the `send_test_event` function within the `main.rs`.

## License

This project is licensed under the AGPL-3.0 License. See the [LICENSE](LICENSE) file for more details.

---

_This readme was gen'd with AI. I'm tired and haven't had time to write a proper one. bite me_
