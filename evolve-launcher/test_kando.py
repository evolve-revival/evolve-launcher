#!/usr/bin/env python3
"""
Kando auth-flow smoke-test against the local Revival server.

Usage:
  1. pnpm tauri dev  ->  click Launch  (starts server, patches ca-bundle.crt)
  2. python3 test_kando.py

Tests the same request sequence the game makes, using the same hostname
resolution (/etc/hosts -> 127.0.0.1) and the same TLS trust chain (ca-bundle.crt).
"""

import json, ssl, sys, urllib.request
from urllib.error import URLError

CACERT    = "/home/navitank/Desktop/EvolveFilesLegacy/ca-bundle.crt"
STEAM_ID  = "76561198385128404"
FAKE_2K   = "2357d522a223d8d57b05071505274b6b"  # from Pinenut auth logs

# ── helpers ────────────────────────────────────────────────────────────────────

def _ctx():
    c = ssl.create_default_context(cafile=CACERT)
    return c

def post(url, body=None, raw=False):
    data = body if isinstance(body, bytes) else (json.dumps(body).encode() if body else b"{}")
    ctype = "application/octet-stream" if raw else "application/json"
    req = urllib.request.Request(url, data=data, headers={"Content-Type": ctype})
    try:
        with urllib.request.urlopen(req, context=_ctx(), timeout=5) as r:
            return json.loads(r.read())
    except URLError as e:
        return {"_error": str(e)}
    except Exception as e:
        return {"_error": f"parse error: {e}"}

def get(url):
    req = urllib.request.Request(url)
    try:
        with urllib.request.urlopen(req, context=_ctx(), timeout=5) as r:
            return json.loads(r.read())
    except URLError as e:
        return {"_error": str(e)}

PASS = "\033[32m  OK  \033[0m"
FAIL = "\033[31m  FAIL\033[0m"

def check(name, resp, *keys):
    if "_error" in resp:
        print(f"{FAIL} {name}: {resp['_error']}")
        return False
    resp_str = json.dumps(resp)
    for key in keys:
        if key not in resp_str:
            print(f"{FAIL} {name}: missing '{key}'")
            print(f"       got: {resp_str[:300]}")
            return False
    print(f"{PASS} {name}")
    return True

def svc(services, name, fallback):
    inst = services.get(name)
    if not inst:
        return fallback
    return f"https://{inst['host']}/{inst['baseUri']}"

# ── test sequence ──────────────────────────────────────────────────────────────

print("\n══ kando smoke-test ══════════════════════════════════════\n")

# 1. Doorman
print("[1] Doorman")
dr = post("https://doorman.my.2k.com/doorman/1", {
    "params": {
        "serviceNames": ["doorman","content","singlesignon","sessions","apps",
                         "entitlements","players","storage","storefront",
                         "telemetry","stats","news"],
        "doormanInfo": {"firstPartyPlayerId": "00000000000000000110000116a75abc"},
        "locale": "en-US",
    },
    "header": {
        "appContext": 1002, "appVersion": "1.8.1.3",
        "action": "configs.generate", "actionVersion": 1,
        "appPublicId":    "9335e64893401fa9f8e69e8e92853b9c",
        "appSecret":      "6a6a3e33e01167454dd083726347a5a1",
        "doormanSecret":  "e285811a70b8e99e24d27eab91702502",
    },
})
ok = check("doorman/1", dr, "services", "clientConfigSettings")

services = {}
for s in dr.get("services", []):
    services[s["serviceName"]] = s["serviceInstances"][0]
if services:
    for n, i in services.items():
        print(f"       {n:16s}  https://{i['host']}/{i['baseUri']}")

# 2. Auth
print("\n[2] Auth")
# kando sends a binary protobuf — our extractor scans for the literal bytes
# "2k_player_id" followed by a 32-char hex UUID somewhere in the body.
fake_body = b"\x0a\x20" + FAKE_2K.encode() + b"\x12\x0c" + b"2k_player_id" + FAKE_2K.encode()
auth_base = svc(services, "players", "https://api.my.2k.com/players/1")
ar = post(f"{auth_base}/{STEAM_ID}/auth/two_k", fake_body, raw=True)
ok = check("auth/two_k", ar, "playerId", "accessToken") and ok
player_id = (ar.get("result") or {}).get("playerId", STEAM_ID)

# 3. Profile
print("\n[3] Profile")
pr = post(f"{auth_base}/{STEAM_ID}/profile/get_by_platform_account_id",
          {"guid": FAKE_2K, "2k_player_id": FAKE_2K})
ok = check("profile/get", pr, "player") and ok

# 4. Entitlements
print("\n[4] Entitlements")
eb = svc(services, "entitlements", "https://entitlements.my.2k.com/entitlements/1")
GROUP = "c3dc178f670ee769fe59e244610d66e2"
er = post(f"{eb}/checkAppOwnership/{GROUP}", {})
ok = check("checkAppOwnership", er, "ownsApp") and ok

gr = post(f"{eb}/grants/find", {"appGroupId": GROUP, "playerId": player_id})
ok = check("grants/find", gr, "grants") and ok

# 5. SSO
print("\n[5] SSO")
sb = svc(services, "singlesignon", "https://sso.my.2k.com/sso/1")
sr = post(f"{sb}/auths.logon", {"steamTicket": "AAAA", "locale": "en-US"})
# We don't know the exact path kando uses — fallback stub is fine
check("sso logon (stub ok)", sr)

# 6. Sessions
print("\n[6] Sessions")
ssb = svc(services, "sessions", "https://api.my.2k.com/sessions/1")
sess = post(f"{ssb}/create", {"playerId": player_id, "platformType": 1})
ok = check("sessions/create", sess, "sessionId") and ok

hb = post(f"{ssb}/heartbeat", {"sessionId": "test"})
check("sessions/heartbeat (stub ok)", hb)

# 7. Stats
print("\n[7] Stats")
stb = svc(services, "stats", "https://stats.my.2k.com/stats/1")
st = get(f"{stb}/configs")
check("stats/configs (stub ok)", st)

print(f"\n══ {'ALL PASS' if ok else 'SOME FAILED'} ═════════════════════════════════════════\n")
sys.exit(0 if ok else 1)
