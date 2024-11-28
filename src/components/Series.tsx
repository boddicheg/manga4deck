import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { VolumeResponseInterface, fetchVolumes } from "../services/Api";

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
        default:
          console.log(`Key pressed: ${event.key}`);
      }
    };
  
    useEffect(() => {
      getSeries();
      window.addEventListener("keydown", handleKey);
      return () => {
        window.removeEventListener("keydown", handleKey); // Clean up
      };
    }, []);
  
    useEffect(() => {
      currentIndexRef.current = currentIndex;
      volumesSizeRef.current = volumes.length
    }, [currentIndex, volumes]);
  
    if (loading) {
      return <p>Loading...</p>;
    }
  
    if (error) {
      return <p>Error: {error}</p>;
    }

  return (
    <>
    <div className="w-full h-screen p-4 bg-zinc-900">
      <h1 className="text-3xl text-white font-bold mb-6 text-center">
        Volumes
      </h1>

      <div className="grid grid-cols-8 gap-4">
        {volumes.map((volume, index) => (
          <div>
            <div
              key={index}
              data-route={`/viewer/${volume.chapter_id}/${volume.pages}/${volume.read}`}
              ref={(el) => (divRefs.current[index] = el)} // Assign ref
              tabIndex={-1} // Make it focusable but not in tab order
              className={`p-4 border rounded focus:outline-none ${
                currentIndex === index
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
                className={`text-white truncate text-center min-w-[150px] pl-1 pr-1 ${
                  volume.read === volume.pages
                    ? "bg-green-700"
                    : "" 
                }
                
                ${
                  currentIndex === index
                  ? "text-red-500"
                  : "text-white"
                }
                `}
              >
                {volume.title}
              </div>
              <div
                className={`text-white text-sm truncate text-center text-sm min-w-[150px] pl-1 pr-1 ${
                  volume.read === volume.pages
                    ? "bg-green-700"
                    : ""
                }
                ${
                  currentIndex === index
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