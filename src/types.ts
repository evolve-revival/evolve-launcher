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
  | 'repairing';
