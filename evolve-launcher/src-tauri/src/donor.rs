// evolve-launcher/src-tauri/src/donor.rs

/// Steam App ID of the free donor game used for SDR authentication.
/// Must be free-to-own on Steam and have ISteamNetworkingSockets SDR enabled.
/// SpaceWar (480) — Valve's official Steamworks SDK sample app, available to
/// every Steam account via steam://run/480. Intended for exactly this use case.
pub const DONOR_APP_ID: u32 = 480;

/// The filename of the real Steamworks DLL after we rename it.
pub const REAL_STEAM_API_DLL: &str = "steam_api64_real.dll";

/// Evolve's actual App ID — returned by shim's GetAppID() intercept.
pub const EVOLVE_APP_ID: u32 = 273350;
