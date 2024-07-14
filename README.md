# solana-escrow

This repository contains a sample implementation of an escrow system on the Solana blockchain.

## Overview

The general concept of escrow is illustrated in the following diagram:

![General Escrow Concept](https://github.com/user-attachments/assets/e879329d-3971-4db2-a14e-c648202c9ee0)

Here's how this escrow concept is implemented on Solana:

![Solana Escrow Implementation](https://github.com/user-attachments/assets/7b35aaa6-7698-44a1-97e6-d21e835527ef)

This repository includes the escrow program, client SDKs, and CLI tools. Feel free to explore the code to understand the
Solana-specific implementation details.

## Getting Started

### Deploying the Program

To build and deploy the escrow program:

```bash
$ cargo build-sbf
$ solana program deploy --program-id target/deploy/escrow_program-keypair.json target/deploy/escrow_program.so
```

### Using the Client

To create an escrow account:

```bash
$ cargo run --bin escrow-cli -- --escrow-program-id $(solana address -k target/deploy/escrow_program-keypair.json) [SEND_MINT_TOKEN_ADDRESS] 1 [RECEIVE_MINT_TOKEN_ADDRESS]
```

This will output the created escrow account address.

To interact with an existing escrow account:

```bash
$ cargo run --bin escrow-cli -- --escrow-program-id $(solana address -k target/deploy/escrow_program-keypair.json) [ESCROW_ACCOUNT_ADDRESS]
```

## Further Reading

For a detailed explanation of this implementation, check out the following resource:

- [Solana Builders Handbook - Introduction to Escrow (JA)](https://github.com/k-kinzal/solana-escrow-book)

This book provides in-depth information about the escrow implementation and can help answer any questions you might
have.
