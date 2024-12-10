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

export const fetchSeries= async (
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

export const fetchVolumes= async (
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