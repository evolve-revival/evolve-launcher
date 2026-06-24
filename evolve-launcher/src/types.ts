export interface Config {
  install_dir: string;
  server_url: string;
}

export interface Component {
  id: string;
  name: string;
  description: string;
  required: boolean;
  enabled: boolean;
  size_bytes: number;
}

export interface Tier {
  id: string;
  name: string;
  description: string;
  components: string[];
  size_bytes: number;
  recommended: boolean;
  selected: boolean;
}

export interface InstallStatus {
  state: AppState;
  install_dir: string;
  installed_build: number | null;
}

export interface DownloadProgress {
  downloaded_bytes: number;
  total_bytes: number;
  current_file: string;
  speed_bps: number;
  eta_secs: number;
}

export type AppState =
  | 'not-installed'
  | 'downloading'
  | 'paused'
  | 'ready'
  | 'update-available'
  | 'repairing'
  | 'steam-setup'
  | 'playing';

export interface SteamAccount {
  steam_id: string;
  persona_name: string;
  account_name: string;
}

export interface DonorStatus {
  installed: boolean;
  dll_ready: boolean;
  donor_name: string;
  donor_app_id: number;
}

export interface VersionInfo {
  id: string;
  name: string;
  install_dir: string;
  state: string;
  installed_build: number | null;
  is_active: boolean;
}
