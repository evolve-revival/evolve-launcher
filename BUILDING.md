# Building Evolve Revival — Developer Guide

Three build targets: **evolve-pak** (Go CLI), **dbghelp.dll** (RSA key injector), **evolve_shim.dll** (Steam API shim). All cross-compiled on Linux for Windows x64.

---

## Prerequisites

```bash
# Go 1.22+
go version

# MinGW-w64 cross compiler (for both DLLs)
# Arch/Manjaro:
sudo pacman -S mingw-w64-gcc
# Debian/Ubuntu:
sudo apt install gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64

# CMake (for evolve_shim.dll only)
sudo pacman -S cmake   # or: sudo apt install cmake
```

---

## 1. evolve-pak

Go CLI for pak inspection, key discovery, and building signed `.pak` files.

```bash
cd evolve-pak/

# Build
go build -o evolve-pak ./cmd

# Or install to $GOPATH/bin
go install ./cmd

# Run tests
go test ./...
```

### Commands

```bash
# Inspect a pak (list entries, show encryption info)
./evolve-pak list Game/Game.pak

# Find the RSA public key in a game binary
./evolve-pak keyfind bin64_SteamRetail/Evolve.exe

# Generate your own RSA-1024 keypair
./evolve-pak keygen --out-dir ~/my-keys

# Pack a directory into a signed .pak
./evolve-pak pack --privkey ~/my-keys/revival.priv ./mod-dir ./MyMod.pak

# Audit pak encryption integrity
./evolve-pak audit Game/Game.pak
```

---

## 2. dbghelp.dll — RSA Key Injector

Patches the vanilla Turtle Rock RSA-1024 public key in Evolve's process memory at startup, replacing it with the `revival.pub` key. This lets the game load `.pak` files signed with the revival keypair instead of TRS's original (lost) key.

**Mechanism:** Windows loads `dbghelp.dll` from the application directory before `System32`. Dropping our DLL into `bin64_SteamRetail/` causes it to load on game start. It scans all readable memory pages for the 140-byte vanilla PKCS#1 DER key pattern and overwrites it with `revival.pub` using `VirtualProtect`.

```bash
cd injector/

# Build (cross-compile on Linux)
make

# Output: dbghelp.dll (117 KB)
```

### Deploy

```bash
cp injector/dbghelp.dll   /path/to/Evolve/bin64_SteamRetail/
cp ~/my-keys/revival.pub  /path/to/Evolve/bin64_SteamRetail/

# Logs written to bin64_SteamRetail/revival_inject.log on startup
```

If `revival.pub` is missing or the key pattern isn't found, the DLL logs to `revival_inject.log` and exits cleanly — the game runs normally with vanilla paks.

---

## 3. evolve_shim.dll — Steam API Shim

Sits between `Evolve.exe` and the real Steam client. Deployed as `steam_api64.dll` in `bin64_SteamRetail/`, with the real Steam library renamed to `steam_api64_real.dll`.

**What it intercepts:**
- `SteamAPI_ISteamApps_BIsDlcInstalled` → always `true` (all DLC owned)
- `SteamAPI_ISteamApps_BIsSubscribedApp` → always `true`
- `SteamAPI_ISteamUtils_GetAppID` → returns `273350` (Evolve's real app ID for internal logic)
- `getaddrinfo` / `WinHttpConnect` / `WinHttpOpenRequest` → redirects `*.my.2k.com` to `evolve.navitank.org` (the revival backend)

All other `SteamAPI_*` calls are forwarded to `steam_api64_real.dll`, so real Steam P2P / SDR networking (lobby creation, NAT traversal) works normally with app ID 480 (SpaceWar).

```bash
cd evolve-shim/

# Configure and build
cmake -B build -DCMAKE_TOOLCHAIN_FILE=mingw-w64-x86_64.cmake
cmake --build build

# Output: build/evolve_shim.dll
```

Or one-liner without CMake:

```bash
x86_64-w64-mingw32-g++ -O2 -Wall -shared \
    -o build/evolve_shim.dll shim.cpp shim.def
```

### Deploy

```bash
# Rename the real Steam library
mv bin64_SteamRetail/steam_api64.dll bin64_SteamRetail/steam_api64_real.dll

# Drop the shim in its place
cp evolve-shim/build/evolve_shim.dll bin64_SteamRetail/steam_api64.dll
```

---

## Quick Demo: Build + Pack a Mod

```bash
# 1. Build tools
cd evolve-pak && go build -o evolve-pak ./cmd && cd ..
cd injector && make && cd ..

# 2. Generate a personal keypair (keep revival.priv secret)
./evolve-pak/evolve-pak keygen --out-dir ~/my-keys

# 3. Create a test mod (e.g. a patched config)
mkdir -p /tmp/testmod/Engine/Config
echo "[SystemSettings]" > /tmp/testmod/Engine/Config/BaseSystemSettings.ini

# 4. Pack it
./evolve-pak/evolve-pak pack \
    --privkey ~/my-keys/revival.priv \
    /tmp/testmod \
    /tmp/TestMod.pak

# 5. Deploy
cp injector/dbghelp.dll         /path/to/Evolve/bin64_SteamRetail/
cp ~/my-keys/revival.pub        /path/to/Evolve/bin64_SteamRetail/
cp /tmp/TestMod.pak             /path/to/Evolve/Game/

# 6. Launch Evolve — dbghelp.dll patches the key at startup,
#    the game loads TestMod.pak with your revival key.
#    Check bin64_SteamRetail/revival_inject.log to confirm.
```

---

## Notes

- **Revival keypairs are personal** — each mod author generates their own. The `dbghelp.dll` reads `revival.pub` from disk at startup, so you can swap keys without recompiling the DLL.
- **Signing header (bytes 6–139 of EOCD comment)** — currently written as zeros. The game may or may not validate this. If it rejects unsigned paks, a second memory patch for the signature check will be needed (see `docs/superpowers/specs/2026-06-13-pack-injector-design.md` §6).
- **The revival backend** (`evolve.navitank.org`) handles auth, entitlements, and player storage. You don't need to run your own server to use the pack tools.
