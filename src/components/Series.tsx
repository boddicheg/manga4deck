import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { VolumeResponseInterface, fetchCacheSeries, fetchVolumes, fetchReadVolume, fetchUnReadVolume } from "../services/Api";
import { useLocalStorage } from "../services/useLocalStorage";

interface SeriesParams {
  [id: string]: string | undefined;
}

const Series: React.FC = () => {
  const { id } = useParams<SeriesParams>();
  const [volumes, setVolumes] = useState<Array<VolumeResponseInterface>>([]);
  const volumesSizeRef = useRef(volumes.length);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);
  const { setItem, getItem } = useLocalStorage("selected_volume_id");

  // const fetchInterval = 1000;

  const navigate = useNavigate();
  const navigateTo = (uri: string | null | undefined) => {
    if (uri) navigate(uri);
  };

  const cycleFocus = (direction: "next" | "prev") => {
    const nextIndex =
      direction === "next"
        ? currentIndexRef.current + 1 >= volumesSizeRef.current
          ? volumesSizeRef.current - 1
          : currentIndexRef.current + 1
        : currentIndexRef.current - 1 < 0
          ? 0
          : currentIndexRef.current - 1;

    setCurrentIndex(nextIndex);
    divRefs.current[nextIndex]?.focus();
  };

  const getSeries = async () => {
    try {
      const data = await fetchVolumes(id);
      setVolumes(data);
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("An unexpected error occurred");
      }
    } finally {
      setLoading(false);
    }
  };

  const enterDirectory = () => {
    const currentDiv = divRefs.current[currentIndexRef.current];
    const route = currentDiv?.getAttribute("data-route");
    const id = currentDiv?.getAttribute("data-volume-id");
    setItem(id + "")
    navigateTo(route);
  };

  const handleKey: (this: Window, ev: KeyboardEvent) => any = function (
    this: Window,
    event: KeyboardEvent
  ) {
    switch (event.key) {
      case "ArrowLeft":
        cycleFocus("prev");
        break;
      case "ArrowRight":
        cycleFocus("next");
        break;
      case "Enter":
        enterDirectory();
        break;
      case "Backspace":
        navigate(-1);
        break;
      case "F1":
        // F1 - mark volume as completed
        const readVolume = async (series_id: string | undefined, volume_id: string | undefined) => {
          await fetchReadVolume(series_id, volume_id);
        }
        const unreadVolume = async (series_id: string | undefined, volume_id: string | undefined) => {
          await fetchUnReadVolume(series_id, volume_id);
        }
        var current_volume = divRefs.current[currentIndexRef.current]?.getAttribute("data-key")
        var pages = divRefs.current[currentIndexRef.current]?.getAttribute("data-pages")
        var read = divRefs.current[currentIndexRef.current]?.getAttribute("data-read")
        if (pages == read)
          unreadVolume(id, current_volume + "")
        else
          readVolume(id, current_volume + "")
        break;
      case "F2":
        // F2 - cache whole serie
        const startCaching = async (id: string | undefined) => {
          await fetchCacheSeries(id);
        }
        startCaching(id)
        break;
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  useEffect(() => {
    getSeries();
    window.addEventListener("keydown", handleKey);
    // const intervalId = setInterval(getSeries, fetchInterval);
    return () => {
      window.removeEventListener("keydown", handleKey);
      // clearInterval(intervalId);
    };
  }, []);

  useEffect(() => {
    currentIndexRef.current = currentIndex;
    volumesSizeRef.current = volumes.length
  }, [currentIndex, volumes]);

  useEffect(() => {
    var selected_volume_id = getItem()
    if (selected_volume_id)
    {
      for (let index = 0; index < divRefs.current.length; index++) {
        const element = divRefs.current[index];
        if (element)
        {
          const id = element?.getAttribute("data-volume-id");
          if (id == selected_volume_id)
          {
            setCurrentIndex(index);
            divRefs.current[index]?.focus()
          }
        }
      }
    }
  }, [volumes]);

  if (loading) {
    return <>
      <div className="w-full h-screen min-w-[1280px] p-4 bg-zinc-900">
        <div className="text-xl text-white font-bold mb-6 text-center">
          Loading...
        </div>
      </div>
    </>
  }

  if (error) {
    return <>
      <div className="w-full h-screen min-w-[1280px] p-4 bg-zinc-900">
        <div className="text-xl text-white font-bold mb-6 text-center">
        Error: {error}
        </div>
      </div>
    </>
  }

  return (
    <>
      <div className="w-full h-full h-screen p-4 bg-zinc-900">
        <div className="text-xl text-white font-bold mb-1 text-center">
          Volumes
        </div>
        <div className="text-l text-white mb-2 text-center">
          F1/Y - mark volume as read, F2/X - start cache all volumes
        </div>

        <div className="grid grid-cols-8 gap-4">
          {volumes.map((volume, index) => (
            <div>
              <div
                key={volume.volume_id}
                data-key={volume.volume_id}
                data-volume-id={volume.volume_id}
                data-pages={volume.pages}
                data-read={volume.read}
                data-route={`/viewer/${volume.series_id}/${volume.volume_id}/${volume.chapter_id}/${volume.pages}/${volume.read}`}
                ref={(el) => (divRefs.current[index] = el)} // Assign ref
                tabIndex={-1} // Make it focusable but not in tab order
                className={`p-4 border rounded focus:outline-none ${currentIndex === index
                  ? "border-2 border-red-500"
                  : "border-gray-300"
                  }`}
                style={{
                  width: "150px",
                  height: "200px",
                  backgroundImage: `url(http://localhost:11337/api/volumes-cover/${volume.volume_id})`,
                  backgroundSize: "cover",
                  backgroundPosition: "center",
                }}
              >
              </div>
              <div
                className={`text-white truncate text-center min-w-[150px] pl-1 pr-1 ${volume.read === volume.pages
                  ? "bg-green-700"
                  : volume.cached ? "bg-yellow-500" : ""
                  }
                
                ${currentIndex === index
                    ? "text-red-500"
                    : "text-white"
                  }
                `}
              >
                {volume.title}
              </div>
              <div
                className={`text-white text-sm truncate text-center text-sm min-w-[150px] pl-1 pr-1 ${volume.read === volume.pages
                  ? "bg-green-700"
                  : volume.cached ? "bg-yellow-500" : ""
                  }
                ${currentIndex === index
                    ? "text-red-500"
                    : "text-white"
                  }
                `}
              >
                {volume.read}/{volume.pages}
              </div>
            </div>
          ))}
        </div>
      </div>
    </>
  );
};

export default Series;