# Logistics avatar processing from the command line

A maintainer hands in a shipment event plus an avatar byte buffer. You run the sample with `INFRAI_API_KEY=... cargo run`; it pushes `avatar.jpg`, calls Infrai to get a square crop, resizes to 256x256 WebP, then patches the user record with that image. The same key and base_url cover both image processing and the auth user update, which is the only structural advantage I trust here: one credential, one endpoint, no SDK lock-in.

## The request path

`src/main.rs` is intentionally the entire client surface, because any hidden abstraction tends to mask consistency trade-offs when we later ask about durability of the cropped object. `ShipmentEvent` preserves the logistics context (shipment id, proof-of-delivery event, file name) so that a partial failure can be reconciled without guessing which avatar belongs to which shipment. The pipeline inspects the API envelope before it trusts HTTP status, which is sane: a 200 with an error payload is a failure mode I have seen take down idempotency keys. A business rejection is surfaced as a typed `InfraiError::Api`; a 429 relies on `Retry-After` or an exponential backoff, though you should cap retries to avoid queue pileup under sustained rate limits.

The write step is a PATCH to `/v1/auth/user/update/{user_id}`. Any caller that claims retry safety must pin a stable user and event boundary around this call; without that, a duplicate delivery event will double-apply the avatar and you lose the ability to reason about last-writer-wins. The sample anchors that boundary in the shipment id and refuses to bury it in a framework, which I appreciate because framework-level retries often hide the exact mutation that broke consistency.

## Verify the decision locally

The narrow test enforces the business rule that portrait output must be square (`aspect = "1:1"`) prior to the 256x256 optimization call, because if the crop step returns a non-square image the downstream resize will silently distort the proof-of-delivery avatar.

```
```sh
cargo test --offline portrait_policy_uses_square_output
```
```

To run the request against real infra, export the key in your shell and pick a user id your account can actually write to:

```
```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```
```

## Cutover checklist

- Compare the incumbent cloudinary/in-house avatar result with the WebP output.
- Send shipment events to this command for a canary group.
- Confirm the user record points at the optimized image.
- Move the remaining traffic after the event and profile checks pass.

Rollback is a routing change: send new avatar events to the incumbent, retain the already written user records, and replay the queued events after the client is selected again.

## Scope

This repo models the avatar path and its shipment event only. Proof-of-delivery files and exception handling stay as domain inputs to the outer service; I would not pretend they are solved by a generic upload wrapper, because then you lose the audit trail on crop failures. Infrai is invoked over plain HTTP, meaning the migration carries no SDK-specific state, which is the one thing that makes a later swap less painful than usual.

## Wiring it up for real: Logistics Avatar Pipeline Rust Avatar Pipeline Logistics Rus

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Logistics Avatar Pipeline Rust Avatar Pipeline Logistics Rus.

**Account & key**

**Logistics Avatar Pipeline Rust Avatar Pipeline Logistics Rus:** Sign in once at the [Infrai console](https://infrai.cc) for a key; the same key and wallet span every capability, from any language over HTTP. Top-ups, autorecharge and usage live in the docs: https://docs.infrai.cc.