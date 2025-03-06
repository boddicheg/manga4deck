export interface CommandStatusInterface {
  status: string;
}

export const fetchUpdateLibrary = async (): Promise<CommandStatusInterface> => {
  const response = await fetch("http://localhost:11337/api/update-lib");
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export const fetchClearCache = async (): Promise<CommandStatusInterface> => {
  const response = await fetch("http://localhost:11337/api/clear-cache");
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export interface ServerStatusInterface {
  status: boolean;
  ip: string;
  logged_as: string;
  cache: number;
}

export const fetchServerStatus = async (): Promise<ServerStatusInterface> => {
  const response = await fetch("http://localhost:11337/api/status");
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export interface LibraryResponseInterface {
  id: number;
  title: string;
}

export const fetchLibrary = async (): Promise<
  Array<LibraryResponseInterface>
> => {
  const response = await fetch("http://localhost:11337/api/library");
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export interface SeriesResponseInterface {
  id: number;
  pages: number;
  read: number;
  title: string;
  cached: boolean;
}

export const fetchSeries = async (
  id: string | undefined
): Promise<
  Array<SeriesResponseInterface>
> => {
  const response = await fetch("http://localhost:11337/api/series/" + id);
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export interface VolumeResponseInterface {
  chapter_id: number;
  pages: number;
  read: number;
  series_id: number;
  title: string;
  volume_id: number;
  cached: boolean;
}

export const fetchVolumes = async (
  id: string | undefined
): Promise<
  Array<VolumeResponseInterface>
> => {
  const response = await fetch("http://localhost:11337/api/volumes/" + id);
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export const fetchCacheSeries = async (
  id: string | undefined
): Promise<CommandStatusInterface> => {
  const response = await fetch("http://localhost:11337/api/cache/serie/" + id);
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export const fetchReadVolume = async (
  series_id: string | undefined,
  volume_id: string | undefined
): Promise<CommandStatusInterface> => {
  const response = await fetch("http://localhost:11337/api/read-volume/" + series_id + "/" + volume_id);
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export const fetchUnReadVolume = async (
  series_id: string | undefined,
  volume_id: string | undefined
): Promise<CommandStatusInterface> => {
  const response = await fetch("http://localhost:11337/api/unread-volume/" + series_id + "/" + volume_id);
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export interface ServerSettingsInterface {
  ip: string;
  username: string;
  offline_mode: boolean;
  logged_as: string;
  api_key?: string;
  has_password?: boolean;
}

export const fetchServerSettings = async (): Promise<ServerSettingsInterface> => {
  const response = await fetch("http://localhost:11337/api/server-settings");
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.json();
};

export interface UpdateServerSettingsInterface {
  ip?: string;
  username?: string;
  password?: string;
  api_key?: string;
}

export interface ServerSettingsResponseInterface {
  status: string;
  message: string;
  current_settings?: {
    ip: string;
    username: string;
    offline_mode: boolean;
    logged_as: string;
    url: string;
    api_key?: string;
    has_password?: boolean;
  };
}

export const updateServerSettings = async (
  settings: UpdateServerSettingsInterface
): Promise<ServerSettingsResponseInterface> => {
  console.log("Sending server settings update request:", { 
    ...settings, 
    password: settings.password ? "******" : undefined 
  });
  
  try {
    const response = await fetch("http://localhost:11337/api/server-settings", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(settings),
    });
    
    const data = await response.json();
    console.log("Server settings update response:", data);
    
    if (!response.ok) {
      throw new Error(data.message || "Failed to update server settings");
    }
    
    return data;
  } catch (error) {
    console.error("Error in updateServerSettings:", error);
    throw error;
  }
};