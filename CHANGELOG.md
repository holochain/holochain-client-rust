# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[Unreleased\]

### Added
### Changed
### Fixed
### Removed

## 2024-04-24: v0.5.0-dev.32
### Added
- New admin call `issue_app_auth_token` which allows you to issue an app auth token. This is now required when creating
  an app websocket connection. See the example for `AppWebsocket::connect` for how to use this.
- Missing app interface function `list_wasm_host_functions` has been added.

### Changed
- **BREAKING**: The admin call `list_app_interfaces` now returns a `Vec<AppInterfaceInfo>` instead of a `Vec<u16>`. You can map the response to a `Vec<u16>` to get the previous result.
- **BREAKING**: The admin call `attach_app_interface` now takes an additional parameter of type `InstalledAppId` which allows you to restrict an app interface to a single app.
- **BREAKING**: The app call `app_info` no longer takes an `installed_app_id` parameter. The conductor uses the app that you have authenticated with.

### Fixed
### Removed
- **BREAKING**: The old `AppWebsocket` is gone, its functionality was merged into the `AppAgentWebsocket` which has been renamed to `AppWebsocket`.

## 2024-03-27: v0.5.0-dev.31
### Changed
- **BREAKING**: The underlying package `holochain_websocket` changed. All websockets in this client follow the new `connect` function and take a socket address that implements `ToSocketAddr` instead of a URL `String`. Examples for the new parameter are `"localhost:30000"` and `([127.0.0.1], 30000)`. See trait [`ToSocketAddr`](https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html#tymethod.to_socket_addrs).
- **BREAKING**: The `attach_app_interface` method of the `AdminWebsocket` now takes an additional parameter of type `AllowedOrigins` which specifies what origins are allowed to connect to the created app interface.

## 2024-03-11: v0.5.0-dev.30
### Changed
- **BREAKING**: The underlying package `holochain_websocket` changed. All websockets in this client follow the new `connect` function and take a socket address that implements `ToSocketAddr` instead of a URL `String`. Examples for the new parameter are `"localhost:30000"` and `([127.0.0.1], 30000)`. See trait [`ToSocketAddr`](https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html#tymethod.to_socket_addrs).

## 2024-03-04: v0.5.0-dev.29
### Removed
- **BREAKING**: The utilities crate, it is now replaced by signing built into the client. Please see the updated tests for examples of how to use this.
- **BREAKING**: `sign_zome_call_with_client` which was used internally but also exposed in the public interface. You probably don't need to call this but if you wish to for some reason then use one of the two new `*Signer` types, and convert them to a `Arc<Box<dyn AgentSigner>>`, then use the `sign` method to compute a signature. The logic to prepare the data to be signed is no longer public so you would have to set this up yourself following the `sign_zome_call` function in the `signer` module.

### Added
- Capability to create zome call signing credentials with the `AdminWebsocket` using `authorize_signing_credentials`.
- `ClientAgentSigner` type which can store (in memory) signing credentials created with `authorize_signing_credentials`.
- `LairAgentSigner` which is analagous to the `ClientAgentSigner` but is a wrapper around a Lair client instead so that private keys are stored in Lair.
- `from_existing` method to the `AppAgentWebsocket` which allows it to wrap an existing `AppWebsocket` instead of having to open a new connection. This is useful if you already have an `AppWebsocket` but otherwise you should just use the `connect` method of the `AppAgentWebsocket` rather than two steps.

### Changed
- **BREAKING**: `AppAgentWebsocket::connect` now takes an `Arc<Box<dyn AgentSigner>>` instead of a `LairClient`. The `Arc<Box<dyn AgentSigner>>` can be created from a `.into()` on either a `ClientAgentSigner` or a `LairAgentSigner`. Use the latter to restore the previous behaviour.
- **BREAKING**: `AppAgentWebsocket::call_zome` used to take a `RoleName` as its first parameter. This is now a `ZomeCallTarget`. There is a `.into()` which restores the previous behaviour. Now you can also pass a `CloneCellId` or a `CellId`, also using a `.into()`. Using `CellId` is stronly recommended for now. Please see the doc comments on `ZomeCallTarget` if you intend to use the other options.

## 2024-02-29: v0.5.0-dev.28
### Added
- Export `AdminWebsocket::EnableAppResponse` to be available downstream.

## 2024-02-01: v0.5.0-dev.27
### Added
- Added the `update_coordinators` call in the `AdminWebsocket`.

## 2024-01-26: v0.5.0-dev.26
### Added
- `AppAgentWebsocket` as an app websocket tied to a specific app and agent. Recommended for most applications.
- `on_signal`: event handler for reacting to app signals; implemented on `AppWebsocket` and `AppAgentWebsocket`.
### Changed
- Bump deps to holochain-0.3.0-beta-dev.26

## 2023-11-23: v0.5.0-dev.25
### Changed
- Bump deps to holochain-0.3.0-beta-dev.25

## 2023-11-15: v0.5.0-dev.24
### Changed
- Bump deps to holochain-0.3.0-beta-dev.24

## 2023-11-02: v0.5.0-dev.23
### Changed
- Bump deps to holochain-0.3.0-beta-dev.23

## 2023-10-20: v0.5.0-dev.0
### Changed
- Bump deps to holochain-0.3.0-beta-dev.22

## 2024-04-17: v0.4.11
### Changed
- Remove expectation of an IPv4 address and allow for IPv4 and IPv6.

## 2024-04-09: v0.4.10
### Added
- Signal handling on app websocket.
### Changed
- Update deps to Holochain v0.2.7.
- Adapt admin and app websocket to new version of package `holochain_websocket`.

## 2024-03-05: v0.4.8
### Added
- Capability to create zome call signing credentials with the `AdminWebsocket` using `authorize_signing_credentials`.
- `ClientAgentSigner` type which can store (in memory) signing credentials created with `authorize_signing_credentials`.
- `AppAgentWebsocket` to simplify making zome calls. It is a wrapper around a `AppWebsocket` but can be created directly.
### Removed
- The utilities crate, it is now replaced by signing built into the client. Please see the updated tests for examples of how to use this.

## 2024-02-29: v0.4.7
### Added
- Export `AdminWebsocket::EnableAppResponse` to be available downstream.

## 2024-02-02: v0.4.7-rc.2
### Changed
- Bump deps to Holochain v0.2.6-rc.0

## 2024-01-26: v0.4.7-rc.1
### Changed
- Bump deps to Holochain v0.2.5-rc.1

## 2024-01-20: v0.4.7-rc.0
### Changed
- Bump deps to Holochain v0.2.5-rc.0

## 2023-12-11: v0.4.6-rc.0
### Changed
- Bump deps to Holochain v0.2.4-rc.0

## 2023-11-23: v0.4.5
### Changed
- Bump deps to Holochain v0.2.3

## 2023-11-15: v0.4.5-rc.2
### Changed
- Bump deps to Holochain v0.2.3-rc.1

## 2023-11-08: v0.4.5-rc.1
### Changed
- Bump deps to Holochain v0.2.3-rc.0

## 2023-10-11: v0.4.5-rc.0
### Changed
- Remove unreachable code in `AppWebsocket::send`.
- Bump deps to holochain-0.2.3-beta-rc.0
### Fixed
- Upgrade to security patched version of `webpki`.

## 2023-10-02: v0.4.4
### Changed
- Pin serde to max v1.0.166 properly.

## 2023-09-28: v0.4.3
### Changed
- Pin serde to v1.0.166
- Upgrade holochain_serialized_bytes to v0.0.53

## 2023-09-13: v0.4.2
### Changed
- Upgrade to Holochain v0.2.2.

## 2023-09-11: v0.4.2-rc.3
### Changed
- Upgrade to Holochain v0.2.2-beta-rc.3.

## 2023-08-31: v0.4.2-rc.0
### Changed
- Upgrade to Holochain v0.2.2-beta-rc.0.

## 2023-08-07: v0.4.1
### Added
- Admin API call `graft_records`.
### Changed
- Upgrade to Holochain v0.2.1.

## 2023-04-21: v0.4.0
### Added
- Add `storage_info` to the admin websocket.
- Add `network_info` to the app websocket.
### Changed
- **BREAKING CHANGE**: Upgrade to Holochain 0.2 release candidate ahead of the holochain 0.2 release.

## 2023-02-15: v0.3.1
### Changed
- Upgrade to latest Holochain dependencies.
- Switch to Nix flake for develop environment. Run `nix develop` from now on instead of `nix-shell`. Pass on `--extra-experimental-features nix-command --extra-experimental-features flakes` or enable these features for your user in [`~/.config/nix/nix.conf`](https://nixos.org/manual/nix/stable/command-ref/conf-file.html#conf-experimental-features).

## 2023-01-23: v0.3.0
### Added
- Admin API call `get_dna_definition`
- Utility crate for authorizing credentials and signing zome calls
### Changed
- **BREAKING CHANGE**: Upgrade to Holochain 0.1.0-beta-rc.3
- **BREAKING CHANGE**: Require all zome calls to be signed.
- **BREAKING CHANGE**: Rename `install_app_bundle` to `install_app`.
- **BREAKING CHANGE**: Rename `archive_clone_cell` to `disable_clone_cell`.
- **BREAKING CHANGE**: Rename `restore_archived_clone_cell` to `enable_clone_cell`.
- **BREAKING CHANGE**: Move `enable_clone_cell` to App API.
- **BREAKING CHANGE**: Refactor `delete_clone_cell` to delete a single disabled clone cell.
- **BREAKING CHANGE**: Refactor `app_info` to return all cells and DNA modifiers.
- **BREAKING CHANGE**: Rename `request_agent_info` to `agent_info`.

## 2022-10-03: v0.2.0

Compatible with Holochain >= v0.0.165

### Added
- Added calls for clone cell management:
    - App API: create clone cell
    - App API: archive clone cell
    - Admin API: restore clone cell
    - Admin API: delete archived clone cells
- Added test fixture and tests for clone cells calls

### Changed
- Upgrade to Holochain v0.0.165

## 2022-08-18: v0.1.1

### Changed
- Upgrade to Holochain v0.0.154

## 2022-01-20: v0.1.0

### Changed
- Upgrade to latest Holochain v0.0.147

## 2022-01-20: v0.0.1

### Added
- Initial release & publication as a crate
