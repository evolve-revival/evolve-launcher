#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <cstdint>

static HMODULE g_real = nullptr;

BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH)
        g_real = LoadLibraryA("steam_api64_real.dll");
    return TRUE;
}

// Intercept 1: always report all DLCs as installed
extern "C" __declspec(dllexport)
bool __cdecl SteamAPI_ISteamApps_BIsDlcInstalled(void* /*self*/, uint32_t /*dlcId*/) {
    return true;
}

// Intercept 2: always report the game as owned
extern "C" __declspec(dllexport)
bool __cdecl SteamAPI_ISteamApps_BIsSubscribedApp(void* /*self*/, uint32_t /*appId*/) {
    return true;
}

// Intercept 3: report Evolve's real App ID so game-internal logic is unaffected
extern "C" __declspec(dllexport)
uint32_t __cdecl SteamAPI_ISteamUtils_GetAppID(void* /*self*/) {
    return 273350u;
}
